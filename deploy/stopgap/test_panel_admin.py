#!/usr/bin/env python3
"""Tests for the admin console's write path.

The rule: a value the daemons will ignore must never be accepted. A console
that appears to save a setting the system then disregards is worse than one
that refuses, because the operator believes the light is armed when it is not.
"""
import importlib
import importlib.machinery
import json
import os
import pathlib
import tempfile
import unittest


def load(settings_path, case):
    """A module pair pointed at a throwaway settings file.

    `case` registers the cleanup. These tests mutate a PROCESS-GLOBAL: the
    settings path lives in an environment variable and the module caches what
    it read. Without restoring both, `panel-adsb-status` in the same run picks
    up this test's temporary ambient and three of its band assertions fail --
    which is exactly what happened when this file was first written. A suite
    that passes only in isolation is worse than one that fails.
    """
    before = os.environ.get("PANEL_SETTINGS_JSON")
    os.environ["PANEL_SETTINGS_JSON"] = str(settings_path)
    import panel_settings
    importlib.reload(panel_settings)

    def restore():
        if before is None:
            os.environ.pop("PANEL_SETTINGS_JSON", None)
        else:
            os.environ["PANEL_SETTINGS_JSON"] = before
        importlib.reload(panel_settings)
    case.addCleanup(restore)

    admin = importlib.machinery.SourceFileLoader(
        "panel_admin", str(pathlib.Path(__file__).with_name("panel-admin"))).load_module()
    admin.panel_settings = panel_settings
    return admin, panel_settings


