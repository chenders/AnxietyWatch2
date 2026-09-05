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
from itertools import pairwise
from typing import Any, ClassVar

OUTPUT = pathlib.Path(
    os.environ.get(
        "PANEL_VITALS_STATUS_JSON",
        "/opt/anxietywatch-panel/docs/design/pi-panel/direction-a-clinical/"
        "vitals_status.json",
    )
)
HISTORY_OUTPUT = pathlib.Path(
    os.environ.get(
        "PANEL_H10_HISTORY_JSON", str(OUTPUT.with_name("vitals_h10_history.json"))
    )
)
LIVE_EMAY = pathlib.Path(
    os.environ.get("EMAY_STATUS_JSON", str(OUTPUT.with_name("emay_status.json")))
)
PIPELINE_HEALTH = pathlib.Path(
    os.environ.get("VITALS_HEALTH_JSON", str(OUTPUT.with_name("health.json")))
)
DATABASE = os.environ.get("PANEL_VITALS_DATABASE", "vitals")
INTERVAL = float(os.environ.get("PANEL_VITALS_INTERVAL_SECONDS", "1"))
HISTORY_PUBLISH_INTERVAL = float(
    os.environ.get("PANEL_H10_HISTORY_INTERVAL_SECONDS", "300")
)
HISTORY_WINDOW_SECONDS = float(os.environ.get("PANEL_HISTORY_WINDOW_SECONDS", "86400"))
LIVE_H10 = pathlib.Path(
    os.environ.get("H10_LIVE_STATUS_JSON", "/run/vitals-h10/status.json")
)
LIVE_H10_HISTORY = pathlib.Path(
    os.environ.get("H10_LIVE_HISTORY_JSONL", "/run/vitals-h10/history.jsonl")
)
LIVE_H10_MAX_AGE = float(os.environ.get("H10_LIVE_STATUS_MAX_AGE_SECONDS", "10"))
LIVE_H10_HISTORY_MAX_AGE = float(
    os.environ.get("H10_LIVE_HISTORY_MAX_AGE_SECONDS", "86400")
)
LIVE_EMAY_MAX_AGE = float(os.environ.get("EMAY_LIVE_STATUS_MAX_AGE_SECONDS", "15"))
PIPELINE_HEALTH_MAX_AGE = float(os.environ.get("VITALS_HEALTH_MAX_AGE_SECONDS", "120"))

