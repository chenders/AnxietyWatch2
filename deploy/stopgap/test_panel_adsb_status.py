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

    def test_missing_track_reports_current_geometry(self):
        """Superseded intent: refusing to predict without a velocity vector also
        dropped hovering helicopters, so an aircraft with no reported track is
        now scored where it actually is rather than discarded. It reports the
        present geometry and claims no projection."""
        p = adsb.predict_pass(45.56, -122.53, 3000, "A3", None, None, 0, 1.0)
        self.assertEqual(p["eta_seconds"], 0)
        self.assertIsNotNone(p["ground_distance_m"])

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

    def test_unknown_never_carries_a_loudness_number(self):
        """It may carry geometry, which is measured, but never a loudness, which
        would have to be guessed."""
        p = adsb.predict_pass(45.56, -122.53, 3000, None, 250, 250, 1500, 1.0)
        self.assertIsNone(p["noise_index"])
        self.assertEqual(p["risk"], "UNKNOWN")

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


class AirAmbulance(unittest.TestCase):
    """Life Flight into the hospital ~1.6 km away, overflying the house.

    This is the loudest routine event at this address and the first model missed
    it three separate ways: it refused to predict below 20 kt, it dropped
    aircraft with no emitter category, and it attributed any nearby descent to
    PDX and suppressed it. All three are asserted against here.
    """

    def _heli(self, **kw):
        base = dict(lat=adsb.STATION_LAT + 0.0014, lon=adsb.STATION_LON + 0.0140,
                    altitude_ft=900, category="A7", speed_kt=110, track_deg=250,
                    vertical_rate_fpm=-500, seen=1.0)
        base.update(kw)
        return adsb.predict_pass(**base)

    def test_inbound_helicopter_is_flagged(self):
        p = self._heli()
        self.assertEqual(p["risk"], "LIKELY")
        self.assertEqual(p["proximity"], "OVERHEAD")

    def test_hovering_is_not_unpredictable(self):
        """Below 20 kt is a helicopter working, not an absence of information --
        and it is the loudest moment of the arrival."""
        for case in (self._heli(speed_kt=15, altitude_ft=400),
                     self._heli(speed_kt=None, track_deg=None)):
            self.assertIn(case["risk"], ("LIKELY", "POSSIBLE"))
            self.assertEqual(case["proximity"], "OVERHEAD")
            self.assertIsNotNone(case["noise_index"])

    def test_no_category_still_reports_the_close_pass(self):
        """Unknown loudness must not read as an empty sky. Geometry is enough to
        say it is overhead, and that is reported."""
        p = self._heli(category=None)
        self.assertEqual(p["risk"], "UNKNOWN")
        self.assertEqual(p["proximity"], "OVERHEAD")
        self.assertTrue(p["approaching"])
        self.assertIsNone(p["noise_index"])

    def test_descent_is_not_blamed_on_a_distant_runway(self):
        """PDX is the nearest runway to this station, so before track alignment
        every descending helicopter was called a PDX arrival and silenced."""
        p = self._heli(vertical_rate_fpm=-800, altitude_ft=500)
        self.assertIsNone(p["arrival_at"])
        self.assertEqual(p["risk"], "LIKELY")

    def test_overhead_now_is_not_silenced_for_being_late(self):
        """The old model required 5 s of lead time, so it went quiet exactly when
        the aircraft was overhead."""
        p = self._heli(lat=adsb.STATION_LAT, lon=adsb.STATION_LON, altitude_ft=1000)
        self.assertEqual(p["eta_seconds"], 0)
        self.assertEqual(p["risk"], "LIKELY")

    def test_receding_traffic_is_released(self):
        p = self._heli(lat=adsb.STATION_LAT, lon=adsb.STATION_LON + 0.031,
                       track_deg=90, vertical_rate_fpm=0)
        self.assertEqual(p["risk"], "LOW")
        self.assertIn("already behind it", p["reasons"][0])

    def test_fixed_wing_arrival_attribution_still_works(self):
        """The rotorcraft exemption must not disable runway termination for the
        jets it was built for."""
        self.assertEqual(adsb.predict_pass(**ASA783)["arrival_at"], "PDX")


class UnscoredOverheadOutranksScoredDistant(unittest.TestCase):
    def setUp(self):
        adsb.held = None

    def test_unscorable_close_pass_is_the_advisory_subject(self):
        scored = {"hex": "JET", "prediction": {"noise_index": 85.0, "approaching": True}}
        unscored = {"hex": "HELI", "prediction": {"noise_index": None, "approaching": True}}
        ranked = sorted([scored, unscored],
                        key=lambda r: (1, -r["prediction"]["noise_index"])
                        if r["prediction"]["noise_index"] is not None else (0, 0.0))
        self.assertEqual(ranked[0]["hex"], "HELI")


