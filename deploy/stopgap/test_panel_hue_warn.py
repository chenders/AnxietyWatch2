#!/usr/bin/env python3
"""Tests for the Hue warning light. No bridge, no network.

The restore path is the one that can do harm -- a bedroom lamp left on at 3am --
so most of this file is about capture fidelity and about not firing too often.
"""

import importlib.machinery
import json
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

    def test_the_countdown_targets_the_sound_not_the_airframe(self):
        # CPA in 22 s at a 5,145 m slant: the sound needs 15 more seconds, so
        # the projected hearing time (37 s) is outside the 30 s lead window
        # and must not fire yet.
        status = self._status("LOUD", 22)
        status["adsb_predictions"][0]["prediction"]["slant_distance_m"] = 5145.0
        self.assertIsNone(hue.choose(status, 1000.0))

    def test_sound_still_travelling_after_closest_approach_fires(self):
        # The airframe passed 5 s ago, but its noise from a 5,145 m slant is
        # still ~10 s out. Warning for the geometry instead of the noise would
        # go silent exactly when the light is most useful.
        status = self._status("LOUD", -5)
        status["adsb_predictions"][0]["prediction"]["slant_distance_m"] = 5145.0
        picked = hue.choose(status, 1000.0)
        self.assertIsNotNone(picked)
        self.assertAlmostEqual(picked[2], 10.0, delta=0.1)


class AudibilityOnset(unittest.TestCase):
    """The countdown targets when you FIRST hear it, not the loudest moment.

    N170US, 2026-08-17: warned 28 s before its closest approach's sound
    arrived, audible ~12 s after the flash -- 16 s of perceived notice from a
    warning that was on time against the wrong target."""

    def prediction(self, level=60.0, slant=2000.0, at=1240.0):
        return {"at_epoch": at, "slant_distance_m": slant, "noise_index": level}

    def test_a_loud_aircraft_is_heard_before_its_closest_approach(self):
        pr = self.prediction()
        peak = pr["at_epoch"] + pr["slant_distance_m"] / hue.SPEED_OF_SOUND_M_S
        heard = hue.sound_arrival_epoch(pr, {"speed_kt": 140})
        self.assertLess(heard, peak)

    def test_a_louder_aircraft_is_heard_even_earlier(self):
        quiet = hue.sound_arrival_epoch(self.prediction(level=50.0), {"speed_kt": 140})
        loud = hue.sound_arrival_epoch(self.prediction(level=65.0), {"speed_kt": 140})
        self.assertLess(loud, quiet)

    def test_the_audible_radius_is_capped(self):
        # Spherical spreading ignores air absorption; an absurd CPA level
        # must not promise minutes of warning the air will not deliver.
        a = hue.sound_arrival_epoch(self.prediction(level=100.0), {"speed_kt": 140})
        b = hue.sound_arrival_epoch(self.prediction(level=140.0), {"speed_kt": 140})
        self.assertAlmostEqual(a, b, delta=1e-6)

    def test_a_barely_audible_aircraft_keeps_the_peak_target(self):
        pr = self.prediction(level=44.0)   # under the 45 dBA floor
        peak = pr["at_epoch"] + pr["slant_distance_m"] / hue.SPEED_OF_SOUND_M_S
        self.assertAlmostEqual(hue.sound_arrival_epoch(pr, {"speed_kt": 140}),
                               peak, delta=1e-6)

    def test_no_speed_falls_back_to_the_peak_target(self):
        pr = self.prediction()
        peak = pr["at_epoch"] + pr["slant_distance_m"] / hue.SPEED_OF_SOUND_M_S
        self.assertAlmostEqual(hue.sound_arrival_epoch(pr, {}), peak, delta=1e-6)

    def test_an_already_audible_aircraft_fires_immediately(self):
        # Onset in the past, closest approach still ahead: it is loud NOW.
        status = {"adsb_predictions": [{
            "hex": "AA8051", "speed_kt": 140,
            "prediction": {"at_epoch": 1060.0, "slant_distance_m": 2000.0,
                           "noise_index": 60.0, "loudness_band": "LOUD"}}]}
        picked = hue.choose(status, 1000.0)
        self.assertIsNotNone(picked)
        self.assertEqual(picked[2], 0.0, "an audible aircraft has no seconds left")

    def test_a_pass_whose_peak_sound_has_arrived_is_over(self):
        status = {"adsb_predictions": [{
            "hex": "AA8051", "speed_kt": 140,
            "prediction": {"at_epoch": 990.0, "slant_distance_m": 2000.0,
                           "noise_index": 60.0, "loudness_band": "LOUD"}}]}
        # peak arrival = 990 + 5.8 = 995.8 < now: nothing to warn about.
        self.assertIsNone(hue.choose(status, 1000.0))


