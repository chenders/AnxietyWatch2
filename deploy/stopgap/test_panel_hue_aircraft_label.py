#!/usr/bin/env python3
"""Tests for Hue-button aircraft ground-truth capture."""

import importlib.machinery
import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest import mock


SCRIPT = pathlib.Path(__file__).with_name("panel-hue-aircraft-label")
loader = importlib.machinery.SourceFileLoader("panel_hue_aircraft_label", str(SCRIPT))
spec = importlib.util.spec_from_loader(loader.name, loader)
label = importlib.util.module_from_spec(spec)
loader.exec_module(label)


class EventParsing(unittest.TestCase):
    def test_only_initial_press_for_configured_button_is_a_press(self):
        payload = json.dumps([{
            "type": "update",
            "creationtime": "2026-08-17T01:00:00Z",
            "data": [
                {"id": label.BUTTON_ID, "type": "button",
                 "button": {"button_report": {"event": "initial_press", "updated": "press-1"}}},
                {"id": label.BUTTON_ID, "type": "button",
                 "button": {"button_report": {"event": "short_release", "updated": "release"}}},
                {"id": "some-other-button", "type": "button",
                 "button": {"button_report": {"event": "initial_press", "updated": "wrong"}}},
            ],
        }])
        self.assertEqual(label.events_from_payload(payload), ["press-1"])

    def test_invalid_event_data_is_ignored(self):
        self.assertEqual(label.events_from_payload("not json"), [])
        self.assertEqual(label.events_from_payload("{}"), [])


class Health(unittest.TestCase):
    def test_health_transition_preserves_last_label(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = pathlib.Path(tmp) / "state.json"
            state.write_text(json.dumps({"last_label": "human_heard"}))
            with mock.patch.object(label, "STATE", state), mock.patch.object(label.time, "time", return_value=1000.0):
                label.write_health("disconnected", "bridge timeout")
                first = json.loads(state.read_text())
                label.write_health("connected")
                second = json.loads(state.read_text())
            self.assertEqual(first["last_label"], "human_heard")
            self.assertEqual(first["stream_error"], "bridge timeout")
            self.assertEqual(second["stream_state"], "connected")
            self.assertNotIn("stream_error", second)


class DurableLabels(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = pathlib.Path(self.tmp.name)
        self.paths = {
            "LABELS": root / "labels.jsonl",
            "STATE": root / "state.json",
            "PASS_LOG": root / "passes.jsonl",
            "STATUS": root / "status.json",
        }
        self.patchers = [mock.patch.object(label, name, value) for name, value in self.paths.items()]
        for patcher in self.patchers:
            patcher.start()
        self.addCleanup(self._cleanup)
        self.paths["STATUS"].write_text(json.dumps({
            "written_at": 999.0,
            "adsb_planes": [{"hex": "ABC123", "alt_ft": 2400}],
            "adsb_predictions": [{"hex": "ABC123", "prediction": {"eta_seconds": 30}}],
        }))
        self.paths["PASS_LOG"].write_text(json.dumps({
            "at": 990.0, "identity": {"hex": "ABC123"},
            "detail": {"rssi_dbm": -15.0, "airframe_class": "jet"},
        }) + "\n")

    def _cleanup(self):
        for patcher in reversed(self.patchers):
            patcher.stop()
        self.tmp.cleanup()

    def row(self):
        return json.loads(self.paths["LABELS"].read_text())

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_single_press_is_real_human_heard_training_evidence(self, run, _time):
        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 1)
        row = self.row()
        self.assertEqual(row["label"], "human_heard")
        self.assertEqual(row["priority"], 1)
        self.assertEqual(row["provenance"], "hue_button")
        self.assertTrue(row["eligible_for_training"])
        self.assertEqual(row["live_candidates"][0]["aircraft"]["hex"], "ABC123")
        self.assertEqual(row["recent_completed_passes"][0]["detail"]["rssi_dbm"], -15.0)
        run.assert_called_once()

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_double_press_is_distinct_top_priority_cat_evidence(self, run, _time):
        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 2)
        row = self.row()
        self.assertEqual(row["label"], "cat_scared_top_priority")
        self.assertEqual(row["priority"], 2)
        self.assertEqual(row["press_count"], 2)
        self.assertIn("TOP PRIORITY", run.call_args.args[0][-1])

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_synthetic_event_is_never_training_evidence(self, run, _time):
        with mock.patch.object(label, "SYNTHETIC", True):
            label.record_label(1000.0, 2)
        row = self.row()
        self.assertEqual(row["provenance"], "synthetic")
        self.assertFalse(row["eligible_for_training"])
        run.assert_not_called()
        state = json.loads(self.paths["STATE"].read_text())
        self.assertIn("ineligible", state["correlator"]["skipped"])

    def test_recent_context_excludes_future_and_stale_passes(self):
        self.paths["PASS_LOG"].write_text("\n".join(json.dumps({"at": at}) for at in (700, 800, 999, 1001)) + "\n")
        self.assertEqual([row["at"] for row in label.recent_passes(1000)], [800, 999])


if __name__ == "__main__":
    unittest.main()
