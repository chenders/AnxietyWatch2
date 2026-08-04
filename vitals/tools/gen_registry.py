#!/usr/bin/env python3
"""Generate Rust, SQL and JSON artifacts from vitals/registry.toml.

`registry.toml` is the single source of truth for every source, type, canonical
unit and valid range in the system. Everything downstream is generated from it,
so a source label like "polar_h10" exists in exactly one file and is an integer
everywhere else.

    gen_registry.py            regenerate every artifact in place
    gen_registry.py --check    regenerate to a temp dir and diff; nonzero on drift

Artifacts:
    crates/vitals-registry/src/generated.rs   #[repr(i32)] enums + range/unit
    vitals/generated/registry.sql            reference-data INSERTs only
    vitals/generated/registry.json           the parsed model, for the API layer

No Swift artifact: the phone is a thin client that reads registry.json over
HTTP. It does not link a generated enum.

Structure is NOT generated. `db/deploy/` (sqitch) owns every CREATE TABLE, the
hypertable, compression and the continuous aggregate. An earlier version emitted
`CREATE TABLE samples` here too — with the pre-amendment-05 primary key — so the
two systems disagreed about the schema and whichever ran last won.
"""

from __future__ import annotations

import argparse
import filecmp
import json
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parents[2]
REGISTRY = REPO_ROOT / "vitals" / "registry.toml"
GENERATED = REPO_ROOT / "vitals" / "generated"

# Each artifact's real home. generated.rs is a crate source file, not a data
# artifact, so it lives with the crate that compiles it -- Cargo has no notion
# of an out-of-tree module and `include!` would hide it from rustfmt and clippy.
ARTIFACT_DESTS = {
    "generated.rs": REPO_ROOT / "crates" / "vitals-registry" / "src" / "generated.rs",
    "registry.sql": GENERATED / "registry.sql",
    "registry.json": GENERATED / "registry.json",
}
ARTIFACTS = tuple(ARTIFACT_DESTS)
BANNER_RS = "// GENERATED FROM registry.toml — DO NOT EDIT"
BANNER_SQL = "-- GENERATED FROM registry.toml — DO NOT EDIT"


def _pascal(name: str) -> str:
    """snake_case -> PascalCase. `bp_systolic` -> `BpSystolic`,
    `ad_ua651ble` -> `AdUa651ble`, `polar_h10` -> `PolarH10`."""
    return "".join(part.capitalize() for part in name.split("_"))


# ── model ────────────────────────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class SampleType:
    name: str
    id: int
    unit: str
    range_lo: float
    range_hi: float
    precision: float

    @property
    def rust_variant(self) -> str:
        return _pascal(self.name)


@dataclass(frozen=True, slots=True)
class Source:
    name: str
    id: int
    transport: str
    provides: tuple[str, ...]
    service: str | None = None
    indication: bool = False

    @property
    def rust_variant(self) -> str:
        return _pascal(self.name)


@dataclass(frozen=True, slots=True)
class Registry:
    types: tuple[SampleType, ...]
    sources: tuple[Source, ...]

    @classmethod
    def load(cls, path: Path) -> Registry:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))

        # tomllib nests [type.hr] as raw["type"]["hr"]. Reading top-level keys
        # and filtering on a "type." prefix matches nothing — which is how an
        # earlier version silently emitted a lookup table with no rows.
        types = tuple(
            sorted(
                (
                    SampleType(
                        name=name,
                        id=d["id"],
                        unit=d["unit"],
                        range_lo=float(d["range"][0]),
                        range_hi=float(d["range"][1]),
                        precision=float(d["precision"]),
                    )
                    for name, d in raw.get("type", {}).items()
                ),
                key=lambda t: t.id,
            )
        )
        sources = tuple(
            sorted(
                (
                    Source(
                        name=name,
                        id=d["id"],
                        transport=d["transport"],
                        provides=tuple(d.get("provides", ())),
                        service=d.get("service"),
                        indication=bool(d.get("indication", False)),
                    )
                    for name, d in raw.get("source", {}).items()
                ),
                key=lambda s: s.id,
            )
        )
        if not types or not sources:
            raise ValueError(f"{path} declares no [type.*] or [source.*] entries")
        _check_unique(types, sources)
        return cls(types=types, sources=sources)