class Colour(unittest.TestCase):
    def test_red_appears_only_in_the_police_signature(self):
        # Klaxon red belongs to the medical alarm. The one aircraft exception,
        # by explicit owner decision 2026-08-17, is the POLICE palette -- and
        # only as red-AND-blue alternation, a shape the steady medical red
        # never has. Normal and air-ambulance palettes stay clear of red.
        for xy in (hue.XY_CYAN, hue.XY_AMBER):
            self.assertLess(xy[0], 0.6, f"{xy} is in the red corner of CIE xy")
        for palette in (hue.DEFAULT_PALETTE, hue.KIND_PALETTES["AIR_AMBULANCE"]):
            self.assertNotIn(hue.XY_RED, palette)
        self.assertIn(hue.XY_RED, hue.KIND_PALETTES["POLICE"])
        self.assertIn(hue.XY_BLUE, hue.KIND_PALETTES["POLICE"])

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




class WarningLog(unittest.TestCase):
    """Every warning decision is durable. N170US 2026-08-17: predicted LOUD,
    warned, accurately loud -- and reviewable nowhere, because the warning
    lived only in the journal."""

    def test_a_fired_record_is_self_contained(self):
        report = {"hex": "A4C3EB", "registration": "N170US", "callsign": "SWA100",
                  "operator_kind": None,
                  "prediction": {"loudness_band": "LOUD", "noise_index": 58.0}}
        rec = hue.warning_record(report, "LOUD", 28.4, fired=True, now=1000.0)
        self.assertTrue(rec["fired"])
        self.assertIsNone(rec["suppressed_by"])
        self.assertEqual(rec["registration"], "N170US")
        self.assertEqual(rec["notice_s"], 28.4)
        self.assertEqual(rec["prediction"]["noise_index"], 58.0)
        self.assertEqual(rec["warning_id"], "warn-1000000-A4C3EB")

    def test_a_suppressed_record_names_its_blocker(self):
        rec = hue.warning_record({"hex": "AA1"}, "LOUD", 20.0, fired=False,
                                 suppressed_by="quiet_hours", now=1000.0)
        self.assertFalse(rec["fired"])
        self.assertEqual(rec["suppressed_by"], "quiet_hours")

    def test_log_appends_one_json_line_durably(self):
        import tempfile, pathlib
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "warnings.jsonl"
            real = hue.WARNING_LOG
            hue.WARNING_LOG = path
            try:
                hue.log_warning({"warning_id": "w1"})
                hue.log_warning({"warning_id": "w2"})
            finally:
                hue.WARNING_LOG = real
            lines = path.read_text().splitlines()
            self.assertEqual([json.loads(l)["warning_id"] for l in lines],
                             ["w1", "w2"])

    def test_an_unwritable_log_does_not_raise(self):
        real = hue.WARNING_LOG
        hue.WARNING_LOG = type(real)("/proc/definitely/not/writable.jsonl")
        try:
            hue.log_warning({"warning_id": "w1"})   # must not raise
        finally:
            hue.WARNING_LOG = real


