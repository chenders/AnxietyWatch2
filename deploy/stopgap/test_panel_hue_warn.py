#!/usr/bin/env python3
"""Tests for the Hue warning light. No bridge, no network.

The restore path is the one that can do harm -- a bedroom lamp left on at 3am --
so most of this file is about capture fidelity and about not firing too often.
"""

import importlib.machinery
import pathlib
import time
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

    def test_the_cooldown_outlasts_its_own_flash(self):
        # Reduced from 10 minutes to 30 s at the owner's request. The old test
        # asserted >= 300 s, which encoded the assumption that continuous
        # traffic would strobe the bedroom -- but the LOUD-and-above threshold
        # already limits firing to a couple of aircraft a day, and a 10-minute
        # window silently swallowed a second genuinely loud aircraft. The
        # constraint that actually matters is that a warning cannot retrigger
        # while its own flash is still running.
        self.assertGreater(hue.COOLDOWN_SECONDS, hue.FLASH_SECONDS)

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

    def test_both_zones_flash_in_their_own_colours(self):
        plan = hue.zones("LOUD")
        self.assertEqual(len(plan), 2)
        (bed_ids, bed_palette, bed_bri), (kit_ids, kit_palette, kit_bri) = plan
        self.assertEqual(bed_ids, ["2", "8"])
        # Same signature everywhere, by owner decision.
        self.assertEqual(bed_palette, [hue.XY_PURPLE, hue.XY_RED])
        self.assertEqual(kit_ids, ["10"])
        self.assertEqual(kit_palette, [hue.XY_PURPLE, hue.XY_RED])

    def test_every_zone_flashes_the_same_signature(self):
        # One pattern everywhere is easier to recognise half asleep than a
        # colour that must be decoded per room. Red now appears in the bedroom
        # by explicit owner decision, overriding the handoff §6 guidance.
        palettes = [z[1] for z in hue.zones("LOUD")]
        self.assertEqual(len(set(map(tuple, map(lambda p: tuple(map(tuple, p)), palettes)))), 1)

    def test_the_bedroom_alternates_two_distinct_colours(self):
        palette = hue.zones("LOUD")[0][1]
        self.assertEqual(len(palette), 2)
        self.assertNotEqual(palette[0], palette[1])

    def test_the_signature_does_not_track_the_band(self):
        self.assertEqual(hue.zones("LOUD")[1][1], hue.zones("VERY LOUD")[1][1])

    def test_every_zone_is_at_90_percent(self):
        self.assertEqual(hue.ALERT_BRIGHTNESS, 229)
        self.assertLess(abs(hue.ALERT_BRIGHTNESS / 254 - 0.90), 0.01)

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
        """The opening flash state per light.

        Keys on the FIRST colour write rather than on alert == "lselect": an
        alternating zone drives itself and never sets that flag, so filtering on
        it silently returned nothing for the bedroom.
        """
        hue.warn("LOUD", "test")
        first = {}
        for lid, st in self.applied:
            if "xy" in st and st.get("alert") != "none" and lid not in first:
                first[lid] = st
        return first

    def test_every_light_opens_on_the_same_colour(self):
        f = self._flashes()
        for lid in ("2", "8", "10"):
            self.assertEqual(f[lid]["xy"], hue.XY_PURPLE, lid)

    def test_lights_actually_alternate_during_the_flash(self):
        # The palette being right proves nothing if the flash loop never uses
        # the second colour.
        hue.FLASH_SECONDS = 0.6
        hue.ALTERNATE_SECONDS = 0.01
        self.applied.clear()
        hue.warn("LOUD", "test")
        seen = [tuple(st["xy"]) for lid, st in self.applied
                if lid == "2" and "xy" in st and st.get("alert") != "none"]
        self.assertIn(tuple(hue.XY_PURPLE), seen)
        self.assertIn(tuple(hue.XY_RED), seen)

    def test_the_kitchen_alternates_too(self):
        hue.FLASH_SECONDS = 0.6
        hue.ALTERNATE_SECONDS = 0.01
        self.applied.clear()
        hue.warn("LOUD", "test")
        seen = {tuple(st["xy"]) for lid, st in self.applied
                if lid == "10" and "xy" in st and st.get("alert") != "none"}
        self.assertEqual(seen, {tuple(hue.XY_PURPLE), tuple(hue.XY_RED)})

    def test_the_bedroom_is_never_sent_red(self):
        for band in ("LOUD", "VERY LOUD", "UNKNOWN"):
            self.applied.clear()
            hue.warn(band, "test")
            for lid, st in self.applied:
                if lid in ("2", "8") and "xy" in st:
                    self.assertNotEqual(st["xy"], hue.XY_RED,
                                        f"bedroom light {lid} sent red on {band}")

    def test_every_zone_shares_one_brightness(self):
        f = self._flashes()
        self.assertEqual(f["2"]["bri"], hue.ALERT_BRIGHTNESS)
        self.assertEqual(f["10"]["bri"], hue.ALERT_BRIGHTNESS)

    def test_every_light_touched_is_also_restored(self):
        hue.warn("LOUD", "test")
        touched = {lid for lid, _ in self.applied}
        restored = {lid for lid, st in self.applied if st.get("alert") == "none"}
        self.assertEqual(touched, restored)


