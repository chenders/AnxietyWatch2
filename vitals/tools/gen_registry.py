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
import shutil
import subprocess
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


def optional_u32(value: int | None) -> str:
    """Render an optional bound as the Rust literal the accessor returns.

    Module scope on purpose: `validate()` checks the emitted arms against
    registry.toml, and if it spelled `None` differently from the emitter the
    check would pass on output it had not actually verified.
    """
    # Underscore-separated: clippy denies `unreadable_literal`, and a generated
    # file is linted like any other source. `2_592_000` also reads as a
    # duration where `2592000` reads as noise.
    return "None" if value is None else f"Some({value:_})"


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
    expected_cadence_s: int | None = None
    max_sample_arrival_s: int | None = None
    sample_silence_failure_s: int | None = None
    max_event_age_s: int | None = None
    max_link_silence_s: int | None = None
    on_demand: bool = False
    # A device no longer in service. Its id stays declared forever so historical
    # rows keep resolving to the thing that wrote them, but it is excluded from
    # liveness monitoring: a retired source is absent by declaration, not
    # broken, and a card reporting it Unknown forever trains the operator to
    # ignore cards.
    retired: bool = False
    conversions: tuple[tuple[str, float], ...] = ()

    @property
    def rust_variant(self) -> str:
        return _pascal(self.name)

    @property
    def cadence(self) -> str:
        """The Rust expression for this source's declared cadence.

        Exactly one of `expected_cadence_s` or `on_demand` is present —
        `_check_cadence` enforces it — so this needs no fallback.
        """
        if self.expected_cadence_s is not None:
            return f"Cadence::EverySeconds({self.expected_cadence_s})"
        return "Cadence::OnDemand"


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
                        expected_cadence_s=d.get("expected_cadence_s"),
                        max_sample_arrival_s=d.get("max_sample_arrival_s"),
                        sample_silence_failure_s=d.get("sample_silence_failure_s"),
                        max_event_age_s=d.get("max_event_age_s"),
                        max_link_silence_s=d.get("max_link_silence_s"),
                        on_demand=bool(d.get("on_demand", False)),
                        retired=bool(d.get("retired", False)),
                        conversions=tuple(
                            (type_name, float(factor))
                            for type_name, factor in d.get("conversion", {}).items()
                        ),
                    )
                    for name, d in raw.get("source", {}).items()
                ),
                key=lambda s: s.id,
            )
        )
        if not types or not sources:
            raise ValueError(f"{path} declares no [type.*] or [source.*] entries")
        _check_unique(types, sources)
        _check_cadence(sources)
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
        conversion_types = {type_name for type_name, _ in s.conversions}
        unknown_conversions = conversion_types - known
        if unknown_conversions:
            raise ValueError(
                f"source {s.name} converts unknown types: {sorted(unknown_conversions)}"
            )
        if not conversion_types <= set(s.provides):
            raise ValueError(f"source {s.name} converts a type it does not provide")


def _check_cadence(sources: tuple[Source, ...]) -> None:
    """Every source declares exactly one cadence.

    The liveness law (SPEC-admin-console §2.5) needs a declared expectation
    for every source; a source without one must be a build-time error, not a
    silently-`LIVE` card (§9 M0 criterion 2). A source declaring both is a
    contradiction — which one is the truth? And a non-positive interval makes
    the age-vs-cadence comparison degenerate, so it is rejected by name.
    """
    for s in sources:
        has_cadence = s.expected_cadence_s is not None
        if has_cadence and s.on_demand:
            raise ValueError(
                f"source {s.name} declares both expected_cadence_s "
                "and on_demand — pick exactly one"
            )
        if not has_cadence and not s.on_demand:
            raise ValueError(
                f"source {s.name} declares no cadence: expected_cadence_s "
                "or on_demand is required"
            )
        if has_cadence and s.expected_cadence_s <= 0:
            raise ValueError(
                f"source {s.name} has non-positive expected_cadence_s "
                f"{s.expected_cadence_s}"
            )
        _check_liveness_bounds(s)