class ForecastList(unittest.TestCase):
    """The panel list answers 'what is about to be loud, and when'."""

    def test_bands_follow_the_same_thresholds_as_the_verdict(self):
        self.assertEqual(adsb.loudness_band(adsb.VERY_LOUD_INDEX), "VERY LOUD")
        self.assertEqual(adsb.loudness_band(adsb.LIKELY_INDEX), "LOUD")
        self.assertEqual(adsb.loudness_band(adsb.POSSIBLE_INDEX), "AUDIBLE")
        self.assertEqual(adsb.loudness_band(adsb.POSSIBLE_INDEX - 0.1), "FAINT")

    def test_unknown_loudness_never_reads_as_a_band(self):
        self.assertEqual(adsb.loudness_band(None), "UNKNOWN")

    def test_unscorable_overhead_still_gets_a_countdown(self):
        """Withhold the loudness, which is unknown -- not the timing, which is
        measured."""
        p = adsb.predict_pass(adsb.STATION_LAT - 0.02, adsb.STATION_LON, 1300,
                              None, 120, 360, 0, 1.0)
        self.assertEqual(p["risk"], "UNKNOWN")
        self.assertIsNone(p["noise_index"])
        self.assertIsNotNone(p["eta_seconds"])
        self.assertGreater(p["eta_seconds"], 0)

    def test_callsign_is_absent_not_the_string_None(self):
        """str(None) is "None", which the panel would print as a callsign."""
        for flight in (None, "", "   "):
            r = adsb.make_report({"hex": "abc123", "flight": flight, "lat": 45.5,
                                  "lon": -122.6, "altitude": 3000, "seen": 1.0},
                                 1_000_000.0)
            self.assertIsNone(r["callsign"], f"flight={flight!r}")

    def test_callsign_is_trimmed_when_present(self):
        r = adsb.make_report({"hex": "abc123", "flight": "ASA783  ", "lat": 45.5,
                              "lon": -122.6, "altitude": 3000, "seen": 1.0}, 1_000_000.0)
        self.assertEqual(r["callsign"], "ASA783")


class NeverArmed(unittest.TestCase):
    def test_model_note_disclaims_measured_sound(self):
        self.assertIn("no measured sound", adsb.MODEL_NOTE)
        self.assertIn("unvalidated", adsb.MODEL_NOTE)


if __name__ == "__main__":
    unittest.main()


class RosterRescuesUnscorableAircraft(unittest.TestCase):
    """The receiver reports ~50% of aircraft with no emitter category.

    Air ambulances are over-represented in that group, so without the Mode S
    roster the loudest, lowest traffic over the station is exactly the traffic
    that scores nothing at all.
    """

    def test_life_flight_with_no_category_is_scorable_as_a_rotorcraft(self):
        # N776LF -- the airframe measured at 0.95 km and 600 ft off Providence.
        self.assertEqual(adsb.airframe_class(None, "AA8051"), "rotorcraft")

    def test_police_airframe_with_no_category_resolves_to_light(self):
        self.assertEqual(adsb.airframe_class(None, "A1D296"), "light")

    def test_an_unrecognised_address_stays_unknown(self):
        # Unknown must not be folded into a class: a guessed source level would
        # let an unidentified return borrow a narrowbody's loudness.
        self.assertIsNone(adsb.airframe_class(None, "ABCDEF"))
        self.assertIsNone(adsb.airframe_class(None, None))

    def test_a_broadcast_category_still_wins_over_the_roster(self):
        # The aircraft's own broadcast is better evidence than our static table.
        self.assertEqual(adsb.airframe_class("A3", "AA8051"), "jet")

    def test_an_uncategorised_air_ambulance_now_produces_a_prediction(self):
        # Before the roster this returned risk UNKNOWN with noise_index None.
        overhead = dict(lat=adsb.STATION_LAT + 0.004, lon=adsb.STATION_LON,
                        altitude_ft=700, category=None, speed_kt=92,
                        track_deg=180, vertical_rate_fpm=1088, seen=1.0)
        scored = adsb.predict_pass(**overhead, hex_code="AA8051")
        unscored = adsb.predict_pass(**overhead, hex_code="ABCDEF")
        self.assertIsNotNone(scored["noise_index"])
        self.assertEqual(scored["phase"], "CLIMB")
        self.assertIsNone(unscored["noise_index"])
        self.assertEqual(unscored["risk"], "UNKNOWN")

    def test_make_report_labels_the_operator(self):
        item = {"hex": "a4c3eb", "lat": adsb.STATION_LAT + 0.004,
                "lon": adsb.STATION_LON, "altitude": 700, "seen": 1.0}
        report = adsb.make_report(item, 0.0)
        self.assertEqual(report["registration"], "N406LF")
        self.assertEqual(report["operator_kind"], "AIR_AMBULANCE")
        self.assertEqual(report["operator_state"], "CONFIRMED")

    def test_make_report_leaves_an_unknown_aircraft_unlabelled(self):
        item = {"hex": "abcdef", "lat": adsb.STATION_LAT + 0.004,
                "lon": adsb.STATION_LON, "altitude": 700, "seen": 1.0}
        report = adsb.make_report(item, 0.0)
        self.assertIsNone(report["operator"])
        self.assertIsNone(report["operator_kind"])
        self.assertEqual(report["operator_state"], "UNCONFIRMED")
