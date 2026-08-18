#!/usr/bin/env python3
"""Tests for the spool -> PostgreSQL drain. No database, no network.

Everything below exercises `normalise`, which is where a bad row either gets
rejected or gets quietly written into the permanent record as something it is
not. The database round-trip is covered by db/verify/aircraft.sql, which asserts
the same invariants as CHECK constraints.
"""

import importlib.machinery
import json
import pathlib
import tempfile
import unittest

_HERE = pathlib.Path(__file__).parent
drain = importlib.machinery.SourceFileLoader(
    "panel_noise_drain", str(_HERE / "panel-noise-drain")).load_module()


def row(**over):
    base = {"at": 1786528746.0, "provenance": "adsb", "retain_until": None,
            "identity": {"callsign": "N450LF", "hex": "A5731C", "reg": None,
                         "type": None, "fr24_id": "4121ee82"},
            "air_ambulance": "CONFIRMED", "distance_km": 2.078, "altitude_ft": 1175,
            "phase": "DESCENT", "noise_index": None, "loudness_band": "UNKNOWN",
            "reason": "matches Life Flight", "notable": True}
    base.update(over)
    return base


class Normalisation(unittest.TestCase):
    def test_a_good_receiver_row_survives(self):
        out = drain.normalise(row())
        self.assertIsNotNone(out)
        self.assertEqual(out["provenance"], "adsb")
        self.assertEqual(out["icao_hex"], "A5731C")
        self.assertIsNone(out["retain_until"], "receiver rows must carry no deletion deadline")

    def test_hex_is_upper_cased_to_match_the_roster(self):
        out = drain.normalise(row(identity={"hex": "a5731c"}))
        self.assertEqual(out["icao_hex"], "A5731C")

    def test_an_unscored_row_keeps_its_null(self):
        # The row this whole system exists to catch. It must never acquire a
        # number on the way into the database.
        out = drain.normalise(row(noise_index=None, loudness_band="UNKNOWN"))
        self.assertIsNone(out["noise_index"])
        self.assertEqual(out["loudness_band"], "UNKNOWN")

    def test_a_row_whose_band_and_index_disagree_is_rejected(self):
        # Rejected, not repaired: the disagreement is a bug upstream, and
        # coercing it here would hide the bug and store a wrong loudness.
        self.assertIsNone(drain.normalise(row(noise_index=None, loudness_band="FAINT")))
        self.assertIsNone(drain.normalise(row(noise_index=91.0, loudness_band="UNKNOWN")))

    def test_an_fr24_row_without_a_deadline_is_refused(self):
        # The vendor deadline is mandatory for their data. A row that cannot be
        # swept is not admitted at all.
        self.assertIsNone(drain.normalise(row(provenance="fr24", retain_until=None)))

    def test_an_fr24_row_with_a_deadline_is_accepted(self):
        out = drain.normalise(row(provenance="fr24", retain_until=1802253546.0))
        self.assertEqual(out["provenance"], "fr24")
        self.assertEqual(out["retain_until"], 1802253546.0)

    def test_a_receiver_row_never_inherits_a_deadline(self):
        # Even if one is somehow present in the spool.
        out = drain.normalise(row(provenance="adsb", retain_until=1802253546.0))
        self.assertIsNone(out["retain_until"])

    def test_junk_is_rejected_rather_than_crashing_the_batch(self):
        for bad in (None, [], "text", {}, {"at": "soon", "provenance": "adsb"},
                    row(provenance="guess"), row(loudness_band="VERY QUIET")):
            self.assertIsNone(drain.normalise(bad))

    def test_an_unrecognised_phase_becomes_null_rather_than_being_invented(self):
        self.assertIsNone(drain.normalise(row(phase="HOVER"))["phase"])
        self.assertEqual(drain.normalise(row(phase="CLIMB"))["phase"], "CLIMB")


