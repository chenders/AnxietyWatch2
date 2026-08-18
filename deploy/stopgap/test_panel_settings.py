#!/usr/bin/env python3
"""Tests for the runtime settings layer.

The rule these all serve: a malformed value is REJECTED and the previous
behaviour kept, with the reason available. Silently defaulting would let a
typo disarm the warning light, and a light that does not fire is
indistinguishable from a quiet sky.
"""
import importlib
import json
import os
import pathlib
import tempfile
import unittest


def load(path=None, env=None):
    """A fresh module with its own settings path and environment."""
    for key in [k for k in os.environ if k.startswith("TEST_")]:
        del os.environ[key]
    os.environ.update(env or {})
    os.environ["PANEL_SETTINGS_JSON"] = str(path or "/nonexistent/settings.json")
    import panel_settings
    return importlib.reload(panel_settings)


class Precedence(unittest.TestCase):
    """settings file > environment > code default."""

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.path = pathlib.Path(self.dir.name) / "settings.json"

    def write(self, settings):
        self.path.write_text(json.dumps({"settings": settings}))

    def test_the_code_default_applies_when_nothing_overrides(self):
        s = load(self.path)
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 40.0)

    def test_the_environment_overrides_the_default(self):
        s = load(self.path, {"TEST_AMBIENT": "35"})
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 35.0)

    def test_the_file_overrides_the_environment(self):
        self.write({"ambient_dba": 30})
        s = load(self.path, {"TEST_AMBIENT": "35"})
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 30.0)

    def test_a_missing_file_is_not_an_error(self):
        # It means nothing has been overridden, which is the state of every
        # box until someone opens the admin console.
        s = load(self.path)
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 40.0)
        self.assertIsNone(s.last_error())

    def test_a_null_in_the_file_falls_through_rather_than_blanking(self):
        self.write({"ambient_dba": None})
        s = load(self.path, {"TEST_AMBIENT": "35"})
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 35.0)


class MalformedInputIsRejected(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.path = pathlib.Path(self.dir.name) / "settings.json"

    def test_a_half_written_file_keeps_the_previous_configuration(self):
        """A save in progress must not blank a running warning system."""
        self.path.write_text('{"settings": {"ambient_dba": 3')
        s = load(self.path)
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 40.0)
        self.assertIsNotNone(s.last_error())

    def test_the_error_names_the_file(self):
        # Guards must say why, and which file.
        self.path.write_text("not json at all")
        s = load(self.path)
        self.assertIn(str(self.path), s.last_error())

    def test_a_non_numeric_value_falls_back(self):
        self.path.write_text(json.dumps({"settings": {"ambient_dba": "quiet"}}))
        s = load(self.path)
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 40.0)

    def test_an_out_of_range_value_is_refused_not_clamped(self):
        """Clamping is forbidden here for the same reason it is forbidden for
        readings: a clamped value is indistinguishable downstream from one that
        was actually asked for. An ambient of 0 would call every aircraft VERY
        LOUD; one of 200 would silence the system."""
        for bad in (0, 200, -5):
            self.path.write_text(json.dumps({"settings": {"ambient_dba": bad}}))
            s = load(self.path)
            self.assertEqual(
                s.number("ambient_dba", "TEST_AMBIENT", 40.0, minimum=25.0, maximum=70.0),
                40.0, bad)

    def test_a_value_inside_the_range_is_honoured(self):
        self.path.write_text(json.dumps({"settings": {"ambient_dba": 32}}))
        s = load(self.path)
        self.assertEqual(
            s.number("ambient_dba", "TEST_AMBIENT", 40.0, minimum=25.0, maximum=70.0), 32.0)


class Reloading(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.path = pathlib.Path(self.dir.name) / "settings.json"

    def test_a_change_is_picked_up_without_a_restart(self):
        """The whole point: the publisher rewrites the panel once a second and
        restarting it drops the warning state mid-pass."""
        self.path.write_text(json.dumps({"settings": {"ambient_dba": 30}}))
        s = load(self.path)
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 30.0)
        os.utime(self.path, (0, 0))   # force a distinct mtime
        self.path.write_text(json.dumps({"settings": {"ambient_dba": 50}}))
        self.assertEqual(s.number("ambient_dba", "TEST_AMBIENT", 40.0), 50.0)


class Words(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.path = pathlib.Path(self.dir.name) / "settings.json"

    def test_a_list_and_a_comma_string_are_equivalent(self):
        default = frozenset(("VERY LOUD", "LOUD"))
        self.path.write_text(json.dumps({"settings": {"hue_bands": ["LOUD", "AUDIBLE"]}}))
        s = load(self.path)
        self.assertEqual(s.words("hue_bands", "TEST_BANDS", default),
                         frozenset(("LOUD", "AUDIBLE")))
        os.utime(self.path, (0, 0))
        self.path.write_text(json.dumps({"settings": {"hue_bands": "LOUD, AUDIBLE"}}))
        self.assertEqual(s.words("hue_bands", "TEST_BANDS", default),
                         frozenset(("LOUD", "AUDIBLE")))

    def test_an_empty_set_is_honoured_not_replaced_by_the_default(self):
        """"Fire on nothing" is a legitimate request. Quietly re-arming a light
        someone deliberately disarmed would be worse than obeying them."""
        self.path.write_text(json.dumps({"settings": {"hue_bands": []}}))
        s = load(self.path)
        self.assertEqual(s.words("hue_bands", "TEST_BANDS",
                                 frozenset(("VERY LOUD",))), frozenset())


class DisplayUnits(unittest.TestCase):
    def test_the_units_setting_exists_and_defaults_imperial(self):
        # Owner decision 2026-08-17: miles unless the toggle says otherwise.
        spec = load().SCHEMA["display_metric"]
        self.assertEqual(spec["kind"], "flag")
        self.assertFalse(spec["default"])

    def test_the_lead_schema_default_matches_the_warn_daemon(self):
        # The admin page renders the schema default; the daemon uses its call
        # site default. They drifted once (45 vs 30) -- this pins them.
        self.assertEqual(load().SCHEMA["hue_lead_seconds"]["default"], 30.0)


class Snapshot(unittest.TestCase):
    def test_snapshot_reports_what_is_overridden_and_from_where(self):
        s = load()
        snap = s.snapshot()
        self.assertIn("path", snap)
        self.assertFalse(snap["present"])
        self.assertEqual(snap["settings"], {})


if __name__ == "__main__":
    unittest.main()