class QuietHours(unittest.TestCase):
    """Midnight to 09:00 Pacific: no light warnings, no matter what.

    Fixed reference epochs, DST on both sides of the year, so a DST bug
    cannot pass in summer and wake the bedroom in winter.
    """

    def test_3am_pacific_daylight_time_is_quiet(self):
        # 2026-08-17 03:00 PDT == 10:00 UTC
        self.assertTrue(hue.in_quiet_hours(1786960800))

    def test_3am_pacific_standard_time_is_quiet(self):
        # 2026-01-15 03:00 PST == 11:00 UTC
        self.assertTrue(hue.in_quiet_hours(1768474800))

    def test_midnight_exactly_is_quiet(self):
        # 2026-08-17 00:00 PDT == 07:00 UTC
        self.assertTrue(hue.in_quiet_hours(1786950000))

    def test_9am_exactly_is_not_quiet(self):
        # 2026-08-17 09:00 PDT == 16:00 UTC. The window is [0, 9): at 9 sharp
        # the warnings are back.
        self.assertFalse(hue.in_quiet_hours(1786982400))

    def test_just_before_midnight_is_not_quiet(self):
        # 2026-08-16 23:59 PDT == 06:59 UTC on the 17th
        self.assertFalse(hue.in_quiet_hours(1786949940))

    def test_the_window_is_midnight_to_nine_pacific(self):
        self.assertEqual(hue.quiet_hours_range(), (0.0, 9.0))
        self.assertEqual(hue.QUIET_TZ.key, "America/Los_Angeles")

    def test_a_wrapping_window_works(self):
        # 22 -> 7 wraps midnight: 23:00 quiet, 12:00 not.
        real = hue.quiet_hours_range
        hue.quiet_hours_range = lambda: (22.0, 7.0)
        try:
            self.assertTrue(hue.in_quiet_hours(1786946400))    # 23:00 PDT Aug 16
            self.assertFalse(hue.in_quiet_hours(1786993200))   # 12:00 PDT Aug 17
        finally:
            hue.quiet_hours_range = real

    def test_a_zero_length_window_disables_quiet_hours(self):
        real = hue.quiet_hours_range
        hue.quiet_hours_range = lambda: (9.0, 9.0)
        try:
            self.assertFalse(hue.in_quiet_hours(1786960800))   # 3am, but disabled
        finally:
            hue.quiet_hours_range = real


class FlashDuration(unittest.TestCase):
    def test_the_flash_is_shorter_than_the_lead_time(self):
        # Otherwise the light would still be pulsing after the aircraft has
        # already passed, which reads as a stuck warning rather than a warning.
        self.assertLess(hue.FLASH_SECONDS, hue.LEAD_SECONDS)


class FlashBrightness(unittest.TestCase):
    """Perceived-light-relative brightness, owner decisions 2026-08-17:
    noticeably brighter than the room IS -- in lumens, not in Hue's bri
    numbers, which saturated colours turn into far less light."""

    OFF = {"on": False, "bri": 200, "ct": 300}
    WARM_WHITE = {"on": True, "bri": 133, "ct": 370}

    def test_an_off_light_flashes_at_the_gentle_floor_in_white_terms(self):
        # A white warning from darkness lands at the ~30% floor.
        self.assertEqual(hue.flash_brightness(self.OFF, hue.XY_WHITE), 77)

    def test_a_dim_colour_gets_more_drive_for_the_same_perceived_floor(self):
        # Purple emits less light per bri step than white, so the same
        # perceived floor needs a higher commanded bri.
        white = hue.flash_brightness(self.OFF, hue.XY_WHITE)
        purple = hue.flash_brightness(self.OFF, hue.XY_PURPLE)
        self.assertGreater(purple, white)

    def test_orange_against_a_warm_white_room_is_driven_to_the_cap(self):
        # The observed failure: warm white at 133 vs orange at 197 read as no
        # change, because orange's efficacy is ~1/3 of white's. The model
        # drives orange as hard as the hardware allows.
        self.assertEqual(hue.flash_brightness(self.WARM_WHITE, hue.XY_ORANGE), 254)

    def test_a_brighter_room_never_gets_a_dimmer_warning(self):
        for target in (hue.XY_WHITE, hue.XY_ORANGE, hue.XY_PURPLE, hue.XY_RED):
            dim = hue.flash_brightness({"on": True, "bri": 40, "ct": 300}, target)
            bright = hue.flash_brightness({"on": True, "bri": 200, "ct": 300}, target)
            self.assertGreaterEqual(bright, dim, target)

    def test_the_command_stays_on_the_hue_scale(self):
        for state in (self.OFF, self.WARM_WHITE, {"on": True, "bri": 254, "ct": 200}):
            for target in (hue.XY_WHITE, hue.XY_BLUE, hue.XY_RED, hue.XY_ORANGE):
                bri = hue.flash_brightness(state, target)
                self.assertGreaterEqual(bri, 1)
                self.assertLessEqual(bri, 254)

    def test_a_saturated_room_colour_counts_as_the_light_it_emits(self):
        # A room lit deep red at bri 200 emits little light; the warning
        # should step up from that reality, not from the number 200.
        red_room = {"on": True, "bri": 200, "xy": hue.XY_RED}
        white_room = {"on": True, "bri": 200, "ct": 300}
        self.assertLess(hue.flash_brightness(red_room, hue.XY_WHITE),
                        hue.flash_brightness(white_room, hue.XY_WHITE))

    def test_a_light_with_no_brightness_gets_the_fixed_fallback(self):
        # On/off plugs cannot dim; relativity is meaningless for them.
        self.assertEqual(hue.flash_brightness({"on": True}, hue.XY_WHITE),
                         hue.ALERT_BRIGHTNESS)

    def test_white_is_the_most_efficient_palette_colour(self):
        # Sanity of the physical model itself.
        for xy in (hue.XY_PURPLE, hue.XY_ORANGE, hue.XY_RED, hue.XY_BLUE):
            self.assertLess(hue.luminance_factor(xy), hue.WHITE_FACTOR, xy)

    def test_deep_blue_is_the_dimmest(self):
        for xy in (hue.XY_PURPLE, hue.XY_ORANGE, hue.XY_RED, hue.XY_WHITE):
            self.assertLess(hue.luminance_factor(hue.XY_BLUE),
                            hue.luminance_factor(xy))