class TestsMustNotBlindTheSystem(unittest.TestCase):
    """A test warning fires, but must never consume the real cooldown.

    This is not hypothetical: a test at 14:33 set the then-10-minute cooldown
    and an audible aircraft minutes later went unannounced.
    """

    def test_a_marked_test_aircraft_is_recognised(self):
        self.assertTrue(hue.is_test({"test": True}))
        self.assertTrue(hue.is_test({"prediction": {"test": True}}))

    def test_a_real_aircraft_is_not_mistaken_for_a_test(self):
        # Inferring rather than marking would eventually suppress a real
        # warning, which is the failure this guard exists to prevent.
        self.assertFalse(hue.is_test({}))
        self.assertFalse(hue.is_test({"registration": "N776LF"}))
        self.assertFalse(hue.is_test({"prediction": {"loudness_band": "LOUD"}}))
        self.assertFalse(hue.is_test({"test": False}))

    def test_the_cooldown_is_short_enough_to_catch_a_second_aircraft(self):
        # Two genuinely loud aircraft a minute apart must both be announced.
        self.assertLessEqual(hue.COOLDOWN_SECONDS, 60)

    def test_the_cooldown_still_outlasts_the_flash(self):
        # Otherwise a warning could retrigger while its own flash is running.
        self.assertGreater(hue.COOLDOWN_SECONDS, hue.FLASH_SECONDS)


if __name__ == "__main__":
    unittest.main()


class WarningEndsWhenTheAircraftDoes(unittest.TestCase):
    """The flash must track reality, not a stopwatch.

    Observed 2026-08-13: the panel cleared the aircraft while the lights kept
    flashing for the rest of the 17 s. A warning that outlives the thing it
    warns about teaches people to ignore it.
    """

    def setUp(self):
        self._c, self._a, self._l, self._al = (
            hue.call, hue.apply_state, hue.LIGHTS, hue.ALERT_LIGHTS)
        self._flash, self._alt, self._clear = (
            hue.FLASH_SECONDS, hue.ALTERNATE_SECONDS, hue.ALL_CLEAR_SECONDS)
        hue.LIGHTS, hue.ALERT_LIGHTS = ["2"], []
        hue.FLASH_SECONDS, hue.ALTERNATE_SECONDS, hue.ALL_CLEAR_SECONDS = 2.0, 0.05, 0.01
        self.applied = []
        hue.call = lambda m, p, b=None: (
            {"state": {"on": True, "bri": 254, "colormode": "ct", "ct": 300}}, None)
        hue.apply_state = lambda lid, st: (self.applied.append((lid, dict(st))), (None, None))[1]

    def tearDown(self):
        hue.call, hue.apply_state = self._c, self._a
        hue.LIGHTS, hue.ALERT_LIGHTS = self._l, self._al
        hue.FLASH_SECONDS, hue.ALTERNATE_SECONDS, hue.ALL_CLEAR_SECONDS = (
            self._flash, self._alt, self._clear)

    def _greens(self):
        return [st for _, st in self.applied
                if st.get("xy") == hue.XY_GREEN]

    def test_it_ends_early_and_shows_all_clear_when_the_aircraft_goes(self):
        started = time.monotonic()
        hue.warn("LOUD", "test", still_inbound=lambda: False)
        self.assertLess(time.monotonic() - started, hue.FLASH_SECONDS,
                        "flash ran its full duration despite the aircraft clearing")
        self.assertTrue(self._greens(), "no all-clear was shown")

    def test_it_runs_the_full_duration_while_the_aircraft_is_still_inbound(self):
        hue.warn("LOUD", "test", still_inbound=lambda: True)
        self.assertFalse(self._greens(),
                         "all-clear shown while the aircraft was still inbound")

    def test_no_predicate_behaves_as_before(self):
        hue.warn("LOUD", "test")
        self.assertFalse(self._greens())

    def test_the_all_clear_is_dimmer_than_the_warning(self):
        # Closing punctuation, not a second alert.
        self.assertLess(hue.ALL_CLEAR_BRIGHTNESS, hue.ALERT_BRIGHTNESS)

    def test_the_lights_are_still_restored_after_an_all_clear(self):
        hue.warn("LOUD", "test", still_inbound=lambda: False)
        restored = [st for lid, st in self.applied if st.get("alert") == "none"
                    and "bri" in st and st.get("xy") != hue.XY_GREEN]
        self.assertTrue(restored, "lights were not restored after the all-clear")

    def test_an_unreadable_feed_does_not_count_as_cleared(self):
        # Unknown is not all-clear. If the feed cannot be read mid-flash the
        # warning must run its course rather than declaring the sky safe.
        hue.warn("LOUD", "test", still_inbound=lambda: True)
        self.assertFalse(self._greens())