STATUS_SQL = r"""
WITH bp_latest AS (
 SELECT session_id,seq,timestamp FROM samples WHERE source=6 AND type=6 AND stored=true
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
  ON s.source=6 AND s.session_id=b.session_id AND s.seq=b.seq AND s.stored=true
 GROUP BY b.session_id,b.seq,b.timestamp
), h10 AS (
 SELECT json_build_object('sample_at',extract(epoch FROM timestamp),
  'source',source,'type',type,'session_id',session_id,'seq',seq,'hr',value,'test_origin',
  CASE WHEN source=7 THEN 'polar_h10_emulator'
       WHEN extra IS NOT NULL AND convert_from(extra,'UTF8')
         LIKE '%test_origin=polar_h10_emulator%' THEN 'polar_h10_emulator'
       WHEN extra IS NOT NULL AND convert_from(extra,'UTF8')
         LIKE '%test_origin=physical_device%' THEN 'physical_device'
       ELSE 'unknown' END) value
 FROM samples WHERE source IN (1,7) AND type=1 AND stored=true
 ORDER BY timestamp DESC LIMIT 1
), emay_latest AS (
 SELECT session_id,seq,max(timestamp) AS timestamp
 FROM samples WHERE source=0 AND type IN (1,2) AND stored=true
 GROUP BY session_id,seq
 ORDER BY max(timestamp) DESC,session_id DESC,seq DESC LIMIT 1
), emay AS (
 SELECT json_build_object('sample_at',extract(epoch FROM e.timestamp),
  'source',0,'type',2,'session_id',e.session_id,'seq',e.seq,
  'spo2',max(s.value) FILTER(WHERE s.type=2),
  'pulse_bpm',max(s.value) FILTER(WHERE s.type=1),
  'stored',true,'synthetic',false,'link','stored') value
 FROM emay_latest e JOIN samples s
  ON s.source=0 AND s.session_id=e.session_id AND s.seq=e.seq AND s.stored=true
 GROUP BY e.session_id,e.seq,e.timestamp
), h10_hr_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=1 AND type=1 AND stored=true
  AND timestamp >= to_timestamp(:'hr_since'::double precision)
), h10_rr_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=1 AND type=5 AND stored=true
  AND timestamp >= to_timestamp(:'rr_since'::double precision)
), emay_spo2_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=0 AND type=2 AND stored=true
  AND timestamp >= to_timestamp(:'spo2_since'::double precision)
), emay_pulse_delta AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=0 AND type=1 AND stored=true
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
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=1 AND type=1 AND stored=true
  AND timestamp >= to_timestamp(:'history_since'::double precision)
), h10_rr_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=1 AND type=5 AND stored=true
  AND timestamp >= to_timestamp(:'history_since'::double precision)
), emay_spo2_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=0 AND type=2 AND stored=true
  AND timestamp >= to_timestamp(:'history_since'::double precision)
), emay_pulse_history AS (
 SELECT coalesce(json_agg(json_build_object(
  't',extract(epoch FROM timestamp),'v',value,'source',source,'type',type,
  'session_id',session_id,'seq',seq)
  ORDER BY timestamp,session_id,seq), '[]'::json) value
 FROM samples WHERE source=0 AND type=1 AND stored=true
  AND timestamp >= to_timestamp(:'history_since'::double precision)
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

Point = dict[str, Any]


@dataclass
class HistoryState:
    """Append-only durable points keyed by source identity, never by value/time."""

    hr: list[Point] = field(default_factory=list)
    rr: list[Point] = field(default_factory=list)
    spo2: list[Point] = field(default_factory=list)
    pulse: list[Point] = field(default_factory=list)
    # The metric name carries the fixed (source, type) pair selected by SQL;
    # session_id and seq are the source-provided durable identity.
    _hr_keys: set[tuple[int, int, int, int]] = field(default_factory=set)
    _rr_keys: set[tuple[int, int, int, int]] = field(default_factory=set)
    _spo2_keys: set[tuple[int, int, int, int]] = field(default_factory=set)
    _pulse_keys: set[tuple[int, int, int, int]] = field(default_factory=set)

    _IDENTITY_BY_KIND: ClassVar[dict[str, tuple[int, int]]] = {
        "hr": (1, 1),
        "rr": (1, 5),
        "spo2": (0, 2),
        "pulse": (0, 1),
    }

    @staticmethod
    def _point(raw: Any) -> tuple[Point, tuple[int, int, int, int]] | None:
        if not isinstance(raw, dict):
            return None
        try:
            timestamp = float(raw["t"])
            value = float(raw["v"])
            source = raw["source"]
            value_type = raw["type"]
            session_id = raw["session_id"]
            seq = raw["seq"]
        except (KeyError, TypeError, ValueError):
            return None
        # A point without the source's identity is not durable history. In
        # particular, do not fall back to (timestamp, value): equal readings
        # at different source sequences are distinct samples.
        if (
            isinstance(source, bool)
            or not isinstance(source, int)
            or isinstance(value_type, bool)
            or not isinstance(value_type, int)
            or isinstance(session_id, bool)
            or not isinstance(session_id, int)
            or isinstance(seq, bool)
            or not isinstance(seq, int)
            or not math.isfinite(timestamp)
            or not math.isfinite(value)
        ):
            return None
        return (
            {
                "t": timestamp,
                "v": value,
                "source": source,
                "type": value_type,
                "session_id": session_id,
                "seq": seq,
            },
            (source, value_type, session_id, seq),
        )

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
            parsed = self._point(raw)
            if parsed is None:
                continue
            point, identity = parsed
            if self._IDENTITY_BY_KIND.get(kind) != (identity[0], identity[1]):
                continue
            key = identity
            if key in keys:
                continue
            if point["t"] < last_time:
                needs_sort = True
            target.append(point)
            keys.add(key)
            last_time = max(last_time, point["t"])
            changed = True
        if needs_sort:
            target.sort(
                key=lambda point: (point["t"], point["session_id"], point["seq"])
            )
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
        """Return valid source-identified points not already present."""
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
                parsed = self._point(raw)
                if parsed is None:
                    continue
                point, identity = parsed
                if self._IDENTITY_BY_KIND.get(kind) != (identity[0], identity[1]):
                    continue
                key = identity
                if key not in keys:
                    unseen.merge(kind, [point])
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

    def prune(self, cutoff: float) -> bool:
        """Keep the configured time window bounded while retaining source identity."""
        changed = False
        for kind in ("hr", "rr", "spo2", "pulse"):
            target = getattr(self, kind)
            keys = getattr(self, f"_{kind}_keys")
            kept = [point for point in target if point["t"] >= cutoff]
            changed = len(kept) != len(target) or changed
            target[:] = kept
            keys.clear()
            keys.update(
                (point["source"], point["type"], point["session_id"], point["seq"])
                for point in target
            )
        return changed

    def gaps(self) -> list[dict[str, Any]]:
        """Report missing source sequence numbers without filling their values."""
        groups: dict[tuple[str, int, int, int], set[int]] = {}
        for kind in ("hr", "rr", "spo2", "pulse"):
            for point in getattr(self, kind):
                group = (
                    kind,
                    point["source"],
                    point["type"],
                    point["session_id"],
                )
                groups.setdefault(group, set()).add(point["seq"])
        gaps: list[dict[str, Any]] = []
        for (kind, source, value_type, session_id), sequences in groups.items():
            ordered = sorted(sequences)
            for previous, current in pairwise(ordered):
                if current > previous + 1:
                    gaps.append(
                        {
                            "source": kind,
                            "kind": "sequence_gap",
                            "session_id": session_id,
                            "from_seq": previous,
                            "to_seq": current,
                            "missing": current - previous - 1,
                            "reason": "source sequence gap; values were not interpolated",
                            "source_id": source,
                            "type_id": value_type,
                        }
                    )
        return gaps

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
        detail = (
            process.stderr.strip() or process.stdout.strip() or "no psql diagnostic"
        )
        raise RuntimeError(
            f"psql query failed: returncode={process.returncode}, detail={detail}"
        )
    return json.loads(process.stdout.strip())


def _fresh(timestamp: Any, maximum_age: float) -> bool:
    try:
        age = time.time() - float(timestamp)
    except (TypeError, ValueError):
        return False
    return math.isfinite(age) and 0.0 <= age <= maximum_age


def read_live_status() -> dict[str, Any] | None:
    """Read the H10 bedside overlay; it is not durable history."""
    try:
        with LIVE_H10.open(encoding="utf-8") as stream:
            live = json.load(stream)
        if not isinstance(live, dict) or not _fresh(
            live["written_at"], LIVE_H10_MAX_AGE
        ):
            return None
        sample_at = float(live["sample_at"])
        session_id = live["session_id"]
        seq = live["seq"]
        hr = float(live["hr"])
        if (
            not math.isfinite(sample_at)
            or sample_at > time.time()
            or isinstance(session_id, bool)
            or not isinstance(session_id, (int, str))
            or (isinstance(session_id, str) and not session_id)
            or isinstance(seq, bool)
            or not isinstance(seq, int)
            or not math.isfinite(hr)
        ):
            return None
        return {
            "written_at": float(live["written_at"]),
            "sample_at": sample_at,
            "session_id": session_id,
            "seq": seq,
            "hr": hr,
            "rr_count": live["rr_count"],
            "rr_points": live.get("rr_points", []),
            "test_origin": live["test_origin"],
            "live": True,
            "identity": "source_session_seq_pending_commit",
            "durability": "pending_or_status_only_not_committed",
        }
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return None


def read_live_emay_status() -> dict[str, Any] | None:
    """Read EMAY's anonymous live status without manufacturing an identity."""
    try:
        with LIVE_EMAY.open(encoding="utf-8") as stream:
            live = json.load(stream)
        if not isinstance(live, dict) or not _fresh(
            live["written_at"], LIVE_EMAY_MAX_AGE
        ):
            return None
        reading = live.get("reading")
        if not isinstance(reading, dict):
            reading = live

        def finite_or_none(name: str) -> float | None:
            value = reading.get(name)
            if value is None:
                return None
            number = float(value)
            return number if math.isfinite(number) else None

        sample_at = reading.get("sample_at")
        if sample_at is not None:
            sample_at = float(sample_at)
            if not math.isfinite(sample_at) or sample_at > time.time():
                return None
        spo2 = finite_or_none("spo2")
        if "pulse_bpm" not in reading and "pulse" in reading:
            reading = {**reading, "pulse_bpm": reading.get("pulse")}
        pulse = finite_or_none("pulse_bpm")
        if spo2 is None and pulse is None and sample_at is None:
            return None
        frames = reading.get("frames")
        if isinstance(frames, bool) or (
            frames is not None and not isinstance(frames, int)
        ):
            frames = None
        finger = reading.get("finger", reading.get("finger_present"))
        return {
            "written_at": float(live["written_at"]),
            "sample_at": sample_at,
            "spo2": spo2,
            "pulse_bpm": pulse,
            "finger": finger if isinstance(finger, bool) else None,
            "link": (
                reading.get("link")
                if isinstance(reading.get("link"), str)
                else "unknown"
            ),
            "frames": frames,
            "stored": False,
            "synthetic": False,
            "live": True,
            "identity": "anonymous_not_durable",
            "durability": "status_only_not_committed",
        }
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return None


