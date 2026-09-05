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


def identified(
    t: float,
    value: float,
    session_id: int,
    seq: int,
    source: int = 1,
    value_type: int = 1,
) -> dict[str, object]:
    return {
        "t": t,
        "v": value,
        "source": source,
        "type": value_type,
        "session_id": session_id,
        "seq": seq,
    }


def rr(t: float, value: float, session_id: int, seq: int) -> dict[str, object]:
    return identified(t, value, session_id, seq, source=1, value_type=5)


def spo2(t: float, value: float, session_id: int, seq: int) -> dict[str, object]:
    return identified(t, value, session_id, seq, source=0, value_type=2)


def pulse(t: float, value: float, session_id: int, seq: int) -> dict[str, object]:
    return identified(t, value, session_id, seq, source=0, value_type=1)


def test_history_merge_uses_durable_identity_and_preserves_equal_readings(
    publisher: ModuleType,
) -> None:
    state = publisher.HistoryState()
    assert state.merge(
        "hr",
        [
            identified(2, 62, 7, 2),
            identified(1, 61, 7, 1),
            identified(2, 62, 8, 2),
            identified(math.nan, 63, 7, 3),
            identified(3, math.inf, 7, 4),
            {"t": 4, "v": 64},
        ],
    )
    assert state.hr == [
        identified(1.0, 61.0, 7, 1),
        identified(2.0, 62.0, 7, 2),
        identified(2.0, 62.0, 8, 2),
    ]
    assert not state.merge("hr", [identified(2, 999, 7, 2)])
    assert state.merge("hr", [identified(2, 999, 7, 5)])


def test_unseen_payload_does_not_republish_history_points(
    publisher: ModuleType,
) -> None:
    history = publisher.HistoryState()
    history.merge_payload(
        {
            "hr": [identified(10, 70, 1, 10)],
            "rr": [rr(10, 0.850, 1, 10)],
        }
    )
    unseen = history.unseen_payload(
        {
            "hr": [identified(10, 999, 1, 10), identified(11, 71, 1, 11)],
            "rr": [rr(10, 999, 1, 10), rr(11, 0.840, 1, 11)],
        }
    )
    assert unseen == {
        "hr": [identified(11.0, 71.0, 1, 11)],
        "rr": [rr(11.0, 0.840, 1, 11)],
    }
    assert history.latest_times() == (10.0, 10.0), "inspection must not mutate history"


def test_emay_history_has_separate_metrics_and_incremental_cursor_times(
    publisher: ModuleType,
) -> None:
    history = publisher.HistoryState()
    history.merge_payload(
        {
            "spo2": [spo2(20, 96, 3, 20), spo2(21, 97, 3, 21)],
            "pulse": [pulse(20, 68, 3, 20)],
        }
    )
    assert history.emay_payload() == {
        "spo2": [spo2(20.0, 96.0, 3, 20), spo2(21.0, 97.0, 3, 21)],
        "pulse": [pulse(20.0, 68.0, 3, 20)],
    }
    assert history.latest_emay_times() == (21.0, 20.0)
    assert history.unseen_payload(
        {
            "spo2": [spo2(21, 999, 3, 21), spo2(22, 98, 3, 22)],
            "pulse": [pulse(21, 69, 3, 21)],
        },
        ("spo2", "pulse"),
    ) == {
        "spo2": [spo2(22.0, 98.0, 3, 22)],
        "pulse": [pulse(21.0, 69.0, 3, 21)],
    }


