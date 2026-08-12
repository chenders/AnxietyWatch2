#!/usr/bin/env python3
"""Identify known-loud operators, preferably without spending a credit.

The problem this solves
-----------------------

A 7-minute survey of the local receiver measured ~50% of aircraft broadcasting
no ADS-B emitter category at all. Without a category the noise model cannot pick
an airframe class, so it scores nothing and reports UNKNOWN -- and the aircraft
that actually generate complaints here are over-represented in that group. The
loudest, lowest traffic over this station is exactly the traffic the receiver
cannot classify.

FR24 supplies the ICAO type, but not reliably for these aircraft: for the
Portland-based Life Flight tails it returns `reg`, `hex` and `callsign` as null
from the flight-summary and registration endpoints, while still returning them
in flight-tracks and live positions. Identification cannot depend on any single
field being present.

The roster below closes the gap for the fleets that actually fly here. It maps
the Mode S address -- which is what dump1090 reports, and which an aircraft
broadcasts whether or not it also broadcasts a category -- to the airframe and
its operator. That makes identification free and local: no API call, no credit,
no network, and it keeps working when FR24 is unreachable or the subscription
lapses.

Two operator classes are represented, because the owner reports both as dominant
noise sources at this address:

  - AIR_AMBULANCE -- Life Flight Network, serving the Providence Portland
    helipad 1.6 km west of the station. Measured 2026-08-11: a departure off
    that pad passes the station at under 1 km, at 600 ft, climbing at over
    1000 fpm.
  - POLICE -- Portland Police Bureau air support, which the owner reports as
    extremely loud and near-continuously airborne. Loiter, not transit: a patrol
    aircraft orbits, so it is loud for far longer than a helicopter passing
    through, and a model tuned only to closest-approach will understate it.

Provenance of the roster
------------------------

Collected 2026-08-12 from Flightradar24's public aircraft pages
(flightradar24.com/data/aircraft/<reg>), each of which lists operator, ICAO type
code and Mode S address. Cross-checked: the API independently returned hex
A556C2 for N443LF, matching the site. These are static airframe facts about
specific aircraft, not accumulated API data, so the 30-day API storage rule does
not reach them; the same facts are in the public FAA N-number registry, which is
the citation to prefer if this list ever needs defending.

The list is deliberately hand-maintained rather than fetched. It is a handful of
rows, it changes only when an operator buys or retires an airframe, and a lookup
that silently returns nothing is far worse here than one that is occasionally
stale -- a missing entry only falls back to the callsign test below.
"""

import re

AIR_AMBULANCE = "AIR_AMBULANCE"
POLICE = "POLICE"

CONFIRMED = "CONFIRMED"
UNCONFIRMED = "UNCONFIRMED"

OPERATORS = {
    "LIFE_FLIGHT": {
        "name": "Life Flight Network",
        "kind": AIR_AMBULANCE,
        # Every Life Flight tail observed over this station matches N<digits>LF.
        # Testing the pattern as well as the roster means a tail the roster has
        # never seen is still recognised the first time it flies, instead of
        # being silently unclassified until someone updates a table.
        "callsign_pattern": re.compile(r"^N\d+LF$", re.IGNORECASE),
        # Rotorcraft types in this operator's fleet. Type alone is never
        # identification -- other operators fly the same airframes -- so it only
        # ever produces a hint.
        "fleet_types": frozenset({"A119", "B407", "EC35", "EC45"}),
    },
    "PORTLAND_POLICE": {
        "name": "Portland Police Bureau",
        "kind": POLICE,
        # Deliberately None. One spotter source lists N2163J operating as flight
        # ID "IDAHO44", but that is a single unverified claim and the flight ID
        # is pilot-entered, frequently blank, and rotatable. Filtering on it
        # would both miss real flights and mis-attribute unrelated ones. The
        # Mode S address below is burned into the transponder and is the only
        # stable identifier -- match on that.
        "callsign_pattern": None,
        # Empty on purpose. PPB flies Cessna 182Ts, an airframe shared with
        # thousands of civil aircraft, so type carries no signal whatsoever
        # here -- unlike the air-ambulance rotorcraft, where it is at least a
        # hint.
        "fleet_types": frozenset(),
    },
}

# Registrations that must NEVER be attributed to an operator, with the reason.
#
# N739MR is the 1978 Cessna 172N that PPB surplused and sold in 2023. It is now
# privately owned and still flying locally -- it was in fact the single most
# frequent registration in the 14-day Aurora (KUAO) sample pulled on 2026-08-12,
# with five flights. Any identification scheme built from stale PPB rosters will
# attribute those flights to the police. This list exists so that mistake is
# made impossible rather than merely unlikely.
RETIRED_REGISTRATIONS = {
    "N739MR": "surplused and sold by Portland Police Bureau in 2023; now private",
}