def read_pipeline_health() -> dict[str, Any]:
    """Return validated stage evidence, or an explicit fail-closed UNKNOWN."""
    stages = (
        "received",
        "spooled",
        "fsynced",
        "drained",
        "committed",
        "published",
        "displayed",
    )

    def unknown(reason: str) -> dict[str, Any]:
        return {
            "schema": 1,
            "state": "unknown",
            "reason": reason,
            "stages": {stage: None for stage in stages},
        }

    try:
        with PIPELINE_HEALTH.open(encoding="utf-8") as stream:
            health = json.load(stream)
        if not isinstance(health, dict):
            return unknown("health document is not an object")
        timestamp = health.get("written_at", health.get("checked_at"))
        if not _fresh(timestamp, PIPELINE_HEALTH_MAX_AGE):
            return unknown("pipeline health evidence is missing or stale")
        source_stages = health.get("stages")
        if not isinstance(source_stages, dict):
            source_stages = health.get("pipeline")
        if not isinstance(source_stages, dict):
            return unknown("pipeline stages are missing")
        if any(stage not in source_stages for stage in stages):
            return unknown("pipeline stage evidence is incomplete")

        def json_safe(value: Any) -> bool:
            if isinstance(value, float):
                return math.isfinite(value)
            if isinstance(value, dict):
                return all(
                    json_safe(key) and json_safe(item) for key, item in value.items()
                )
            if isinstance(value, list):
                return all(json_safe(item) for item in value)
            return value is None or isinstance(value, (bool, int, str))

        if not all(json_safe(source_stages[stage]) for stage in stages):
            return unknown("pipeline stage evidence contains a non-finite value")
        state = health.get("state")
        if not isinstance(state, str) or state not in {
            "healthy",
            "degraded",
            "unknown",
        }:
            state = "degraded"
        return {
            "schema": 1,
            "state": state,
            "reason": health.get("reason"),
            "checked_at": float(timestamp),
            "stages": {stage: source_stages[stage] for stage in stages},
        }
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return unknown("pipeline health document is unavailable or malformed")


