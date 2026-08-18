#!/usr/bin/env python3
"""Tests for the definitive-track archiver, especially its spend bounds.

The attempt bookkeeping mirrors the sibling project's FlightDiscovery
pattern: without it a permanently-broken flight id would re-charge ~40
estimated credits every timer tick, forever.
"""
import importlib.machinery
import json
import pathlib
import tempfile
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).with_name("panel-fr24-track-fetch")
fetcher = importlib.machinery.SourceFileLoader("panel_fr24_track_fetch", str(SCRIPT)).load_module()


def attribution_doc():
    return {"events": [{
        "verdict": "CONFIRMED", "event_id": "e1", "observed_at": 1000.0,
        "confirmed_candidate": {
            "identity": "A58966", "registration": "N456LF",
            "raw": {"hex": "A58966", "fr24_id": "4121ee82"},
            "evidence": [],
        },
    }]}


def tracks_payload():
    return [{"fr24_id": "4121ee82", "tracks": [
        {"timestamp": "2026-08-17T04:11:00Z", "lat": 45.51, "lon": -122.62,
         "alt": 1200, "gspeed": 92, "track": 300, "vspeed": -500, "callsign": "LIFE1"},
        {"timestamp": None, "lat": 45.52, "lon": -122.63},              # junk
        {"timestamp": "2026-08-17T04:11:05Z", "lat": None, "lon": -122.63},  # junk
    ]}]


class Parsing(unittest.TestCase):
    def test_confirmed_flights_come_from_the_raw_candidate(self):
        flights = fetcher.confirmed_fr24_flights(attribution_doc())
        self.assertEqual(flights, [{"fr24_id": "4121ee82", "event_id": "e1",
                                    "hex": "A58966", "registration": "N456LF",
                                    "callsign": None}])

    def test_fr24_id_falls_back_to_confirmed_evidence_rows(self):
        doc = attribution_doc()
        doc["events"][0]["confirmed_candidate"]["raw"] = {}
        doc["events"][0]["confirmed_candidate"]["evidence"] = [
            {"feed": "fr24", "aircraft": {"fr24_id": "9999aaaa"}}]
        flights = fetcher.confirmed_fr24_flights(doc)
        self.assertEqual(flights[0]["fr24_id"], "9999aaaa")

    def test_unconfirmed_and_fr24less_events_yield_nothing(self):
        doc = attribution_doc()
        doc["events"][0]["verdict"] = "AMBIGUOUS"
        self.assertEqual(fetcher.confirmed_fr24_flights(doc), [])

    def test_track_rows_parse_and_carry_the_retention_deadline(self):
        rows, skipped = fetcher.track_rows(tracks_payload(),
                                           {"fr24_id": "4121ee82", "hex": "A58966",
                                            "registration": "N456LF"})
        self.assertEqual(len(rows), 1)
        self.assertEqual(skipped, 2)
        row = rows[0]
        self.assertEqual(row["provenance"], "fr24")
        self.assertEqual(row["hex"], "A58966")
        self.assertEqual(row["callsign"], "LIFE1")
        self.assertAlmostEqual(row["retain_until"] - row["at"],
                               fetcher.FR24_RETENTION_SECONDS)

    def test_cycle_resets_on_the_billing_day_not_the_first(self):
        import calendar
        before = calendar.timegm((2026, 8, 5, 12, 0, 0))
        after = calendar.timegm((2026, 8, 6, 12, 0, 0))
        self.assertEqual(fetcher.cycle_key(before, 6), "2026-07")
        self.assertEqual(fetcher.cycle_key(after, 6), "2026-08")


class SpendBounds(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root = pathlib.Path(self.tmp.name)
        for name, path in (("ATTRIBUTIONS", root / "attr.json"),
                           ("STATE", root / "state.json"),
                           ("TRACK_SPOOL", root / "track.jsonl"),
                           ("LEDGER", root / "credits.json")):
            patcher = mock.patch.object(fetcher, name, path)
            patcher.start()
            self.addCleanup(patcher.stop)
        for value in (("TOKEN", "test-token"), ("REQUEST_SPACING_SECONDS", 0.0)):
            patcher = mock.patch.object(fetcher, value[0], value[1])
            patcher.start()
            self.addCleanup(patcher.stop)
        fetcher.ATTRIBUTIONS.write_text(json.dumps(attribution_doc()))

    def state(self):
        return json.loads(fetcher.STATE.read_text())

    def test_success_archives_points_and_clears_attempts(self):
        with mock.patch.object(fetcher, "fetch_track", return_value=tracks_payload()):
            self.assertEqual(fetcher.main(), 0)
        state = self.state()
        self.assertIn("4121ee82", state["fetched"])
        self.assertNotIn("4121ee82", state.get("attempts", {}))
        spool = fetcher.TRACK_SPOOL.read_text().splitlines()
        self.assertEqual(len(spool), 1)
        self.assertEqual(json.loads(state["fetched"]["4121ee82"] and spool[0])["fr24_id"],
                         "4121ee82")

    def test_failures_are_bounded_and_then_skipped_with_the_error_named(self):
        for expected in (1, 2, 3):
            with mock.patch.object(fetcher, "fetch_track",
                                   side_effect=OSError("410 gone")):
                fetcher.main()
            self.assertEqual(self.state()["attempts"]["4121ee82"]["count"], expected)
        # Exhausted: no further network call, no further charge.
        spent_before = json.loads(fetcher.LEDGER.read_text())["spent"]
        with mock.patch.object(fetcher, "fetch_track") as fetch:
            self.assertEqual(fetcher.main(), 0)
            fetch.assert_not_called()
        self.assertEqual(json.loads(fetcher.LEDGER.read_text())["spent"], spent_before)
        self.assertIn("410 gone", self.state()["attempts"]["4121ee82"]["error"])

    def test_attempt_is_durable_before_the_network_call(self):
        with mock.patch.object(fetcher, "fetch_track",
                               side_effect=lambda _:
                               (_ for _ in ()).throw(OSError("mid-fetch"))):
            fetcher.main()
        record = self.state()["attempts"]["4121ee82"]
        self.assertEqual(record["count"], 1)
        self.assertIn("mid-fetch", record["error"])

    def test_empty_track_is_recorded_not_retried_forever(self):
        with mock.patch.object(fetcher, "fetch_track", return_value=[]):
            fetcher.main()
        record = self.state()["attempts"]["4121ee82"]
        self.assertIn("no track data", record["error"])
        self.assertNotIn("4121ee82", self.state().get("fetched", {}))

    def test_missing_token_is_a_named_failure_only_when_there_is_work(self):
        with mock.patch.object(fetcher, "TOKEN", None):
            self.assertEqual(fetcher.main(), 1)
        fetcher.ATTRIBUTIONS.write_text(json.dumps({"events": []}))
        with mock.patch.object(fetcher, "TOKEN", None):
            self.assertEqual(fetcher.main(), 0)

    def test_credit_ceiling_blocks_before_the_call(self):
        fetcher.LEDGER.parent.mkdir(parents=True, exist_ok=True)
        fetcher.LEDGER.write_text(json.dumps(
            {"cycle": fetcher.cycle_key(__import__("time").time(), fetcher.BILLING_DAY),
             "spent": fetcher.CREDIT_CEILING}))
        with mock.patch.object(fetcher, "fetch_track") as fetch:
            self.assertEqual(fetcher.main(), 0)
            fetch.assert_not_called()


if __name__ == "__main__":
    unittest.main()
