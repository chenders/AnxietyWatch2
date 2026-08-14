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