def read_live_history(cutoff: float) -> dict[str, list[Point]]:
    """Read legacy live records for diagnostics, never merge them into history."""
    hr: list[Point] = []
    rr: list[Point] = []
    minimum_time = max(cutoff, time.time() - LIVE_H10_HISTORY_MAX_AGE)
    try:
        with LIVE_H10_HISTORY.open(encoding="utf-8") as stream:
            for line in stream:
                try:
                    point = json.loads(line)
                    sample_at = float(point["sample_at"])
                    if (
                        sample_at < minimum_time
                        or point["test_origin"] != "physical_device"
                    ):
                        continue
                    # A legacy record without source identity cannot be called
                    # durable and is intentionally not promoted to history.
                    if not isinstance(point.get("session_id"), int) or not isinstance(
                        point.get("seq"), int
                    ):
                        continue
                    hr.append(
                        {
                            "t": sample_at,
                            "v": point["hr"],
                            "session_id": point["session_id"],
                            "seq": point["seq"],
                        }
                    )
                    rr.extend(point.get("rr_points", []))
                except (KeyError, TypeError, ValueError, json.JSONDecodeError):
                    continue
    except OSError:
        pass
    return {"hr": hr, "rr": rr}


def query_history() -> HistoryState:
    data = run_query(
        HISTORY_SQL,
        {"history_since": max(0.0, time.time() - HISTORY_WINDOW_SECONDS)},
    )
    state = HistoryState()
    state.merge_payload(data.get("h10_history"))
    state.merge_payload(data.get("emay_history"))
    return state