# The widest bound the emitted `Option<u32>` can hold. A declaration above this
# is accepted by every other check and then fails to compile, which is a loud
# failure but the wrong one — the generator should reject the number it cannot
# represent.
_U32_MAX = 2**32 - 1

# No STREAM bound above this is a bound. A week of silence scoring as healthy is
# the defect this whole module exists to prevent, and `u32::MAX` passed every
# ordering and positivity check while restoring it exactly.
#
# Applies to the bounds that measure a continuous quantity: stream arrival,
# stream silence, and transport contact.
_STREAM_CEILING_S = 7 * 24 * 3600

# `max_event_age_s` gets its own, larger ceiling because it measures a different
# physical quantity: the age of a DISCRETE event, paced by a person rather than
# by a device. A home blood-pressure cuff is idle for weeks by design -- so a
# one-week
# ceiling rejects every honest value and forces the field to stay undeclared,
# which leaves the card asserting LIVE on month-old evidence.
#
# One ceiling for both was the conflation this module exists to prevent,
# committed in the guard itself. 90 days still bounds: it trips on any gap
# beyond a quarter, and a bound that can never trip is not one.
_EVENT_AGE_CEILING_S = 90 * 24 * 3600

# A failure bound must sit this many multiples above the staleness bound, or the
# grace period the pair exists to provide never applies. `(1799, 1800)` passed
# ordering and gave a one-second grace band.
_MIN_ESCALATION_MARGIN = 2