def test_query_status_keeps_durable_delta_and_live_overlay_separate(
    publisher: ModuleType, monkeypatch: pytest.MonkeyPatch
) -> None:
    durable = {
        "schema": 2,
        "units": {"hr": "bpm", "rr": "seconds"},
        "h10": {"sample_at": 9, "hr": 60},
        "emay": None,
        "h10_delta": {
            "hr": [identified(10, 70, 1, 10)],
            "rr": [rr(10, 0.850, 1, 10)],
        },
        "emay_delta": {"spo2": [], "pulse": []},
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
    monkeypatch.setattr(publisher, "read_live_emay_status", lambda: None)
    monkeypatch.setattr(
        publisher,
        "read_pipeline_health",
        lambda: {"schema": 1, "state": "unknown", "stages": {}},
    )
    monkeypatch.setattr(
        publisher,
        "read_live_history",
        lambda _cutoff: pytest.fail("status polling must not reread full live history"),
    )

    result = publisher.query_status(10, 10)
    assert result["h10"] == live
    assert result["h10_durable"]["sample_at"] == 9
    assert result["h10_live"] == live
    assert result["h10_delta"] == {
        "hr": [identified(10.0, 70.0, 1, 10)],
        "rr": [rr(10.0, 0.850, 1, 10)],
    }
    assert result["pending_live"] == {"h10": live, "emay": None}
    assert result["emay_delta"] == {"spo2": [], "pulse": []}


def test_history_reports_source_sequence_gaps_without_filling_values(
    publisher: ModuleType,
) -> None:
    history = publisher.HistoryState()
    history.merge("hr", [identified(1, 70, 4, 1), identified(3, 70, 4, 3)])
    assert history.gaps() == [
        {
            "source": "hr",
            "kind": "sequence_gap",
            "session_id": 4,
            "from_seq": 1,
            "to_seq": 3,
            "missing": 1,
            "reason": "source sequence gap; values were not interpolated",
            "source_id": 1,
            "type_id": 1,
        }
    ]
    assert all(point["seq"] != 2 for point in history.hr)


def test_history_prune_keeps_the_configured_window_and_identity(
    publisher: ModuleType,
) -> None:
    history = publisher.HistoryState()
    history.merge("hr", [identified(9, 69, 4, 9), identified(10, 70, 4, 10)])
    history.prune(10)
    assert history.hr == [identified(10.0, 70.0, 4, 10)]
    assert not history.unseen_payload({"hr": [identified(10, 999, 4, 10)]})["hr"]


def test_emay_live_overlay_is_anonymous_and_stale_is_unknown(
    publisher: ModuleType, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    now = 1_700_000_000.0
    status_path = tmp_path / "emay.json"
    monkeypatch.setattr(publisher, "LIVE_EMAY", status_path)
    monkeypatch.setattr(publisher.time, "time", lambda: now)
    status_path.write_text(
        json.dumps(
            {
                "written_at": now - 1,
                "sample_at": now - 1,
                "spo2": 97,
                "pulse_bpm": 70,
                "finger": True,
                "link": "up",
                "frames": 12,
            }
        ),
        encoding="utf-8",
    )
    live = publisher.read_live_emay_status()
    assert live is not None
    assert live["identity"] == "anonymous_not_durable"
    assert live["durability"] == "status_only_not_committed"

    status_path.write_text(
        json.dumps({"written_at": now - 16, "sample_at": now - 16, "spo2": 97}),
        encoding="utf-8",
    )
    assert publisher.read_live_emay_status() is None


def test_pipeline_health_is_fresh_complete_evidence_or_unknown(
    publisher: ModuleType, monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    now = 1_700_000_000.0
    health_path = tmp_path / "health.json"
    monkeypatch.setattr(publisher, "PIPELINE_HEALTH", health_path)
    monkeypatch.setattr(publisher.time, "time", lambda: now)
    stages = {
        stage: {"count": index}
        for index, stage in enumerate(
            (
                "received",
                "spooled",
                "fsynced",
                "drained",
                "committed",
                "published",
                "displayed",
            )
        )
    }
    health_path.write_text(
        json.dumps({"written_at": now - 1, "state": "healthy", "stages": stages}),
        encoding="utf-8",
    )
    result = publisher.read_pipeline_health()
    assert result["state"] == "healthy"
    assert result["stages"] == stages

    health_path.write_text(
        json.dumps({"written_at": now - 121, "state": "healthy", "stages": stages}),
        encoding="utf-8",
    )
    stale = publisher.read_pipeline_health()
    assert stale["state"] == "unknown"
    assert "stale" in stale["reason"]

    health_path.write_text("{malformed", encoding="utf-8")
    malformed = publisher.read_pipeline_health()
    assert malformed["state"] == "unknown"
    assert all(value is None for value in malformed["stages"].values())


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
    assert "'source',source,'type',type" in publisher.STATUS_SQL
    assert "'source',source,'type',type" in publisher.HISTORY_SQL
    assert "stored=true" in publisher.STATUS_SQL
    assert "stored=true" in publisher.HISTORY_SQL
    assert "history_since" in publisher.HISTORY_SQL
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
