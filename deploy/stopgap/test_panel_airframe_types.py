#!/usr/bin/env python3
"""Tests for ICAO type -> airframe noise class."""

import unittest
import panel_airframe_types as types


class Classification(unittest.TestCase):
    def test_the_types_this_station_actually_sees(self):
        # Every one of these was received by the dongle or returned by a lookup
        # during development.
        cases = {
            "B739": "jet",        # Delta 737-900ER, previously untypeable
            "B763": "heavy",      # 767 freighter — NOT the same as a 737
            "A320": "jet",
            "B77W": "heavy",
            "E175": "jet",        # shares emitter category A3 with the 737 MAX
            "B38M": "jet",
            "C172": "light",
            "PC12": "light",
            "B407": "rotorcraft", # the Life Flight fleet
            "A119": "rotorcraft",
            "EC35": "rotorcraft",
            "EC45": "rotorcraft",
            "B06": "rotorcraft",  # Clark County's OH-58C
            "C182": "light",      # both Portland Police aircraft
            "R172": "light",
        }
        for code, expected in cases.items():
            self.assertEqual(types.classify(code), expected, code)

    def test_the_distinction_the_emitter_category_cannot_make(self):
        # A3 covers both. The whole point of using the exact type is that these
        # stop being the same source level.
        self.assertEqual(types.classify("B38M"), "jet")
        self.assertEqual(types.classify("B763"), "heavy")
        self.assertNotEqual(types.classify("B38M"), types.classify("B763"))

    def test_an_unknown_designator_stays_unknown(self):
        # Never folded into "jet" because most traffic is jets: that would let
        # an unidentified return borrow a narrowbody's source level.
        self.assertIsNone(types.classify("ZZZZ"))
        self.assertIsNone(types.classify(None))
        self.assertIsNone(types.classify(""))

    def test_an_emitter_category_is_refused_as_a_type(self):
        # "A3" is a size class, not a designator. Accepting it would classify
        # every such row as some Airbus A3-something. This exact conflation
        # already happened once and put categories in a type column.
        self.assertIsNone(types.normalise_type("A3"))
        self.assertIsNone(types.classify("A3"))
        # But a real 4-character designator beginning A3 is fine.
        self.assertEqual(types.classify("A332"), "heavy")

    def test_designators_are_case_and_space_insensitive(self):
        self.assertEqual(types.classify(" b739 "), "jet")
        self.assertEqual(types.classify("ec35"), "rotorcraft")

    def test_malformed_input_is_refused_rather_than_guessed(self):
        for bad in ("B", "TOOLONG", "B-73", "747-400", 739, {}):
            self.assertIsNone(types.normalise_type(bad))

    def test_no_designator_is_in_two_classes(self):
        # A duplicate would make classification order-dependent and silently
        # wrong for whichever set lost.
        seen = {}
        for members, name in types._CLASSES:
            for code in members:
                self.assertNotIn(code, seen,
                                 f"{code} is in both {seen.get(code)} and {name}")
                seen[code] = name

    def test_the_table_has_not_been_accidentally_truncated(self):
        self.assertGreater(types.known_count(), 200)


if __name__ == "__main__":
    unittest.main()
