"""Tests for the panel's split status/history publisher."""

from __future__ import annotations

import importlib.util
import json
import math
import sys
from pathlib import Path
from types import ModuleType, SimpleNamespace

import pytest

MODULE_PATH = Path(__file__).resolve().parents[3] / "tools" / "panel_vitals_status.py"


def load_module() -> ModuleType:
    spec = importlib.util.spec_from_file_location("panel_vitals_status", MODULE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture
def publisher() -> ModuleType:
    return load_module()


def test_history_merge_is_incremental_sorted_and_finite(publisher: ModuleType) -> None:
    state = publisher.HistoryState()
    assert state.merge(
        "hr",
        [
            {"t": 2, "v": 62},
            {"t": 1, "v": 61},
            {"t": 2, "v": 62},
            {"t": math.nan, "v": 63},
            {"t": 3, "v": math.inf},
            {"bad": "point"},
        ],
    )
    assert state.hr == [{"t": 1.0, "v": 61.0}, {"t": 2.0, "v": 62.0}]
    assert not state.merge("hr", [{"t": 2, "v": 62}])


def test_unseen_payload_does_not_republish_history_points(publisher: ModuleType) -> None:
    history = publisher.HistoryState()
    history.merge_payload({"hr": [{"t": 10, "v": 70}], "rr": [{"t": 10, "v": 0.850}]})
    unseen = history.unseen_payload(
        {
            "hr": [{"t": 10, "v": 70}, {"t": 11, "v": 71}],
            "rr": [{"t": 10, "v": 0.850}, {"t": 11, "v": 0.840}],
        }
    )
    assert unseen == {
        "hr": [{"t": 11.0, "v": 71.0}],
        "rr": [{"t": 11.0, "v": 0.840}],
    }
    assert history.latest_times() == (10.0, 10.0), "inspection must not mutate history"


def test_emay_history_has_separate_metrics_and_incremental_cursor_times(
    publisher: ModuleType,
) -> None:
    history = publisher.HistoryState()
    history.merge_payload(
        {
            "spo2": [{"t": 20, "v": 96}, {"t": 21, "v": 97}],
            "pulse": [{"t": 20, "v": 68}],
        }
    )
    assert history.emay_payload() == {
        "spo2": [{"t": 20.0, "v": 96.0}, {"t": 21.0, "v": 97.0}],
        "pulse": [{"t": 20.0, "v": 68.0}],
    }
    assert history.latest_emay_times() == (21.0, 20.0)
    assert history.unseen_payload(
        {
            "spo2": [{"t": 21, "v": 97}, {"t": 22, "v": 98}],
            "pulse": [{"t": 21, "v": 69}],
        },
        ("spo2", "pulse"),
    ) == {
        "spo2": [{"t": 22.0, "v": 98.0}],
        "pulse": [{"t": 21.0, "v": 69.0}],
    }


def test_query_status_combines_only_recent_durable_and_live_points(
    publisher: ModuleType, monkeypatch: pytest.MonkeyPatch
) -> None:
    durable = {
        "schema": 2,
        "units": {"hr": "bpm", "rr": "seconds"},
        "h10": {"sample_at": 9, "hr": 60},
        "h10_delta": {
            "hr": [{"t": 10, "v": 70}],
            "rr": [{"t": 10, "v": 0.850}],
        },
    }
    live = {
        "sample_at": 11,
        "session_id": "session",
        "seq": 2,
        "hr": 71,
        "rr_count": 1,
        "rr_points": [{"t": 11, "v": 0.840}],
        "test_origin": "physical_device",
    }
    monkeypatch.setattr(publisher, "run_query", lambda *_args, **_kwargs: durable)
    monkeypatch.setattr(publisher, "read_live_status", lambda: live)
    monkeypatch.setattr(
        publisher,
        "read_live_history",
        lambda _cutoff: pytest.fail("one-second status polling reread full live history"),
    )

    result = publisher.query_status(10, 10)
    assert result["h10"] == live
    assert result["h10_delta"] == {
        "hr": [{"t": 10.0, "v": 70.0}, {"t": 11.0, "v": 71.0}],
        "rr": [{"t": 10.0, "v": 0.850}, {"t": 11.0, "v": 0.840}],
    }
    assert result["emay_delta"] == {"spo2": [], "pulse": []}


def test_status_query_is_cursor_based_and_full_history_is_separate(
    publisher: ModuleType,
) -> None:
    assert "hr_since" in publisher.STATUS_SQL
    assert "rr_since" in publisher.STATUS_SQL
    assert "spo2_since" in publisher.STATUS_SQL
    assert "pulse_since" in publisher.STATUS_SQL
    assert "h10_hr_history" not in publisher.STATUS_SQL
    assert "h10_hr_history" in publisher.HISTORY_SQL
    assert "h10_rr_history" in publisher.HISTORY_SQL
    assert "emay_spo2_history" in publisher.HISTORY_SQL
    assert "emay_pulse_history" in publisher.HISTORY_SQL
    assert "emay_delta" in publisher.STATUS_SQL
    assert "emay_history" in publisher.HISTORY_SQL
    sql = publisher.STATUS_SQL + publisher.HISTORY_SQL
    assert "value * 1000" not in sql
    assert "'rr','seconds'" in publisher.STATUS_SQL
    assert "'rr','seconds'" in publisher.HISTORY_SQL
    assert "'spo2','percent'" in publisher.STATUS_SQL
    assert "'spo2','percent'" in publisher.HISTORY_SQL


def test_atomic_documents_carry_timestamp_and_reject_nan(
    publisher: ModuleType, tmp_path: Path
) -> None:
    output = tmp_path / "status.json"
    publisher.publish_json(output, {"schema": 2, "h10_delta": {"hr": [], "rr": []}})
    document = json.loads(output.read_text(encoding="utf-8"))
    assert document["schema"] == 2
    assert document["written_at"] > 0
    assert not list(tmp_path.glob(".status.json.*"))

    with pytest.raises(ValueError, match="JSON compliant"):
        publisher.publish_json(output, {"bad": math.nan})


def test_psql_variables_use_the_long_option_form_that_psql_expands(
    publisher: ModuleType, monkeypatch: pytest.MonkeyPatch
) -> None:
    captured: list[str] = []

    def fake_run(command: list[str], **kwargs: object) -> SimpleNamespace:
        captured.extend(command)
        assert kwargs["check"] is False
        assert kwargs["input"] == "SELECT :'cursor'"
        return SimpleNamespace(returncode=0, stdout='{"ok":true}', stderr="")

    monkeypatch.setattr(publisher.subprocess, "run", fake_run)
    assert publisher.run_query("SELECT :'cursor'", {"cursor": 12.5}) == {"ok": True}
    assert "--set" not in captured
    assert "-c" not in captured


def test_psql_failure_names_the_database_diagnostic(
    publisher: ModuleType, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(
        publisher.subprocess,
        "run",
        lambda *_args, **_kwargs: SimpleNamespace(
            returncode=1, stdout="", stderr='ERROR: relation "samples" does not exist\n'
        ),
    )
    with pytest.raises(RuntimeError, match='relation "samples" does not exist'):
        publisher.run_query("SELECT 1")


def test_revision_is_strictly_monotonic(publisher: ModuleType) -> None:
    first = publisher.next_revision()
    assert publisher.next_revision(first) > first
