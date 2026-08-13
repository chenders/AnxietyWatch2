#!/usr/bin/env python3
"""Map an ICAO type designator to an airframe noise class.

Why this exists
---------------

The ADS-B emitter category is uselessly coarse: `A3` covers a 737 MAX and an
E175 alike, and roughly half the aircraft this receiver hears broadcast no
category at all. An exact ICAO type designator -- `B739`, `EC35`, `C172` -- is
far more informative, and a free lookup service supplies one for most airframes.

This module is the bridge between that designator and the four source-level
classes the noise model actually uses.

What this is NOT
----------------

It is not measured noise data. It is a mapping from type to the same four coarse
buckets the model already had; the improvement is that the bucket is now chosen
from the actual airframe rather than from a category that cannot distinguish a
widebody from a regional jet. Replacing the buckets themselves with certificated
noise figures (ICAO Annex 16 / FAA AC36-1H publish EPNdB per type) is a separate
and much larger piece of work.

Unknown stays unknown
---------------------

An unrecognised designator returns None. It is NOT folded into "jet" on the
grounds that most traffic is jets: that would let an unidentified return borrow
a narrowbody's source level, which is the exact failure the three-state rule
exists to prevent. A type this table has never seen is reported as unscorable,
and shows on the panel as UNKNOWN.
"""

import re

# AUDITED 2026-08-13 against ICAO Doc 8643 (engine type and wake category per
# designator). That audit found five real errors, all of the same shape as the
# TBM 850 miss: aircraft sitting in a quieter bucket than they belong in.
#
#   B752/B753  heavy -> jet        single-aisle, WTC M
#   PC24       light -> jet        the Pilatus PC-24 is a jet
#   GA5C       light -> jet        Gulfstream business jet
#   DHC2       turboprop -> light  the Beaver is radial-piston
#
# It also caught six entries that are not Doc 8643 designators at all and would
# never have matched anything: PA28, R172, E175, DA20, A220, B731. The real
# codes are P28A/P28S, C172, E75L/E75S, DV20, BCS1/BCS3 and B732.
#
# H125/H130/H135/H145/H155 and the 747/777 freighter codes are absent from the
# reference dataset because it predates those designators, not because they are
# wrong. They are retained deliberately.
#
# Explicit designators, grouped by the model's source-level classes. Listed
# rather than pattern-matched wherever a pattern would be wrong: "B06" is a Bell
# 206 helicopter while "B06x" prefixes nothing else, but "B74" is a 747 and
# "B733" a 737, so a bare "B7" prefix rule would be useless.
HEAVY = frozenset({
    # Widebodies. Four-engine and twin-aisle, the loudest civil traffic here.
    "A124", "A332", "A333", "A337", "A338", "A339", "A342", "A343", "A345",
    "A346", "A359", "A35K", "A388", "B741", "B742", "B743", "B744", "B748",
    "B74R", "B74S", "B762", "B763", "B764",
    "B772", "B773", "B77L", "B77W", "B778", "B779", "B788", "B789",
    "B78X", "DC10", "MD11", "IL76", "C5M", "C17",
})
JET = frozenset({
    # Narrowbodies and regional jets.
    "A19N", "A20N", "A21N", "A318", "A319", "A320", "A321", "B37M", "B38M",
    "B39M", "B3XM", "B712", "B721", "B722", "B732", "B733", "B734",
    "B735", "B736", "B737", "B738", "B739", "B752", "B753", "BCS1", "BCS3", "CRJ1", "CRJ2",
    "CRJ7", "CRJ9", "CRJX", "E135", "E145", "E170", "E190", "E195",
    "E75L", "E75S", "E290", "E295", "MD81", "MD82", "MD83", "MD87", "MD88",
    "MD90", "SU95", "PC24", "GA5C",
    # Business jets. Smaller, but still turbofan.
    "C25A", "C25B", "C25C", "C25M", "C500", "C510", "C525", "C550", "C55B",
    "C560", "C56X", "C650", "C680", "C68A", "C700", "C750", "CL30", "CL35",
    "CL60", "E50P", "E55P", "F2TH", "FA50", "FA7X", "FA8X", "GL5T", "GLEX",
    "GLF4", "GLF5", "GLF6", "G280", "H25B", "HA4T", "LJ35", "LJ40", "LJ45",
    "LJ60", "LJ75", "PRM1",
})
LIGHT = frozenset({
    # Piston and turboprop singles and light twins.
    "AA5", "AC11", "BE33", "BE35", "BE36", "BE58", "C150", "C152", "C162", "C172", "C177", "C180", "C182", "C185",
    "C206", "C210", "C310", "C337", "C82S", "DHC2", "C72R", "DA40", "DA42", "DV20",
    "GLST", "M20P", "P28A", "P28B", "P28R", "P28S", "P32R", "PA12", "PA18", "PA22", "PA24", "PA31",
    "PA32", "PA34", "PA44", "PA46", "RV4", "RV6",
    "RV7", "RV8", "RV9", "RV10", "RV12", "RV14", "S22T", "SR20", "SR22",
    "T206", 
})
# Single- and twin-engine turboprops. Split out of LIGHT after a TBM 850 passed
# 1.07 km overhead at 2,075 ft on 2026-08-13 and the model called it FAINT while
# the owner plainly heard it. A 700 hp PT6 is not a Cessna 172, and grouping
# them cost a real warning.
TURBOPROP = frozenset({
    "TBM7", "TBM8", "TBM9", "PC12", "P46T", "M600", "EPIC", "C208", "DHC6",
    "BE20", "B350", "BE9L", "BE9T", "C441", "AT8T", "AT5T", "SF34",
    "DH8A", "DH8B", "DH8C", "DH8D", "AT43", "AT45", "AT72", "AT75", "AT76",
    "E110", "SW4", "BE99", "C212", "D228", "PC6T",
})
ROTORCRAFT = frozenset({
    "A109", "A119", "A139", "A169", "A189", "AS32", "AS3B", "AS50", "AS55",
    "AS65", "B06", "B06T", "B105", "B212", "B222", "B230", "B407", "B412",
    "B429", "B430", "B505", "EC20", "EC25", "EC30", "EC35", "EC45", "EC55",
    "EC75", "H125", "H130", "H135", "H145", "H155", "H160", "H500", "H60",
    "H64", "MD52", "MD60", "R22", "R44", "R66", "S76", "S92", "UH1", "S64",
})

