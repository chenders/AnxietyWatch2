#!/usr/bin/env python3
"""Tests for the Hue warning light. No bridge, no network.

The restore path is the one that can do harm -- a bedroom lamp left on at 3am --
so most of this file is about capture fidelity and about not firing too often.
"""

import importlib.machinery
import pathlib
import unittest

_HERE = pathlib.Path(__file__).parent
hue = importlib.machinery.SourceFileLoader(
    "panel_hue_warn", str(_HERE / "panel-hue-warn")).load_module()


class CaptureFidelity(unittest.TestCase):
    """What comes back must be what was there, not an approximation."""

    def _capture(self, state):
        hue.call = lambda method, path, body=None: ({"state": state}, None)
        return hue.capture("4")[0]

    def test_a_colour_temperature_light_restores_ct_not_xy(self):
        # The real bedroom lamp: colormode 'ct', but the bridge ALSO reports an
        # xy value. Restoring xy would return the lamp a different colour than
        # it was, which is the subtle version of getting this wrong.
        got = self._capture({"on": True, "bri": 254, "colormode": "ct",
                             "ct": 346, "xy": [0.445, 0.4067],
                             "hue": 8596, "sat": 121})
        self.assertEqual(got, {"on": True, "bri": 254, "ct": 346})
        self.assertNotIn("xy", got)

    def test_an_xy_light_restores_xy(self):
        got = self._capture({"on": True, "bri": 100, "colormode": "xy",
                             "xy": [0.2, 0.3], "ct": 200})
        self.assertEqual(got["xy"], [0.2, 0.3])
        self.assertNotIn("ct", got)

    def test_a_hue_sat_light_restores_both(self):
        got = self._capture({"on": True, "bri": 40, "colormode": "hs",
                             "hue": 12000, "sat": 200, "xy": [0.4, 0.4]})
        self.assertEqual(got["hue"], 12000)
        self.assertEqual(got["sat"], 200)
        self.assertNotIn("xy", got)

    def test_a_light_that_was_off_restores_its_colour_as_well_as_being_off(self):
        # Restoring only {"on": False} looks right -- the light is off, as it
        # was -- but leaves the bridge holding the FLASH colour as that light's
        # stored state, so the next manual switch-on comes up cyan. The failure
        # is invisible at the time and surfaces hours later.
        #
        # The bridge accepts bri/ct alongside "on": False without lighting the
        # room, verified against the real hardware.
        got = self._capture({"on": False, "bri": 155, "colormode": "ct", "ct": 300})
        self.assertEqual(got, {"on": False, "bri": 155, "ct": 300})

    def test_an_off_light_still_follows_colormode(self):
        got = self._capture({"on": False, "bri": 80, "colormode": "xy",
                             "xy": [0.3, 0.35], "ct": 250})
        self.assertEqual(got, {"on": False, "bri": 80, "xy": [0.3, 0.35]})
        self.assertNotIn("ct", got)

    def test_a_light_with_no_state_is_an_error_not_a_guess(self):
        hue.call = lambda method, path, body=None: ({"state": {}}, None)
        state, error = hue.capture("4")
        self.assertIsNone(state)
        self.assertIn("no state", error)

    def test_a_bridge_error_propagates_rather_than_being_swallowed(self):
        hue.call = lambda method, path, body=None: (None, "bridge unreachable")
        state, error = hue.capture("4")
        self.assertIsNone(state)
        self.assertIn("unreachable", error)


