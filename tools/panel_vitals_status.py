#!/usr/bin/env python3
"""Publish durable BP/H10/EMAY status plus incrementally refreshed history.

PostgreSQL is deliberately authoritative: the panel only claims a durable
reading after the spool drainer commits it. The small status document is polled
once per second. Full H10 and EMAY history is written separately and carries a
revision; the status document contains only points added since that revision.
"""

from __future__ import annotations

import contextlib
import json
import math
import os
import pathlib
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from typing import Any

OUTPUT = pathlib.Path(
    os.environ.get(
        "PANEL_VITALS_STATUS_JSON",
        "/opt/anxietywatch-panel/docs/design/pi-panel/direction-a-clinical/"
        "vitals_status.json",
    )
)
HISTORY_OUTPUT = pathlib.Path(
    os.environ.get("PANEL_H10_HISTORY_JSON", str(OUTPUT.with_name("vitals_h10_history.json")))
)
DATABASE = os.environ.get("PANEL_VITALS_DATABASE", "vitals")
INTERVAL = float(os.environ.get("PANEL_VITALS_INTERVAL_SECONDS", "1"))
HISTORY_PUBLISH_INTERVAL = float(os.environ.get("PANEL_H10_HISTORY_INTERVAL_SECONDS", "300"))
LIVE_H10 = pathlib.Path(os.environ.get("H10_LIVE_STATUS_JSON", "/run/vitals-h10/status.json"))
LIVE_H10_HISTORY = pathlib.Path(
    os.environ.get("H10_LIVE_HISTORY_JSONL", "/run/vitals-h10/history.jsonl")
)
LIVE_H10_MAX_AGE = float(os.environ.get("H10_LIVE_STATUS_MAX_AGE_SECONDS", "10"))
LIVE_H10_HISTORY_MAX_AGE = float(
    os.environ.get("H10_LIVE_HISTORY_MAX_AGE_SECONDS", "86400")
)

STATUS_SQL = r"""
WITH bp_latest AS (
 SELECT session_id,seq,timestamp FROM samples WHERE source=6 AND type=6
 ORDER BY timestamp DESC LIMIT 1
), bp AS (
 SELECT json_build_object('sample_at',extract(epoch FROM b.timestamp),
  'session_id',b.session_id,'seq',b.seq,
  'systolic',max(s.value) FILTER(WHERE s.type=6),
  'diastolic',max(s.value) FILTER(WHERE s.type=7),
  'map',max(s.value) FILTER(WHERE s.type=8),
  'pulse_bpm',max(s.value) FILTER(WHERE s.type=1),
  -- Read from the stored row, never hardcoded: when cuff event time gains a
  -- device clock, its provenance must surface without changing this publisher.
  'event_time',coalesce(substring(max(convert_from(s.extra,'UTF8'))
   from 'event_time=([a-z_]+)'),'unknown'),
  'irregular_pulse',coalesce(max(convert_from(s.extra,'UTF8'))
   LIKE '%irregular_pulse=true%',false),
  'body_movement',coalesce(max(convert_from(s.extra,'UTF8'))
   LIKE '%body_movement=true%',false),
  'cuff_too_loose',coalesce(max(convert_from(s.extra,'UTF8'))
   LIKE '%cuff_too_loose=true%',false)) value
 FROM bp_latest b JOIN samples s
  ON s.source=6 AND s.session_id=b.session_id AND s.seq=b.seq
 GROUP BY b.session_id,b.seq,b.timestamp
), h10 AS (
 SELECT json_build_object('sample_at',extract(epoch FROM timestamp),
  'session_id',session_id,'seq',seq,'hr',value,'test_origin',
  CASE WHEN source=7 THEN 'polar_h10_emulator'
       WHEN extra IS NOT NULL AND convert_from(extra,'UTF8')
         LIKE '%test_origin=polar_h10_emulator%' THEN 'polar_h10_emulator'
       WHEN extra IS NOT NULL AND convert_from(extra,'UTF8')
         LIKE '%test_origin=physical_device%' THEN 'physical_device'
       ELSE 'unknown' END) value
 FROM samples WHERE source IN (1,7) AND type=1
 ORDER BY timestamp DESC LIMIT 1
), emay_latest AS (
 SELECT session_id,seq,max(timestamp) AS timestamp
 FROM samples WHERE source=0 AND type IN (1,2)
 GROUP BY session_id,seq
 ORDER BY max(timestamp) DESC,session_id DESC,seq DESC LIMIT 1
), emay AS (
 SELECT json_build_object('sample_at',extract(epoch FROM e.timestamp),
  'session_id',e.session_id,'seq',e.seq,
  'spo2',max(s.value) FILTER(WHERE s.type=2),
  'pulse_bpm',max(s.value) FILTER(WHERE s.type=1),
  'stored',true,'synthetic',false,'link','stored') value
 FROM emay_latest e JOIN samples s
  ON s.source=0 AND s.session_id=e.session_id AND s.seq=e.seq
 GROUP BY e.session_id,e.seq,e.timestamp
), h10_hr_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=1 AND type=1
  AND timestamp >= to_timestamp(:'hr_since'::double precision)
), h10_rr_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=1 AND type=5
  AND timestamp >= to_timestamp(:'rr_since'::double precision)
), emay_spo2_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=0 AND type=2
  AND timestamp >= to_timestamp(:'spo2_since'::double precision)
), emay_pulse_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=0 AND type=1
  AND timestamp >= to_timestamp(:'pulse_since'::double precision)
)
SELECT json_build_object('schema',2,
 'units',json_build_object('hr','bpm','rr','seconds','spo2','percent','pulse','bpm'),
 'history_origin','physical_device',
 'emay_history_origin','physical_device_stored',
 'bp',(SELECT value FROM bp),'h10',(SELECT value FROM h10),
 'emay',(SELECT value FROM emay),
 'h10_delta',json_build_object('hr',(SELECT value FROM h10_hr_delta),
  'rr',(SELECT value FROM h10_rr_delta)),
 'emay_delta',json_build_object('spo2',(SELECT value FROM emay_spo2_delta),
  'pulse',(SELECT value FROM emay_pulse_delta)))::text;
"""

