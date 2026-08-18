#!/usr/bin/env python3
"""Tests for per-event aircraft-noise attribution.

The invariants that matter: one real event can confirm one aircraft without a
recurring-tail gate; a weak or close field stays AMBIGUOUS rather than being
guessed; synthetic never enters; operator reports are as real as presses;
corroborations attach to events without becoming events.
"""
import importlib.machinery
import json
import pathlib
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name("panel-aircraft-attribute")
a = importlib.machinery.SourceFileLoader("panel_aircraft_attribute", str(SCRIPT)).load_module()


def label(event, at, candidates, presses=1, provenance="hue_button", eligible=True):
    return {"event_id": event, "observed_at": at, "press_count": presses,
            "label": "cat_scared_top_priority" if presses == 2 else "human_heard",
            "provenance": provenance, "eligible_for_training": eligible,
            "live_candidates": [{"feed": "adsb_predictions", "aircraft": row} for row in candidates]}


def aircraft(hex_code, noise, distance_m, eta, callsign=None, icao_type=None):
    return {"hex": hex_code, "callsign": callsign, "icao_type": icao_type,
            "prediction": {"noise_index": noise, "ground_distance_m": distance_m,
                           "eta_seconds": eta}}


class Attribution(unittest.TestCase):
    def test_one_event_can_confirm_one_aircraft_without_repeat_tail_gate(self):
        event = label("e1", 1000, [aircraft("LOUD", 64, 400, 10),
                                   aircraft("FAINT", 35, 8000, 120)])
        result = a.attribute(event, [])
        self.assertEqual(result["verdict"], "CONFIRMED")
        self.assertEqual(result["confirmed_candidate"]["identity"], "LOUD")
        self.assertIn("flight_key", result)

    def test_close_scores_remain_ambiguous_and_unassigned(self):
        event = label("e1", 1000, [aircraft("ONE", 50, 2000, 20),
                                   aircraft("TWO", 49, 2100, 20)])
        result = a.attribute(event, [])
        self.assertEqual(result["verdict"], "AMBIGUOUS")
        self.assertNotIn("confirmed_candidate", result)
        self.assertIn("margin", result["verdict_reason"])

    def test_sole_weak_candidate_is_not_confirmed(self):
        # One aircraft, but far and quiet: below the confirmation floor. A
        # lone candidate must still be physically plausible to be blamed.
        event = label("e1", 1000, [aircraft("WEAK", 20, 11000, 170)])
        result = a.attribute(event, [])
        self.assertEqual(result["verdict"], "AMBIGUOUS")
        self.assertIn("below confirmation floor", result["verdict_reason"])

    def test_sole_candidate_with_no_physical_evidence_is_not_confirmed(self):
        event = label("e1", 1000, [{"hex": "GHOST"}])
        result = a.attribute(event, [])
        self.assertEqual(result["verdict"], "AMBIGUOUS")
        self.assertIn("no acoustic estimate and no", result["verdict_reason"])

    def test_no_candidates_is_unattributed_with_reason(self):
        result = a.attribute(label("e1", 1000, []), [])
        self.assertEqual(result["verdict"], "UNATTRIBUTED")
        self.assertIn("no aircraft observed", result["verdict_reason"])

    def test_double_press_is_both_human_and_cat_in_legacy_rows(self):
        event = label("e1", 1000, [aircraft("LOUD", 64, 400, 10)], presses=2)
        result = a.attribute(event, [])
        self.assertEqual(result["outcomes"], ["human_heard", "cat_scared_top_priority"])

    def test_candidates_carry_verbatim_raw_for_the_archive(self):
        event = label("e1", 1000, [aircraft("LOUD", 64, 400, 10)])
        result = a.attribute(event, [])
        self.assertEqual(result["candidates"][0]["raw"]["hex"], "LOUD")

    def test_candidates_are_stamped_with_the_operator_kind(self):
        """Stamped from stable identifiers at attribution time, so a rebuild
        retroactively repairs feed rows recorded while the callsign-matched
        medical fleet carried kind None (2026-08-17)."""
        police = aircraft("A50E9F", 64, 400, 10, callsign="N425EJ")   # roster hex
        medical = aircraft("ABC123", 60, 500, 12, callsign="N999LF")  # pattern only
        plain = aircraft("D00D00", 55, 900, 15, callsign="ASA123")
        result = a.attribute(label("e1", 1000, [police, medical, plain]), [])
        kinds = {c["identity"]: c["operator_kind"] for c in result["candidates"]}
        self.assertEqual(kinds["A50E9F"], "POLICE")
        self.assertEqual(kinds["ABC123"], "AIR_AMBULANCE")
        self.assertIsNone(kinds["D00D00"])

    def test_rotorcraft_class_comes_from_the_audited_type_table(self):
        event = label("e1", 1000, [aircraft("HELI", 55, 900, 15, icao_type="EC35"),
                                   aircraft("PLANE", 55, 900, 15, icao_type="C172")])
        result = a.attribute(event, [])
        by_id = {c["identity"]: c for c in result["candidates"]}
        self.assertEqual(by_id["HELI"]["airframe_class"], "rotorcraft")
        self.assertEqual(by_id["PLANE"]["airframe_class"], "light")
        self.assertGreater(by_id["HELI"]["score"], by_id["PLANE"]["score"])

    def test_pass_log_candidates_join_within_the_window(self):
        passes = [{"at": 1100.0, "provenance": "adsb",
                   "identity": {"hex": "PASSED", "reg": "N123XX"},
                   "distance_km": 0.8, "noise_index": 58.0}]
        result = a.attribute(label("e1", 1000, []), passes)
        self.assertEqual(result["candidates"][0]["identity"], "PASSED")
        self.assertEqual(result["verdict"], "CONFIRMED")