class WarningNormalisation(unittest.TestCase):
    def test_fired_historical_warning_is_accepted(self):
        row = drain.normalise_warning({"warning_id": "journal-1-N170US", "at": 1000.0,
            "fired": True, "suppressed_by": None, "test": False, "hex": "a11b00",
            "registration": "N170US", "band": "LOUD", "notice_s": 28.0,
            "prediction": None, "matched_pass": {"noise_index": 48.9}})
        self.assertEqual(row["icao_hex"], "A11B00")
        self.assertEqual(row["band"], "LOUD")
        self.assertEqual(row["notice_s"], 28.0)

    def test_suppressed_warning_must_name_valid_blocker(self):
        base = {"warning_id": "w", "at": 1000.0, "fired": False,
                "band": "LOUD"}
        self.assertIsNone(drain.normalise_warning(base))
        row = dict(base, suppressed_by="quiet_hours")
        self.assertEqual(drain.normalise_warning(row)["suppressed_by"], "quiet_hours")

    def test_review_has_three_explicit_verdicts(self):
        for verdict in ("accurate", "inaccurate", "unknown"):
            good = drain.normalise_warning_review(
                {"warning_id": "w", "reviewed_at": 1001.0, "verdict": verdict})
            self.assertEqual(good["verdict"], verdict)
        self.assertIsNone(drain.normalise_warning_review(
            {"warning_id": "w", "reviewed_at": 1001.0, "verdict": "maybe"}))


class WarningExpiry(unittest.TestCase):
    def test_unreviewed_warning_expiry_uses_three_hour_window(self):
        class Cursor:
            rowcount = 7
            def execute(self, sql, params):
                self.sql, self.params = sql, params
        cur = Cursor()
        result = drain.expire_unreviewed_warnings(cur)
        self.assertEqual(cur.params, (3 * 60 * 60, 3 * 60 * 60))
        self.assertIn("expired_unreviewed", cur.sql)
        self.assertIn("NOT EXISTS", cur.sql)
        self.assertEqual(result["drained"], 7)


