#!/usr/bin/env python3
"""Tests for Hue-button aircraft ground-truth capture."""

import importlib.machinery
import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest import mock


SCRIPT = pathlib.Path(__file__).with_name("panel-hue-aircraft-label")
loader = importlib.machinery.SourceFileLoader("panel_hue_aircraft_label", str(SCRIPT))
spec = importlib.util.spec_from_loader(loader.name, loader)
label = importlib.util.module_from_spec(spec)
loader.exec_module(label)


class EventParsing(unittest.TestCase):
    def test_all_reports_for_configured_button_are_surfaced(self):
        """Releases are evidence too: a press was lost on 2026-08-17 because
        only initial_press was parsed and its release was discarded."""
        payload = json.dumps([{
            "type": "update",
            "creationtime": "2026-08-17T01:00:00Z",
            "data": [
                {"id": label.BUTTON_ID, "type": "button",
                 "button": {"button_report": {"event": "initial_press", "updated": "press-1"}}},
                {"id": label.BUTTON_ID, "type": "button",
                 "button": {"button_report": {"event": "short_release", "updated": "release-1"}}},
                {"id": "some-other-button", "type": "button",
                 "button": {"button_report": {"event": "initial_press", "updated": "wrong"}}},
            ],
        }])
        self.assertEqual(label.button_reports_from_payload(payload),
                         [{"event": "initial_press", "updated": "press-1"},
                          {"event": "short_release", "updated": "release-1"}])

    def test_invalid_event_data_is_ignored(self):
        self.assertEqual(label.button_reports_from_payload("not json"), [])
        self.assertEqual(label.button_reports_from_payload("{}"), [])
        malformed = json.dumps([{"type": "update", "data": [None, "bad", {"id": label.BUTTON_ID, "type": "button", "button": []}]}])
        self.assertEqual(label.button_reports_from_payload(malformed), [])
        self.assertEqual(label.current_candidates({"adsb_planes": [None, "bad"], "adsb_predictions": {}}), [])


class PressRecovery(unittest.TestCase):
    """The extractor decides what counts as a press, and recovers presses
    whose initial_press was lost in a reconnect blind window."""

    def extractor(self):
        self.logged = []
        return label.PressExtractor(clock=lambda: 1000.0,
                                    log=lambda *a, **k: self.logged.append(a[0]))

    @staticmethod
    def report(event, iso):
        return {"event": event, "updated": iso}

    def test_normal_press_release_pair_counts_once(self):
        x = self.extractor()
        verdicts = [x.feed(self.report("initial_press", "2026-08-17T04:00:00.000Z")),
                    x.feed(self.report("short_release", "2026-08-17T04:00:00.300Z"))]
        self.assertEqual([v for v in verdicts if v],
                         [("press", "2026-08-17T04:00:00.000Z")])

    def test_unpaired_release_recovers_the_lost_press(self):
        x = self.extractor()
        verdict = x.feed(self.report("short_release", "2026-08-17T04:09:49.500Z"))
        self.assertEqual(verdict, ("press", "2026-08-17T04:09:49.500Z"))
        self.assertTrue(any("recovered press" in line for line in self.logged))

    def test_release_long_after_a_press_is_a_new_recovered_press(self):
        x = self.extractor()
        x.feed(self.report("initial_press", "2026-08-17T04:00:00.000Z"))
        verdict = x.feed(self.report("short_release", "2026-08-17T04:02:00.000Z"))
        self.assertEqual(verdict[0], "press")

    def test_held_button_after_lost_initial_recovers_exactly_one_press(self):
        # The recovered press has no measurable start, so the release can
        # never claim a hold: human_heard is the conservative reading.
        x = self.extractor()
        verdicts = [x.feed(self.report("repeat", "2026-08-17T04:00:01.000Z")),
                    x.feed(self.report("repeat", "2026-08-17T04:00:01.800Z")),
                    x.feed(self.report("long_release", "2026-08-17T04:00:02.500Z"))]
        real = [v for v in verdicts if v]
        self.assertEqual([v[0] for v in real if v[0] == "press"], ["press"])
        self.assertNotIn("hold", [v[0] for v in real])

    def test_a_two_second_hold_is_the_warning_unheard_gesture(self):
        x = self.extractor()
        verdicts = [x.feed(self.report("initial_press", "2026-08-17T04:00:00.000Z")),
                    x.feed(self.report("repeat", "2026-08-17T04:00:00.800Z")),
                    x.feed(self.report("repeat", "2026-08-17T04:00:01.600Z")),
                    x.feed(self.report("long_release", "2026-08-17T04:00:02.100Z"))]
        kinds = [v[0] for v in verdicts if v]
        self.assertEqual(kinds, ["press", "extend", "extend", "hold"])
        self.assertTrue(any("warning-unheard" in line for line in self.logged))

    def test_a_short_hold_stays_an_ordinary_press(self):
        # 1.5 s is a slow press, not the 2 s gesture. The repeat defers the
        # gesture (extend); the release neither mints a press nor a hold.
        x = self.extractor()
        verdicts = [x.feed(self.report("initial_press", "2026-08-17T04:00:00.000Z")),
                    x.feed(self.report("repeat", "2026-08-17T04:00:00.800Z")),
                    x.feed(self.report("long_release", "2026-08-17T04:00:01.500Z"))]
        self.assertEqual([v[0] for v in verdicts if v], ["press", "extend"])

    def test_the_hold_is_measured_from_the_first_press_not_the_last_repeat(self):
        # The pairing anchor slides with each repeat; the hold must not, or a
        # 2.4 s hold would measure 0.6 s from its final repeat and vanish.
        x = self.extractor()
        x.feed(self.report("initial_press", "2026-08-17T04:00:00.000Z"))
        x.feed(self.report("repeat", "2026-08-17T04:00:00.800Z"))
        x.feed(self.report("repeat", "2026-08-17T04:00:01.800Z"))
        verdict = x.feed(self.report("long_release", "2026-08-17T04:00:02.400Z"))
        self.assertEqual(verdict[0], "hold")

    def test_every_report_is_logged_for_diagnosability(self):
        x = self.extractor()
        x.feed(self.report("initial_press", "2026-08-17T04:00:00.000Z"))
        x.feed(self.report("short_release", "2026-08-17T04:00:00.300Z"))
        self.assertEqual(sum(1 for line in self.logged if "button_report" in line), 2)


