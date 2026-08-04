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

[source.ad_ua651ble]
id = 6
transport = "ble"
indication = true
provides = ["bp_systolic", "hr"]
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