def _check_unique(types: tuple[SampleType, ...], sources: tuple[Source, ...]) -> None:
    """Ids are permanent and must never collide or be reused.

    A duplicate id silently remaps historical samples to a different meaning,
    which is unrecoverable once written.
    """
    for label, items in (("type", types), ("source", sources)):
        seen: dict[int, str] = {}
        for it in items:
            if it.id in seen:
                raise ValueError(f"duplicate {label} id {it.id}: {seen[it.id]} and {it.name}")
            seen[it.id] = it.name

    known = {t.name for t in types}
    for s in sources:
        unknown = set(s.provides) - known
        if unknown:
            raise ValueError(f"source {s.name} provides unknown types: {sorted(unknown)}")


# ── emitters ─────────────────────────────────────────────────────────────────


def sql_literal(value: str) -> str:
    """Quote a SQL string literal, escaping embedded quotes.

    Registry names are constrained today, but this DDL executes verbatim against
    the system of record. Concatenating without escaping is how a generator
    becomes an injection vector the first time a name contains an apostrophe.
    """
    escaped = value.replace("'", "''")
    return f"'{escaped}'"


def emit_rust(reg: Registry) -> str:
    """`#[repr(i32)]` enums, not integer constants.

    An `i32` that can hold any value is exactly what lets a source id and a type
    id be transposed at a call site without the compiler noticing — and both are
    small integers drawn from overlapping ranges, so the mistake is invisible in
    a debugger too. A real enum makes it a compile error.

    `no_std`, no allocation: this crate is a dependency of the decoders.
    """
    # A doc comment per variant: the workspace sets missing_docs, and the unit
    # and range are the two things a reader needs at the use site anyway.
    types = "\n".join(
        f"    /// `{t.name}` — `{t.unit}`, valid {t.range_lo}..{t.range_hi}\n"
        f"    {t.rust_variant} = {t.id},"
        for t in reg.types)
    sources = "\n".join(
        f"    /// `{s.name}` — `{s.transport}`"
        + (f", service `{s.service}`" if s.service else "")
        + (" (indication)" if s.indication else "")
        + f"\n    {s.rust_variant} = {s.id},"
        for s in reg.sources)
    type_ranges = "\n".join(
        f"            Self::{t.rust_variant} => ({t.range_lo!r}, {t.range_hi!r}),"
        for t in reg.types)
    type_units = "\n".join(
        f'            Self::{t.rust_variant} => "{t.unit}",' for t in reg.types)
    type_from = "\n".join(
        f"            {t.id} => Some(Self::{t.rust_variant})," for t in reg.types)
    source_from = "\n".join(
        f"            {s.id} => Some(Self::{s.rust_variant})," for s in reg.sources)
    return f"""{BANNER_RS}
//! Types and sources generated from `vitals/registry.toml`.
//!
//! Ids are permanent: never reuse, never renumber. A reused id silently
//! reinterprets every historical row that carries it.

/// A measurement type. The canonical unit and valid range come from the
/// registry, never from a literal at the call site.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VitalsType {{
{types}
}}

/// Where a sample came from. Provenance, and it must survive aggregation.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VitalsSource {{
{sources}
}}

impl VitalsType {{
    /// Valid range from `registry.toml`. Values outside it are **quarantined
    /// with a reason, never clamped** — a clamped reading is indistinguishable
    /// from a real one downstream.
    #[must_use]
    // Arms are one-per-variant on purpose. Several types share a unit or a
    // range, and collapsing them would make it non-obvious which type maps
    // where -- exactly the ambiguity this enum exists to remove.
    #[allow(clippy::match_same_arms)]
    pub const fn range(self) -> (f64, f64) {{
        match self {{
{type_ranges}
        }}
    }}

    /// Canonical unit. Conversion happens once, at the decoder, so the store
    /// holds exactly one unit per type.
    #[must_use]
    // Arms are one-per-variant on purpose. Several types share a unit or a
    // range, and collapsing them would make it non-obvious which type maps
    // where -- exactly the ambiguity this enum exists to remove.
    #[allow(clippy::match_same_arms)]
    pub const fn unit(self) -> &'static str {{
        match self {{
{type_units}
        }}
    }}

    /// Rejects an unknown id rather than transmuting it. The wire and the
    /// database both hand us plain integers.
    #[must_use]
    pub const fn from_id(id: i32) -> Option<Self> {{
        match id {{
{type_from}
            _ => None,
        }}
    }}

    /// The stable integer id. Permanent — see the module docs.
    #[must_use]
    pub const fn id(self) -> i32 {{
        self as i32
    }}
}}

impl VitalsSource {{
    /// Rejects an unknown id rather than transmuting it.
    #[must_use]
    pub const fn from_id(id: i32) -> Option<Self> {{
        match id {{
{source_from}
            _ => None,
        }}
    }}

    /// The stable integer id. Permanent — see the module docs.
    #[must_use]
    pub const fn id(self) -> i32 {{
        self as i32
    }}
}}

/// Every type, for exhaustive iteration in tests and admin views.
pub const ALL_TYPES: [VitalsType; {len(reg.types)}] = [
{chr(10).join(f"    VitalsType::{t.rust_variant}," for t in reg.types)}
];

/// Every source.
pub const ALL_SOURCES: [VitalsSource; {len(reg.sources)}] = [
{chr(10).join(f"    VitalsSource::{s.rust_variant}," for s in reg.sources)}
];
"""