class Configuration(unittest.TestCase):
    def test_synthetic_mode_fails_closed_on_typo(self):
        self.assertTrue(label.synthetic_mode(" on "))
        self.assertFalse(label.synthetic_mode("off"))
        with self.assertRaisesRegex(ValueError, "unrecognized"):
            label.synthetic_mode("synthetci")

    def test_all_private_identifiers_are_required(self):
        with mock.patch.object(label, "BRIDGE", None), mock.patch.object(label, "USERNAME", None), mock.patch.object(label, "BUTTON_ID", None):
            with self.assertRaisesRegex(SystemExit, "HUE_BRIDGE.*HUE_USERNAME.*HUE_AIRCRAFT_BUTTON_ID"):
                label.validate_configuration()

    @mock.patch.object(label.subprocess, "run")
    def test_real_capture_requires_attribution_builder(self, run):
        run.return_value = mock.Mock(returncode=0, stdout="old help", stderr="")
        configured = (mock.patch.object(label, "BRIDGE", "bridge"),
                      mock.patch.object(label, "USERNAME", "user"),
                      mock.patch.object(label, "BUTTON_ID", "button"),
                      mock.patch.object(label, "CERTIFICATE", "cert"),
                      mock.patch.object(label, "SYNTHETIC", False))
        with configured[0], configured[1], configured[2], configured[3], configured[4]:
            with self.assertRaisesRegex(SystemExit, "lacks required --output"):
                label.validate_configuration()

    @mock.patch.object(label.subprocess, "run")
    def test_supported_attribution_builder_passes_startup_check(self, run):
        run.return_value = mock.Mock(returncode=0, stdout="--output OUTPUT", stderr="")
        configured = (mock.patch.object(label, "BRIDGE", "bridge"),
                      mock.patch.object(label, "USERNAME", "user"),
                      mock.patch.object(label, "BUTTON_ID", "button"),
                      mock.patch.object(label, "CERTIFICATE", "cert"),
                      mock.patch.object(label, "SYNTHETIC", False))
        with configured[0], configured[1], configured[2], configured[3], configured[4]:
            label.validate_configuration()


