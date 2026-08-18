#!/usr/bin/env python3
import importlib.machinery
import io
import json
import pathlib
import tempfile
import unittest

backfill = importlib.machinery.SourceFileLoader(
    "panel_warning_backfill",
    str(pathlib.Path(__file__).with_name("panel-warning-backfill"))).load_module()


class WarningBackfill(unittest.TestCase):
    def journal(self, micros, message):
        return io.StringIO(json.dumps({"__REALTIME_TIMESTAMP": str(micros),
                                       "MESSAGE": message}) + "\n")

    def test_parses_old_warning_wording(self):
        rows = list(backfill.parse_journal(self.journal(
            1787006512711693,
            "panel-hue-warn: warned — N170US LOUD heard in 28s")))
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["warning_id"], "journal-1787006512711693-N170US")
        self.assertEqual(rows[0]["band"], "LOUD")
        self.assertEqual(rows[0]["notice_s"], 28.0)

    def test_parses_unlocated_warning(self):
        row = next(backfill.parse_journal(self.journal(
            1000000, "panel-hue-warn: warned — N776LF heard, position unknown — no countdown possible")))
        self.assertEqual(row["band"], "UNLOCATED")
        self.assertIsNone(row["notice_s"])

    def test_ignores_arm_and_service_lines(self):
        rows = list(backfill.parse_journal(self.journal(
            1000000, "panel-hue-warn: warnings ARMED")))
        self.assertEqual(rows, [])

    def test_enriches_from_nearest_matching_pass(self):
        warning = next(backfill.parse_journal(self.journal(
            1_000_000_000, "panel-hue-warn: warned — N170US LOUD in 28s")))
        passes = [{"at": 1002.0, "identity": {"hex": "A11B00", "reg": "N170US",
                   "callsign": "AAL2986", "type": "A321"}, "noise_index": 48.9,
                   "loudness_band": "AUDIBLE", "provenance": "adsb"}]
        row = backfill.enrich(warning, passes)
        self.assertEqual(row["hex"], "A11B00")
        self.assertEqual(row["aircraft_type"], "A321")
        self.assertEqual(row["matched_pass"]["noise_index"], 48.9)

    def test_append_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "warnings.jsonl"
            row = {"warning_id": "w1", "at": 1}
            self.assertEqual(backfill.append_new(path, [row]), 1)
            self.assertEqual(backfill.append_new(path, [row]), 0)
            self.assertEqual(len(path.read_text().splitlines()), 1)


if __name__ == "__main__":
    unittest.main()
