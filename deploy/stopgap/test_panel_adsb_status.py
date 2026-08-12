#!/usr/bin/env python3
"""Tests for the bedside panel's aircraft proximity forecast.

The ASA783 cases replay a real disagreement between this forecast and
FlightRadar24, captured at 2026-08-11 10:25 local (FlightAS783.jpg). Positions
are georeferenced from that screenshot: the station marker and the PDX terminal
gave two independent pixel scales, 55.0 and 52.0 m/px, which agree, so the
derived aircraft position is good to a few hundred metres.

At that moment the old model rated ASA783 LIKELY with a projected altitude of
0 ft. It was on short final to PDX -- 700 ft, 134 kt, gear down -- and the
runway is 6.4 km north of the station. Every assertion below exists to keep
some part of that failure from coming back.
"""

import importlib.machinery
import os
import pathlib
import unittest

# The real station coordinate lives only in the systemd drop-in on the Pi, never
# in git. These tests do not need it: the screenshot's own scale check showed
# the station sits 6.45 km due south of the PDX terminal (117 px at 55.0 m/px,
# with a 3 px east-west offset that is inside the measurement error). Deriving
# the fixture from PDX reproduces the geometry exactly without committing a home
# address.
PDX_LAT, PDX_LON = 45.5887, -122.5975
STATION_SOUTH_OF_PDX_M = 6450.0

os.environ.setdefault("STATION_LAT", repr(PDX_LAT - STATION_SOUTH_OF_PDX_M / 111132.0))
os.environ.setdefault("STATION_LON", repr(PDX_LON))
os.environ.setdefault("STATION_ALT", "200")

_SCRIPT = pathlib.Path(__file__).with_name("panel-adsb-status")
adsb = importlib.machinery.SourceFileLoader("panel_adsb_status", str(_SCRIPT)).load_module()

# ASA783, georeferenced from the screenshot.
ASA783 = dict(lat=45.5628, lon=-122.5319, altitude_ft=700, category="A3",
              speed_kt=134, track_deg=300, vertical_rate_fpm=-1088, seen=1.0)

TOLERANCE = 1e-9


class LandingIsNotAFlyover(unittest.TestCase):
    def test_short_final_is_not_a_close_pass(self):
        p = adsb.predict_pass(**ASA783)
        self.assertEqual(p["risk"], "LOW")
        self.assertFalse(p["approaching"])
        self.assertIn("PDX", p["terminated_by"])

    def test_projection_never_descends_below_the_runway(self):
        p = adsb.predict_pass(**ASA783)
        self.assertGreaterEqual(p["altitude_ft"], 31)

    def test_runway_floor_is_the_field_not_the_station(self):
        """PDX is at 31 ft and this station near 200 ft. Flooring the descent at
        the station would truncate the projection above the runway."""
        p = adsb.predict_pass(**ASA783)
        self.assertEqual(p["arrival_at"], "PDX")
        self.assertLess(p["altitude_ft"], adsb.STATION_ALT_FT)

    def test_guard_names_the_blocker(self):
        p = adsb.predict_pass(**ASA783)
        self.assertIn("PDX", p["reasons"][0])

    def test_arrival_overflying_the_house_high_is_still_scored(self):
        """Not a blanket arrival suppression: an approach that crosses overhead
        with height to spare is the loudest event this address actually gets."""
        overhead = dict(ASA783, lat=adsb.STATION_LAT - 0.011, lon=adsb.STATION_LON,
                        altitude_ft=2200, track_deg=360, vertical_rate_fpm=-700)
        p = adsb.predict_pass(**overhead)
        self.assertTrue(p["approaching"])
        self.assertIn(p["risk"], ("LIKELY", "POSSIBLE"))

    def test_same_aircraft_climbing_away_is_scored_not_terminated(self):
        departing = dict(ASA783, vertical_rate_fpm=2200, altitude_ft=1500)
        p = adsb.predict_pass(**departing)
        self.assertIsNone(p["terminated_by"])
        self.assertEqual(p["phase"], "CLIMB")


class ThrustStateDominates(unittest.TestCase):
    """~15 dB between takeoff and idle. Ignoring it was the largest error."""

    def test_climb_is_louder_than_descent_at_equal_range(self):
        climb = adsb.noise_index(3000.0, "jet", "CLIMB")
        descent = adsb.noise_index(3000.0, "jet", "DESCENT")
        self.assertGreater(climb - descent, 14.0)

    def test_departing_jet_outranks_nearer_light_aircraft(self):
        jet = adsb.noise_index(4000.0, "jet", "CLIMB")
        light = adsb.noise_index(1000.0, "light", "DESCENT")
        self.assertGreater(jet, light)

    def test_phase_boundaries(self):
        self.assertEqual(adsb.thrust_phase(2000), "CLIMB")
        self.assertEqual(adsb.thrust_phase(-2000), "DESCENT")
        self.assertEqual(adsb.thrust_phase(0), "LEVEL")
        self.assertIsNone(adsb.thrust_phase(None))