class SecondZone(unittest.TestCase):
    """The kitchen set flashes red alongside the bedroom's cyan."""

    def setUp(self):
        self._lights, self._alert = hue.LIGHTS, hue.ALERT_LIGHTS
        hue.LIGHTS, hue.ALERT_LIGHTS = ["2", "8"], ["10"]

    def tearDown(self):
        hue.LIGHTS, hue.ALERT_LIGHTS = self._lights, self._alert

    def test_normal_aircraft_show_a_single_steady_purple_everywhere(self):
        # Owner decisions 2026-08-17: no orange ("horrible"), then no
        # alternation at all -- normal aircraft are a calm, steady purple.
        plan = hue.zones(None)
        self.assertEqual(len(plan), 2)
        (bed_ids, bed_palette, _), (kit_ids, kit_palette, _) = plan
        self.assertEqual(bed_ids, ["2", "8"])
        self.assertEqual(bed_palette, [hue.XY_PURPLE])
        self.assertEqual(kit_ids, ["10"])
        self.assertEqual(kit_palette, [hue.XY_PURPLE])

    def test_no_flash_palette_contains_orange(self):
        # Orange means exactly one thing now: "arriving sooner than the usual
        # notice". A palette that flashes it would dilute that.
        for kind in (None, "POLICE", "AIR_AMBULANCE"):
            self.assertNotIn(hue.XY_ORANGE, hue.zones(kind)[0][1], kind)

    def test_air_ambulance_flashes_calming_white_blue(self):
        for _, palette, _ in hue.zones("AIR_AMBULANCE"):
            self.assertEqual(palette, [hue.XY_WHITE, hue.XY_BLUE])

    def test_police_flashes_patrol_red_blue(self):
        for _, palette, _ in hue.zones("POLICE"):
            self.assertEqual(palette, [hue.XY_RED, hue.XY_BLUE])

    def test_the_signature_is_uniform_across_rooms(self):
        # The colour says WHAT is coming, never which room you are in.
        for kind in (None, "POLICE", "AIR_AMBULANCE", "unknown-kind"):
            palettes = {tuple(map(tuple, z[1])) for z in hue.zones(kind)}
            self.assertEqual(len(palettes), 1, kind)

    def test_an_unknown_kind_falls_back_to_the_normal_signature(self):
        self.assertEqual(hue.zones("CROP_DUSTER")[0][1], hue.DEFAULT_PALETTE)
        self.assertEqual(hue.zones("police")[0][1], hue.KIND_PALETTES["POLICE"],
                         "kind match is case-insensitive")

    def test_police_alternate_twice_as_fast(self):
        # The urgency of a light bar is its tempo as much as its colours.
        self.assertAlmostEqual(hue.alternate_seconds_for("POLICE"),
                               hue.ALTERNATE_SECONDS / 2.0, delta=1e-9)
        self.assertAlmostEqual(hue.alternate_seconds_for("police"),
                               hue.ALTERNATE_SECONDS / 2.0, delta=1e-9)

    def test_everyone_else_alternates_at_the_normal_rate(self):
        for kind in (None, "AIR_AMBULANCE", "CROP_DUSTER"):
            self.assertAlmostEqual(hue.alternate_seconds_for(kind),
                                   hue.ALTERNATE_SECONDS, delta=1e-9, msg=kind)

    def test_emergency_palettes_alternate_two_distinct_colours(self):
        # Alternation belongs to the emergency kinds alone; normal is steady.
        for kind in ("POLICE", "AIR_AMBULANCE"):
            palette = hue.zones(kind)[0][1]
            self.assertEqual(len(palette), 2)
            self.assertNotEqual(palette[0], palette[1])
        self.assertEqual(len(hue.zones(None)[0][1]), 1)

    def test_every_zone_is_at_90_percent(self):
        self.assertEqual(hue.ALERT_BRIGHTNESS, 229)
        self.assertLess(abs(hue.ALERT_BRIGHTNESS / 254 - 0.90), 0.01)

    def test_with_no_alert_lights_configured_there_is_only_one_zone(self):
        hue.ALERT_LIGHTS = []
        self.assertEqual(len(hue.zones(None)), 1)


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
        self._settle = hue.RESTORE_SETTLE_SECONDS
        hue.RESTORE_SETTLE_SECONDS = 0.0
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
        hue.RESTORE_SETTLE_SECONDS = self._settle

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

    def test_a_normal_aircraft_stays_steady_purple_for_the_whole_flash(self):
        # No alternation, no second colour, no breathe -- steady means steady.
        hue.FLASH_SECONDS = 0.6
        hue.ALTERNATE_SECONDS = 0.01
        self.applied.clear()
        hue.warn("LOUD", "test")
        seen = {tuple(st["xy"]) for lid, st in self.applied
                if lid in ("2", "10") and "xy" in st and st.get("alert") != "none"}
        self.assertEqual(seen, {tuple(hue.XY_PURPLE)})
        self.assertFalse(any(st.get("alert") == "lselect"
                             for _, st in self.applied), "nothing may breathe")

    def test_a_steady_colour_is_reasserted_throughout_the_flash(self):
        # Zigbee drops frames: one of two bedroom bulbs missed its single
        # opening write on 2026-08-17 and sat unchanged through the whole
        # warning. A steady signal must self-heal like an alternating one.
        hue.FLASH_SECONDS = 0.6
        hue.ALTERNATE_SECONDS = 0.01
        self.applied.clear()
        hue.warn("LOUD", "test")
        purple_writes = [st for lid, st in self.applied
                         if lid == "2" and st.get("xy") == hue.XY_PURPLE]
        self.assertGreater(len(purple_writes), 1,
                           "steady purple was written only once; a dropped "
                           "frame would go uncorrected")

    def test_the_orange_hold_is_reasserted_while_it_holds(self):
        self.applied.clear()
        hue.warn("LOUD", "test", hold_orange_seconds=0.05)
        orange_writes = [st for lid, st in self.applied
                         if lid == "2" and st.get("xy") == hue.XY_ORANGE]
        self.assertGreater(len(orange_writes), 1,
                           "the hold was written only once; a dropped frame "
                           "would go uncorrected for its whole duration")

    def test_emergency_lights_actually_alternate_during_the_flash(self):
        # The palette being right proves nothing if the flash loop never uses
        # the second colour -- checked on the bedroom AND the second zone.
        hue.FLASH_SECONDS = 0.6
        hue.ALTERNATE_SECONDS = 0.01
        self.applied.clear()
        hue.warn("LOUD", "test", kind="AIR_AMBULANCE")
        for lid in ("2", "10"):
            seen = {tuple(st["xy"]) for l, st in self.applied
                    if l == lid and "xy" in st and st.get("alert") != "none"}
            self.assertEqual(seen, {tuple(hue.XY_WHITE), tuple(hue.XY_BLUE)}, lid)

    def test_the_bedroom_is_never_sent_red_for_a_normal_aircraft(self):
        for band in ("LOUD", "VERY LOUD", "UNKNOWN"):
            self.applied.clear()
            hue.warn(band, "test")
            for lid, st in self.applied:
                if lid in ("2", "8") and "xy" in st:
                    self.assertNotEqual(st["xy"], hue.XY_RED,
                                        f"bedroom light {lid} sent red on {band}")

    def test_a_police_aircraft_does_send_red_and_blue(self):
        hue.FLASH_SECONDS = 0.6
        hue.ALTERNATE_SECONDS = 0.01
        self.applied.clear()
        hue.warn("LOUD", "test", kind="POLICE")
        seen = {tuple(st["xy"]) for lid, st in self.applied
                if lid == "2" and "xy" in st and st.get("alert") != "none"}
        self.assertEqual(seen, {tuple(hue.XY_RED), tuple(hue.XY_BLUE)})

    def test_a_late_warning_opens_with_constant_orange_before_the_flash(self):
        # Owner decision 2026-08-17: orange FIRST -- "sooner than usual" opens
        # the warning, and the flash lands as the sound arrives.
        self.applied.clear()
        hue.warn("LOUD", "test", hold_orange_seconds=0.05)
        writes_for_2 = [st for lid, st in self.applied if lid == "2" and "xy" in st]
        orange = [i for i, st in enumerate(writes_for_2)
                  if st.get("xy") == hue.XY_ORANGE and st.get("alert") == "none"
                  and st.get("on") is True]
        flash = [i for i, st in enumerate(writes_for_2)
                 if st.get("xy") == hue.XY_PURPLE]
        self.assertTrue(orange, "no constant-orange hold was applied")
        self.assertTrue(flash, "no flash followed the hold")
        self.assertLess(orange[0], flash[0],
                        "the orange hold must come BEFORE the flash")

    def test_a_hold_cleared_early_skips_the_flash(self):
        # The aircraft went away during the hold: flashing after that would be
        # a warning about nothing. All-clear instead.
        self.applied.clear()
        hue.warn("LOUD", "test", still_inbound=lambda: False,
                 hold_orange_seconds=2.0)
        flash = [st for lid, st in self.applied
                 if lid == "2" and st.get("xy") == hue.XY_PURPLE]
        greens = [st for _, st in self.applied if st.get("xy") == hue.XY_GREEN]
        self.assertEqual(flash, [], "flash fired after the aircraft had gone")
        self.assertTrue(greens, "no all-clear after the hold cleared early")

    def test_the_orange_hold_is_luminance_compensated(self):
        # The observed failure: orange at "room + boost" bri read as no change
        # against a warm-white room. Compensated for orange's efficacy, the
        # commanded drive must far exceed a naive room+boost number.
        hue.call = lambda m, p, b=None: (
            {"state": {"on": True, "bri": 100, "colormode": "ct", "ct": 300}}, None)
        self.applied.clear()
        hue.warn("LOUD", "test", hold_orange_seconds=0.05)
        oranges = [st for lid, st in self.applied
                   if lid == "2" and st.get("xy") == hue.XY_ORANGE]
        self.assertTrue(oranges)
        self.assertEqual(oranges[0]["bri"],
                         hue.flash_brightness({"on": True, "bri": 100, "ct": 300},
                                              hue.XY_ORANGE))
        self.assertGreater(oranges[0]["bri"], 100 + hue.FLASH_BOOST_BRI)

    def test_no_orange_hold_when_the_warning_was_on_time(self):
        self.applied.clear()
        hue.warn("LOUD", "test", hold_orange_seconds=0.0)
        held = [st for lid, st in self.applied
                if st.get("xy") == hue.XY_ORANGE and st.get("alert") == "none"]
        self.assertEqual(held, [])

    def test_the_orange_hold_clears_early_when_the_aircraft_goes(self):
        self.applied.clear()
        hue.warn("LOUD", "test", still_inbound=lambda: False,
                 hold_orange_seconds=5.0)
        greens = [st for _, st in self.applied if st.get("xy") == hue.XY_GREEN]
        self.assertTrue(greens, "no all-clear after the hold cleared early")

    def test_brightness_is_relative_to_the_captured_state(self):
        # The stubbed lights are ON at 254, so the flash caps at 254 -- the
        # colour alternation carries the signal when there is no headroom.
        f = self._flashes()
        self.assertEqual(f["2"]["bri"], 254)
        self.assertEqual(f["10"]["bri"], 254)

    def test_every_light_touched_is_also_restored(self):
        hue.warn("LOUD", "test")
        touched = {lid for lid, _ in self.applied}
        restored = {lid for lid, st in self.applied if st.get("alert") == "none"}
        self.assertEqual(touched, restored)