class OffsetTracking(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        drain.STATE = pathlib.Path(self.tmp.name) / "drain.json"

    def tearDown(self):
        self.tmp.cleanup()

    def test_a_missing_state_file_starts_from_zero(self):
        self.assertEqual(drain.read_offset(), 0)

    def test_the_offset_round_trips(self):
        drain.write_offset(4096)
        self.assertEqual(drain.read_offset(), 4096)

    def test_a_corrupt_state_file_restarts_rather_than_crashing(self):
        drain.STATE.write_text("{not json")
        self.assertEqual(drain.read_offset(), 0)

    def test_a_negative_offset_is_refused(self):
        drain.STATE.write_text(json.dumps({"offset_bytes": -5}))
        self.assertEqual(drain.read_offset(), 0)


def label_row(**over):
    base = {"event_id": "hue-1786935446780-1", "observed_at": 1786935446.78,
            "recorded_at": 1786935448.0, "press_count": 1,
            "outcomes": ["human_heard"], "label": "human_heard",
            "provenance": "hue_button", "eligible_for_training": True,
            "weather": {"state": "measured", "observation": {"temperature_c": 17.2}}}
    base.update(over)
    return base


class LabelNormalisation(unittest.TestCase):
    def test_a_real_press_becomes_an_event_with_full_evidence(self):
        kind, payload = drain.normalise_label(label_row())
        self.assertEqual(kind, "event")
        self.assertTrue(payload["human_heard"])
        self.assertFalse(payload["cat_scared"])
        self.assertEqual(payload["evidence"]["event_id"], payload["event_id"])
        self.assertEqual(payload["weather"]["observation"]["temperature_c"], 17.2)

    def test_a_hold_drains_with_both_outcomes_false_and_a_named_note(self):
        # warning_unheard: a light warning fired but nothing was heard. Both
        # outcome booleans are honestly false; the note says why, so the row
        # can never be mistaken for one that lost its outcomes.
        kind, payload = drain.normalise_label(label_row(
            event_id="hue-1786945459468-hold", label="warning_unheard",
            outcomes=["warning_unheard"]))
        self.assertEqual(kind, "event")
        self.assertFalse(payload["human_heard"])
        self.assertFalse(payload["cat_scared"])
        self.assertIn("warning_unheard", payload["note"])

    def test_synthetic_is_refused_by_provenance_and_counted(self):
        kind, payload = drain.normalise_label(label_row(provenance="synthetic"))
        self.assertEqual(kind, "synthetic")
        self.assertIsNone(payload)

    def test_operator_reports_are_real_events(self):
        kind, payload = drain.normalise_label(label_row(
            provenance="operator_report",
            operator_report={"note": "faint but notable"}))
        self.assertEqual(kind, "event")
        self.assertEqual(payload["note"], "faint but notable")

    def test_corroborations_attach_and_never_become_events(self):
        kind, payload = drain.normalise_label(label_row(
            event_id="corroboration-abc", corroborates="hue-1786935446780-1",
            provenance="operator_report", eligible_for_training=False,
            operator_report={"note": "told you within seconds", "source": "chat"}))
        self.assertEqual(kind, "corroboration")
        self.assertEqual(payload["event_id"], "hue-1786935446780-1")
        self.assertEqual(payload["note"], "told you within seconds")

    def test_legacy_double_press_records_both_outcomes(self):
        row = label_row(label="cat_scared_top_priority", press_count=2)
        del row["outcomes"]
        kind, payload = drain.normalise_label(row)
        self.assertTrue(payload["human_heard"])
        self.assertTrue(payload["cat_scared"])

    def test_junk_is_rejected_not_crashed_on(self):
        for bad in (None, [], "x", {}, label_row(observed_at="soon"),
                    label_row(event_id=None), label_row(eligible_for_training=False)):
            kind, _ = drain.normalise_label(bad)
            self.assertNotEqual(kind, "event")


class CandidateNormalisation(unittest.TestCase):
    def candidate(self, **over):
        base = {"identity": "A58966", "callsign": "LIFE1", "registration": "N456LF",
                "aircraft_type": "B407", "airframe_class": "rotorcraft",
                "feeds": ["adsb_planes"], "score": 68.2, "reasons": ["close"],
                "raw": {"hex": "A58966", "dist": 4024.0, "alt_ft": 2550,
                         "prediction": {"phase": "DESCENT", "noise_index": 55.4}}}
        base.update(over)
        return base

    def test_projection_and_verbatim_raw_both_survive(self):
        row = drain.candidate_db_row("e1", 1000.0, self.candidate())
        self.assertEqual(row["icao_hex"], "A58966")
        self.assertAlmostEqual(row["distance_km"], 4.024)
        self.assertEqual(row["noise_index"], 55.4)
        self.assertEqual(row["phase"], "DESCENT")
        self.assertEqual(row["raw"]["hex"], "A58966")
        self.assertIsNone(row["retain_until"], "locally-seen data never expires")

    def test_fr24_only_candidates_carry_the_retention_deadline(self):
        row = drain.candidate_db_row("e1", 1000.0, self.candidate(feeds=["fr24"]))
        self.assertEqual(row["retain_until"], 1000.0 + drain.FR24_RETENTION_SECONDS)

    def test_unknown_airframe_class_is_dropped_not_stored(self):
        row = drain.candidate_db_row("e1", 1000.0, self.candidate(airframe_class="blimp"))
        self.assertIsNone(row["airframe_class"])


class AttributionNormalisation(unittest.TestCase):
    def event(self, **over):
        base = {"event_id": "e1", "verdict": "CONFIRMED",
                "verdict_reason": "score 68, margin 20", "margin": 20.0,
                "candidate_count": 3, "method": "per-event physical ranking",
                "flight_key": "fr24:4121ee82",
                "confirmed_candidate": {"identity": "A58966",
                                          "registration": "N456LF",
                                          "aircraft_type": "B407"}}
        base.update(over)
        return base

    def test_confirmed_verdict_round_trips(self):
        row = drain.attribution_db_row(self.event(), 2000.0)
        self.assertEqual(row["confirmed_hex"], "A58966")
        self.assertEqual(row["flight_key"], "fr24:4121ee82")

    def test_signature_ignores_generated_at_so_reruns_are_not_history(self):
        one = drain.attribution_db_row(self.event(), 2000.0)
        two = drain.attribution_db_row(self.event(), 9000.0)
        self.assertEqual(drain.attribution_signature(one),
                         drain.attribution_signature(two))
        changed = drain.attribution_db_row(self.event(verdict="AMBIGUOUS",
                                                       confirmed_candidate=None), 9000.0)
        self.assertNotEqual(drain.attribution_signature(one),
                            drain.attribution_signature(changed))

    def test_unknown_verdict_is_rejected(self):
        self.assertIsNone(drain.attribution_db_row(self.event(verdict="MAYBE"), 1.0))


class TrackNormalisation(unittest.TestCase):
    def point(self, **over):
        base = {"at": 1000.0, "provenance": "adsb", "hex": "a58966",
                "lat": 45.51, "lon": -122.62, "alt_ft": 1200, "speed_kt": 92,
                "track_deg": 300.0, "vertical_rate_fpm": -500,
                "rssi_dbm": -18.0, "distance_km": 4.0, "bearing_deg": 270.0,
                "retain_until": None}
        base.update(over)
        return base

    def test_receiver_points_never_expire_and_hex_is_normalised(self):
        row = drain.normalise_track(self.point(retain_until=999999.0))
        self.assertEqual(row["icao_hex"], "A58966")
        self.assertIsNone(row["retain_until"])

    def test_fr24_points_without_a_deadline_are_refused(self):
        self.assertIsNone(drain.normalise_track(self.point(provenance="fr24",
                                                             retain_until=None)))
        ok = drain.normalise_track(self.point(provenance="fr24", retain_until=2000.0))
        self.assertEqual(ok["retain_until"], 2000.0)

    def test_impossible_coordinates_are_rejected(self):
        self.assertIsNone(drain.normalise_track(self.point(lat=91.0)))
        self.assertIsNone(drain.normalise_track(self.point(lon=-181.0)))
        self.assertIsNone(drain.normalise_track(self.point(lat=None)))


class NewInsertParameterParity(unittest.TestCase):
    """Every placeholder has a value and every value a placeholder, for every
    NEW insert. The sibling project's zeroed-analysis bug was exactly a
    field-name drift between a builder and its consumer."""

    import re as _re
    NAMED = _re.compile(r"%\((\w+)\)s")

    def assert_parity(self, sql, keys):
        placeholders = set(self.NAMED.findall(sql))
        self.assertTrue(placeholders)
        self.assertEqual(placeholders - keys, set(), "SQL binds names the builder omits")
        self.assertEqual(keys - placeholders, set(), "builder returns names the SQL drops")

    def test_event_insert(self):
        _, payload = drain.normalise_label(label_row())
        self.assert_parity(drain.INSERT_EVENT, set(payload))

    def test_corroboration_insert(self):
        _, payload = drain.normalise_label(label_row(
            corroborates="hue-1", provenance="operator_report",
            eligible_for_training=False))
        self.assert_parity(drain.INSERT_CORROBORATION, set(payload))

    def test_candidate_insert(self):
        row = drain.candidate_db_row("e1", 1000.0, {
            "identity": "A58966", "feeds": ["fr24"], "score": 1.0,
            "reasons": [], "raw": {}})
        self.assert_parity(drain.INSERT_CANDIDATE, set(row))

    def test_attribution_insert(self):
        row = drain.attribution_db_row(
            {"event_id": "e1", "verdict": "UNATTRIBUTED",
             "verdict_reason": "none", "method": "m"}, 1.0)
        self.assert_parity(drain.INSERT_ATTRIBUTION, set(row))

    def test_track_insert(self):
        row = drain.normalise_track({"at": 1.0, "provenance": "adsb",
                                     "lat": 45.0, "lon": -122.0})
        self.assert_parity(drain.INSERT_TRACK, set(row))


class KeyedOffsets(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        drain.STATE = pathlib.Path(self.tmp.name) / "drain.json"
        self.addCleanup(self.tmp.cleanup)

    def test_each_spool_has_an_independent_offset(self):
        drain.write_offset(100)
        drain.write_offset(200, "labels_offset_bytes")
        drain.write_offset(300, "track_offset_bytes")
        self.assertEqual(drain.read_offset(), 100)
        self.assertEqual(drain.read_offset("labels_offset_bytes"), 200)
        self.assertEqual(drain.read_offset("track_offset_bytes"), 300)


if __name__ == "__main__":
    unittest.main()


class ParameterBinding(unittest.TestCase):
    """Every placeholder must have a value, and every value a placeholder.

    Binding was positional until 2026-08-13, with retain_until spliced in twice
    by hand. That produced a parameter-count bug that failed only at execute()
    against a live database. Named binding makes the mismatch checkable here,
    and these two tests are what stop a column being added to the INSERT and
    forgotten in normalise(), or the reverse.
    """

    import re as _re
    NAMED = _re.compile(r"%\((\w+)\)s")

    def test_every_placeholder_has_a_value(self):
        placeholders = set(self.NAMED.findall(drain.INSERT))
        self.assertTrue(placeholders, "INSERT uses no named placeholders")
        missing = placeholders - set(drain.normalise(row()))
        self.assertEqual(missing, set(), f"INSERT binds names normalise() omits: {missing}")

    def test_every_value_has_a_placeholder(self):
        extra = set(drain.normalise(row())) - set(self.NAMED.findall(drain.INSERT))
        self.assertEqual(extra, set(), f"normalise() returns names the INSERT drops: {extra}")

    def test_no_positional_placeholders_remain(self):
        # A stray bare %s alongside named ones is a psycopg2 error at execute
        # time, not a parse error here.
        self.assertEqual(drain.INSERT.count("%s"), 0)

    def test_the_column_list_and_the_values_list_are_the_same_length(self):
        head = drain.INSERT.split("(", 1)[1].split(")", 1)[0]
        self.assertEqual(head.count(",") + 1, len(set(self.NAMED.findall(drain.INSERT))))


class DetailColumns(unittest.TestCase):
    """What the panel showed, recorded so a pass can be reconstructed.

    Added 2026-08-13. Until then the log kept identity, one distance, one
    altitude, a phase and a score, and discarded the rest -- so a disputed
    verdict could only be reviewed by replaying a live feed.
    """

    DETAIL = {"bearing_deg": 167.4, "speed_kt": 96, "track_deg": 78.0,
              "vertical_rate_fpm": 0, "rssi_dbm": -18.3, "emitter_category": "A1",
              "airframe_class": "light", "operator_source": "ROSTER",
              "cpa_ground_m": 480.0, "cpa_slant_m": 600.0,
              "cpa_altitude_ft": 1382, "eta_seconds": 42, "ambient_dba": 40.0}

    def test_detail_round_trips(self):
        out = drain.normalise(row(detail=self.DETAIL))
        for key, value in self.DETAIL.items():
            self.assertEqual(out[key], value, key)

    def test_a_row_written_before_detail_existed_still_normalises(self):
        # 700+ such rows are already in the spool and the table.
        out = drain.normalise(row())
        self.assertIsNotNone(out)
        for key in self.DETAIL:
            self.assertIsNone(out[key], key)

    def test_an_unknown_airframe_class_is_dropped_not_stored(self):
        """The schema CHECK would reject the whole row, and one typo upstream
        must not block the spool behind it forever."""
        out = drain.normalise(row(detail={"airframe_class": "helicopter"}))
        self.assertIsNotNone(out)
        self.assertIsNone(out["airframe_class"])

    def test_an_unknown_operator_source_is_dropped_not_stored(self):
        out = drain.normalise(row(detail={"operator_source": "guess"}))
        self.assertIsNone(out["operator_source"])

    def test_a_string_is_never_coerced_into_a_measurement(self):
        out = drain.normalise(row(detail={"bearing_deg": "167.4", "speed_kt": "96"}))
        self.assertIsNone(out["bearing_deg"])
        self.assertIsNone(out["speed_kt"])

    def test_a_bool_is_not_a_number(self):
        # True == 1 in Python, so an unguarded isinstance check stores a
        # vertical rate of 1 fpm for a boolean.
        out = drain.normalise(row(detail={"vertical_rate_fpm": True}))
        self.assertIsNone(out["vertical_rate_fpm"])

    def test_the_ambient_is_recorded_so_a_score_stays_interpretable(self):
        """The same column held an invented relative index before 2026-08-13
        and estimated dBA after. Without the ambient the two are
        indistinguishable."""
        self.assertEqual(drain.normalise(row(detail={"ambient_dba": 40.0}))["ambient_dba"], 40.0)
        self.assertIsNone(drain.normalise(row())["ambient_dba"])