class WhenItFires(unittest.TestCase):
    def _status(self, band, eta, index=91.0, proximity="OVERHEAD"):
        return {"adsb_predictions": [{
            "hex": "AA8051", "registration": "N776LF",
            "prediction": {"at_epoch": 1000.0 + eta, "loudness_band": band,
                           "noise_index": index, "proximity": proximity}}]}

    def test_a_loud_aircraft_inside_the_lead_window_fires(self):
        picked = hue.choose(self._status("LOUD", 20), 1000.0)
        self.assertIsNotNone(picked)
        self.assertEqual(picked[1], "LOUD")

    def test_a_quiet_aircraft_does_not(self):
        self.assertIsNone(hue.choose(self._status("FAINT", 20), 1000.0))
        self.assertIsNone(hue.choose(self._status("AUDIBLE", 20), 1000.0))

    def test_an_aircraft_too_far_ahead_does_not_fire_yet(self):
        # Warning three minutes early trains the household to ignore it. The
        # real budget is ~34 s.
        self.assertIsNone(hue.choose(self._status("LOUD", 300), 1000.0))

    def test_an_aircraft_already_past_does_not_fire(self):
        self.assertIsNone(hue.choose(self._status("LOUD", -30), 1000.0))

    def test_an_unscored_aircraft_overhead_fires(self):
        # Absence of a score is not evidence of quiet, and air ambulances are
        # over-represented among aircraft broadcasting no category.
        picked = hue.choose(self._status("UNKNOWN", 20, index=None), 1000.0)
        self.assertIsNotNone(picked)
        self.assertEqual(picked[1], "UNKNOWN")

    def test_an_unscored_aircraft_that_is_NOT_overhead_does_not_fire(self):
        self.assertIsNone(hue.choose(
            self._status("UNKNOWN", 20, index=None, proximity="DISTANT"), 1000.0))

    def test_a_prediction_with_no_time_is_ignored_rather_than_assumed_imminent(self):
        status = {"adsb_predictions": [{"prediction": {"loudness_band": "LOUD"}}]}
        self.assertIsNone(hue.choose(status, 1000.0))

    def test_an_empty_feed_fires_nothing(self):
        self.assertIsNone(hue.choose({}, 1000.0))
        self.assertIsNone(hue.choose({"adsb_predictions": []}, 1000.0))


class Colour(unittest.TestCase):
    def test_red_is_never_used(self):
        # Klaxon red belongs to the medical alarm and appears nowhere else. A
        # bedroom turning red for an aeroplane teaches the household to
        # distrust the colour that matters.
        for xy in (hue.XY_CYAN, hue.XY_AMBER):
            self.assertLess(xy[0], 0.6, f"{xy} is in the red corner of CIE xy")

    def test_the_cooldown_is_long_enough_to_not_strobe_a_bedroom(self):
        self.assertGreaterEqual(hue.COOLDOWN_SECONDS, 300)

    def test_the_lead_window_is_near_the_real_warning_budget(self):
        # Measured: ~34 s from first fix to closest approach off the Providence
        # pad. A much longer window would fire before anything is audible.
        self.assertLessEqual(hue.LEAD_SECONDS, 90)


if __name__ == "__main__":
    unittest.main()


class FlashDuration(unittest.TestCase):
    def test_the_alert_is_refreshed_before_the_bridge_drops_it(self):
        # `lselect` stops after roughly 15 s. If the refresh cadence were not
        # shorter than that, a long flash would sit solid for its tail.
        self.assertLess(hue.ALERT_REFRESH_SECONDS, 15.0)

    def test_the_flash_is_shorter_than_the_lead_time(self):
        # Otherwise the light would still be pulsing after the aircraft has
        # already passed, which reads as a stuck warning rather than a warning.
        self.assertLess(hue.FLASH_SECONDS, hue.LEAD_SECONDS)


class FlashBrightness(unittest.TestCase):
    def test_the_flash_is_bright_enough_to_notice_but_within_range(self):
        self.assertGreaterEqual(hue.FLASH_BRIGHTNESS, 190)
        self.assertLessEqual(hue.FLASH_BRIGHTNESS, 254)

    def test_brightness_is_clamped_to_the_hue_scale(self):
        # Hue rejects anything outside 1-254; an out-of-range env value should
        # be clamped rather than sent and rejected mid-warning.
        self.assertGreaterEqual(hue.FLASH_BRIGHTNESS, 1)