class UnknownIsNotOK(unittest.TestCase):
    """Three states. A return we cannot score is never scored as quiet."""

    def test_missing_track_is_unknown_not_low(self):
        p = adsb.predict_pass(45.56, -122.53, 3000, "A3", None, None, 0, 1.0)
        self.assertEqual(p["risk"], "UNKNOWN")
        self.assertIn("no groundspeed or ground track", p["reasons"][0])

    def test_missing_category_is_unknown_not_low(self):
        p = adsb.predict_pass(45.56, -122.53, 3000, None, 250, 250, 1500, 1.0)
        self.assertEqual(p["risk"], "UNKNOWN")
        self.assertIn("emitter category", p["reasons"][0])

    def test_missing_vertical_rate_is_unknown_not_low(self):
        p = adsb.predict_pass(45.56, -122.53, 3000, "A3", 250, 250, None, 1.0)
        self.assertEqual(p["risk"], "UNKNOWN")
        self.assertIn("vertical rate", p["reasons"][0])

    def test_distant_negative_index_is_scored_not_unknown(self):
        """The relative index goes negative far out; truthiness must not read
        that as unscorable."""
        self.assertLess(adsb.noise_index(40000.0, "jet", "CLIMB"), 0.0)
        p = adsb.predict_pass(45.90, -122.10, 30000, "A3", 450, 270, 0, 1.0)
        self.assertEqual(p["risk"], "LOW")
        self.assertIsNotNone(p["noise_index"])

    def test_unknown_never_carries_a_number(self):
        p = adsb.predict_pass(45.56, -122.53, 3000, None, 250, 250, 1500, 1.0)
        self.assertIsNone(p["noise_index"])
        self.assertIsNone(p["ground_distance_m"])

    def test_unknown_class_is_not_folded_into_jet(self):
        self.assertIsNone(adsb.airframe_class(None))
        self.assertIsNone(adsb.airframe_class("A0"))
        self.assertIsNone(adsb.noise_index(1000.0, None, "CLIMB"))


class StationGeometry(unittest.TestCase):
    def test_screenshot_range_matches_the_georeferenced_image(self):
        """Independent check: pixel geometry put ASA783 ~6.0 km ENE of the house."""
        distance, bearing = adsb.geometry(ASA783["lat"], ASA783["lon"], 700)
        self.assertLess(abs(distance - 6020.0), 400.0)
        self.assertLess(abs(bearing - 53.5), 4.0)

    def test_closest_approach_matches_the_georeferenced_image(self):
        """Pixel geometry put the CPA of that 300 deg track at 5.50 km."""
        p = adsb.predict_pass(**ASA783)
        self.assertLess(abs(p["ground_distance_m"] - 5500.0), 400.0)

    def test_pdx_is_north_of_the_station(self):
        field, metres = adsb.nearest_airport(PDX_LAT, PDX_LON)
        self.assertEqual(field["id"], "PDX")
        self.assertLess(metres, TOLERANCE + 1.0)
        distance, bearing = adsb.geometry(PDX_LAT, PDX_LON, 31)
        self.assertLess(abs(distance - STATION_SOUTH_OF_PDX_M), 250.0)
        self.assertLess(min(abs(bearing - 0.0), abs(bearing - 360.0)), 5.0)

    def test_the_runway_corridor_is_not_overhead(self):
        """The structural fact behind every finding here: PDX's 10/28 corridor
        runs 6.4 km north, so straight-in traffic never overflies the house."""
        distance, _ = adsb.geometry(PDX_LAT, PDX_LON + 0.05, 3000)
        self.assertGreater(distance, 6000.0)


class CandidateStability(unittest.TestCase):
    def setUp(self):
        adsb.held = None

    def _report(self, hex_id, index, approaching=True):
        return {"hex": hex_id, "prediction": {"noise_index": index, "approaching": approaching}}

    def test_holds_its_subject_while_it_is_still_approaching(self):
        first = adsb.select_candidate([self._report("AAA", 80.0)])
        self.assertEqual(first["hex"], "AAA")
        held = adsb.select_candidate(
            [self._report("BBB", 95.0), self._report("AAA", 79.0)])
        self.assertEqual(held["hex"], "AAA")

    def test_releases_when_the_subject_has_passed(self):
        adsb.select_candidate([self._report("AAA", 80.0)])
        nxt = adsb.select_candidate(
            [self._report("AAA", 80.0, approaching=False), self._report("BBB", 70.0)])
        self.assertEqual(nxt["hex"], "BBB")

    def test_releases_when_the_subject_disappears(self):
        adsb.select_candidate([self._report("AAA", 80.0)])
        nxt = adsb.select_candidate([self._report("BBB", 70.0)])
        self.assertEqual(nxt["hex"], "BBB")

    def test_no_candidates_is_none_not_a_stale_hold(self):
        adsb.select_candidate([self._report("AAA", 80.0)])
        self.assertIsNone(adsb.select_candidate([]))


class NeverArmed(unittest.TestCase):
    def test_model_note_disclaims_measured_sound(self):
        self.assertIn("no measured sound", adsb.MODEL_NOTE)
        self.assertIn("unvalidated", adsb.MODEL_NOTE)


if __name__ == "__main__":
    unittest.main()