class Health(unittest.TestCase):
    def test_health_transition_preserves_last_label(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = pathlib.Path(tmp) / "state.json"
            state.write_text(json.dumps({"last_label": "human_heard"}))
            with mock.patch.object(label, "STATE", state), mock.patch.object(label.time, "time", return_value=1000.0):
                label.write_health("disconnected", "bridge timeout")
                first = json.loads(state.read_text())
                label.write_health("connected")
                second = json.loads(state.read_text())
            self.assertEqual(first["last_label"], "human_heard")
            self.assertEqual(first["stream_error"], "bridge timeout")
            self.assertEqual(second["stream_state"], "connected")
            self.assertNotIn("stream_error", second)

    def test_battery_is_resolved_through_button_owner(self):
        opener = mock.Mock()
        button = {"data": [{"owner": {"rid": "device-1"}}]}
        power = {"data": [{"owner": {"rid": "device-1"},
                            "power_state": {"battery_state": "critical", "battery_level": 1}}]}
        contexts = []
        for document in (button, power):
            context = mock.MagicMock()
            context.__enter__.return_value = mock.Mock()
            contexts.append(context)
        opener.open.side_effect = contexts
        with mock.patch.object(label.json, "load", side_effect=(button, power)), \
                mock.patch.object(label, "merge_state") as merge, \
                mock.patch.object(label.time, "time", return_value=1000.0):
            label.refresh_battery(opener)
        battery = merge.call_args.args[0]["battery"]
        self.assertEqual(battery["battery_state"], "critical")
        self.assertEqual(battery["battery_level"], 1)
        self.assertEqual(battery["measured_at"], 1000.0)

    def test_battery_refresh_is_throttled_while_a_measurement_is_fresh(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = pathlib.Path(tmp) / "state.json"
            state.write_text(json.dumps({"battery": {"battery_state": "normal",
                                                       "measured_at": 990.0}}))
            opener = mock.Mock()
            with mock.patch.object(label, "STATE", state), \
                    mock.patch.object(label.time, "time", return_value=1000.0):
                label.maybe_refresh_battery(opener)
            opener.open.assert_not_called()

    def test_failed_refresh_never_clobbers_the_last_good_measurement(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = pathlib.Path(tmp) / "state.json"
            state.write_text(json.dumps({"battery": {"battery_state": "normal",
                                                       "battery_level": 76,
                                                       "measured_at": 1.0}}))
            opener = mock.Mock()
            opener.open.side_effect = OSError("bridge unreachable")
            with mock.patch.object(label, "STATE", state), \
                    mock.patch.object(label.time, "time", return_value=100000.0):
                label.maybe_refresh_battery(opener)
            after = json.loads(state.read_text())
            self.assertEqual(after["battery"]["battery_level"], 76)
            self.assertIn("bridge unreachable", after["battery_refresh_error"]["error"])

    def test_activity_heartbeat_is_persisted_but_throttled(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = pathlib.Path(tmp) / "state.json"
            now = [1000.0]
            with mock.patch.object(label, "STATE", state):
                monitor = label.ActivityMonitor(clock=lambda: now[0])
                monitor.note()                       # first: persists
                now[0] = 1010.0
                monitor.note()                       # within throttle: no write
                first = json.loads(state.read_text())["stream_last_activity_at"]
                self.assertEqual(first, 1000.0)
                now[0] = 1000.0 + label.ACTIVITY_PERSIST_SECONDS + 1
                monitor.note()
                second = json.loads(state.read_text())["stream_last_activity_at"]
                self.assertEqual(second, now[0])


class Gestures(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.state = pathlib.Path(self.tmp.name) / "state.json"
        self.now = 1000.0
        self.emitted = []
        self.patch = mock.patch.object(label, "STATE", self.state)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        self.addCleanup(self.tmp.cleanup)

    def tracker(self):
        return label.GestureTracker(
            emit=lambda at, count, hold=False: self.emitted.append(
                (at, count, hold) if hold else (at, count)),
            clock=lambda: self.now, scheduler=None)

    def test_duplicate_token_is_ignored_and_double_press_is_classified(self):
        tracker = self.tracker()
        tracker.press("same")
        tracker.press("same")
        self.now += 0.5
        tracker.press("second")
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.assertEqual(self.emitted, [(1000.0, 2)])

    def test_single_flushes_on_its_own_deadline(self):
        tracker = self.tracker()
        tracker.press("one")
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.assertEqual(self.emitted, [(1000.0, 1)])

    def test_pending_gesture_survives_restart(self):
        self.tracker().press("one")
        self.now += label.DOUBLE_SECONDS + 0.01
        recovered = self.tracker()
        recovered.tick()
        self.assertEqual(self.emitted, [(1000.0, 1)])

    def test_failed_emit_is_retried_without_losing_pending_gesture(self):
        attempts = []

        def emit(at, count, hold=False):
            attempts.append((at, count))
            if len(attempts) == 1:
                raise RuntimeError("temporary")

        tracker = label.GestureTracker(emit=emit, clock=lambda: self.now, scheduler=None)
        tracker.press("one")
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.assertEqual(tracker.pending_count, 1)
        tracker.tick()
        self.assertEqual(attempts, [(1000.0, 1), (1000.0, 1)])
        self.assertEqual(tracker.pending_count, 0)

    def test_later_press_is_queued_while_older_finalization_retries(self):
        attempts = []
        unavailable = [True]

        def emit(at, count, hold=False):
            attempts.append((at, count))
            if at == 1000.0 and unavailable[0]:
                raise RuntimeError("correlator unavailable")

        tracker = label.GestureTracker(emit=emit, clock=lambda: self.now, scheduler=None)
        tracker.press("old")
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.now += 10.0
        tracker.press("new")
        self.assertEqual(tracker.pending, [{"at": 1000.0, "count": 1},
                                           {"at": self.now, "count": 1}])
        persisted = json.loads(self.state.read_text())
        self.assertEqual(persisted["pending_gestures"], tracker.pending)

        unavailable[0] = False
        tracker.tick()
        self.assertEqual(tracker.pending, [{"at": self.now, "count": 1}])
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.assertEqual(attempts, [(1000.0, 1), (1000.0, 1),
                                    (1000.0, 1), (1011.36, 1)])
        self.assertEqual(tracker.pending, [])

    def test_a_hold_defers_then_finalises_as_warning_unheard(self):
        # press -> repeat (extend) -> hold release. Without the extend, the
        # gesture would flush as human_heard at 1.35 s, before the release.
        tracker = self.tracker()
        tracker.signal("press", "p1")
        self.now += 0.8
        tracker.signal("extend", "r1")
        self.now += 0.7            # 1.5 s in: past DOUBLE, but deferred
        tracker.tick()
        self.assertEqual(self.emitted, [], "deferred gesture flushed early")
        self.now += 0.6            # 2.1 s in: the release proves the hold
        tracker.signal("hold", "h1")
        self.assertEqual(self.emitted, [(1000.0, 1, True)])

    def test_a_short_deferred_press_still_becomes_human_heard(self):
        tracker = self.tracker()
        tracker.signal("press", "p1")
        self.now += 0.8
        tracker.signal("extend", "r1")   # released at 1.5 s: no hold arrives
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.assertEqual(self.emitted, [(1000.0, 1)])

    def test_a_duplicate_hold_release_cannot_double_record(self):
        tracker = self.tracker()
        tracker.signal("press", "p1")
        tracker.signal("hold", "h1")
        tracker.signal("hold", "h1")
        self.assertEqual(self.emitted, [(1000.0, 1, True)])

    def test_a_hold_whose_press_was_already_finalised_still_records(self):
        # The repeats were lost, so nothing deferred the flush and the press
        # went out as human_heard. The surviving release is still evidence.
        tracker = self.tracker()
        tracker.signal("press", "p1")
        self.now += label.DOUBLE_SECONDS + 0.01
        tracker.tick()
        self.assertEqual(self.emitted, [(1000.0, 1)])
        tracker.signal("hold", "h1")
        self.assertEqual(self.emitted[-1], (self.now, 1, True))

    def test_replayed_press_after_restart_cannot_become_false_double(self):
        self.tracker().press("one")
        recovered = self.tracker()
        self.now += 0.5
        recovered.press("one")
        self.now += label.DOUBLE_SECONDS + 0.01
        recovered.tick()
        self.assertEqual(self.emitted, [(1000.0, 1)])


class DurableLabels(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = pathlib.Path(self.tmp.name)
        self.paths = {
            "LABELS": root / "labels.jsonl",
            "STATE": root / "state.json",
            "PASS_LOG": root / "passes.jsonl",
            "STATUS": root / "status.json",
            "FR24_STATUS": root / "fr24.json",
            "WARN_STATUS": root / "hue_status.json",
        }
        self.patchers = [mock.patch.object(label, name, value) for name, value in self.paths.items()]
        for patcher in self.patchers:
            patcher.start()
        self.addCleanup(self._cleanup)
        self.paths["STATUS"].write_text(json.dumps({
            "written_at": 999.0,
            "adsb_planes": [{"hex": "ABC123", "alt_ft": 2400}],
            "adsb_predictions": [{"hex": "ABC123", "prediction": {"eta_seconds": 30}}],
        }))
        self.paths["FR24_STATUS"].write_text(json.dumps({
            "written_at": 999.5, "feed_state": "FRESH",
            "aircraft": [{"hex": "DEF456", "reg": "N456FR", "distance_km": 1.2}],
        }))
        self.paths["PASS_LOG"].write_text(json.dumps({
            "at": 990.0, "identity": {"hex": "ABC123"},
            "detail": {"rssi_dbm": -15.0, "airframe_class": "jet"},
        }) + "\n")

    def _cleanup(self):
        for patcher in reversed(self.patchers):
            patcher.stop()
        self.tmp.cleanup()

    def row(self):
        return json.loads(self.paths["LABELS"].read_text())

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_single_press_is_real_human_heard_training_evidence(self, run, _time):
        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        self.paths["STATE"].write_text(json.dumps({"stream_state": "connected"}))
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 1)
        row = self.row()
        self.assertEqual(row["label"], "human_heard")
        self.assertEqual(row["priority"], 1)
        self.assertEqual(row["provenance"], "hue_button")
        self.assertTrue(row["eligible_for_training"])
        self.assertEqual(row["live_candidates"][0]["aircraft"]["hex"], "ABC123")
        self.assertEqual([candidate["aircraft"]["hex"] for candidate in row["live_candidates"]
                          if candidate["feed"] == "fr24"], ["DEF456"])
        self.assertEqual(row["fr24_feed"]["feed_state"], "FRESH")
        self.assertEqual(row["recent_completed_passes"][0]["detail"]["rssi_dbm"], -15.0)
        run.assert_called_once()
        self.assertEqual(json.loads(self.paths["STATE"].read_text())["stream_state"], "connected")

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_a_hold_records_warning_unheard_with_the_warning_it_disputes(self, run, _time):
        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        self.paths["WARN_STATUS"].write_text(json.dumps({
            "written_at": 998.0, "last_warned_at": 970.0,
            "last_subject": "N425EJ", "warnings_enabled": True}))
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 1, hold=True)
        row = self.row()
        self.assertEqual(row["label"], "warning_unheard")
        self.assertEqual(row["outcomes"], ["warning_unheard"])
        self.assertEqual(row["priority"], 0)
        self.assertTrue(row["event_id"].endswith("-hold"))
        self.assertTrue(row["eligible_for_training"])
        # The warning it disputes rides on the row, verbatim.
        self.assertTrue(row["warning_context"]["available"])
        self.assertEqual(row["warning_context"]["status"]["last_subject"], "N425EJ")
        # It must never read as a heard event.
        self.assertNotIn("human_heard", row["outcomes"])

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_a_hold_with_no_warn_status_says_so_explicitly(self, run, _time):
        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 1, hold=True)
        row = self.row()
        self.assertFalse(row["warning_context"]["available"])
        self.assertIn("panel-hue-warn", row["warning_context"]["reason"])

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_double_press_is_distinct_top_priority_cat_evidence(self, run, _time):
        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 2)
        row = self.row()
        self.assertEqual(row["label"], "cat_scared_top_priority")
        self.assertEqual(row["outcomes"], ["human_heard", "cat_scared_top_priority"])
        self.assertEqual(row["priority"], 2)
        self.assertEqual(row["press_count"], 2)
        run.assert_called_once()
        command = run.call_args.args[0]
        self.assertEqual(command[0], "/usr/local/bin/panel-aircraft-attribute")
        self.assertIn("--output", command)

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_failed_correlation_preserves_pending_for_idempotent_retry(self, run, _time):
        run.return_value = mock.Mock(returncode=1, stdout="", stderr="unavailable")
        self.paths["STATE"].write_text(json.dumps({
            "pending_gesture": {"at": 1000.0, "count": 1},
        }))
        with mock.patch.object(label, "SYNTHETIC", False):
            with self.assertRaisesRegex(RuntimeError, "correlation incomplete"):
                label.record_label(1000.0, 1)
        state = json.loads(self.paths["STATE"].read_text())
        self.assertEqual(state["pending_gesture"], {"at": 1000.0, "count": 1})
        self.assertEqual(state["correlator"]["returncode"], 1)

        run.return_value = mock.Mock(returncode=0, stdout="marked", stderr="")
        with mock.patch.object(label, "SYNTHETIC", False):
            label.record_label(1000.0, 1)
        self.assertNotIn("pending_gesture", json.loads(self.paths["STATE"].read_text()))
        self.assertEqual(len(self.paths["LABELS"].read_text().splitlines()), 1)

    def test_retry_after_label_append_does_not_duplicate_evidence(self):
        event_id = "hue-1000000-1"
        self.paths["LABELS"].write_text(json.dumps({"event_id": event_id}) + "\n")
        with mock.patch.object(label, "SYNTHETIC", True):
            label.record_label(1000.0, 1)
        rows = self.paths["LABELS"].read_text().splitlines()
        self.assertEqual(len(rows), 1)

    @mock.patch.object(label.time, "time", return_value=1002.0)
    @mock.patch.object(label.subprocess, "run")
    def test_synthetic_event_is_never_training_evidence(self, run, _time):
        with mock.patch.object(label, "SYNTHETIC", True):
            label.record_label(1000.0, 2)
        row = self.row()
        self.assertEqual(row["provenance"], "synthetic")
        self.assertFalse(row["eligible_for_training"])
        run.assert_not_called()
        state = json.loads(self.paths["STATE"].read_text())
        self.assertIn("ineligible", state["correlator"]["skipped"])

    def test_recent_context_excludes_future_and_stale_passes(self):
        self.paths["PASS_LOG"].write_text("\n".join(json.dumps({"at": at}) for at in (700, 800, 999, 1001)) + "\n")
        self.assertEqual([row["at"] for row in label.recent_passes(1000)], [800, 999])


if __name__ == "__main__":
    unittest.main()


class JustPairedBattery(unittest.TestCase):
    def test_empty_power_state_is_not_measured_and_never_pins_the_throttle(self):
        """A just-paired button has an empty power_state until its first
        report. Storing that as a measurement (with measured_at) would both
        lie and pin the 6 h refresh throttle on the lie."""
        opener = mock.Mock()
        button = {"data": [{"owner": {"rid": "device-1"}}]}
        power = {"data": [{"owner": {"rid": "device-1"}, "power_state": {}}]}
        contexts = []
        for _ in (button, power):
            context = mock.MagicMock()
            context.__enter__.return_value = mock.Mock()
            contexts.append(context)
        opener.open.side_effect = contexts
        with mock.patch.object(label.json, "load", side_effect=(button, power)), \
                mock.patch.object(label, "merge_state") as merge:
            label.refresh_battery(opener)
        battery = merge.call_args.args[0]["battery"]
        self.assertEqual(battery["state"], "not_measured")
        self.assertIn("no battery report", battery["reason"])
        self.assertNotIn("measured_at", battery)


class HeldButtonRegression(unittest.TestCase):
    def test_the_live_3_second_hold_counts_exactly_once(self):
        """Regression for the live 2026-08-17 05:19 hold: initial 08.228,
        repeats to 10.627, long_release 11.299 -- 3.07 s after the initial,
        just past the pairing window. The sliding anchor keeps the release
        paired to the last repeat; the hold is one press, not two."""
        logged = []
        x = label.PressExtractor(clock=lambda: 0.0,
                                 log=lambda *a, **k: logged.append(a[0]))
        seq = [("initial_press", "2026-08-17T05:19:08.228Z"),
               ("long_press", "2026-08-17T05:19:09.018Z"),
               ("repeat", "2026-08-17T05:19:09.023Z"),
               ("repeat", "2026-08-17T05:19:09.826Z"),
               ("repeat", "2026-08-17T05:19:10.627Z"),
               ("long_release", "2026-08-17T05:19:11.299Z")]
        tokens = [x.feed({"event": e, "updated": u}) for e, u in seq]
        kinds = [token[0] for token in tokens if token]
        # "extend" is intentionally emitted for each repeat so the gesture
        # tracker does not finalise early; it is not another counted press.
        self.assertEqual(kinds.count("press"), 1)
        self.assertEqual(kinds.count("hold"), 1)
        self.assertFalse(any("recovered press" in line for line in logged))