# Mode S / ICAO 24-bit address -> airframe. Keys are uppercase hex without
# separators, which is the form dump1090 emits and the form FR24 returns.
ROSTER = {
    # --- Life Flight Network ---
    # Portland-based. Its flight history is almost entirely Hillsboro (HIO) and
    # PDX legs, several a day -- by volume this is the airframe most likely to
    # be overhead, and it is the one to check first when something is missed.
    "A4C3EB": {"reg": "N406LF", "type": "B407", "model": "Bell 407GXi",
               "operator": "LIFE_FLIGHT"},
    "AA8051": {"reg": "N776LF", "type": "A119", "model": "Leonardo AW119Kx Koala",
               "operator": "LIFE_FLIGHT"},
    "AB4825": {"reg": "N826LF", "type": "EC35", "model": "Airbus Helicopters H135",
               "operator": "LIFE_FLIGHT"},
    "AB8BFE": {"reg": "N843LF", "type": "EC45", "model": "Airbus Helicopters H145",
               "operator": "LIFE_FLIGHT"},
    "A590D4": {"reg": "N458LF", "type": "B407", "model": "Bell 407GXi",
               "operator": "LIFE_FLIGHT"},
    # Redmond-based; reaches Portland only occasionally. Kept because it is the
    # one tail FR24's API will name, which makes it the reference case when
    # checking whether identification is working at all.
    "A556C2": {"reg": "N443LF", "type": "B407", "model": "Bell 407GXi",
               "operator": "LIFE_FLIGHT"},

    # --- Portland Police Bureau air support ---
    # Both airframes are FIXED-WING Cessna 182T. PPB operates no helicopters, so
    # a rotorcraft orbiting Portland belongs to some other agency (OSP, Clark
    # County, federal, or a news ship) and must be logged as such rather than
    # attributed here.
    #
    # This has a direct consequence for the FR24 poller: it filters on
    # `categories=H,N`, and a Cessna 182T is category T (General Aviation). PPB
    # aircraft are therefore INVISIBLE to that poll. They are, however, exactly
    # the case the local receiver handles well -- they orbit for long periods at
    # 3,400-5,400 ft, well inside line of sight -- so the roster is what makes
    # them identifiable, for free, without touching the API. See README.
    "A1D296": {"reg": "N2163J", "type": "C182", "model": "Cessna 182T (2003)",
               "operator": "PORTLAND_POLICE"},
    "A50E9F": {"reg": "N425EJ", "type": "C182", "model": "Cessna 182T (2015)",
               "operator": "PORTLAND_POLICE"},
}


def normalise_hex(value):
    """Uppercase bare hex, or None.

    Rejects the '~' prefix dump1090 uses for non-ICAO addresses: a TIS-B or
    ADS-R track carries a synthetic address that must never be matched against a
    real airframe.
    """
    text = str(value or "").strip().upper()
    if not text or text.startswith("~"):
        return None
    return text if re.fullmatch(r"[0-9A-F]{6}", text) else None


def lookup_hex(hex_code):
    """Roster entry for a Mode S address, or None."""
    key = normalise_hex(hex_code)
    return ROSTER.get(key) if key else None


def operator_name(entry):
    record = OPERATORS.get(entry.get("operator")) if entry else None
    return record["name"] if record else "unknown operator"


def operator_kind(entry):
    """AIR_AMBULANCE, POLICE, or None when the airframe is not on the roster."""
    record = OPERATORS.get(entry.get("operator")) if entry else None
    return record["kind"] if record else None


def identify(hex_code=None, callsign=None, aircraft_type=None):
    """Three states, never two.

    Returns (state, reason, airframe). `state` is CONFIRMED only when the Mode S
    address is on the roster or the callsign matches an operator's registration
    pattern. Everything else is UNCONFIRMED -- which does NOT mean "not a
    known-loud operator" and must never be rendered, ranked or logged as though
    it did. A shared airframe type downgrades to a hint, not an identity.

    `airframe` is the roster entry when the address was recognised, else None,
    so a caller can obtain the ICAO type and registration for an aircraft that
    broadcast no emitter category at all, and that FR24 declined to name. That
    is the entire point of the roster.
    """
    call = str(callsign or "").strip()

    # Retired airframes are rejected before anything else, including the roster.
    # A sold aircraft keeps its Mode S address and its registration, so a stale
    # entry anywhere upstream would otherwise still match.
    retired = RETIRED_REGISTRATIONS.get(call.upper())
    if retired is not None:
        return UNCONFIRMED, f"{call.upper()} was {retired}", None

    entry = lookup_hex(hex_code)
    if entry is not None:
        if entry["reg"].upper() in RETIRED_REGISTRATIONS:
            return UNCONFIRMED, (
                f"{entry['reg']} was {RETIRED_REGISTRATIONS[entry['reg'].upper()]}"
            ), None
        return CONFIRMED, (
            f"Mode S {normalise_hex(hex_code)} is {entry['reg']}, "
            f"{entry['model']} operated by {operator_name(entry)}"
        ), entry

    if call:
        for record in OPERATORS.values():
            pattern = record["callsign_pattern"]
            if pattern is not None and pattern.match(call):
                return CONFIRMED, (
                    f"registration {call.upper()} matches {record['name']}"
                ), None

    kind = str(aircraft_type or "").strip().upper()
    if kind:
        for record in OPERATORS.values():
            if kind in record["fleet_types"]:
                return UNCONFIRMED, (
                    f"type {kind} is flown by {record['name']} but also by other "
                    "operators; no matching Mode S address or registration reported"
                ), None

    if not call and not kind and normalise_hex(hex_code) is None:
        return UNCONFIRMED, "no Mode S address, callsign or type reported", None

    return UNCONFIRMED, "does not match a known-loud airframe or registration", None
