"""Tests for the registry generator.

Every test here corresponds to a defect that actually shipped or to an invariant
whose violation would be silent. Coverage of happy paths is incidental; the
point is the failure modes.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from gen_registry import (
    Registry,
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


def test_no_check_constraint_subqueries(registry: Registry) -> None:
    """PostgreSQL and SQLite both prohibit subqueries in CHECK constraints; such
    a table fails at CREATE time. The REFERENCES clauses already enforce it.
    """
    statements = _executable_sql(emit_sql(registry)).upper()
    assert "CHECK" in statements, "sanity: the DDL should still contain a CHECK"
    assert "IN (SELECT" not in statements


def test_samples_matches_the_spec_shape(registry: Registry) -> None:
    sql = emit_sql(registry)
    for col in ("hlc_physical", "hlc_logical", "node_id", "extra"):
        assert col in sql, f"samples is missing {col}"
    assert "PRIMARY KEY (source, type, timestamp)" in sql
    assert "TIMESTAMPTZ" in sql
    assert "create_hypertable" in sql


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
    assert {p.name for p in out.iterdir()} == {
        "vitals_types.h",
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


def test_check_detects_drift(tmp_path: Path) -> None:
    reg_path = tmp_path / "registry.toml"
    reg_path.write_text(MINIMAL, encoding="utf-8")
    out = tmp_path / "gen"

    assert main(["--registry", str(reg_path), "--out", str(out)]) == 0
    assert main(["--check", "--registry", str(reg_path), "--out", str(out)]) == 0

    (out / "vitals_types.h").write_text("/* hand-edited */\n", encoding="utf-8")
    assert main(["--check", "--registry", str(reg_path), "--out", str(out)]) == 1


def test_check_fails_when_artifacts_absent(tmp_path: Path) -> None:
    reg_path = tmp_path / "registry.toml"
    reg_path.write_text(MINIMAL, encoding="utf-8")
    assert main(["--check", "--registry", str(reg_path), "--out", str(tmp_path / "none")]) == 1
