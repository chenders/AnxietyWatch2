#!/usr/bin/env python3
"""Tests for the fail-closed synthetic aircraft-label injector."""
import importlib.machinery
import pathlib
import tempfile
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).with_name("panel-aircraft-inject")
inject = importlib.machinery.SourceFileLoader("panel_aircraft_inject", str(SCRIPT)).load_module()


class SyntheticOnly(unittest.TestCase):
    def test_one_and_two_press_rows_are_always_ineligible_synthetic(self):
        for presses, name in ((1, "human_heard"), (2, "cat_scared_top_priority")):
            row = inject.build(presses, 1000.0, "test")
            self.assertEqual(row["label"], name)
            self.assertEqual(row["provenance"], "synthetic")
            self.assertIs(row["eligible_for_training"], False)
            self.assertTrue(row["event_id"].startswith("synthetic-"))

    def test_cli_has_no_provenance_or_eligibility_override(self):
        with self.assertRaises(SystemExit):
            inject.main(["--presses", "1", "--note", "test", "--provenance", "hue_button"])
        with self.assertRaises(SystemExit):
            inject.main(["--presses", "1", "--note", "test", "--eligible-for-training"])

    def test_dry_run_does_not_write(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "labels.jsonl"
            with mock.patch.object(inject.time, "time", return_value=1000.0):
                self.assertEqual(inject.main(["--presses", "2", "--note", "test",
                                              "--output", str(path), "--dry-run"]), 0)
            self.assertFalse(path.exists())

    def test_append_is_durable_jsonl(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "labels.jsonl"
            inject.append(path, inject.build(1, 1000.0, "first"))
            inject.append(path, inject.build(2, 1001.0, "second"))
            self.assertEqual(len(path.read_text().splitlines()), 2)


if __name__ == "__main__":
    unittest.main()