class Validation(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.path = pathlib.Path(self.dir.name) / "settings.json"
        self.admin, self.settings = load(self.path, self)

    def test_a_valid_ambient_is_accepted(self):
        ok, reasons = self.admin.save({"ambient_dba": 35})
        self.assertTrue(ok, reasons)
        self.assertEqual(json.loads(self.path.read_text())["settings"]["ambient_dba"], 35.0)

    def test_an_out_of_range_ambient_is_refused_with_the_value_named(self):
        ok, reasons = self.admin.save({"ambient_dba": 0})
        self.assertFalse(ok)
        self.assertTrue(any("ambient_dba" in r and "0" in r for r in reasons), reasons)

    def test_an_unknown_setting_is_refused_rather_than_stored(self):
        """Storing it would put a key in the file that nothing reads, which
        looks like a working setting on the next page load."""
        ok, reasons = self.admin.save({"make_it_quieter": True})
        self.assertFalse(ok)
        self.assertIn("make_it_quieter", reasons[0])

    def test_an_unknown_band_is_refused(self):
        ok, reasons = self.admin.save({"hue_bands": ["LOUD", "DEAFENING"]})
        self.assertFalse(ok)
        self.assertIn("DEAFENING", reasons[0])

    def test_nothing_is_written_when_any_value_is_rejected(self):
        """All-or-nothing. A partial save leaves the page showing values that
        are half applied, which is harder to reason about than a refusal."""
        self.admin.save({"ambient_dba": 35})
        before = self.path.read_text()
        ok, _ = self.admin.save({"ambient_dba": 42, "hue_bands": ["NONSENSE"]})
        self.assertFalse(ok)
        self.assertEqual(self.path.read_text(), before)

    def test_every_reason_is_reported_not_just_the_first(self):
        ok, reasons = self.admin.save({"ambient_dba": 999, "hue_bands": ["NOPE"]})
        self.assertFalse(ok)
        self.assertEqual(len(reasons), 2, reasons)

    def test_an_empty_band_list_is_accepted(self):
        # "Fire on nothing" is a legitimate request and must be obeyable.
        ok, reasons = self.admin.save({"hue_bands": []})
        self.assertTrue(ok, reasons)
        self.assertEqual(json.loads(self.path.read_text())["settings"]["hue_bands"], [])

    def test_an_empty_body_clears_everything_to_defaults(self):
        self.admin.save({"ambient_dba": 35})
        ok, _ = self.admin.save({})
        self.assertTrue(ok)
        self.assertEqual(json.loads(self.path.read_text())["settings"], {})

    def test_null_clears_one_setting_without_touching_the_rest(self):
        self.admin.save({"ambient_dba": 35, "hue_lead_seconds": 60})
        ok, _ = self.admin.save({"ambient_dba": None, "hue_lead_seconds": 60})
        self.assertTrue(ok)
        stored = json.loads(self.path.read_text())["settings"]
        self.assertNotIn("ambient_dba", stored)
        self.assertEqual(stored["hue_lead_seconds"], 60.0)

    def test_a_non_object_body_is_refused(self):
        for bad in ([], "text", 5, None):
            ok, _ = self.admin.save(bad)
            self.assertFalse(ok, bad)

    def test_what_is_saved_is_what_the_daemons_then_read(self):
        """The whole contract. Validated, written, and read back through the
        same layer the publisher uses."""
        self.admin.save({"ambient_dba": 33})
        self.assertEqual(
            self.settings.number("ambient_dba", "UNSET_ENV", 40.0,
                                 minimum=25.0, maximum=70.0), 33.0)


class AircraftButtonHealth(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.admin, _ = load(pathlib.Path(self.dir.name) / "s.json", self)

    def test_missing_health_is_degraded_and_names_absence(self):
        health = self.admin.button_health({}, 1000.0)
        self.assertEqual(health["state"], "degraded")
        self.assertIn("stream_state=None", health["blockers"])
        self.assertIn("stream_changed_at is absent", health["blockers"])

    def test_connected_stream_with_fresh_activity_is_healthy(self):
        health = self.admin.button_health({"stream_state": "connected",
                                           "stream_changed_at": 900.0,
                                           "stream_last_activity_at": 980.0}, 1000.0)
        self.assertEqual(health["state"], "healthy")
        self.assertEqual(health["pending_gestures"], 0)
        self.assertEqual(health["stream_activity_age_s"], 20.0)

    def test_connected_claim_without_received_bytes_is_degraded(self):
        """'connected' is a claim; received bytes are the evidence. A half-open
        TCP session looks connected forever without this check."""
        health = self.admin.button_health({"stream_state": "connected",
                                           "stream_changed_at": 900.0}, 1000.0)
        self.assertEqual(health["state"], "degraded")
        self.assertTrue(any("no activity was ever recorded" in b
                            for b in health["blockers"]), health["blockers"])

    def test_connected_with_stale_activity_names_the_age_and_limit(self):
        stale = 1000.0 - self.admin.STREAM_ACTIVITY_STALE_SECONDS - 60
        health = self.admin.button_health({"stream_state": "connected",
                                           "stream_changed_at": 900.0,
                                           "stream_last_activity_at": stale}, 1000.0)
        self.assertEqual(health["state"], "degraded")
        self.assertTrue(any("no bridge traffic for" in b and "limit" in b
                            for b in health["blockers"]), health["blockers"])

    def test_pending_retry_and_critical_battery_are_visible(self):
        health = self.admin.button_health({
            "stream_state": "connected", "stream_changed_at": 900.0,
            "stream_last_activity_at": 990.0,
            "pending_gestures": [{"at": 950.0, "count": 1}],
            "battery": {"battery_state": "critical", "battery_level": 1},
        }, 1000.0)
        self.assertEqual(health["state"], "degraded")
        self.assertIn("pending_gestures=1", health["blockers"])
        self.assertIn("battery_state=critical", health["blockers"])

    def test_disconnect_names_state_error_and_duration(self):
        health = self.admin.button_health({"stream_state": "disconnected",
                                           "stream_changed_at": 600.0,
                                           "stream_error": "authorization failed"}, 1000.0)
        self.assertIn("stream_state='disconnected'", health["blockers"])
        self.assertIn("disconnected for 400.0s", health["blockers"])
        self.assertIn("stream_error=authorization failed", health["blockers"])


class WarningReview(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.admin, _ = load(pathlib.Path(self.dir.name) / "s.json", self)
        self.admin.WARNINGS_JSONL = pathlib.Path(self.dir.name) / "warnings.jsonl"
        self.admin.WARNING_REVIEWS_JSONL = pathlib.Path(self.dir.name) / "reviews.jsonl"
        self.admin.WARNINGS_JSONL.write_text(
            json.dumps({"warning_id": "w1", "at": 1000.0, "registration": "N170US"}) + "\n")

    def test_review_is_durable_and_joined_to_warning(self):
        ok, row = self.admin.review_warning(
            {"warning_id": "w1", "verdict": "accurate"}, now=1100.0)
        self.assertTrue(ok, row)
        state = self.admin.warning_history(now=1300.0)
        self.assertEqual(state["total"], 1)
        self.assertEqual(state["rows"][0]["review"]["verdict"], "accurate")

    def test_review_can_be_revised_and_latest_wins(self):
        self.admin.review_warning({"warning_id": "w1", "verdict": "accurate"}, now=1100.0)
        self.admin.review_warning({"warning_id": "w1", "verdict": "inaccurate"}, now=1200.0)
        review = self.admin.warning_history(now=1300.0)["rows"][0]["review"]
        self.assertEqual(review["verdict"], "inaccurate")
        self.assertEqual(len(self.admin.WARNING_REVIEWS_JSONL.read_text().splitlines()), 2)

    def test_warning_drops_from_queue_after_three_hours(self):
        before = self.admin.warning_history(now=1000.0 + 3 * 60 * 60)
        after = self.admin.warning_history(now=1000.0 + 3 * 60 * 60 + 0.1)
        self.assertEqual(before["total"], 1)
        self.assertEqual(after["total"], 0)
        self.assertEqual(after["archived_total"], 1)

    def test_unknown_warning_is_refused_by_name(self):
        ok, reason = self.admin.review_warning(
            {"warning_id": "missing", "verdict": "accurate"}, now=1100.0)
        self.assertFalse(ok)
        self.assertIn("missing", reason)

    def test_unknown_verdict_is_durable_but_explicit(self):
        ok, row = self.admin.review_warning(
            {"warning_id": "w1", "verdict": "unknown"}, now=1100.0)
        self.assertTrue(ok, row)
        self.assertEqual(
            self.admin.warning_history(now=1300.0)["rows"][0]["review"]["verdict"],
            "unknown")

    def test_invalid_verdict_is_refused_by_value(self):
        ok, reason = self.admin.review_warning(
            {"warning_id": "w1", "verdict": "maybe"}, now=1100.0)
        self.assertFalse(ok)
        self.assertIn("maybe", reason)


class AdminPageSeparationAndAccessibility(unittest.TestCase):
    def setUp(self):
        root = pathlib.Path(__file__).parent
        self.status = (root / "admin.html").read_text()
        self.settings = (root / "admin-settings.html").read_text()

    def test_status_page_has_no_settings_form_or_save_button(self):
        self.assertNotIn('id="settings"', self.status)
        self.assertNotIn('id="save"', self.status)
        self.assertIn('href="/settings"', self.status)

    def test_settings_are_on_their_own_page(self):
        self.assertIn('id="settings"', self.settings)
        self.assertIn('id="save"', self.settings)
        self.assertIn('href="/"', self.settings)

    def test_tables_default_to_ten_and_url_state_uses_history_api(self):
        self.assertIn("allowed = [10, 25, 100]", self.status)
        self.assertIn("history.pushState", self.status)
        self.assertIn('addEventListener("popstate"', self.status)

    def test_live_designs_use_responsive_cards_not_wide_tables(self):
        self.assertIn('class="dashboard"', self.status)
        self.assertIn('@media(max-width:700px)', self.status)
        self.assertIn('@media(max-width:430px)', self.status)
        self.assertNotIn('<table', self.status)

    def test_dynamic_evidence_fields_are_html_escaped(self):
        self.assertIn('esc(r.suppressed_by||"reason unavailable")', self.status)
        self.assertIn('esc(e.candidate_count??"—")', self.status)
        self.assertIn('esc(e.margin??"—")', self.status)

    def test_contextual_help_is_accessible(self):
        self.assertIn('id="help-dialog"', self.status)
        self.assertIn('data-help="warning_review"', self.status)
        self.assertIn('data-help="event_verdict"', self.status)
        self.assertIn('aria-label="Explain warning reviews"', self.status)

    def test_operator_selected_compact_text_scale(self):
        self.assertIn("font:13px/1.5", self.status)
        self.assertIn("font-size:12px", self.status)
        self.assertIn("font:13px/1.55", self.settings)

    def test_all_three_live_designs_share_a_persistent_picker(self):
        for page in (self.status, self.settings):
            self.assertIn('id="view-select"', page)
            self.assertIn('value="operations"', page)
            self.assertIn('value="briefing"', page)
            self.assertIn('value="map"', page)
            self.assertIn('noiseWatchAdminDesign', page)
            self.assertIn('localStorage.setItem(STORE', page)
        self.assertIn('return"operations"', self.status)
        self.assertIn('return"operations"', self.settings)

    def test_direct_view_links_are_preserved_between_pages(self):
        self.assertIn('u.searchParams.set("view",view)', self.status)
        self.assertIn('"/settings?view="+view', self.status)
        self.assertIn('u.searchParams.set("view",v)', self.settings)
        self.assertIn('"/?view="+v', self.settings)

    def test_map_visibility_preference_is_persistent(self):
        self.assertIn('noiseWatchHideMap', self.status)
        self.assertIn('syncMapVisibility()', self.status)

    def test_map_never_invents_an_aircraft_position(self):
        self.assertIn('!Number.isFinite(r.bearing)||!Number.isFinite(r.distance_km)',
                      self.status)
        self.assertIn('r.distance_km<0||r.distance_km>16.1', self.status)
        self.assertIn('OUTSIDE AIRCRAFT NOT PLOTTED', self.status)
        self.assertIn('No watched aircraft with a measured position', self.status)

    def test_global_alarm_control_requires_typed_confirmation(self):
        self.assertIn('id="danger"', self.settings)
        self.assertIn('id="disable-dialog"', self.settings)
        self.assertIn('e.target.value!=="DISABLE ALARMS"', self.settings)
        self.assertIn('DISABLE ALARMS GLOBALLY', self.settings)
        self.assertIn('ENABLE ALARMS GLOBALLY', self.settings)

    def test_global_alarm_change_preserves_other_persisted_overrides(self):
        self.assertIn('payload={...(STATE.settings.settings||{}),hue_enabled:enabled}',
                      self.settings)
        self.assertIn('global alarm change blocked: unsaved settings are present',
                      self.settings)

    def test_disabled_state_is_unmissable_on_both_admin_pages(self):
        for page in (self.status, self.settings):
            self.assertIn('ALL NOISE WATCH HUE ALARMS ARE DISABLED', page)
            self.assertIn('id="disarmed"', page)
        self.assertIn('id="alarm-control-link"', self.status)
        self.assertIn('role="alert"', self.status)
        self.assertIn('role="alert"', self.settings)

    def test_both_pages_preserve_colors_when_printed(self):
        for page in (self.status, self.settings):
            self.assertIn("print-color-adjust:exact", page)
            self.assertIn("@media print", page)


class LiveMapGeometry(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        root = pathlib.Path(self.dir.name)
        self.admin, _ = load(root / "settings.json", self)
        for name in ("STATUS_JSON", "FR24_JSON", "HUE_JSON", "BUTTON_JSON",
                     "ATTRIBUTIONS_JSON", "WEATHER_JSON"):
            path = root / (name.lower() + ".json")
            path.write_text("{}")
            setattr(self.admin, name, path)
        for name in ("PROPOSALS_JSONL", "WARNINGS_JSONL", "WARNING_REVIEWS_JSONL"):
            path = root / (name.lower() + ".jsonl")
            path.write_text("")
            setattr(self.admin, name, path)
        self.admin.unit_state = lambda _name: "active"
        self.admin.disk_free = lambda: {"free_gb": 10, "used_pct": 1}

    def test_state_exposes_real_receiver_geometry_for_map_lens(self):
        self.admin.STATUS_JSON.write_text(json.dumps({"adsb_predictions": [{
            "hex": "A11B00", "registration": "N170US", "bearing": 271.5,
            "distance_km": 4.25, "alt_ft": 6800,
            "prediction": {"loudness_band": "LOUD", "eta_seconds": 28},
        }]}))
        row = self.admin.state()["watching"][0]
        self.assertEqual(row["bearing"], 271.5)
        self.assertEqual(row["distance_km"], 4.25)
        self.assertEqual(row["alt_ft"], 6800)

    def test_missing_geometry_stays_absent(self):
        self.admin.STATUS_JSON.write_text(json.dumps({"adsb_predictions": [{
            "hex": "ABC123", "prediction": {"loudness_band": "UNKNOWN"},
        }]}))
        row = self.admin.state()["watching"][0]
        self.assertIsNone(row["bearing"])
        self.assertIsNone(row["distance_km"])


class SchemaIsTheOnlySourceOfBounds(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.admin, self.settings = load(pathlib.Path(self.dir.name) / "s.json", self)

    def test_every_setting_declares_a_label_and_a_note(self):
        # The console renders these; a missing one would ship a blank control.
        for name, spec in self.settings.SCHEMA.items():
            self.assertTrue(spec.get("label"), name)
            self.assertTrue(spec.get("note"), name)

    def test_every_number_declares_both_bounds(self):
        for name, spec in self.settings.SCHEMA.items():
            if spec["kind"] == "number":
                self.assertIn("min", spec, name)
                self.assertIn("max", spec, name)
                self.assertLess(spec["min"], spec["max"], name)

    def test_every_default_passes_its_own_validator(self):
        # A default outside its own bounds would make the revert button write
        # a value the daemons refuse.
        for name, spec in self.settings.SCHEMA.items():
            ok, _ = self.settings.validate(name, spec["default"])
            self.assertTrue(ok, name)

    def test_every_word_setting_lists_its_choices(self):
        for name, spec in self.settings.SCHEMA.items():
            if spec["kind"] == "words":
                self.assertTrue(spec.get("choices"), name)
                for value in spec["default"]:
                    self.assertIn(value, spec["choices"], name)


if __name__ == "__main__":
    unittest.main()
