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
    # --- Neighbouring agencies ---
    #
    # These are NOT Portland Police. They are listed so their flights are
    # attributed correctly instead of being mistaken for PPB: several operate
    # the same Cessna airframes, out of the same fields, on the same kind of
    # orbiting mission. Distinguishing them is the point.
    #
    # They are still POLICE for display purposes -- an orbiting sheriff's
    # aircraft is exactly as loud over this house as an orbiting city one -- so
    # the glyph is the same and only the named operator differs.
    "WASHINGTON_CO_SO": {
        "name": "Washington County Sheriff (OR)",
        "kind": POLICE, "callsign_pattern": None, "fleet_types": frozenset(),
    },
    "MULTNOMAH_CO_SO": {
        "name": "Multnomah County Sheriff (OR)",
        "kind": POLICE, "callsign_pattern": None, "fleet_types": frozenset(),
    },
    "CLACKAMAS_CO_SO": {
        "name": "Clackamas County Sheriff (OR)",
        "kind": POLICE, "callsign_pattern": None, "fleet_types": frozenset(),
    },
    "CLARK_CO_SO": {
        "name": "Clark County Sheriff (WA)",
        "kind": POLICE, "callsign_pattern": None, "fleet_types": frozenset(),
    },
    "OREGON_STATE_POLICE": {
        "name": "Oregon State Police",
        "kind": POLICE, "callsign_pattern": None, "fleet_types": frozenset(),
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
    "N739MR": ("surplused by Portland Police Bureau and re-registered to a private "
               "owner on 2024-02-07; still based at Aurora"),
}

# The same exclusion keyed on the Mode S address, which is the only join key
# that actually reaches this code from a receiver.
#
# A9ED9D is N739MR's address, confirmed twice over: named in the FAA registry
# research and observed directly in the 14-day Aurora sample on 2026-08-12,
# where it was the single most frequent registration with five flights. It is
# flying near this station regularly, right now, under an address that a stale
# Portland Police roster would happily match. Excluding it by registration alone
# would not have worked -- a receiver reports the address, not the tail.
RETIRED_HEXES = {
    "A9ED9D": ("N739MR, surplused by Portland Police Bureau and sold to a private "
               "owner in 2024; five flights observed at Aurora in the 14 days to "
               "2026-08-12"),
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
    # Added 2026-08-12 after the running system caught it unaided. It is not
    # Portland-based -- its history is mostly Pendleton and eastern Oregon -- but
    # it flew three passes over this station overnight, the closest at 1.31 km,
    # and the roster had never heard of it. It was identified anyway by the
    # N<digits>LF callsign pattern below, which is the entire reason that
    # fallback exists: a tail nobody has added yet still gets recognised the
    # first time it flies. Adding the address recovers the registration and type
    # that FR24 withholds, and with them the flight-playback links.
    "A5731C": {"reg": "N450LF", "type": "B407", "model": "Bell 407GXi",
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
    # N2163J flies as IDAHO44 ("IDAHO" is the law-enforcement ATC callsign
    # series). N425EJ's callsign is undocumented -- observed flight IDs are
    # logged per hex so the table builds itself rather than being guessed.
    "A1D296": {"reg": "N2163J", "type": "C182", "model": "Cessna 182T (2003)",
               "operator": "PORTLAND_POLICE"},
    "A50E9F": {"reg": "N425EJ", "type": "C182", "model": "Cessna 182T (2015)",
               "operator": "PORTLAND_POLICE"},

    # --- Neighbouring agencies, so their flights are not misattributed ---
    # ACE9D0 and A9ED9D below are not theory: both appeared in the 14-day
    # Aurora sample pulled 2026-08-12, with exactly these addresses.
    # Doc 8643 lists the R172 under C172; "R172" is not a designator and would
    # classify as unknown, silently un-scoring a sheriff's aircraft.
    "A76288": {"reg": "N575RW", "type": "C172", "model": "Cessna R172K Hawk XP",
               "operator": "WASHINGTON_CO_SO"},
    "AC9869": {"reg": "N910TW", "type": "C172", "model": "Cessna R172K",
               "operator": "MULTNOMAH_CO_SO"},
    "ACE9D0": {"reg": "N931EM", "type": "C182", "model": "Cessna 182P",
               "operator": "CLACKAMAS_CO_SO"},
    # The only police ROTORCRAFT in the area. If a police helicopter is
    # overhead it is this one, not Portland -- PPB has never operated one.
    "ACBD1A": {"reg": "N92PD", "type": "B06", "model": "Bell OH-58C",
               "operator": "CLARK_CO_SO"},
    "A1EF84": {"reg": "N2237S", "type": "C182", "model": "Cessna T182T",
               "operator": "OREGON_STATE_POLICE"},
    # Registered to "State of Oregon" rather than literally to OSP; the
    # association is probable, not certain, and the model string says so.
    "A1D9C2": {"reg": "N2181N", "type": "C182",
               "model": "Cessna 182T (State of Oregon; OSP use probable)",
               "operator": "OREGON_STATE_POLICE"},
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


def kind_for(hex_code=None, callsign=None):
    """AIR_AMBULANCE, POLICE, or None -- from EITHER identifier.

    Exists because a Life Flight airframe is CONFIRMED by its registration
    pattern, a path that carries no roster entry, so operator_kind() alone
    reported the entire medical fleet as kind None (seen on the admin page
    2026-08-17: police rows carried their kind, medical rows did not).

    Sources are the hand-curated roster and callsign patterns ONLY -- never
    learned lookup data, which must not be able to promote an airframe to a
    kind that fires a bedroom light. Retired airframes yield None.
    """
    key = normalise_hex(hex_code)
    if key and key in RETIRED_HEXES:
        return None
    entry = lookup_hex(hex_code)
    if entry is not None and entry["reg"].upper() not in RETIRED_REGISTRATIONS:
        return operator_kind(entry)
    call = str(callsign or "").strip()
    if call and call.upper() not in RETIRED_REGISTRATIONS:
        for record in OPERATORS.values():
            pattern = record["callsign_pattern"]
            if pattern is not None and pattern.match(call):
                return record["kind"]
    return None


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
    # A sold aircraft keeps flying under the same Mode S address, so a stale
    # entry anywhere upstream would otherwise still match. The address is
    # checked first because that is what a receiver actually reports.
    retired_hex = RETIRED_HEXES.get(normalise_hex(hex_code) or "")
    if retired_hex is not None:
        return UNCONFIRMED, f"excluded: {retired_hex}", None

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
