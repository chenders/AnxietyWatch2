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
        self.assertEqual(out[1], "adsb")
        self.assertEqual(out[2], "A5731C")
        self.assertIsNone(out[-1], "receiver rows must carry no deletion deadline")

    def test_hex_is_upper_cased_to_match_the_roster(self):
        out = drain.normalise(row(identity={"hex": "a5731c"}))
        self.assertEqual(out[2], "A5731C")

    def test_an_unscored_row_keeps_its_null(self):
        # The row this whole system exists to catch. It must never acquire a
        # number on the way into the database.
        out = drain.normalise(row(noise_index=None, loudness_band="UNKNOWN"))
        self.assertIsNone(out[10])
        self.assertEqual(out[11], "UNKNOWN")

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
        self.assertEqual(out[1], "fr24")
        self.assertEqual(out[-1], 1802253546.0)

    def test_a_receiver_row_never_inherits_a_deadline(self):
        # Even if one is somehow present in the spool.
        out = drain.normalise(row(provenance="adsb", retain_until=1802253546.0))
        self.assertIsNone(out[-1])

    def test_junk_is_rejected_rather_than_crashing_the_batch(self):
        for bad in (None, [], "text", {}, {"at": "soon", "provenance": "adsb"},
                    row(provenance="guess"), row(loudness_band="VERY QUIET")):
            self.assertIsNone(drain.normalise(bad))

    def test_an_unrecognised_phase_becomes_null_rather_than_being_invented(self):
        self.assertIsNone(drain.normalise(row(phase="HOVER"))[9])
        self.assertEqual(drain.normalise(row(phase="CLIMB"))[9], "CLIMB")


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
    """The INSERT builds its last column with a CASE that needs the value twice.

    Getting the count wrong fails only at execute() against a live database,
    which is exactly where it did fail. Counting statically catches it here.
    """

    def test_placeholder_count_matches_the_bound_tuple(self):
        placeholders = drain.INSERT.count("%s")
        sample = drain.normalise(row())
        bound = sample[:15] + (sample[15], sample[15])
        self.assertEqual(placeholders, len(bound))

    def test_a_normalised_row_has_one_field_per_column(self):
        columns = drain.INSERT.split("(", 1)[1].split(")", 1)[0].count(",") + 1
        self.assertEqual(len(drain.normalise(row())), columns)