HISTORY_SQL = r"""
WITH h10_hr_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=1 AND type=1
), h10_rr_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=1 AND type=5
), emay_spo2_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=0 AND type=2
), emay_pulse_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value) ORDER BY timestamp), '[]'::json) value
 FROM samples WHERE source=0 AND type=1
)
SELECT json_build_object('schema',2,
 'units',json_build_object('hr','bpm','rr','seconds','spo2','percent','pulse','bpm'),
 'history_origin','physical_device',
 'emay_history_origin','physical_device_stored',
 'h10_history',json_build_object('hr',(SELECT value FROM h10_hr_history),
  'rr',(SELECT value FROM h10_rr_history)),
 'emay_history',json_build_object('spo2',(SELECT value FROM emay_spo2_history),
  'pulse',(SELECT value FROM emay_pulse_history)))::text;
"""

Point = dict[str, float]


@dataclass
class HistoryState:
    """Append-only point collections with O(1) retransmission deduplication."""

    hr: list[Point] = field(default_factory=list)
    rr: list[Point] = field(default_factory=list)
    spo2: list[Point] = field(default_factory=list)
    pulse: list[Point] = field(default_factory=list)
    _hr_keys: set[tuple[float, float]] = field(default_factory=set)
    _rr_keys: set[tuple[float, float]] = field(default_factory=set)
    _spo2_keys: set[tuple[float, float]] = field(default_factory=set)
    _pulse_keys: set[tuple[float, float]] = field(default_factory=set)

    def merge(self, kind: str, additions: Any) -> bool:
        collections = {
            "hr": (self.hr, self._hr_keys),
            "rr": (self.rr, self._rr_keys),
            "spo2": (self.spo2, self._spo2_keys),
            "pulse": (self.pulse, self._pulse_keys),
        }
        collection = collections.get(kind)
        if collection is None or not isinstance(additions, list):
            return False
        target, keys = collection
        changed = False
        needs_sort = False
        last_time = target[-1]["t"] if target else float("-inf")
        for raw in additions:
            if not isinstance(raw, dict):
                continue
            try:
                timestamp = float(raw["t"])
                value = float(raw["v"])
            except (KeyError, TypeError, ValueError):
                continue
            if not (math.isfinite(timestamp) and math.isfinite(value)):
                continue
            key = (timestamp, value)
            if key in keys:
                continue
            if timestamp < last_time:
                needs_sort = True
            target.append({"t": timestamp, "v": value})
            keys.add(key)
            last_time = max(last_time, timestamp)
            changed = True
        if needs_sort:
            target.sort(key=lambda point: (point["t"], point["v"]))
        return changed

    def merge_payload(self, payload: Any) -> bool:
        if not isinstance(payload, dict):
            return False
        changed = False
        for kind in ("hr", "rr", "spo2", "pulse"):
            changed = self.merge(kind, payload.get(kind)) or changed
        return changed

    def payload(self) -> dict[str, list[Point]]:
        """Return the H10 payload kept for the existing status contract."""
        return self.h10_payload()

    def h10_payload(self) -> dict[str, list[Point]]:
        return {"hr": self.hr, "rr": self.rr}

    def emay_payload(self) -> dict[str, list[Point]]:
        return {"spo2": self.spo2, "pulse": self.pulse}

    def unseen_payload(
        self,
        payload: Any,
        kinds: tuple[str, ...] = ("hr", "rr"),
    ) -> dict[str, list[Point]]:
        """Return valid points not already present without mutating this state."""
        unseen = HistoryState()
        if not isinstance(payload, dict):
            return unseen.payload_for(kinds)
        collections = {
            "hr": self._hr_keys,
            "rr": self._rr_keys,
            "spo2": self._spo2_keys,
            "pulse": self._pulse_keys,
        }
        for kind in kinds:
            keys = collections.get(kind)
            additions = payload.get(kind)
            if keys is None or not isinstance(additions, list):
                continue
            for raw in additions:
                if not isinstance(raw, dict):
                    continue
                try:
                    timestamp = float(raw["t"])
                    value = float(raw["v"])
                except (KeyError, TypeError, ValueError):
                    continue
                key = (timestamp, value)
                if math.isfinite(timestamp) and math.isfinite(value) and key not in keys:
                    unseen.merge(kind, [{"t": timestamp, "v": value}])
        return unseen.payload_for(kinds)

    def payload_for(self, kinds: tuple[str, ...]) -> dict[str, list[Point]]:
        return {kind: getattr(self, kind) for kind in kinds}

    def latest_times(self) -> tuple[float, float]:
        hr_time = self.hr[-1]["t"] if self.hr else 0.0
        rr_time = self.rr[-1]["t"] if self.rr else 0.0
        return hr_time, rr_time

    def latest_emay_times(self) -> tuple[float, float]:
        spo2_time = self.spo2[-1]["t"] if self.spo2 else 0.0
        pulse_time = self.pulse[-1]["t"] if self.pulse else 0.0
        return spo2_time, pulse_time

    def clear(self) -> None:
        self.hr.clear()
        self.rr.clear()
        self.spo2.clear()
        self.pulse.clear()
        self._hr_keys.clear()
        self._rr_keys.clear()
        self._spo2_keys.clear()
        self._pulse_keys.clear()


