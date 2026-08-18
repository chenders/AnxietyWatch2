#!/usr/bin/env python3
"""Tests for the METAR cache that gives noise events their acoustic weather."""
import importlib.machinery
import json
import pathlib
import tempfile
import unittest
from unittest import mock

SCRIPT = pathlib.Path(__file__).with_name("panel-weather-poll")
weather = importlib.machinery.SourceFileLoader("panel_weather_poll", str(SCRIPT)).load_module()

METAR = {"icaoId": "KPDX", "reportTime": "2026-08-17 04:53:00",
         "temp": 17.2, "dewp": 12.8, "wdir": 300, "wspd": 9, "wgst": 17,
         "visib": "10+", "altim": 1015.2,
         "clouds": [{"cover": "FEW", "base": 4000}],
         "rawOb": "KPDX 170453Z 30009G17KT 10SM FEW040 17/13 A2998"}


class Projection(unittest.TestCase):
    def test_projection_keeps_absent_absent(self):
        out = weather.observation_from({"icaoId": "KPDX"})
        self.assertIsNone(out["temperature_c"])
        self.assertIsNone(out["wind_gust_kt"])
        self.assertIsNone(out["raw_metar"])

    def test_projection_carries_the_verbatim_record(self):
        out = weather.observation_from(METAR)
        self.assertEqual(out["temperature_c"], 17.2)
        self.assertEqual(out["wind_gust_kt"], 17.0)
        self.assertEqual(out["raw_metar"], METAR["rawOb"])
        self.assertEqual(out["clouds"], METAR["clouds"])


class Polling(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cache = pathlib.Path(self.tmp.name) / "weather.json"
        patcher = mock.patch.object(weather, "CACHE", self.cache)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_successful_poll_writes_the_cache(self):
        with mock.patch.object(weather, "fetch", return_value=METAR):
            self.assertEqual(weather.poll(now=1000.0), 0)
        doc = json.loads(self.cache.read_text())
        self.assertEqual(doc["fetched_at"], 1000.0)
        self.assertEqual(doc["observation"]["temperature_c"], 17.2)

    def test_failed_fetch_never_erases_the_last_good_observation(self):
        with mock.patch.object(weather, "fetch", return_value=METAR):
            weather.poll(now=1000.0)
        with mock.patch.object(weather, "fetch", side_effect=OSError("edge down")):
            self.assertEqual(weather.poll(now=2000.0), 1)
        doc = json.loads(self.cache.read_text())
        self.assertEqual(doc["observation"]["temperature_c"], 17.2)
        self.assertEqual(doc["fetched_at"], 1000.0)
        self.assertIn("edge down", doc["last_error"]["error"])
        self.assertEqual(doc["last_error"]["at"], 2000.0)


if __name__ == "__main__":
    unittest.main()