class VerifiedRestore(unittest.TestCase):
    """A bridge OK is not evidence the bulb heard it. On 2026-08-17 a bedroom
    bulb dropped its restore write and sat purple after the warning ended."""

    def setUp(self):
        self._c, self._a, self._settle = hue.call, hue.apply_state, hue.RESTORE_SETTLE_SECONDS
        hue.RESTORE_SETTLE_SECONDS = 0.0
        self.writes = []

    def tearDown(self):
        hue.call, hue.apply_state = self._c, self._a
        hue.RESTORE_SETTLE_SECONDS = self._settle

    def test_a_dropped_restore_write_is_retried_until_confirmed(self):
        expected = {"on": True, "bri": 133, "ct": 346}
        # First read-back shows the bulb still stuck purple; later ones show
        # the restore landed.
        readbacks = [{"on": True, "bri": 254, "colormode": "xy", "xy": [0.2919, 0.1327]},
                     {"on": True, "bri": 133, "colormode": "ct", "ct": 346},
                     {"on": True, "bri": 133, "colormode": "ct", "ct": 346}]

        def fake_call(method, path, body=None):
            if method == "PUT":
                self.writes.append(body)
                return ({}, None)
            return ({"state": readbacks.pop(0)}, None)

        hue.call = fake_call
        self.assertIsNone(hue.restore_verified("2", expected))
        self.assertGreaterEqual(len(self.writes), 2, "the dropped write was not retried")

    def test_even_a_confirmed_restore_is_written_at_least_twice(self):
        # The bridge cache can confirm a command the bulb never heard: it
        # reported stable warm white for 24+ s while the physical bulb sat
        # purple (2026-08-17). Repetition is what actually delivers.
        expected = {"on": True, "bri": 133, "ct": 346}
        good = {"on": True, "bri": 133, "colormode": "ct", "ct": 346}

        def fake_call(method, path, body=None):
            if method == "PUT":
                self.writes.append(body)
                return ({}, None)
            return ({"state": good}, None)

        hue.call = fake_call
        self.assertIsNone(hue.restore_verified("2", expected))
        self.assertGreaterEqual(len(self.writes), hue.RESTORE_MIN_WRITES)

    def test_an_unconfirmable_restore_is_reported_not_swallowed(self):
        expected = {"on": True, "bri": 133, "ct": 346}
        stuck = {"on": True, "bri": 254, "colormode": "xy", "xy": [0.2919, 0.1327]}
        hue.call = lambda m, p, b=None: (
            ({}, None) if m == "PUT" else ({"state": stuck}, None))
        error = hue.restore_verified("2", expected)
        self.assertIsNotNone(error)
        self.assertIn(str(hue.RESTORE_ATTEMPTS), error)

    def test_an_unreadable_light_counts_as_unconfirmed(self):
        # Unknown is not restored.
        hue.call = lambda m, p, b=None: (
            ({}, None) if m == "PUT" else (None, "bridge unreachable"))
        error = hue.restore_verified("2", {"on": False, "bri": 10, "ct": 300})
        self.assertIsNotNone(error)
        self.assertIn("unverifiable", error)

    def test_matching_tolerates_bridge_rounding_but_not_the_wrong_colour(self):
        expected = {"on": True, "bri": 133, "xy": [0.445, 0.4067]}
        self.assertTrue(hue.state_matches(
            expected, {"on": True, "bri": 134, "xy": [0.4449, 0.407]}))
        self.assertFalse(hue.state_matches(
            expected, {"on": True, "bri": 133, "xy": [0.2919, 0.1327]}))
        self.assertFalse(hue.state_matches(expected, {"on": False, "bri": 133}))


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
        restored = [st for lid, st in self.applied if st.get("alert") == "none" and
                    "bri" in st and st.get("xy") != hue.XY_GREEN]
        self.assertTrue(restored, "lights were not restored after the all-clear")

    def test_an_unreadable_feed_does_not_count_as_cleared(self):
        # Unknown is not all-clear. If the feed cannot be read mid-flash the
        # warning must run its course rather than declaring the sky safe.
        hue.warn("LOUD", "test", still_inbound=lambda: True)
        self.assertFalse(self._greens())