class SecondZone(unittest.TestCase):
    """The kitchen set flashes red alongside the bedroom's cyan."""

    def setUp(self):
        self._lights, self._alert = hue.LIGHTS, hue.ALERT_LIGHTS
        hue.LIGHTS, hue.ALERT_LIGHTS = ["2", "8"], ["10"]

    def tearDown(self):
        hue.LIGHTS, hue.ALERT_LIGHTS = self._lights, self._alert

    def test_both_zones_flash_in_their_own_colour(self):
        plan = hue.zones("LOUD")
        self.assertEqual(len(plan), 2)
        (bed_ids, bed_colour, bed_bri), (kit_ids, kit_colour, kit_bri) = plan
        self.assertEqual(bed_ids, ["2", "8"])
        self.assertEqual(bed_colour, hue.XY_PURPLE)
        self.assertEqual(kit_ids, ["10"])
        self.assertEqual(kit_colour, hue.XY_RED)

    def test_the_bedroom_never_goes_red(self):
        # Red belongs to the medical alarm. A bedroom turning red at 3am for an
        # aeroplane erodes that, whatever the kitchen is doing.
        for band in ("LOUD", "VERY LOUD", "UNKNOWN"):
            bed_colour = hue.zones(band)[0][1]
            self.assertNotEqual(bed_colour, hue.XY_RED, band)

    def test_the_bedroom_colour_is_the_same_for_every_band(self):
        colours = {hue.zones(b)[0][1][0] for b in ("LOUD", "VERY LOUD", "UNKNOWN")}
        self.assertEqual(len(colours), 1)

    def test_the_kitchen_colour_does_not_track_the_band(self):
        # Its job is to be unmistakable from another room, not to encode a
        # gradient the reader cannot see the rest of.
        self.assertEqual(hue.zones("LOUD")[1][1], hue.zones("VERY LOUD")[1][1])

    def test_kitchen_is_at_full_brightness(self):
        # It is the only light in that room carrying the warning.
        self.assertEqual(hue.ALERT_BRIGHTNESS, 254)

    def test_with_no_alert_lights_configured_there_is_only_one_zone(self):
        hue.ALERT_LIGHTS = []
        self.assertEqual(len(hue.zones("LOUD")), 1)


class WhatWarnActuallyApplies(unittest.TestCase):
    """Tests the STATE SENT TO THE BRIDGE, not the plan that describes it.

    The earlier tests exercised zones() and passed while warn() was applying the
    last zone's colour to every light -- the bedroom went red at the kitchen's
    setting during a live test. A correct plan proves nothing if the code that
    consumes it ignores it.
    """

    def setUp(self):
        self._c, self._a, self._l, self._al = (
            hue.call, hue.apply_state, hue.LIGHTS, hue.ALERT_LIGHTS)
        self._flash = hue.FLASH_SECONDS
        hue.LIGHTS, hue.ALERT_LIGHTS = ["2", "8"], ["10"]
        hue.FLASH_SECONDS = 0.0
        self.applied = []
        hue.call = lambda m, p, b=None: (
            {"state": {"on": True, "bri": 254, "colormode": "ct", "ct": 300}}, None)
        def record(light_id, state):
            self.applied.append((light_id, dict(state)))
            return (None, None)
        hue.apply_state = record

    def tearDown(self):
        hue.call, hue.apply_state = self._c, self._a
        hue.LIGHTS, hue.ALERT_LIGHTS = self._l, self._al
        hue.FLASH_SECONDS = self._flash

    def _flashes(self):
        hue.warn("LOUD", "test")
        return {lid: st for lid, st in self.applied if st.get("alert") == "lselect"
                and "xy" in st}

    def test_the_bedroom_flashes_purple_and_the_kitchen_red(self):
        f = self._flashes()
        self.assertEqual(f["2"]["xy"], hue.XY_PURPLE)
        self.assertEqual(f["8"]["xy"], hue.XY_PURPLE)
        self.assertEqual(f["10"]["xy"], hue.XY_RED)

    def test_the_bedroom_is_never_sent_red(self):
        for band in ("LOUD", "VERY LOUD", "UNKNOWN"):
            self.applied.clear()
            hue.warn(band, "test")
            for lid, st in self.applied:
                if lid in ("2", "8") and "xy" in st:
                    self.assertNotEqual(st["xy"], hue.XY_RED,
                                        f"bedroom light {lid} sent red on {band}")

    def test_each_zone_gets_its_own_brightness(self):
        f = self._flashes()
        self.assertEqual(f["2"]["bri"], hue.FLASH_BRIGHTNESS)
        self.assertEqual(f["10"]["bri"], hue.ALERT_BRIGHTNESS)
        self.assertNotEqual(f["2"]["bri"], f["10"]["bri"])

    def test_every_light_touched_is_also_restored(self):
        hue.warn("LOUD", "test")
        touched = {lid for lid, _ in self.applied}
        restored = {lid for lid, st in self.applied if st.get("alert") == "none"}
        self.assertEqual(touched, restored)