def run_query(sql: str, variables: dict[str, float] | None = None) -> dict[str, Any]:
    command = ["psql", "-X", "-qAt", "-U", "vitals", "-d", DATABASE]
    for name, value in (variables or {}).items():
        command.append(f"--set={name}={value:.6f}")
    # psql does not expand :variables in a `-c` string; stdin is processed as
    # a psql script and does. Values above are formatted finite floats, never
    # user-provided SQL fragments.
    process = subprocess.run(
        command,
        input=sql,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        detail = process.stderr.strip() or process.stdout.strip() or "no psql diagnostic"
        raise RuntimeError(f"psql query failed: returncode={process.returncode}, detail={detail}")
    return json.loads(process.stdout.strip())


def read_live_status() -> dict[str, Any] | None:
    try:
        with LIVE_H10.open(encoding="utf-8") as stream:
            live = json.load(stream)
        if time.time() - float(live["written_at"]) > LIVE_H10_MAX_AGE:
            return None
        return {
            "sample_at": live["sample_at"],
            "session_id": live["session_id"],
            "seq": live["seq"],
            "hr": live["hr"],
            "rr_count": live["rr_count"],
            "rr_points": live.get("rr_points", []),
            "test_origin": live["test_origin"],
            "live": True,
        }
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return None


def read_live_history(cutoff: float) -> dict[str, list[Point]]:
    hr: list[Point] = []
    rr: list[Point] = []
    minimum_time = max(cutoff, time.time() - LIVE_H10_HISTORY_MAX_AGE)
    try:
        with LIVE_H10_HISTORY.open(encoding="utf-8") as stream:
            for line in stream:
                try:
                    point = json.loads(line)
                    sample_at = float(point["sample_at"])
                    if sample_at < minimum_time or point["test_origin"] != "physical_device":
                        continue
                    hr.append({"t": sample_at, "v": point["hr"]})
                    rr.extend(point.get("rr_points", []))
                except (KeyError, TypeError, ValueError, json.JSONDecodeError):
                    continue
    except OSError:
        pass
    return {"hr": hr, "rr": rr}


def query_history() -> HistoryState:
    data = run_query(HISTORY_SQL)
    state = HistoryState()
    state.merge_payload(data.get("h10_history"))
    state.merge_payload(data.get("emay_history"))
    state.merge_payload(read_live_history(0.0))
    return state


def query_status(
    hr_since: float,
    rr_since: float,
    spo2_since: float = 0.0,
    pulse_since: float = 0.0,
) -> dict[str, Any]:
    data = run_query(
        STATUS_SQL,
        {
            "hr_since": hr_since,
            "rr_since": rr_since,
            "spo2_since": spo2_since,
            "pulse_since": pulse_since,
        },
    )
    h10_delta = HistoryState()
    h10_delta.merge_payload(data.get("h10_delta"))
    live = read_live_status()
    if live is not None:
        data["h10"] = live
        if live["test_origin"] == "physical_device":
            h10_delta.merge("hr", [{"t": live["sample_at"], "v": live["hr"]}])
            h10_delta.merge("rr", live.get("rr_points"))
    data["h10_delta"] = h10_delta.h10_payload()

    emay_delta = HistoryState()
    emay_delta.merge_payload(data.get("emay_delta"))
    data["emay_delta"] = emay_delta.emay_payload()
    return data


def next_revision(previous: int = 0) -> int:
    return max(previous + 1, time.time_ns() // 1_000_000)


def publish_json(path: pathlib.Path, data: dict[str, Any]) -> None:
    document = dict(data)
    document["written_at"] = time.time()
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary_path = pathlib.Path(temporary)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(document, stream, separators=(",", ":"), allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        temporary_path.chmod(0o600)
        temporary_path.replace(path)
    finally:
        with contextlib.suppress(FileNotFoundError):
            temporary_path.unlink()


def publish_history(history: HistoryState, revision: int) -> None:
    publish_json(
        HISTORY_OUTPUT,
        {
            "schema": 2,
            "revision": revision,
            "units": {"hr": "bpm", "rr": "seconds", "spo2": "percent", "pulse": "bpm"},
            "history_origin": "physical_device",
            "emay_history_origin": "physical_device_stored",
            "h10_history": history.h10_payload(),
            "emay_history": history.emay_payload(),
        },
    )


def main() -> None:
    history: HistoryState | None = None
    pending = HistoryState()
    history_revision = 0
    history_dirty = False
    last_history_publish = 0.0
    last_good: dict[str, Any] = {
        "schema": 2,
        "units": {"hr": "bpm", "rr": "seconds", "spo2": "percent", "pulse": "bpm"},
        "history_origin": "physical_device",
        "emay_history_origin": "physical_device_stored",
        "bp": None,
        "h10": None,
        "emay": None,
        "h10_delta": {"hr": [], "rr": []},
        "emay_delta": {"spo2": [], "pulse": []},
    }

    while True:
        try:
            if history is None:
                history = query_history()
                history_revision = next_revision(history_revision)
                publish_history(history, history_revision)
                last_history_publish = time.monotonic()

            hr_since, rr_since = history.latest_times()
            spo2_since, pulse_since = history.latest_emay_times()
            status = query_status(hr_since, rr_since, spo2_since, pulse_since)
            h10_delta = history.unseen_payload(
                status.get("h10_delta", {}), ("hr", "rr")
            )
            history_dirty = history.merge_payload(h10_delta) or history_dirty
            pending.merge_payload(h10_delta)
            emay_delta = history.unseen_payload(
                status.get("emay_delta", {}), ("spo2", "pulse")
            )
            history_dirty = history.merge_payload(emay_delta) or history_dirty
            pending.merge_payload(emay_delta)

            if time.monotonic() - last_history_publish >= HISTORY_PUBLISH_INTERVAL:
                # A periodic full resync catches rows whose event timestamps
                # arrived behind the cursor. It is five minutes, not the old
                # one-second full-history query/download loop.
                refreshed = query_history()
                history_dirty = history.merge_payload(refreshed.payload()) or history_dirty
                if history_dirty:
                    history_revision = next_revision(history_revision)
                    publish_history(history, history_revision)
                    pending.clear()
                    history_dirty = False
                last_history_publish = time.monotonic()

            status["h10_delta"] = pending.h10_payload()
            status["emay_delta"] = pending.emay_payload()
            status["history_revision"] = history_revision
            status.pop("error", None)
            last_good = status
            publish_json(OUTPUT, status)
        except Exception as error:
            print(
                f"panel-vitals-status: query failed, keeping last reading: {error}",
                file=sys.stderr,
            )
            stale = dict(last_good)
            stale["error"] = str(error)
            stale["history_revision"] = history_revision
            stale["h10_delta"] = pending.h10_payload()
            stale["emay_delta"] = pending.emay_payload()
            publish_json(OUTPUT, stale)
        time.sleep(INTERVAL)


if __name__ == "__main__":
    main()