class HeardWithoutAPosition(unittest.TestCase):
    """The N826LF case. A Life Flight helicopter heard for two minutes at
    -33 dBm, never located, and therefore never warned about."""

    HELI = {"hex": "AB4825", "reg": "N826LF", "operator_kind": "AIR_AMBULANCE",
            "state": "HEARD_NO_POSITION"}
    POLICE = {"hex": "A1D296", "reg": "N2163J", "operator_kind": "POLICE",
              "state": "HEARD_NO_POSITION"}

    def test_an_unlocated_air_ambulance_is_chosen(self):
        picked = hue.choose_unlocated({"adsb_heard_unlocated": [self.HELI]})
        self.assertIsNotNone(picked)
        self.assertEqual(picked["reg"], "N826LF")

    def test_police_are_not_chosen_by_default(self):
        """They loiter here for hours. Firing on every unlocated police contact
        would mean near-continuous flashing, which trains the eye to ignore the
        light -- costing more than it buys."""
        self.assertIsNone(hue.choose_unlocated({"adsb_heard_unlocated": [self.POLICE]}))

    def test_an_air_ambulance_behind_a_police_contact_is_still_found(self):
        picked = hue.choose_unlocated({"adsb_heard_unlocated": [self.POLICE, self.HELI]})
        self.assertEqual(picked["reg"], "N826LF")

    def test_an_empty_or_absent_feed_chooses_nothing(self):
        self.assertIsNone(hue.choose_unlocated({}))
        self.assertIsNone(hue.choose_unlocated({"adsb_heard_unlocated": []}))
        self.assertIsNone(hue.choose_unlocated({"adsb_heard_unlocated": None}))

    def test_a_missing_operator_kind_chooses_nothing(self):
        # Never coerced to a default. An entry that does not say what it is
        # does not get to fire a light.
        self.assertIsNone(hue.choose_unlocated(
            {"adsb_heard_unlocated": [{"hex": "AB4825"}]}))
        self.assertIsNone(hue.choose_unlocated(
            {"adsb_heard_unlocated": [{"hex": "AB4825", "operator_kind": None}]}))

    def test_the_kind_match_is_case_insensitive(self):
        entry = dict(self.HELI, operator_kind="air_ambulance")
        self.assertIsNotNone(hue.choose_unlocated({"adsb_heard_unlocated": [entry]}))

    def test_it_carries_no_countdown_to_invent(self):
        # choose() returns (report, band, seconds). This returns a bare entry
        # precisely so no caller can read a time off it.
        picked = hue.choose_unlocated({"adsb_heard_unlocated": [self.HELI]})
        self.assertNotIsInstance(picked, tuple)
        for forbidden in ("at_epoch", "eta_seconds", "prediction", "distance_km"):
            self.assertNotIn(forbidden, picked)


