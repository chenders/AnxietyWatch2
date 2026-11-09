"""Tests for the registry generator.

Every test here corresponds to a defect that actually shipped or to an invariant
whose violation would be silent. Coverage of happy paths is incidental; the
point is the failure modes.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from gen_registry import (
    Registry,
    emit_json,
    emit_rust,
    emit_sql,
    main,
    sql_literal,
    validate,
    write_all,
)

MINIMAL = """
[meta]
version = 1

[type.hr]
id = 1
unit = "bpm"
range = [20.0, 250.0]
precision = 1.0

[type.bp_systolic]
id = 6
unit = "mmHg"
range = [50.0, 260.0]
precision = 1.0

[source.polar_h10]
id = 1
transport = "ble"
service = "0x180D"
provides = ["hr"]
expected_cadence_s = 60

[source.ad_ua651ble]
id = 6
transport = "ble"
indication = true
provides = ["bp_systolic", "hr"]
on_demand = true
"""


@pytest.fixture
def registry(tmp_path: Path) -> Registry:
    p = tmp_path / "registry.toml"
    p.write_text(MINIMAL, encoding="utf-8")
    return Registry.load(p)


# ── the bug that shipped ─────────────────────────────────────────────────────


def test_sql_contains_an_insert_for_every_entry(registry: Registry) -> None:
    """The original generator iterated top-level TOML keys looking for a 'type.'
    prefix. tomllib nests [type.hr] under raw['type']['hr'], so the loop matched
    nothing and registry.sql shipped with zero INSERTs — while --check stayed
    green, because the generator and the committed file agreed on emptiness.
    """
    sql = emit_sql(registry)
    assert sql.count("INSERT INTO vitals_type") == 2
    assert sql.count("INSERT INTO vitals_source") == 2
    for name in ("hr", "bp_systolic", "polar_h10", "ad_ua651ble"):
        assert f"'{name}'" in sql


def test_validate_rejects_output_missing_entries(registry: Registry, tmp_path: Path) -> None:
    """validate() must check output against the registry, not against itself —
    that is the whole reason it exists alongside --check.
    """
    out = tmp_path / "gen"
    write_all(registry, out)

    sql_path = out / "registry.sql"
    gutted = "\n".join(
        line
        for line in sql_path.read_text(encoding="utf-8").splitlines()
        if "'polar_h10'" not in line
    )
    sql_path.write_text(gutted, encoding="utf-8")

    with pytest.raises(ValueError, match="no INSERT for source polar_h10"):
        validate(registry, out)


# ── invariants ───────────────────────────────────────────────────────────────


def _executable_sql(sql: str) -> str:
    """Strip `--` comment lines. The generated DDL explains in a comment why it
    does not use a subquery CHECK, so a naive substring search matches the
    explanation rather than any statement.
    """
    return "\n".join(line for line in sql.splitlines() if not line.lstrip().startswith("--"))


def test_sql_contains_no_ddl(registry: Registry) -> None:
    """Sqitch owns structure. An earlier generator also emitted `CREATE TABLE
    samples` — with the pre-amendment-05 primary key — so two systems declared
    the same table and disagreed about its key. Whichever ran last won.
    """
    sql = emit_sql(registry).upper()
    for forbidden in ("CREATE TABLE", "CREATE EXTENSION", "CREATE_HYPERTABLE",
                      "ADD_COMPRESSION_POLICY", "CREATE MATERIALIZED VIEW",
                      "ALTER TABLE"):
        assert forbidden not in sql, f"registry.sql emits DDL: {forbidden}"
    assert "INSERT INTO VITALS_TYPE" in sql, "sanity: reference data is still emitted"


def test_sql_carries_no_superseded_schema(registry: Registry) -> None:
    """The inverse of a test that used to ASSERT this shape was present. It
    pinned `hlc_physical`, `hlc_logical` and `PRIMARY KEY (source, type,
    timestamp)` — the schema amendments 04 §8 and 05 §1 replaced — so the test
    suite was holding the superseded design in place.
    """
    sql = emit_sql(registry).lower()
    for gone in ("hlc_physical", "hlc_logical",
                 "primary key (source, type, timestamp)"):
        assert gone not in sql, f"registry.sql still carries {gone}"


def test_rust_enums_use_the_ids_the_briefs_depend_on() -> None:
    """T-D6 emits `VitalsSource::AdUa651ble` with `BpSystolic`/`BpDiastolic`/
    `BpMap`, and its Verify greps `source = 6 AND type = 6`. A renumbering here
    would leave that query silently matching nothing.
    """
    # The REAL registry, not the minimal fixture: these ids are the contract
    # T-D6's Verify query depends on.
    real = Registry.load(Path(__file__).resolve().parents[2] / "registry.toml")
    rust = emit_rust(real)
    for expected in ("AdUa651ble = 6,", "BpSystolic = 6,", "BpDiastolic = 7,",
                     "BpMap = 8,", "Hr = 1,", "PolarH10 = 1,"):
        assert expected in rust, f"generated.rs missing {expected}"
    # snake_case must not leak into variant names -- the TOML uses it, Rust
    # does not, and a mismatch here is a compile error at every use site.
    # Enum bodies only: `X = N,`. Excludes match arms (`_ => None,`) and the
    # ALL_* array entries.
    variant_lines = [ln.strip() for ln in rust.splitlines()
                     if re.fullmatch(r" {4}[A-Za-z][A-Za-z0-9_]* = -?\d+,", ln)]
    assert variant_lines, "sanity: no enum variants found to check"
    for ln in variant_lines:
        name = ln.split("=")[0].strip()
        assert "_" not in name, f"variant {name!r} is not PascalCase"


def test_rust_rejects_unknown_ids(registry: Registry) -> None:
    """`from_id` returning a variant for an unknown integer would let a wire or
    database value name a type that does not exist."""
    rust = emit_rust(registry)
    assert "_ => None," in rust
    assert "pub const fn from_id(id: i32) -> Option<Self>" in rust


def test_duplicate_ids_are_rejected(tmp_path: Path) -> None:
    """Ids are permanent. A duplicate silently remaps historical samples to a
    different meaning, which is unrecoverable once written.
    """
    p = tmp_path / "registry.toml"
    p.write_text(
        MINIMAL + '\n[type.spo2]\nid = 1\nunit = "percent"\n'
        "range = [50.0, 100.0]\nprecision = 0.1\n",
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="duplicate type id 1"):
        Registry.load(p)


def test_source_providing_unknown_type_is_rejected(tmp_path: Path) -> None:
    p = tmp_path / "registry.toml"
    p.write_text(
        MINIMAL + '\n[source.ghost]\nid = 9\ntransport = "ble"\nprovides = ["does_not_exist"]\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="unknown types"):
        Registry.load(p)


# ── cadence (SPEC §2.5, §9 M0 criterion 2) ───────────────────────────────────


def test_source_without_cadence_is_rejected(tmp_path: Path) -> None:
    """The liveness law needs a declared expectation for every source; a
    source without one must be a build-time error, not a silently-`LIVE` card
    (§9 M0 criterion 2)."""
    p = tmp_path / "registry.toml"
    p.write_text(
        MINIMAL + '\n[source.ghost]\nid = 9\ntransport = "ble"\nprovides = ["hr"]\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="source ghost declares no cadence"):
        Registry.load(p)


def test_source_with_both_cadences_is_rejected(tmp_path: Path) -> None:
    """Both fields is a contradiction — which one is the truth? Either answer
    silently changes the liveness evaluation."""
    p = tmp_path / "registry.toml"
    p.write_text(
        MINIMAL + '\n[source.ghost]\nid = 9\ntransport = "ble"\n'
        'provides = ["hr"]\nexpected_cadence_s = 60\non_demand = true\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="source ghost declares both"):
        Registry.load(p)


def test_source_with_zero_cadence_is_rejected(tmp_path: Path) -> None:
    """A zero interval makes the age-vs-cadence comparison degenerate: every
    source is always stale."""
    p = tmp_path / "registry.toml"
    p.write_text(
        MINIMAL + '\n[source.ghost]\nid = 9\ntransport = "ble"\n'
        'provides = ["hr"]\nexpected_cadence_s = 0\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="source ghost has non-positive expected_cadence_s 0"):
        Registry.load(p)


def test_source_with_negative_cadence_is_rejected(tmp_path: Path) -> None:
    p = tmp_path / "registry.toml"
    p.write_text(
        MINIMAL + '\n[source.ghost]\nid = 9\ntransport = "ble"\n'
        'provides = ["hr"]\nexpected_cadence_s = -5\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="source ghost has non-positive expected_cadence_s -5"):
        Registry.load(p)


def test_periodic_and_on_demand_sources_load(tmp_path: Path) -> None:
    """The passing case: exactly one of the two declarations, on both sides of
    the split, loads with the model fields set."""
    p = tmp_path / "registry.toml"
    p.write_text(MINIMAL, encoding="utf-8")
    reg = Registry.load(p)
    by_name = {s.name: s for s in reg.sources}
    assert by_name["polar_h10"].expected_cadence_s == 60
    assert by_name["polar_h10"].on_demand is False
    assert by_name["ad_ua651ble"].expected_cadence_s is None
    assert by_name["ad_ua651ble"].on_demand is True


def test_cadence_is_emitted_into_rust_and_json(registry: Registry) -> None:
    """The liveness evaluator reads the declared cadence out of generated.rs;
    the API layer reads registry.json. Both must carry it."""
    rust = emit_rust(registry)
    assert "pub enum Cadence" in rust
    assert "EverySeconds(u32)," in rust
    assert "OnDemand," in rust
    assert "Self::PolarH10 => Cadence::EverySeconds(60)," in rust
    assert "Self::AdUa651ble => Cadence::OnDemand," in rust
    assert "_ =>" not in rust.split("expected_cadence")[1].split("}")[0]

    doc = json.loads(emit_json(registry))
    assert doc["source"]["polar_h10"]["cadence"] == {"expected_cadence_s": 60}
    assert doc["source"]["ad_ua651ble"]["cadence"] == {"on_demand": True}


def test_validate_rejects_output_missing_cadence_arm(
    registry: Registry, tmp_path: Path
) -> None:
    """validate() must catch a generated.rs whose cadence match dropped a
    source — the Rust side is exhaustive by construction, so this is the layer
    that guards the liveness law."""
    out = tmp_path / "gen"
    write_all(registry, out)

    rust_path = out / "generated.rs"
    gutted = "\n".join(
        line
        for line in rust_path.read_text(encoding="utf-8").splitlines()
        if "Cadence::EverySeconds(60)," not in line
    )
    rust_path.write_text(gutted, encoding="utf-8")

    with pytest.raises(ValueError, match="no cadence arm for source polar_h10"):
        validate(registry, out)


def test_empty_registry_is_rejected(tmp_path: Path) -> None:
    p = tmp_path / "registry.toml"
    p.write_text("[meta]\nversion = 1\n", encoding="utf-8")
    with pytest.raises(ValueError, match="no \\[type"):
        Registry.load(p)


def test_sql_literal_escapes_quotes() -> None:
    """Generated DDL executes verbatim against the system of record."""
    assert sql_literal("o'brien") == "'o''brien'"
    assert sql_literal("plain") == "'plain'"


def test_no_swift_artifact(registry: Registry, tmp_path: Path) -> None:
    """The phone reads registry.json over HTTP; it does not link a generated
    enum. The Swift artifact existed only because of the same assumption that
    put SQLite on the Pi.
    """
    out = tmp_path / "gen"
    write_all(registry, out)
    assert not (out / "VitalsRegistry.swift").exists()
    assert not (out / "vitals_types.h").exists(), "the C header is superseded"
    assert {p.name for p in out.iterdir()} == {
        "generated.rs",
        "registry.sql",
        "registry.json",
    }


def test_json_round_trips(registry: Registry, tmp_path: Path) -> None:
    out = tmp_path / "gen"
    write_all(registry, out)
    doc = json.loads((out / "registry.json").read_text(encoding="utf-8"))
    assert doc["type"]["hr"]["id"] == 1
    assert doc["type"]["hr"]["range"] == [20.0, 250.0]
    assert doc["source"]["ad_ua651ble"]["indication"] is True
    assert doc["source"]["polar_h10"]["service"] == "0x180D"


# ── CLI ──────────────────────────────────────────────────────────────────────


def test_check_detects_drift(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """A hand-edited generated file must not survive CI. `--out` no longer
    exists: artifacts go to fixed homes (the crate source tree for the Rust
    one), so the test redirects those homes rather than a single directory.
    """
    import gen_registry as gr

    reg_path = tmp_path / "registry.toml"
    reg_path.write_text(MINIMAL, encoding="utf-8")
    dests = {
        "generated.rs": tmp_path / "crate" / "generated.rs",
        "registry.sql": tmp_path / "gen" / "registry.sql",
        "registry.json": tmp_path / "gen" / "registry.json",
    }
    monkeypatch.setattr(gr, "ARTIFACT_DESTS", dests)
    monkeypatch.setattr(gr, "ARTIFACTS", tuple(dests))

    assert main(["--registry", str(reg_path)]) == 0
    assert main(["--check", "--registry", str(reg_path)]) == 0

    dests["generated.rs"].write_text("// hand-edited\n", encoding="utf-8")
    assert main(["--check", "--registry", str(reg_path)]) == 1


def test_check_fails_when_artifacts_absent(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """--check must fail on missing artifacts, not report clean. A check that
    compares a generator against nothing is the vacuous-pass shape this repo has
    now produced five times.
    """
    import gen_registry as gr

    reg_path = tmp_path / "registry.toml"
    reg_path.write_text(MINIMAL, encoding="utf-8")
    dests = {name: tmp_path / "absent" / name
             for name in ("generated.rs", "registry.sql", "registry.json")}
    monkeypatch.setattr(gr, "ARTIFACT_DESTS", dests)
    monkeypatch.setattr(gr, "ARTIFACTS", tuple(dests))
    assert main(["--check", "--registry", str(reg_path)]) == 1


# ── liveness bounds ──────────────────────────────────────────────────────────
#
# Each case below was ACCEPTED by an earlier version of this generator and
# found during review. They are not hypothetical: the `u32::MAX` pair restores
# the original ten-day-outage behaviour exactly, and the on-demand silence
# bound makes an unused blood-pressure cuff report FAILING, which SPEC §2.5
# forbids outright.
@pytest.mark.parametrize(
    ("label", "fields", "expected_fragment"),
    [
        (
            "a u32::MAX pair passes ordering and positivity and means nothing",
            dict(
                expected_cadence_s=1,
                max_sample_arrival_s=2**32 - 2,
                sample_silence_failure_s=2**32 - 1,
            ),
            "above the ceiling",
        ),
        (
            "a one-second grace band defeats the pair's whole purpose",
            dict(
                expected_cadence_s=1,
                max_sample_arrival_s=1799,
                sample_silence_failure_s=1800,
            ),
            "grace period",
        ),
        (
            "on-demand sources are never time-degraded to failing",
            dict(on_demand=True, max_sample_arrival_s=300, sample_silence_failure_s=1800),
            "never time-degraded",
        ),
        (
            "an arrival bound below the production rate can never be met",
            dict(
                expected_cadence_s=21600,
                max_sample_arrival_s=1,
                sample_silence_failure_s=2,
            ),
            "cannot arrive faster than it produces",
        ),
        (
            "event age is the on-demand quantity, not an interval one",
            dict(expected_cadence_s=60, max_event_age_s=3600),
            "event age is the on-demand quantity",
        ),
        (
            "a bound the emitted Option<u32> cannot hold",
            dict(expected_cadence_s=1, max_sample_arrival_s=5_000_000_000),
            "above u32::MAX",
        ),
        (
            "a negative bound",
            dict(expected_cadence_s=1, max_link_silence_s=-1),
            "non-positive",
        ),
    ],
)
def test_incoherent_liveness_bounds_are_rejected_by_name(
    label: str, fields: dict[str, object], expected_fragment: str
) -> None:
    from gen_registry import Source, _check_liveness_bounds

    source = Source(name="probe", id=99, transport="ble", provides=(), **fields)
    with pytest.raises(ValueError, match=re.escape(expected_fragment)):
        _check_liveness_bounds(source)


def test_the_declared_airsense_bounds_are_accepted() -> None:
    """The guard must not be so strict it rejects the measured configuration.

    A gate that rejects everything is as useless as one that accepts
    everything, and only this direction proves the rules are satisfiable.
    """
    from gen_registry import Source, _check_liveness_bounds

    _check_liveness_bounds(
        Source(
            name="resmed_air11",
            id=5,
            transport="ble",
            provides=(),
            expected_cadence_s=1,
            max_sample_arrival_s=300,
            sample_silence_failure_s=1800,
        )
    )


def test_liveness_bounds_reach_registry_json(registry: Registry) -> None:
    """registry.json is the phone client's only view of the registry.

    While the bounds existed solely in generated.rs, every non-Rust consumer
    still saw `expected_cadence_s` as the sole staleness-relevant number --
    which for a 1 Hz source is a sample *rate*. A second implementation of the
    same defect in the phone client is still the defect.
    """
    doc = json.loads(emit_json(registry))
    for source in registry.sources:
        declared = {
            name: value
            for name, value in (
                ("max_sample_arrival_s", source.max_sample_arrival_s),
                ("sample_silence_failure_s", source.sample_silence_failure_s),
                ("max_event_age_s", source.max_event_age_s),
                ("max_link_silence_s", source.max_link_silence_s),
            )
            if value is not None
        }
        entry = doc["source"][source.name]
        if declared:
            assert entry["liveness"] == declared
        else:
            # Absent must stay absent. A zero would make every sample overdue
            # and a default would invent a threshold nobody set.
            assert "liveness" not in entry


def test_source_id_reuse_guard_is_emitted(registry: Registry) -> None:
    """The upserts cannot tell a rename from an id reuse.

    Source id 4 was reassigned from `wellue_ks60fwb` to `acconeer_xe125` while
    holding 156,201 rows, 52,071 of them `spo2` -- a measurement the new device
    physically cannot make. Reconciliation had simply not run yet, which is the
    only reason it stayed detectable.
    """
    sql = emit_sql(registry)
    assert "RAISE EXCEPTION" in sql
    assert "ids are permanent" in sql
    # The guard must precede every upsert, or it protects nothing.
    assert sql.index("RAISE EXCEPTION") < sql.index("INSERT INTO vitals_source")


def test_event_age_and_stream_bounds_have_different_ceilings() -> None:
    """One ceiling for both was the conflation this module exists to prevent.

    `max_event_age_s` measures a discrete, human-paced event; the others
    measure a continuous stream. A home blood-pressure cuff is idle for weeks
    by design -- 89 measured gaps have a maximum of 23.8 days -- so a one-week
    ceiling rejects every honest value and forces the field to stay undeclared,
    leaving the card asserting LIVE on month-old evidence.
    """
    from gen_registry import Source, _check_liveness_bounds

    thirty_days = 30 * 24 * 3600

    # Accepted for a discrete event...
    _check_liveness_bounds(
        Source(
            name="ad_ua651ble",
            id=6,
            transport="ble",
            provides=(),
            on_demand=True,
            max_event_age_s=thirty_days,
        )
    )

    # ...and still rejected for a continuous stream, where a month of silence
    # scoring as healthy is the original defect.
    with pytest.raises(ValueError, match=re.escape("above the ceiling")):
        _check_liveness_bounds(
            Source(
                name="probe",
                id=99,
                transport="ble",
                provides=(),
                expected_cadence_s=1,
                max_sample_arrival_s=thirty_days,
            )
        )

    # And the event ceiling is still a ceiling: a bound that can never trip is
    # not a bound.
    with pytest.raises(ValueError, match=re.escape("above the ceiling")):
        _check_liveness_bounds(
            Source(
                name="probe",
                id=99,
                transport="ble",
                provides=(),
                on_demand=True,
                max_event_age_s=400 * 24 * 3600,
            )
        )
