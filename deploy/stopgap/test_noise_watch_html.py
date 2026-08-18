#!/usr/bin/env python3
import pathlib
import unittest


class MarkedAirframeMapStyle(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.page = pathlib.Path(__file__).with_name("noise-watch.html").read_text()

    def test_marked_airframe_uses_pale_yellow_for_entire_marker(self):
        self.assertIn("--marked:#F3C95B", self.page)
        self.assertIn('const col = h ? "var(--marked)"', self.page)

    def test_disabled_hue_alarms_have_a_compact_crossed_alarm_indicator(self):
        self.assertIn('id="alarm-off"', self.page)
        self.assertIn('aria-label="alarm crossed out"', self.page)
        self.assertIn('ALARMS OFF', self.page)
        self.assertIn('S.hue.warnings_enabled === false', self.page)
        self.assertIn('pull("hue_status.json")', self.page)
        self.assertIn('catch(e){ /* hold state */ }', self.page)

    def test_cat_icon_depicts_fright_not_reporting(self):
        self.assertIn("Wide eyes plus outward startle marks", self.page)
        self.assertIn("cat was scared", self.page)
        self.assertNotIn("cat alarmed", self.page)

    def test_old_history_diamond_is_gone(self):
        self.assertNotIn("histDeco", self.page)
        self.assertNotIn('d="M-9,6 l3.2,3.2', self.page)


if __name__ == "__main__":
    unittest.main()