class RealEvidenceOnly(unittest.TestCase):
    def test_operator_reports_are_first_class_real_evidence(self):
        rows = [label("r1", 1000, [aircraft("LOUD", 64, 400, 10)],
                      provenance="operator_report")]
        doc = a.build_document(rows, [], generated_at=2000)
        self.assertEqual(len(doc["events"]), 1)
        self.assertEqual(doc["events"][0]["provenance"], "operator_report")

    def test_synthetic_and_ineligible_rows_never_become_events(self):
        rows = [label("s1", 1000, [aircraft("LOUD", 64, 400, 10)], provenance="synthetic"),
                label("s2", 1000, [aircraft("LOUD", 64, 400, 10)], eligible=False)]
        doc = a.build_document(rows, [], generated_at=2000)
        self.assertEqual(doc["events"], [])

    def test_corroborations_attach_but_never_become_events(self):
        rows = [label("e1", 1000, [aircraft("LOUD", 64, 400, 10)]),
                {"event_id": "c1", "corroborates": "e1", "observed_at": 1030.0,
                 "provenance": "operator_report", "eligible_for_training": False,
                 "operator_report": {"note": "faint but definitely notable",
                                      "source": "chat"}}]
        doc = a.build_document(rows, [], generated_at=2000)
        self.assertEqual(len(doc["events"]), 1)
        corro = doc["events"][0]["corroborated_by"]
        self.assertEqual(corro[0]["note"], "faint but definitely notable")


class History(unittest.TestCase):
    def test_document_flags_airframe_and_flight_with_history(self):
        rows = [label("e1", 1000, [aircraft("LOUD", 64, 400, 10, "LIFE1")]),
                label("e2", 1010, [aircraft("LOUD", 64, 300, 5, "LIFE1")], presses=2)]
        doc = a.build_document(rows, [], generated_at=2000)
        frame = doc["airframes"]["LOUD"]
        self.assertEqual(frame["event_ids"], ["e1", "e2"])
        self.assertEqual(frame["human_heard_count"], 2)
        self.assertEqual(frame["cat_scared_count"], 1)
        self.assertEqual(frame["first_heard_at"], 1000)
        self.assertEqual(frame["last_heard_at"], 1010)
        flight = next(iter(doc["flights"].values()))
        self.assertEqual(flight["human_heard_count"], 2)
        self.assertEqual(flight["cat_scared_count"], 1)

    def test_flight_key_finds_fr24_id_in_any_evidence_row(self):
        plane = dict(aircraft("LOUD", 64, 400, 10), fr24_id="4121ee82")
        doc = a.build_document([label("e1", 1000, [plane])], [], generated_at=2000)
        self.assertIn("fr24:4121ee82", doc["flights"])

    def test_publish_is_atomic_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "attributes.json"
            a.publish(path, {"schema": 2})
            self.assertEqual(json.loads(path.read_text()), {"schema": 2})
            self.assertFalse(path.with_name(".attributes.json.tmp").exists())


class Timing(unittest.TestCase):
    def test_propagation_corrected_timing_prefers_at_epoch(self):
        plane = {"hex": "TIMED",
                 "prediction": {"noise_index": 50.0, "ground_distance_m": 1000,
                                "altitude_ft": 1000, "at_epoch": 995.0,
                                "eta_seconds": 900}}
        score, reasons, _ = a.score_candidate(plane, "adsb_predictions", observed_at=1000.0)
        self.assertTrue(any("sound travel" in r for r in reasons), reasons)
        # at_epoch(995)+delay(~3s) is ~2s from the press: near-max timing
        # credit, where the bogus 900 s eta would have earned none.
        self.assertGreater(score, 50.0 + 15.0 + 11.0)

    def test_acoustic_delay_uses_the_slant_path(self):
        level = a.acoustic_delay_s(1.0, 0)
        high = a.acoustic_delay_s(1.0, 10000)
        self.assertAlmostEqual(level, 1000.0 / 343.0, places=3)
        self.assertGreater(high, level * 2)
        self.assertIsNone(a.acoustic_delay_s(None, 5000))


if __name__ == "__main__":
    unittest.main()
