#!/usr/bin/env python3
"""Identify an unknown loud aircraft by correlating the owner's ear with ADS-B.

The problem
-----------

Portland Police air support is audibly distinctive -- the owner can recognise it
by sound -- but it frequently broadcasts no registration, and the two known
airframes are Cessna 182Ts whose type is shared with thousands of civil
aircraft. There is no field in the feed that says "this is the loud one".

There is, however, a Mode S address, which is broadcast whether or not anything
else is. So the question becomes: *which address is overhead when the owner
hears it, and not overhead when he does not?*

Why a likelihood ratio and not an intersection
----------------------------------------------

The obvious approach -- take the aircraft overhead at each marked time and
intersect the sets -- is wrong, and wrong in a way that produces a confident
false answer. The busiest airframes are overhead during almost every window,
marked or not, so they survive every intersection and rank first. Intersection
measures presence, and presence is not evidence.

What distinguishes the real aircraft is *contrast*: present when heard, absent
when not. This module scores each address by

    P(overhead | heard) / P(overhead | not heard)

A ratio near 1 means the address is equally common either way -- no information,
however many marks it appears in. A high ratio with enough marks behind it is a
real candidate. An address overhead at every mark AND at every quiet window
scores 1.0 and is correctly rejected, which is exactly the case an intersection
gets backwards.

Three states, as everywhere else
--------------------------------

`rank()` refuses to name a candidate until there is enough evidence, and says so
rather than returning a best guess. A single mark cannot distinguish a police
aircraft from a coincidence, and presenting one as an answer would produce a
roster entry that is then trusted forever. INSUFFICIENT is a real verdict here,
not a failure to compute one.

Nothing in this module writes to the roster. It proposes; a human confirms
against the FAA registry and edits `panel_loud_operators.py` by hand.
"""

import json
import os
import time
from pathlib import Path

MARKS_PATH = Path(os.environ.get(
    "PANEL_HEARD_MARKS", "/var/lib/panel-noise/heard-marks.jsonl"))

# How far either side of a mark to look. The owner marks the sound after
# noticing it, and a loitering aircraft is audible for minutes, so the window is
# generous and asymmetric toward the past.
WINDOW_BEFORE_S = 180.0
WINDOW_AFTER_S = 60.0

# Below this many marks, no candidate is named at any score. Chosen so a single
# coincidence cannot promote an airframe: with 5 marks, an unrelated aircraft
# has to coincide five times.
MIN_MARKS = 5

# A candidate must clear this ratio to be reported. 3.0 means "at least three
# times more likely to be overhead when heard than when not".
MIN_RATIO = 3.0

INSUFFICIENT = "INSUFFICIENT"
CANDIDATE = "CANDIDATE"


def mark_heard(at=None, note=None, suspected=None, path=None, event_id=None):
    """Record one heard event durably; an event ID makes retries idempotent."""
    target = Path(path) if path is not None else MARKS_PATH
    if event_id is not None:
        for existing in read_marks(target):
            if existing.get("event_id") == event_id:
                return existing
    record = {
        "at": time.time() if at is None else at,
        "note": note,
        "suspected": suspected,
        "event_id": event_id,
    }
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("a") as stream:
        stream.write(json.dumps(record, separators=(",", ":"), allow_nan=False))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    return record


def read_marks(path=None):
    target = Path(path) if path is not None else MARKS_PATH
    marks = []
    try:
        with target.open() as stream:
            for line in stream:
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except ValueError:
                    continue
                if isinstance(row, dict) and isinstance(row.get("at"), (int, float)):
                    marks.append(row)
    except OSError:
        return []
    return sorted(marks, key=lambda m: m["at"])


def _covered(mark_at, when):
    return mark_at - WINDOW_BEFORE_S <= when <= mark_at + WINDOW_AFTER_S


def overhead_at(observations, moment, radius_km):
    """Mode S addresses within `radius_km` at `moment`.

    `observations` is an iterable of dicts with `at`, `hex` and `distance_km` --
    the shape the loud-aircraft log and the FR24 cache both produce.
    """
    seen = set()
    for row in observations:
        when, hex_code = row.get("at"), row.get("hex")
        distance = row.get("distance_km")
        if not hex_code or not isinstance(when, (int, float)):
            continue
        if not isinstance(distance, (int, float)) or distance > radius_km:
            continue
        if _covered(moment, when):
            seen.add(str(hex_code).upper())
    return seen


def rank(marks, observations, quiet_moments, radius_km=6.0,
         min_marks=MIN_MARKS, min_ratio=MIN_RATIO):
    """Score every observed address by how well it explains the marks.

    `quiet_moments` are timestamps the owner did NOT report hearing anything.
    They are what turns presence into evidence -- without them every frequent
    flyer scores identically and the result is meaningless. Sample them from
    periods well away from any mark.

    Returns (verdict, rows). `verdict` is INSUFFICIENT until there are enough
    marks, and rows are ranked best-first regardless so the evidence can be
    inspected while it accumulates.
    """
    observations = list(observations)
    quiet_moments = list(quiet_moments)

    heard_sets = [overhead_at(observations, m["at"], radius_km) for m in marks]
    quiet_sets = [overhead_at(observations, q, radius_km) for q in quiet_moments]

    addresses = {h for s in heard_sets for h in s} | {h for s in quiet_sets for h in s}
    rows = []
    for address in addresses:
        heard_hits = sum(1 for s in heard_sets if address in s)
        quiet_hits = sum(1 for s in quiet_sets if address in s)
        p_heard = heard_hits / len(heard_sets) if heard_sets else 0.0
        # Laplace-style floor on the denominator. Without it an address seen at
        # one mark and never in a quiet window divides by zero and outranks an
        # address with ten times the evidence.
        p_quiet = (quiet_hits + 1) / (len(quiet_sets) + 1) if quiet_sets else 1.0
        rows.append({
            "hex": address,
            "heard_hits": heard_hits,
            "heard_total": len(heard_sets),
            "quiet_hits": quiet_hits,
            "quiet_total": len(quiet_sets),
            "ratio": p_heard / p_quiet if p_quiet else 0.0,
        })

    rows.sort(key=lambda r: (-r["ratio"], -r["heard_hits"], r["hex"]))

    if len(marks) < min_marks:
        return INSUFFICIENT, rows
    if not rows or rows[0]["ratio"] < min_ratio:
        return INSUFFICIENT, rows
    return CANDIDATE, rows


def explain(verdict, rows, min_marks=MIN_MARKS, min_ratio=MIN_RATIO):
    """A sentence naming the blocker and the value that blocked it."""
    if not rows:
        return "no aircraft were overhead in any marked window"
    best = rows[0]
    if verdict == INSUFFICIENT:
        if best["heard_total"] < min_marks:
            return (f"only {best['heard_total']} marks recorded; "
                    f"{min_marks} needed before naming a candidate")
        return (f"best ratio {best['ratio']:.1f} is below the {min_ratio:.1f} "
                f"threshold: {best['hex']} was overhead at "
                f"{best['heard_hits']}/{best['heard_total']} marks but also at "
                f"{best['quiet_hits']}/{best['quiet_total']} quiet windows")
    return (f"{best['hex']} was overhead at {best['heard_hits']}/{best['heard_total']} "
            f"marks and only {best['quiet_hits']}/{best['quiet_total']} quiet windows "
            f"(ratio {best['ratio']:.1f}); verify against the FAA registry before "
            "adding it to the roster")
