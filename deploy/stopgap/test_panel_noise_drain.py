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