def _check_liveness_bounds(s: Source) -> None:
    """The liveness bounds are coherent, and belong to the right cadence kind.

    Each rule here was a defect that passed review: a `u32::MAX` pair that
    restored the original bug, a one-second grace band, an arrival bound below
    the source's own production rate, and a silence-failure bound on an
    on-demand source — which SPEC §2.5 forbids outright, because an SD-card
    import or a blood-pressure cuff that has not been used is not failing.
    """
    for name, value in (
        ("max_sample_arrival_s", s.max_sample_arrival_s),
        ("sample_silence_failure_s", s.sample_silence_failure_s),
        ("max_event_age_s", s.max_event_age_s),
        ("max_link_silence_s", s.max_link_silence_s),
    ):
        if value is None:
            continue
        if value <= 0:
            raise ValueError(f"source {s.name} has non-positive {name} {value}")
        if value > _U32_MAX:
            raise ValueError(
                f"source {s.name} has {name} {value} above u32::MAX "
                f"{_U32_MAX}; the generated accessor cannot represent it"
            )
        ceiling = _EVENT_AGE_CEILING_S if name == "max_event_age_s" else _STREAM_CEILING_S
        if value > ceiling:
            raise ValueError(
                f"source {s.name} has {name} {value}s above the ceiling "
                f"{ceiling}s ({ceiling // 86400} days); a bound that long is "
                "not a bound — silence past it would score as healthy, which "
                "is the defect these fields exist to stop"
            )

    # §2.5: an on-demand source is never time-degraded *to failing*. Rejected
    # here by name as well as exempted in the evaluator, because the evaluator
    # is the wrong place to learn that a declaration was meaningless.
    if s.on_demand and s.sample_silence_failure_s is not None:
        raise ValueError(
            f"source {s.name} is on_demand and declares "
            f"sample_silence_failure_s {s.sample_silence_failure_s}; SPEC §2.5 "
            "says on-demand sources are never time-degraded to failing. Use "
            "max_link_silence_s if the transport itself can be proven dead."
        )
    # The mirror: event age bounds a discrete event, which is what on-demand
    # sources have. An interval source's staleness is max_sample_arrival_s.
    if not s.on_demand and s.max_event_age_s is not None:
        raise ValueError(
            f"source {s.name} declares expected_cadence_s and "
            f"max_event_age_s {s.max_event_age_s}; event age is the on-demand "
            "quantity — an interval source uses max_sample_arrival_s"
        )

    if s.max_sample_arrival_s is not None:
        # An arrival bound below the production rate can never be satisfied:
        # the source would be stale between every pair of samples. This is the
        # check that catches a sample *rate* being pasted in as an arrival
        # bound, which is the original defect in its purest form.
        if s.expected_cadence_s is not None and s.max_sample_arrival_s < s.expected_cadence_s:
            raise ValueError(
                f"source {s.name} has max_sample_arrival_s "
                f"{s.max_sample_arrival_s} below expected_cadence_s "
                f"{s.expected_cadence_s}; a source cannot arrive faster than "
                "it produces, so this bound can never be met"
            )
        if s.sample_silence_failure_s is not None:
            floor = _MIN_ESCALATION_MARGIN * s.max_sample_arrival_s
            if s.sample_silence_failure_s < floor:
                raise ValueError(
                    f"source {s.name} has sample_silence_failure_s "
                    f"{s.sample_silence_failure_s} below {floor} "
                    f"({_MIN_ESCALATION_MARGIN}x max_sample_arrival_s "
                    f"{s.max_sample_arrival_s}); the bounds must leave a real "
                    "grace period in which the state is UNKNOWN before it "
                    "escalates, or the pair does nothing"
                )


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
    def rust_float(value: float) -> str:
        if value.is_integer():
            return f"{int(value):_}.0"
        return repr(value)

    type_ranges = "\n".join(
        f"            Self::{t.rust_variant} => ({rust_float(t.range_lo)}, {rust_float(t.range_hi)}),"
        for t in reg.types)
    type_units = "\n".join(
        f'            Self::{t.rust_variant} => "{t.unit}",' for t in reg.types)
    type_from = "\n".join(
        f"            {t.id} => Some(Self::{t.rust_variant})," for t in reg.types)
    source_from = "\n".join(
        f"            {s.id} => Some(Self::{s.rust_variant})," for s in reg.sources)
    cadences = "\n".join(
        f"            Self::{s.rust_variant} => {s.cadence}," for s in reg.sources)

    # One arm per variant, like the cadences above, so adding a source without
    # considering its sample-arrival bounds cannot compile to a default.
    arrival_bounds = "\n".join(
        f"            Self::{s.rust_variant} => {optional_u32(s.max_sample_arrival_s)},"
        for s in reg.sources
    )
    retired_arms = "\n".join(
        f"            Self::{s.rust_variant} => {str(s.retired).lower()},"
        for s in reg.sources
    )
    event_age_bounds = "\n".join(
        f"            Self::{s.rust_variant} => {optional_u32(s.max_event_age_s)},"
        for s in reg.sources
    )
    link_bounds = "\n".join(
        f"            Self::{s.rust_variant} => {optional_u32(s.max_link_silence_s)},"
        for s in reg.sources
    )
    silence_bounds = "\n".join(
        f"            Self::{s.rust_variant} => {optional_u32(s.sample_silence_failure_s)},"
        for s in reg.sources
    )
    conversions = "\n".join(
        f"            (Self::{s.rust_variant}, VitalsType::{_pascal(type_name)}) => {rust_float(factor)},"
        for s in reg.sources
        for type_name, factor in s.conversions
    )
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