_CLASSES = (
    (ROTORCRAFT, "rotorcraft"),
    (HEAVY, "heavy"),
    (JET, "jet"),
    (TURBOPROP, "turboprop"),
    (LIGHT, "light"),
)


def normalise_type(value):
    """Uppercase ICAO type designator, or None.

    Requires a STRING of 3-4 alphanumeric characters.

    ICAO Doc 8643 permits designators of 2-4 characters, so the lower bound here
    is deliberately stricter than the standard. The reason is concrete: every
    ADS-B emitter category is exactly two characters -- a letter and a digit,
    A0-A7, B0-B7 and so on -- and one ("A3") was already written into a type
    column by a fallback in this system. A two-character token arriving here is
    far more likely to be a miscategorised emitter class than a real designator,
    and admitting it would classify those rows as some Airbus A3-something.
    Every designator this module knows is three or four characters, so nothing
    real is lost.

    Non-strings are refused rather than coerced: str(739) is "739", which looks
    like a valid designator and is not one.
    """
    if not isinstance(value, str):
        return None
    text = value.strip().upper()
    return text if re.fullmatch(r"[A-Z0-9]{3,4}", text) else None


def classify(icao_type):
    """Airframe noise class for a designator, or None when it is not known.

    None is a real answer, not a failure. It propagates to an unscorable
    aircraft, which the panel shows as UNKNOWN -- never as quiet.
    """
    code = normalise_type(icao_type)
    if code is None:
        return None
    for members, name in _CLASSES:
        if code in members:
            return name
    return None


def known_count():
    """How many designators this table can classify. Used by the tests to keep
    an accidental truncation of a set from passing silently."""
    return sum(len(members) for members, _ in _CLASSES)