def emit_sql(reg: Registry) -> str:
    """Reference data ONLY — `INSERT`s into the lookup tables.

    **Sqitch owns structure** (`db/deploy/`). An earlier version of this
    generator also emitted `CREATE TABLE samples` with `hlc_physical`,
    `hlc_logical` and `PRIMARY KEY (source, type, timestamp)` — the schema
    amendments 04 §8 and 05 §1 replaced — plus the hypertable, compression and
    continuous-aggregate DDL that `db/deploy/` now owns.

    Two systems creating the same table is how a migration diverges from the
    schema it is migrating, and here they disagreed on the primary key: sqitch's
    is `(source, type, session_id, seq)`. Whichever ran last would have won,
    silently.
    """
    out = [
        BANNER_SQL,
        "-- Reference data for the registry lookup tables.",
        "-- Structure lives in db/deploy/registry_tables.sql — sqitch owns it.",
        "-- Ids are permanent: ON CONFLICT UPDATE so a renamed unit or widened",
        "-- range propagates, but an id is never reused or renumbered.",
        "",
    ]
    for t in reg.types:
        out += [
            "INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)",
            f"VALUES ({t.id}, {sql_literal(t.name)}, {sql_literal(t.unit)}, "
            f"{t.range_lo}, {t.range_hi}, {t.precision})",
            "ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,",
            "    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,",
            "    precision = EXCLUDED.precision;",
        ]
    out.append("")
    for s in reg.sources:
        out += [
            "INSERT INTO vitals_source (id, name, transport)",
            f"VALUES ({s.id}, {sql_literal(s.name)}, {sql_literal(s.transport)})",
            "ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,",
            "    transport = EXCLUDED.transport;",
        ]
    return "\n".join(out) + "\n"


def emit_json(reg: Registry) -> str:
    doc = {
        "_generated_from": "registry.toml — DO NOT EDIT",
        "type": {
            t.name: {
                "id": t.id,
                "unit": t.unit,
                "range": [t.range_lo, t.range_hi],
                "precision": t.precision,
            }
            for t in reg.types
        },
        "source": {
            s.name: {
                "id": s.id,
                "transport": s.transport,
                "provides": list(s.provides),
                **({"service": s.service} if s.service else {}),
                **({"indication": True} if s.indication else {}),
            }
            for s in reg.sources
        },
    }
    return json.dumps(doc, indent=2, sort_keys=False) + "\n"