/// How often a source is expected to deliver, declared in `registry.toml`.
/// The liveness law (SPEC-admin-console §2.5) compares a source's age against
/// this; a source without a declaration is a build-time error, never a
/// silently-`LIVE` card (§9 M0 criterion 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cadence {{
    /// The source is expected to attempt at least once per this many seconds;
    /// silence longer than that degrades it to `UNKNOWN`/`FAILING`, never
    /// `LIVE`. The upper bound when the true cadence is variable.
    EverySeconds(u32),
    /// Event-driven — a manual upload, an SD card. State is the outcome and
    /// age of the last event, never time-degraded.
    OnDemand,
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

    /// The declared cadence from `registry.toml`, used by the liveness
    /// evaluator. Every source has one — the generator rejects a source
    /// without it, and arms are one-per-variant on purpose so a new source
    /// without a cadence cannot compile.
    #[must_use]
    // Arms are one-per-variant on purpose: several sources share a cadence,
    // and collapsing them would make it non-obvious which source maps where.
    #[allow(clippy::match_same_arms)]
    pub const fn expected_cadence(self) -> Cadence {{
        match self {{
{cadences}
        }}
    }}

    /// How stale the newest stored sample may be before liveness stops
    /// treating this source as healthy, from `registry.toml`.
    ///
    /// `None` means no honest bound exists and sample arrival contributes
    /// nothing — the spec's rule for legitimately variable cadence. This is a
    /// different quantity from `expected_cadence`, which is the sample rate
    /// and is also the H10 reconnect idle timeout; comparing a sample age
    /// against that rate is what made a ten-day outage indistinguishable from
    /// healthy operation.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn max_sample_arrival_s(self) -> Option<u32> {{
        match self {{
{arrival_bounds}
        }}
    }}

    /// Beyond this sample age, silence is positive evidence of failure.
    ///
    /// At least twice `max_sample_arrival_s` when present, and never declared
    /// on an on-demand source; the generator rejects both cases by name, so a
    /// source cannot escalate to failing before it has degraded to unknown and
    /// an unused device is never called broken.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn sample_silence_failure_s(self) -> Option<u32> {{
        match self {{
{silence_bounds}
        }}
    }}

    /// Whether this source is retired: declared only so historical rows keep
    /// resolving to the device that wrote them.
    ///
    /// A retired source is **absent by declaration**, not broken. It is excluded
    /// from liveness monitoring, because a card reporting `Unknown` forever for a
    /// device nobody owns any more teaches the operator to ignore cards — and the
    /// whole point of the three-state rule is that a card is worth reading.
    ///
    /// Its id is never reused. Reassigning id 4 from a pulse oximeter to a radar
    /// sensor left 52,071 `spo2` rows attributed to a device that cannot measure
    /// blood oxygen at all, because that needs optical pulse oximetry.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn is_retired(self) -> bool {{
        match self {{
{retired_arms}
        }}
    }}

    /// Beyond this age, an on-demand source's newest event stops certifying it.
    ///
    /// An on-demand quantity only: it bounds the age of a discrete event (a
    /// cuff reading, an SD-card import), not the arrival of a stream. The
    /// generator rejects it on a source that declares `expected_cadence_s`,
    /// because conflating the two is what compared a stream-arrival bound
    /// against an attempt timestamp.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn max_event_age_s(self) -> Option<u32> {{
        match self {{
{event_age_bounds}
        }}
    }}

    /// Beyond this age with no transport contact, the source is failing —
    /// whether or not it was due to produce anything.
    ///
    /// The only bound that escalates on a legitimately idle source, and the
    /// one that can actually catch a dead collector on a device that only
    /// produces data while in use. A CPAP stores nothing for the idle hours a
    /// day; it answers on the radio throughout.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn max_link_silence_s(self) -> Option<u32> {{
        match self {{
{link_bounds}
        }}
    }}

    /// Source-to-canonical conversion factor from `registry.toml`.
    #[must_use]
    pub const fn conversion(self, r#type: VitalsType) -> f64 {{
        match (self, r#type) {{
{conversions}
            _ => 1.0,
        }}
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