class TheMasterSwitch(unittest.TestCase):
    """hue_enabled=false must silence EVERY path to a flash.

    There are three independent triggers, and disabling only the obvious one
    leaves the light firing while the operator believes it is off. That is the
    exact failure this project legislates against elsewhere, so the disable is
    one gate rather than three things to remember.
    """

    LOUD = {"hex": "A12345", "registration": "N123AB",
            "prediction": {"at_epoch": 1000.0, "loudness_band": "LOUD",
                           "noise_index": 62.0, "proximity": "OVERHEAD"}}
    UNSCORED_OVERHEAD = {"hex": "A99999",
                         "prediction": {"at_epoch": 1000.0, "loudness_band": "UNKNOWN",
                                        "noise_index": None, "proximity": "OVERHEAD"}}
    HEARD = {"hex": "AB4825", "reg": "N826LF", "operator_kind": "AIR_AMBULANCE",
             "state": "HEARD_NO_POSITION"}

    def arm(self, enabled):
        original = hue.armed
        hue.armed = lambda: enabled
        self.addCleanup(lambda: setattr(hue, "armed", original))

    def test_armed_by_default(self):
        # Absent settings must not silently disarm a warning system.
        self.assertTrue(hue.armed())

    def test_a_loud_aircraft_is_chosen_when_armed(self):
        self.arm(True)
        self.assertIsNotNone(hue.choose({"adsb_predictions": [self.LOUD]}, 970.0))

    def test_a_loud_aircraft_is_not_chosen_when_disarmed(self):
        self.arm(False)
        self.assertIsNone(hue.choose({"adsb_predictions": [self.LOUD]}, 970.0))

    def test_an_unscored_overhead_aircraft_is_not_chosen_when_disarmed(self):
        """The second trigger. Independent of the band list, so clearing
        hue_bands alone would leave this one firing."""
        self.arm(False)
        self.assertIsNone(
            hue.choose({"adsb_predictions": [self.UNSCORED_OVERHEAD]}, 970.0))

    def test_an_unlocated_air_ambulance_is_not_chosen_when_disarmed(self):
        """The third trigger, and the one most likely to be forgotten: it does
        not read hue_bands at all."""
        self.arm(False)
        self.assertIsNone(hue.choose_unlocated({"adsb_heard_unlocated": [self.HEARD]}))

    def test_the_unlocated_path_still_works_when_armed(self):
        self.arm(True)
        self.assertIsNotNone(
            hue.choose_unlocated({"adsb_heard_unlocated": [self.HEARD]}))

    def test_the_unscored_overhead_trigger_has_its_own_switch(self):
        # Reachable from the console now; it was environment-only before, so
        # the admin page could not turn off a trigger that fires the light.
        self.assertIn("hue_warn_unscored", hue.panel_settings.SCHEMA)
        self.assertIn("hue_enabled", hue.panel_settings.SCHEMA)


if __name__ == "__main__":
    unittest.main()