# ── validation ───────────────────────────────────────────────────────────────


def validate(reg: Registry, out_dir: Path) -> None:
    """Check generated output against the registry, not against itself.

    `--check` cannot catch a broken generator: it regenerates and diffs against
    the committed files, so a generator that silently emits nothing produces
    output identical to a committed file that also contains nothing, and CI
    stays green on an artifact that would reject every insert downstream. That
    is not hypothetical — it is exactly what the previous version did.

    Self-consistency is not correctness.
    """
    problems: list[str] = []
    sql = (out_dir / "registry.sql").read_text(encoding="utf-8")
    rust = (out_dir / "generated.rs").read_text(encoding="utf-8")
    doc = json.loads((out_dir / "registry.json").read_text(encoding="utf-8"))

    for t in reg.types:
        if sql_literal(t.name) not in sql:
            problems.append(f"registry.sql: no INSERT for type {t.name}")
        if f"{t.rust_variant} = {t.id}," not in rust:
            problems.append(f"generated.rs: no enum variant {t.rust_variant}")
    for s in reg.sources:
        if sql_literal(s.name) not in sql:
            problems.append(f"registry.sql: no INSERT for source {s.name}")
        if f"{s.rust_variant} = {s.id}," not in rust:
            problems.append(f"generated.rs: no enum variant {s.rust_variant}")

    # The inverse of the old check. Structure here would collide with sqitch,
    # and the two disagreed on the primary key.
    for kw in ("CREATE TABLE", "create_hypertable", "add_compression_policy",
               "CREATE MATERIALIZED VIEW", "hlc_physical"):
        if kw in sql:
            problems.append(f"registry.sql: contains DDL ({kw}) — sqitch owns structure")

    if len(doc.get("type", {})) != len(reg.types):
        problems.append("registry.json: type count does not match registry.toml")
    if len(doc.get("source", {})) != len(reg.sources):
        problems.append("registry.json: source count does not match registry.toml")

    if problems:
        raise ValueError("generated output is incomplete:\n  " + "\n  ".join(problems))


def install(staged: Path) -> None:
    """Copy staged artifacts to their real homes, creating parents."""
    for name, dest in ARTIFACT_DESTS.items():
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes((staged / name).read_bytes())


def write_all(reg: Registry, out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "generated.rs").write_text(emit_rust(reg), encoding="utf-8")
    (out_dir / "registry.sql").write_text(emit_sql(reg), encoding="utf-8")
    (out_dir / "registry.json").write_text(emit_json(reg), encoding="utf-8")
    validate(reg, out_dir)


# ── entry point ──────────────────────────────────────────────────────────────


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="regenerate to a temp dir and diff; nonzero exit on drift",
    )
    parser.add_argument("--registry", type=Path, default=REGISTRY)
    args = parser.parse_args(argv)

    try:
        reg = Registry.load(args.registry)
    except (OSError, ValueError, KeyError) as exc:
        print(f"registry: {exc}", file=sys.stderr)
        return 2

    if not args.check:
        with TemporaryDirectory() as tmp:
            try:
                write_all(reg, Path(tmp))
            except ValueError as exc:
                print(exc, file=sys.stderr)
                return 1
            install(Path(tmp))
        for name, dest in ARTIFACT_DESTS.items():
            try:
                shown = dest.relative_to(REPO_ROOT)
            except ValueError:      # redirected (tests), or outside the repo
                shown = dest
            print(f"  {name} -> {shown}")
        return 0

    with TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        try:
            write_all(reg, tmp_dir)
        except ValueError as exc:
            print(exc, file=sys.stderr)
            return 1
        drifted = [
            name
            for name, dest in ARTIFACT_DESTS.items()
            if not dest.is_file()
            or not filecmp.cmp(tmp_dir / name, dest, shallow=False)
        ]
        if drifted:
            print(
                "generated files are stale or hand-edited: " + ", ".join(drifted),
                file=sys.stderr,
            )
            print(f"run: {Path(__file__).name}", file=sys.stderr)
            return 1

    print("generated files are current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