def _emit_source_reuse_guard(reg: Registry) -> list[str]:
    """Refuse to rename a source id that already has sample rows.

    The upserts below cannot tell a rename from a reuse: both are
    `ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name`. Source id 4 was
    reassigned from `wellue_ks60fwb` to `acconeer_xe125` while holding 156,201
    rows, and the only reason that was ever detectable is that reconciliation
    had not run since. The next run would have relabelled 52,071 `spo2` rows as
    coming from a radar sensor that cannot measure SpO2.

    This runs before any upsert and raises with the id, the stored name, the
    declared name and the row count. The row count is only computed for ids
    whose name actually differs, so the normal path costs one index lookup per
    source. Failing closed is correct; a `WHERE` clause on the `DO UPDATE` would
    fail closed *silently*, which is not.
    """
    values = ",\n        ".join(f"({s.id}, '{s.name}')" for s in reg.sources)
    return [
        "-- Ids are permanent. This is the guard, not the comment above it.",
        "DO $$",
        "DECLARE",
        "    conflict RECORD;",
        "    row_count BIGINT;",
        "BEGIN",
        "    IF to_regclass('samples') IS NULL THEN",
        "        RETURN;  -- fresh database: nothing to protect yet",
        "    END IF;",
        "    FOR conflict IN",
        "        SELECT stored.id, stored.name AS stored_name, declared.name AS declared_name",
        "        FROM vitals_source AS stored",
        "        JOIN (VALUES",
        f"        {values}",
        "        ) AS declared(id, name) ON declared.id = stored.id",
        "        WHERE stored.name <> declared.name",
        "    LOOP",
        "        SELECT count(*) INTO row_count",
        "        FROM samples WHERE source = conflict.id;",
        "        IF row_count > 0 THEN",
        "            RAISE EXCEPTION",
        "                'source id % holds % sample rows as ''%'' but registry.toml "
        "now declares ''%'': ids are permanent. Retire the old name (retired = true, "
        "same id) and give the new device the next free id.',",
        "                conflict.id, row_count, conflict.stored_name, "
        "conflict.declared_name;",
        "        END IF;",
        "    END LOOP;",
        "END $$;",
        "",
    ]


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
        *_emit_source_reuse_guard(reg),
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
                **({"conversion": dict(s.conversions)} if s.conversions else {}),
                **({"service": s.service} if s.service else {}),
                **({"indication": True} if s.indication else {}),
                "cadence": (
                    {"expected_cadence_s": s.expected_cadence_s}
                    if s.expected_cadence_s is not None
                    else {"on_demand": True}
                ),
                # The liveness bounds belong here too. While they existed only
                # in generated.rs, every non-Rust consumer still saw
                # `expected_cadence_s` as the sole staleness-relevant number —
                # which for the 1 Hz sources is a sample *rate*, and is exactly
                # the comparison that made a ten-day outage look healthy. A
                # second implementation of the bug in the phone client is still
                # the bug.
                **(
                    {"liveness": _liveness_json(s)}
                    if any(
                        bound is not None
                        for bound in (
                            s.max_sample_arrival_s,
                            s.sample_silence_failure_s,
                            s.max_event_age_s,
                            s.max_link_silence_s,
                        )
                    )
                    else {}
                ),
            }
            for s in reg.sources
        },
    }
    return json.dumps(doc, indent=2, sort_keys=False) + "\n"


