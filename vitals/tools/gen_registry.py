#!/usr/bin/env python3
"""Generate C, SQL and JSON artifacts from vitals/registry.toml.

`registry.toml` is the single source of truth for every source, type, canonical
unit and valid range in the system. Everything downstream is generated from it,
so a source label like "polar_h10" exists in exactly one file and is an integer
everywhere else.

    gen_registry.py            regenerate vitals/generated/
    gen_registry.py --check    regenerate to a temp dir and diff; nonzero on drift

Artifacts:
    vitals_types.h   C enums + range/unit accessors
    registry.sql     PostgreSQL 16 + TimescaleDB DDL
    registry.json    the parsed model, for the API layer and any client

No Swift artifact: the phone is a thin client that reads registry.json over
HTTP. It does not link a generated enum.
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

ARTIFACTS = ("vitals_types.h", "registry.sql", "registry.json")
BANNER_C = "/* GENERATED FROM registry.toml — DO NOT EDIT */"
BANNER_SQL = "-- GENERATED FROM registry.toml — DO NOT EDIT"


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
    def c_enum(self) -> str:
        return f"VITALS_TYPE_{self.name.upper()}"


@dataclass(frozen=True, slots=True)
class Source:
    name: str
    id: int
    transport: str
    provides: tuple[str, ...]
    service: str | None = None
    indication: bool = False

    @property
    def c_enum(self) -> str:
        return f"VITALS_SOURCE_{self.name.upper()}"


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


def emit_c_header(reg: Registry) -> str:
    types = ",\n".join(f"    {t.c_enum} = {t.id}" for t in reg.types)
    sources = ",\n".join(f"    {s.c_enum} = {s.id}" for s in reg.sources)
    return f"""{BANNER_C}
#ifndef VITALS_TYPES_H
#define VITALS_TYPES_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {{
#endif

typedef enum {{
{types}
}} vitals_type_t;

typedef enum {{
{sources}
}} vitals_source_t;

#define VITALS_TYPE_COUNT   {len(reg.types)}
#define VITALS_SOURCE_COUNT {len(reg.sources)}

typedef struct {{
    double lo;
    double hi;
}} vitals_range_t;

/* Valid range for a type, from registry.toml. Returns false for an unknown id;
 * *out is untouched. Values outside the range are quarantined, never clamped. */
bool vitals_type_range(int type_id, vitals_range_t *out);

/* Canonical unit for a type. NULL for an unknown id. Conversion happens once,
 * at the decoder — the store holds exactly one unit per type. */
const char *vitals_type_unit(int type_id);

#ifdef __cplusplus
}}
#endif

