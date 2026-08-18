#!/usr/bin/env python3
"""Tests for real operator-reported noise evidence and corroborations."""
import importlib.machinery
import importlib.util
import json
import pathlib
import tempfile
import types
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).with_name("panel-aircraft-report")
_loader = importlib.machinery.SourceFileLoader("panel_aircraft_report", str(SCRIPT))
_spec = importlib.util.spec_from_loader(_loader.name, _loader)
report = importlib.util.module_from_spec(_spec)
_loader.exec_module(report)


def args(**over):
    base = dict(note="test", cat=False, ago=None, at=None, iso=None,
                source="operator_report", corroborates=None, labels=None,
                dry_run=False)
    base.update(over)
    return types.SimpleNamespace(**base)


class TimeParsing(unittest.TestCase):
    def test_exactly_one_time_source_is_enforced(self):
        with self.assertRaisesRegex(SystemExit, "exactly one"):
            report.parse_when(args(at=1000.0, iso="2026-08-17T02:58:03.813Z"), 2000.0)

    def test_naive_iso_is_refused_not_guessed(self):
        # A wall-clock guess would poison the correlation this tool exists for.
        with self.assertRaisesRegex(SystemExit, "no timezone"):
            report.parse_when(args(iso="2026-08-17T02:58:03"), 2000.0)

    def test_utc_iso_converts_exactly_with_provenance(self):
        when, note = report.parse_when(args(iso="2026-08-17T02:58:03.813Z"), 2000.0)
        self.assertAlmostEqual(when, 1786935483.813, places=3)
        self.assertIn("explicit offset", note)

    def test_ago_is_now_relative_and_future_refused(self):
        when, _ = report.parse_when(args(ago=90.0), 2000.0)
        self.assertEqual(when, 1910.0)
        with self.assertRaisesRegex(SystemExit, "future"):
            report.parse_when(args(ago=-5.0), 2000.0)


class Rows(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root = pathlib.Path(self.tmp.name)
        self.patchers = [
            mock.patch.object(report.label, "STATUS", root / "status.json"),
            mock.patch.object(report.label, "FR24_STATUS", root / "fr24.json"),
            mock.patch.object(report.label, "PASS_LOG", root / "passes.jsonl"),
            mock.patch.object(report.label, "WEATHER_CACHE", root / "weather.json"),
        ]
        for p in self.patchers:
            p.start()
            self.addCleanup(p.stop)
        (root / "status.json").write_text(json.dumps(
            {"written_at": 999.0, "adsb_planes": [{"hex": "ABC123"}]}))
        (root / "fr24.json").write_text(json.dumps(
            {"written_at": 999.0, "feed_state": "FRESH", "aircraft": []}))
        (root / "passes.jsonl").write_text("")

    def test_near_live_report_captures_full_context(self):
        row = report.build_event(990.0, 1000.0, args(), "test")
        self.assertEqual(row["provenance"], "operator_report")
        self.assertTrue(row["eligible_for_training"])
        self.assertEqual(row["outcomes"], ["human_heard"])
        self.assertEqual(row["live_candidates"][0]["aircraft"]["hex"], "ABC123")
        self.assertIn("adsb_status", row["feed_snapshots"])

    def test_late_report_refuses_wrong_moment_context(self):
        row = report.build_event(1000.0, 5000.0, args(), "test")
        self.assertEqual(row["live_candidates"], [])
        self.assertIn("wrong moment", row["weather"]["reason"])
        self.assertIn("unavailable", row["live_context"]["state"])

    def test_cat_flag_records_both_outcomes(self):
        row = report.build_event(990.0, 1000.0, args(cat=True), "test")
        self.assertEqual(row["outcomes"], ["human_heard", "cat_scared_top_priority"])
        self.assertEqual(row["priority"], 2)

    def test_corroboration_is_structurally_untrainable(self):
        row = report.build_corroboration("hue-123-1", 990.0, 1000.0,
                                         args(note="I told you"), "test")
        self.assertEqual(row["corroborates"], "hue-123-1")
        self.assertIs(row["eligible_for_training"], False)
        self.assertEqual(row["operator_report"]["note"], "I told you")


class Main(unittest.TestCase):
    def test_corroborating_a_missing_event_is_blocked_by_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            labels = pathlib.Path(tmp) / "labels.jsonl"
            labels.write_text("")
            with self.assertRaisesRegex(SystemExit, "not found"):
                report.main(["--note", "x", "--corroborates", "hue-999-1",
                             "--labels", str(labels)])

    def test_dry_run_writes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            labels = pathlib.Path(tmp) / "labels.jsonl"
            self.assertEqual(report.main(["--note", "x", "--ago", "5",
                                          "--labels", str(labels), "--dry-run"]), 0)
            self.assertFalse(labels.exists())

    @mock.patch("subprocess.run")
    def test_real_report_is_durable_and_rebuilds_attribution(self, run):
        run.return_value = mock.Mock(returncode=0, stdout="", stderr="")
        with tempfile.TemporaryDirectory() as tmp:
            labels = pathlib.Path(tmp) / "labels.jsonl"
            self.assertEqual(report.main(["--note", "heard it", "--ago", "5",
                                          "--labels", str(labels)]), 0)
            row = json.loads(labels.read_text().splitlines()[0])
            self.assertEqual(row["provenance"], "operator_report")
            self.assertTrue(row["eligible_for_training"])
            command = run.call_args.args[0]
            self.assertEqual(command[0], "/usr/local/bin/panel-aircraft-attribute")
            self.assertIn("--output", command)

    @mock.patch("subprocess.run")
    def test_failed_rebuild_still_keeps_the_durable_row(self, run):
        run.return_value = mock.Mock(returncode=1, stdout="", stderr="down")
        with tempfile.TemporaryDirectory() as tmp:
            labels = pathlib.Path(tmp) / "labels.jsonl"
            self.assertEqual(report.main(["--note", "x", "--ago", "5",
                                          "--labels", str(labels)]), 1)
            self.assertEqual(len(labels.read_text().splitlines()), 1)


if __name__ == "__main__":
    unittest.main()