def _annotate_durable(value: Any) -> Any:
    if not isinstance(value, dict):
        return value
    annotated = dict(value)
    annotated["identity"] = "source_session_seq"
    annotated["durability"] = "database_committed"
    annotated["live"] = False
    return annotated


def _gaps(data: dict[str, Any]) -> list[dict[str, str]]:
    gaps: list[dict[str, str]] = []
    for source, live_key, durable_key in (
        ("h10", "h10_live", "h10_durable"),
        ("emay", "emay_live", "emay_durable"),
    ):
        if data.get(live_key) is None:
            gaps.append(
                {
                    "source": source,
                    "kind": "live_evidence",
                    "reason": "live feed missing or stale; current value is UNKNOWN",
                }
            )
        if data.get(durable_key) is None:
            gaps.append(
                {
                    "source": source,
                    "kind": "durable_history",
                    "reason": "no committed source-identified sample is available",
                }
            )
    pipeline = data.get("pipeline")
    if not isinstance(pipeline, dict) or pipeline.get("state") == "unknown":
        gaps.append(
            {
                "source": "pipeline",
                "kind": "stage_evidence",
                "reason": "pipeline stage evidence is missing or stale",
            }
        )
    return gaps


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
    data["h10_durable"] = _annotate_durable(data.get("h10"))
    data["emay_durable"] = _annotate_durable(data.get("emay"))
    data["h10"] = data["h10_durable"]
    data["emay"] = data["emay_durable"]

    h10_delta = HistoryState()
    h10_delta.merge_payload(data.get("h10_delta"))
    live_h10 = read_live_status()
    data["h10_live"] = live_h10
    if live_h10 is not None:
        # The bedside value may be displayed as the current overlay, but it
        # never enters h10_delta or the durable history document here.
        data["h10"] = live_h10
    data["h10_delta"] = h10_delta.h10_payload()

    emay_delta = HistoryState()
    emay_delta.merge_payload(data.get("emay_delta"))
    live_emay = read_live_emay_status()
    data["emay_live"] = live_emay
    if live_emay is not None:
        data["emay"] = live_emay
    data["emay_delta"] = emay_delta.emay_payload()

    data["pending_live"] = {"h10": live_h10, "emay": live_emay}
    data["pipeline"] = read_pipeline_health()
    data["gaps"] = _gaps(data)
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
            "history_window_seconds": HISTORY_WINDOW_SECONDS,
            "durability": "database_committed",
            "gaps": history.gaps(),
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
        "history_window_seconds": HISTORY_WINDOW_SECONDS,
        "bp": None,
        "h10": None,
        "h10_durable": None,
        "h10_live": None,
        "emay": None,
        "emay_durable": None,
        "emay_live": None,
        "pending_live": {"h10": None, "emay": None},
        "pending_durable": {"hr": [], "rr": [], "spo2": [], "pulse": []},
        "pipeline": {
            "schema": 1,
            "state": "unknown",
            "reason": "pipeline health evidence has not been read",
            "stages": {
                stage: None
                for stage in (
                    "received",
                    "spooled",
                    "fsynced",
                    "drained",
                    "committed",
                    "published",
                    "displayed",
                )
            },
        },
        "gaps": [
            {
                "source": "pipeline",
                "kind": "stage_evidence",
                "reason": "pipeline stage evidence is missing or stale",
            }
        ],
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

            cutoff = time.time() - HISTORY_WINDOW_SECONDS
            history_dirty = history.prune(cutoff) or history_dirty
            pending.prune(cutoff)
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
                history_dirty = (
                    history.merge_payload(refreshed.payload()) or history_dirty
                )
                if history_dirty:
                    history_revision = next_revision(history_revision)
                    publish_history(history, history_revision)
                    pending.clear()
                    history_dirty = False
                last_history_publish = time.monotonic()

            status["h10_delta"] = pending.h10_payload()
            status["emay_delta"] = pending.emay_payload()
            status["pending_durable"] = {
                **pending.h10_payload(),
                **pending.emay_payload(),
            }
            status["pending_live"] = status.get(
                "pending_live", {"h10": None, "emay": None}
            )
            status["gaps"] = list(status.get("gaps", [])) + history.gaps()
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
            stale["pending_durable"] = {
                **pending.h10_payload(),
                **pending.emay_payload(),
            }
            publish_json(OUTPUT, stale)
        time.sleep(INTERVAL)


if __name__ == "__main__":
    main()
