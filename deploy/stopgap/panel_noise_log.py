#!/usr/bin/env python3
"""Append-only log of aircraft estimated to be loud over the station.

This exists so the admin console can eventually show "what flew over the house
and when" without the panel having to keep a live process running. It is the
only durable record this stopgap produces.

RETENTION -- READ THIS BEFORE CHANGING IT
-----------------------------------------

FR24's terms (`/docs/storage-rules`) state, verbatim:

    "All data accumulated from the FR24 API should not be stored for more than
    30 days from the date it was first received. After this period, all stored
    data must be permanently deleted."

The owner has directed that both sources be retained for at least six months,
because the point of this log is a season-scale record of what flies over the
house and a 30-day window cannot show a trend. That is a deliberate, informed
decision about the owner's own API licence, recorded here so it is not mistaken
for an oversight by a later reader -- or by a later agent, who must not silently
"fix" it back to 30 days.

The exposure is confined to `FR24_RETENTION_SECONDS` below. Setting it to
30 * 24 * 60 * 60 restores strict compliance and nothing else needs to change.

Retention is still split by provenance:

  - FR24-sourced rows expire after `FR24_RETENTION_SECONDS`.
  - Rows derived from the local ADS-B receiver carry no expiry at all. That
    data is ours -- it came off an antenna on this roof, not from a vendor --
    so no licence governs it, and the long-run record is the entire point.

`retain_until` is stored on each row rather than inferred at prune time so the
rule stays legible to anything that reads this file later, including a future
admin console that must not re-derive the policy and get it wrong.

The log is append-only in normal operation. `prune()` is the sole rewriter.
"""

import json
import os
import time
from pathlib import Path

DEFAULT_PATH = Path(os.environ.get(
    "PANEL_NOISE_LOG", "/var/lib/panel-noise/loud-aircraft.jsonl"))

# Six months, per the owner's decision above. FR24's terms say 30 days; see the
# module docstring before altering this in either direction.
FR24_RETENTION_SECONDS = float(os.environ.get(
    "PANEL_NOISE_FR24_RETENTION_SECONDS", 182 * 24 * 60 * 60))

FR24 = "fr24"
ADSB = "adsb"

# Deep links back to Flightradar24, so a row in the admin console can be opened
# against the source.
#
# Format verified 2026-08-12 against the live site: the aircraft page generates
# its own per-flight links in exactly this shape, e.g.
# `/data/aircraft/n406lf#411c02b5`. This is FR24's convention, not a guess.
#
# Caveat: the fragment selects a row only if that flight is inside the history
# window the account's plan has loaded (the page paginates with "Load earlier
# flights"). An older flight degrades to the aircraft page with no row selected,
# which is acceptable -- it is still the right aircraft.
FR24_SITE = "https://www.flightradar24.com"


def flight_url(registration, fr24_id):
    """Playback link for one specific flight, or None.

    The format is the aircraft page with the flight's fr24_id as a fragment:

        https://www.flightradar24.com/data/aircraft/n406lf#40934418

    It keys on *registration*, not callsign. That matters here, because FR24's
    API withholds `reg` for the Portland air-ambulance tails -- but the Mode S
    roster in `panel_air_ambulance` maps the broadcast address back to the tail,
    so a caller can supply the registration the API refused to. Identification
    done locally is what makes this link constructible at all.

    Returns None rather than a guess when either half is missing. A link built
    from an absent field would 404, and in an admin console a dead link reads as
    "the flight was not recorded" rather than "we were not told who it was".
    """
    if not registration or not fr24_id:
        return None
    return f"{FR24_SITE}/data/aircraft/{str(registration).strip().lower()}#{str(fr24_id).strip()}"


def aircraft_url(registration):
    """History for one airframe across all its flights, or None."""
    if not registration:
        return None
    return f"{FR24_SITE}/data/aircraft/{str(registration).strip().lower()}"


def retain_until_for(provenance, at):
    """When this row must be gone, or None if it may be kept indefinitely."""
    if provenance == FR24:
        return at + FR24_RETENTION_SECONDS
    return None


