#!/usr/bin/env python3
"""Publish the read-only AirSense bedside API as a same-origin panel feed.

The AirSense collector already exposes the latest stream values on its local
console API. This adapter does not control therapy or write samples; it only
copies validated display fields into the panel document root so the kiosk can
read them without a cross-origin request.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import pathlib
import tempfile
import time
from urllib.request import Request, urlopen

# The console deliberately binds to the Pi's private-LAN address, not loopback.
# Keep the overrideable environment variable for deployments with another
# address, while making the shipped bedside service work without extra setup.
DEFAULT_API = "http://pi-host:8480/api/air11"
DEFAULT_STATUS = pathlib.Path("/run/vitals-air11/status.json")
DEFAULT_OUTPUT = pathlib.Path(
    "$HOME/anxietywatch-panel/docs/design/pi-panel/direction-a-clinical/cpap_status.json"
)


def finite(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value))


def fetch_status(url: str) -> dict[str, object]:
    request = Request(url, headers={"Accept": "application/json", "Cache-Control": "no-cache"})
    with urlopen(request, timeout=5) as response:
        value = json.load(response)
    if not isinstance(value, dict):
        raise ValueError("AirSense API did not return an object")
    latest = value.get("latest")
    if not isinstance(latest, list):
        raise ValueError("AirSense API latest series is missing")
    return value


def latest_values(document: dict[str, object]) -> dict[str, object]:
    values: dict[str, dict[str, float]] = {}
    for raw in document.get("latest", []):
        if not isinstance(raw, dict):
            continue
        name = raw.get("name")
        value = raw.get("value")
        timestamp = raw.get("t_micros")
        if not isinstance(name, str) or not finite(value) or not finite(timestamp):
            continue
        values[name] = {"value": float(value), "sample_at": float(timestamp) / 1_000_000.0}

    def field(name: str) -> tuple[float | None, float | None]:
        item = values.get(name)
        if item is None:
            return None, None
        return float(item["value"]), float(item["sample_at"])

    respiratory_rate, rr_at = field("respiratory_rate")
    leak, leak_at = field("leak")
    pressure, pressure_at = field("mask_pressure")
    sample_times = [timestamp for timestamp in (rr_at, leak_at, pressure_at) if timestamp is not None]
    status = document.get("status")
    if not isinstance(status, dict):
        raise ValueError("AirSense API status is missing")
    therapy_state = status.get("therapy_state")
    written_at = time.time()
    output: dict[str, object] = {
        "schema": 1,
        "written_at": written_at,
        "sample_at": max(sample_times, default=None),
        "on": therapy_state == "active",
        "therapy_on": therapy_state == "active",
        "session_active": therapy_state == "active",
        "status": therapy_state if isinstance(therapy_state, str) else "unknown",
        "respiratory_rate": respiratory_rate,
        "leak_lpm": leak,
        "pressure_cmh2o": pressure,
        "source": "resmed_air11",
        "blocker": status.get("blocker"),
    }
    return output


def local_status_values(document: dict[str, object]) -> dict[str, object]:
    therapy_state = document.get("therapy_state")
    link_state = document.get("link_state")
    receipt = document.get("last_host_receipt_unix_micros")
    sample_at = float(receipt) / 1_000_000.0 if finite(receipt) else None
    active = therapy_state == "active" and link_state == "streaming"
    status = therapy_state if isinstance(therapy_state, str) else "unknown"
    return {
        "schema": 1,
        "written_at": time.time(),
        "sample_at": sample_at,
        "on": active,
        "therapy_on": active,
        "session_active": active,
        "status": status,
        "respiratory_rate": None,
        "leak_lpm": None,
        "pressure_cmh2o": None,
        "source": "resmed_air11",
        "blocker": document.get("blocker"),
    }


def publish(path: pathlib.Path, document: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary_path = pathlib.Path(temporary)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(document, stream, separators=(",", ":"), allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        temporary_path.chmod(0o644)
        temporary_path.replace(path)
    finally:
        temporary_path.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--api", default=os.environ.get("CPAP_API_URL"))
    parser.add_argument("--status", type=pathlib.Path, default=pathlib.Path(os.environ.get("CPAP_STATUS_PATH", DEFAULT_STATUS)))
    parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path(os.environ.get("CPAP_STATUS_JSON", DEFAULT_OUTPUT)))
    parser.add_argument("--interval", type=float, default=float(os.environ.get("CPAP_STATUS_INTERVAL_SECONDS", "2")))
    args = parser.parse_args()
    if not math.isfinite(args.interval) or args.interval <= 0:
        parser.error("--interval must be finite and positive")
    while True:
        try:
            if args.api:
                document = latest_values(fetch_status(args.api))
            else:
                raw = json.loads(args.status.read_text(encoding="utf-8"))
                if not isinstance(raw, dict):
                    raise ValueError("AirSense local status did not return an object")
                document = local_status_values(raw)
        except (OSError, ValueError, TypeError, json.JSONDecodeError) as error:
            document = {"schema": 1, "written_at": time.time(), "error": str(error), "status": "unknown"}
        publish(args.output, document)
        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