#endif /* VITALS_TYPES_H */
"""


def emit_sql(reg: Registry) -> str:
    out: list[str] = [
        BANNER_SQL,
        "-- Target: PostgreSQL 16 + TimescaleDB. See amendment 01.",
        "",
        "CREATE EXTENSION IF NOT EXISTS timescaledb;",
        "",
        "CREATE TABLE IF NOT EXISTS vitals_type (",
        "    id        INTEGER PRIMARY KEY,",
        "    name      TEXT             NOT NULL UNIQUE,",
        "    unit      TEXT             NOT NULL,",
        "    range_lo  DOUBLE PRECISION NOT NULL,",
        "    range_hi  DOUBLE PRECISION NOT NULL,",
        "    precision DOUBLE PRECISION NOT NULL,",
        "    CHECK (range_lo < range_hi)",
        ");",
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

    out += [
        "",
        "CREATE TABLE IF NOT EXISTS vitals_source (",
        "    id        INTEGER PRIMARY KEY,",
        "    name      TEXT    NOT NULL UNIQUE,",
        "    transport TEXT    NOT NULL",
        ");",
        "",
    ]
    for s in reg.sources:
        out += [
            "INSERT INTO vitals_source (id, name, transport)",
            f"VALUES ({s.id}, {sql_literal(s.name)}, {sql_literal(s.transport)})",
            "ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,",
            "    transport = EXCLUDED.transport;",
        ]

    out += [
        "",
        "-- Append-only. No UPDATE, no DELETE; removal is a tombstone row.",
        "-- TIMESTAMPTZ rather than epoch REAL: hypertable partitioning and every",
        "-- hyperfunction expect a real timestamp type. This does not weaken the",
        "-- UTC-only rule — TIMESTAMPTZ stores an absolute instant and the cluster",
        "-- timezone is pinned to UTC.",
        "CREATE TABLE IF NOT EXISTS samples (",
        "    source       INTEGER          NOT NULL REFERENCES vitals_source(id),",
        "    type         INTEGER          NOT NULL REFERENCES vitals_type(id),",
        "    timestamp    TIMESTAMPTZ      NOT NULL,",
        "    value        DOUBLE PRECISION NOT NULL,",
        "    extra        BYTEA,",
        "    hlc_physical BIGINT           NOT NULL,",
        "    hlc_logical  INTEGER          NOT NULL,",
        "    node_id      BYTEA            NOT NULL,",
        "    PRIMARY KEY (source, type, timestamp)",
        ");",
        "",
        "-- The REFERENCES clauses above enforce referential integrity. A",
        "-- CHECK (... IN (SELECT ...)) would be redundant and illegal: PostgreSQL",
        "-- and SQLite both prohibit subqueries in CHECK, so such a table fails at",
        "-- CREATE time rather than at insert time.",
        "",
        "SELECT create_hypertable('samples', by_range('timestamp', INTERVAL '1 day'),",
        "                         if_not_exists => TRUE);",
        "",
        "ALTER TABLE samples SET (",
        "    timescaledb.compress,",
        "    timescaledb.compress_segmentby = 'source, type',",
        "    timescaledb.compress_orderby   = 'timestamp DESC'",
        ");",
        "SELECT add_compression_policy('samples', INTERVAL '7 days', if_not_exists => TRUE);",
        "",
        "-- min/max are carried because an average hides a desaturation nadir, and",
        "-- the nadir is the clinically interesting number.",
        "CREATE MATERIALIZED VIEW IF NOT EXISTS samples_1min",
        "WITH (timescaledb.continuous) AS",
        "SELECT source, type,",
        "       time_bucket(INTERVAL '1 minute', timestamp) AS bucket,",
        "       avg(value) AS value, count(*) AS sample_count,",
        "       min(value) AS min_value, max(value) AS max_value",
        "FROM samples",
        "GROUP BY source, type, bucket",
        "WITH NO DATA;",
        "",
        "SELECT add_continuous_aggregate_policy('samples_1min',",
        "    start_offset      => INTERVAL '3 hours',",
        "    end_offset        => INTERVAL '1 minute',",
        "    schedule_interval => INTERVAL '1 minute',",
        "    if_not_exists     => TRUE);",
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
    header = (out_dir / "vitals_types.h").read_text(encoding="utf-8")
    doc = json.loads((out_dir / "registry.json").read_text(encoding="utf-8"))

    for t in reg.types:
        if sql_literal(t.name) not in sql:
            problems.append(f"registry.sql: no INSERT for type {t.name}")
        if f"{t.c_enum} = {t.id}" not in header:
            problems.append(f"vitals_types.h: no enum member {t.c_enum}")
    for s in reg.sources:
        if sql_literal(s.name) not in sql:
            problems.append(f"registry.sql: no INSERT for source {s.name}")
        if f"{s.c_enum} = {s.id}" not in header:
            problems.append(f"vitals_types.h: no enum member {s.c_enum}")

    for kw in ("create_hypertable", "samples_1min", "add_compression_policy"):
        if kw not in sql:
            problems.append(f"registry.sql: missing {kw}")

    if len(doc.get("type", {})) != len(reg.types):
        problems.append("registry.json: type count does not match registry.toml")
    if len(doc.get("source", {})) != len(reg.sources):
        problems.append("registry.json: source count does not match registry.toml")

    if problems:
        raise ValueError("generated output is incomplete:\n  " + "\n  ".join(problems))


def write_all(reg: Registry, out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "vitals_types.h").write_text(emit_c_header(reg), encoding="utf-8")
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
    parser.add_argument("--out", type=Path, default=GENERATED)
    args = parser.parse_args(argv)

    try:
        reg = Registry.load(args.registry)
    except (OSError, ValueError, KeyError) as exc:
        print(f"registry: {exc}", file=sys.stderr)
        return 2

    if not args.check:
        try:
            write_all(reg, args.out)
        except ValueError as exc:
            print(exc, file=sys.stderr)
            return 1
        print(f"generated {len(ARTIFACTS)} artifacts in {args.out}")
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
            for name in ARTIFACTS
            if not (args.out / name).is_file()
            or not filecmp.cmp(tmp_dir / name, args.out / name, shallow=False)
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