def build_event(
    at,
    provenance,
    callsign=None,
    hex_code=None,
    registration=None,
    aircraft_type=None,
    fr24_id=None,
    air_ambulance="UNCONFIRMED",
    air_ambulance_reason=None,
    distance_km=None,
    altitude_ft=None,
    phase=None,
    noise_index=None,
    loudness_band="UNKNOWN",
    reason=None,
):
    """One pass of one aircraft.

    `noise_index` stays None whenever the aircraft could not be scored, and
    `loudness_band` is then "UNKNOWN". Neither is ever defaulted to a number:
    an unscored aircraft that passed overhead is the single most important row
    this log can hold, and writing 0 or "FAINT" there would make it read as a
    quiet one. Absence of a score is not evidence of quiet.
    """
    return {
        "at": at,
        "provenance": provenance,
        "retain_until": retain_until_for(provenance, at),
        "identity": {
            "callsign": callsign,
            "hex": hex_code,
            "reg": registration,
            "type": aircraft_type,
            "fr24_id": fr24_id,
        },
        "links": {
            # Both are None when the identifiers they need were not reported.
            # The admin console should render nothing for a None, never a
            # dead link.
            "flight": flight_url(registration, fr24_id),
            "aircraft": aircraft_url(registration),
        },
        "air_ambulance": air_ambulance,
        "air_ambulance_reason": air_ambulance_reason,
        "distance_km": distance_km,
        "altitude_ft": altitude_ft,
        "phase": phase,
        "noise_index": noise_index,
        "loudness_band": loudness_band,
        "reason": reason,
    }


def append(event, path=None):
    """Append one event. Durable before returning.

    fsync is worth the cost here: these rows are minutes apart at most, and a
    loud pass that vanished in a power cut is unreconstructible -- the aircraft
    is gone and the API will not sell it back beyond its retention window.
    """
    target = Path(path) if path is not None else DEFAULT_PATH
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("a") as stream:
        stream.write(json.dumps(event, separators=(",", ":"), allow_nan=False))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def read_all(path=None):
    """Every readable row, oldest first.

    A corrupt line is skipped rather than fatal. A truncated final row is the
    expected outcome of a power cut mid-append, and refusing to read the file
    because of it would discard every good row before it.
    """
    target = Path(path) if path is not None else DEFAULT_PATH
    rows = []
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
                if isinstance(row, dict):
                    rows.append(row)
    except OSError:
        return []
    return rows


def prune(now=None, path=None):
    """Delete rows whose retention has expired. Returns rows removed.

    Only rows carrying a `retain_until` in the past are removed. A row without
    one is kept forever by design -- see the module docstring.
    """
    target = Path(path) if path is not None else DEFAULT_PATH
    moment = time.time() if now is None else now
    rows = read_all(target)
    if not rows:
        return 0

    keep = [r for r in rows
            if not isinstance(r.get("retain_until"), (int, float)) or r["retain_until"] > moment]
    removed = len(rows) - len(keep)
    if removed == 0:
        return 0

    tmp = target.with_name(f".{target.name}.tmp")
    with tmp.open("w") as stream:
        for row in keep:
            stream.write(json.dumps(row, separators=(",", ":"), allow_nan=False))
            stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(tmp, target)
    return removed


class PassTracker:
    """Collapses a stream of polls into one event per aircraft per pass.

    Without this the log would gain a row every poll -- at a 3 s interval, ~20
    rows for a single 60 s overflight -- and the admin console would show one
    helicopter as twenty events.

    A pass opens when an aircraft is first seen close enough to matter and
    closes when it has been absent for `gap_seconds`. The event is emitted on
    close, carrying the *closest* approach observed during the pass rather than
    the first or last, because the closest point is what was loud.

    The vendor's own geofence example keeps a set of alerted ids and never
    clears it, so an aircraft that leaves and returns is silently suppressed
    forever. That is exactly wrong here: a Life Flight helicopter lands at the
    Providence pad and departs again minutes later, and the departure -- climbing
    at full power over the house -- is the louder of the two events.
    """

    def __init__(self, gap_seconds=120.0):
        self.gap_seconds = gap_seconds
        self.open_passes = {}

    @staticmethod
    def key_for(report):
        """Stable identity for one pass.

        Prefers fr24_id, then hex, then callsign. FR24 withholds hex and
        callsign for the Portland air-ambulance tails in some endpoints, so no
        single field is reliable; falling through keeps a pass from splitting
        into several events when one identifier is missing.
        """
        for field in ("fr24_id", "hex", "callsign"):
            value = report.get(field)
            if value:
                return f"{field}:{value}"
        return None

    def observe(self, report, now):
        """Fold one observation into the open pass for that aircraft."""
        key = self.key_for(report)
        if key is None:
            return
        distance = report.get("distance_km")
        current = self.open_passes.get(key)
        if current is None:
            self.open_passes[key] = {"report": report, "closest": distance, "last_seen": now}
            return
        current["last_seen"] = now
        # Keep the report from the closest observation, so identity and phase
        # describe the moment that mattered rather than whichever poll was last.
        if isinstance(distance, (int, float)) and (
            not isinstance(current["closest"], (int, float)) or distance < current["closest"]
        ):
            current["closest"] = distance
            current["report"] = report

    def close_finished(self, now):
        """Return (key, report) for every pass that has gone quiet."""
        finished = [(key, state) for key, state in self.open_passes.items()
                    if now - state["last_seen"] >= self.gap_seconds]
        for key, _ in finished:
            del self.open_passes[key]
        return [(key, state["report"]) for key, state in finished]