def _liveness_json(s: Source) -> dict[str, int]:
    """Only the bounds this source actually declares.

    Absent means "not measured" and must stay absent rather than appear as a
    zero or a default: a consumer reading `0` would treat every sample as
    overdue, and one reading a default would invent a threshold nobody set.
    """
    declared = (
        ("max_sample_arrival_s", s.max_sample_arrival_s),
        ("sample_silence_failure_s", s.sample_silence_failure_s),
        ("max_event_age_s", s.max_event_age_s),
        ("max_link_silence_s", s.max_link_silence_s),
    )
    return {name: value for name, value in declared if value is not None}


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
        # The cadence arm is what the liveness law depends on; a generated.rs
        # missing one would compile the source without an expectation and the
        # card would render LIVE forever. The Rust match itself is exhaustive
        # (no wildcard), so this is the layer that catches a dropped arm.
        expected_arm = f"Self::{s.rust_variant} => {s.cadence},"
        if expected_arm not in rust:
            problems.append(
                f"generated.rs: no cadence arm for source {s.name}")
        # The liveness-bound arms are load-bearing for the same law, and were
        # initially left out of this check. If the bound f-strings regressed to
        # emitting `None` for every source, `--check` would pass -- it only
        # diffs the generator against itself -- and the AirSense would silently
        # return to the pre-fix behaviour with nothing failing. So the *value*
        # is checked here, against registry.toml.
        for arm_label, value in (
            ("max_sample_arrival_s", s.max_sample_arrival_s),
            ("sample_silence_failure_s", s.sample_silence_failure_s),
            ("max_event_age_s", s.max_event_age_s),
            ("max_link_silence_s", s.max_link_silence_s),
        ):
            arm = f"Self::{s.rust_variant} => {optional_u32(value)},"
            if arm not in rust:
                problems.append(
                    f"generated.rs: no {arm_label} arm for source {s.name} "
                    f"(expected {optional_u32(value)})"
                )
        retired_arm = f"Self::{s.rust_variant} => {str(s.retired).lower()},"
        if retired_arm not in rust:
            problems.append(f"generated.rs: no is_retired arm for source {s.name}")
        # registry.json is the phone client's only view of the registry. While
        # the bounds existed solely in generated.rs, every non-Rust consumer
        # still saw `expected_cadence_s` as the sole staleness number -- a
        # sample rate for the 1 Hz sources, which is the original defect.
        declared = _liveness_json(s)
        if declared and doc["source"][s.name].get("liveness") != declared:
            problems.append(
                f"registry.json: source {s.name} liveness bounds do not match "
                f"registry.toml (expected {declared})"
            )
        if not declared and "liveness" in doc["source"].get(s.name, {}):
            problems.append(
                f"registry.json: source {s.name} declares no liveness bounds "
                "but the artifact carries some — absent must stay absent "
                "rather than becoming a default nobody set"
            )

    if "pub enum Cadence" not in rust:
        problems.append("generated.rs: Cadence enum missing")
    for method in (
        "pub const fn expected_cadence(self) -> Cadence",
        "pub const fn max_sample_arrival_s(self) -> Option<u32>",
        "pub const fn sample_silence_failure_s(self) -> Option<u32>",
        "pub const fn max_event_age_s(self) -> Option<u32>",
        "pub const fn max_link_silence_s(self) -> Option<u32>",
        "pub const fn is_retired(self) -> bool",
    ):
        if method not in rust:
            problems.append(f"generated.rs: {method} missing")

    # The id-reuse guard must reach the artifact, or a reassignment silently
    # relabels historical rows. Source id 4 already lost its identity this way.
    for fragment in ("RAISE EXCEPTION", "ids are permanent"):
        if fragment not in sql:
            problems.append(
                f"registry.sql: id-reuse guard missing ({fragment!r} not found)"
            )

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


def _rustfmt(path: Path) -> None:
    """Format generated Rust in place, if rustfmt is available.

    `cargo fmt --check` is a CI gate, so unformatted generated output fails the
    build on every run and the fix is not obvious from the error. Emitting
    formatted source by hand is possible but fragile — one edit to a template
    and it drifts. Let the formatter be authoritative.
    """
    if shutil.which("rustfmt") is None:
        return
    subprocess.run(  # noqa: S603
        ["rustfmt", "--edition", "2024", str(path)],
        check=False, capture_output=True,
    )


def install(staged: Path) -> None:
    """Copy staged artifacts to their real homes, creating parents."""
    for name, dest in ARTIFACT_DESTS.items():
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes((staged / name).read_bytes())


def write_all(reg: Registry, out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "generated.rs").write_text(emit_rust(reg), encoding="utf-8")
    # Format the STAGED copy, before validate() and before --check compares it
    # against the committed file. Formatting only on install made --check
    # compare unformatted staging against a formatted destination and report
    # drift forever.
    _rustfmt(out_dir / "generated.rs")
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
