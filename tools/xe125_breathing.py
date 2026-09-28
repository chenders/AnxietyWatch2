#!/usr/bin/env python3
"""Bench-only Acconeer XE125/XM125 breathing capture.

This is intentionally an operator tool, not a production collector. It uses
Acconeer's supported ``acconeer-exptool`` A121 reference application and emits
state-preserving JSON Lines. It never writes the vitals spool and it never uses
host arrival time as a durable sample identity.

Install the optional vendor dependency in the environment used for the bench
run, for example::

    uv run --with 'acconeer-exptool[app]' tools/xe125_breathing.py \\
        --serial-port /dev/ttyUSB0 --raw-record xe125-session.h5

The XE125 exposes two virtual serial ports. Use the Enhanced port, not the
Standard port. Firmware and Exploration Tool versions must be compatible.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
import time
from numbers import Integral, Real
from pathlib import Path
from typing import Any, TextIO

SCHEMA_VERSION = 1
DEFAULT_START_M = 0.3
DEFAULT_END_M = 1.5
DEFAULT_FRAME_RATE = 20.0
DEFAULT_NUM_DISTANCES = 3
DEFAULT_DISTANCE_DETERMINATION_SECONDS = 5.0
DEFAULT_LOWEST_BREATHING_RATE = 6.0
DEFAULT_HIGHEST_BREATHING_RATE = 60.0
DEFAULT_TIME_SERIES_SECONDS = 20.0

ESTIMATE_STATE = "ESTIMATE_BREATHING_RATE"
KNOWN_STATES = frozenset(
    {
        "INIT_STATE",
        "NO_PRESENCE_DETECTED",
        "INTRA_PRESENCE_DETECTED",
        "DETERMINE_DISTANCE_ESTIMATE",
        ESTIMATE_STATE,
    }
)


def finite_number(value: Any) -> float | None:
    """Return a finite numeric value, preserving invalid data as absent."""
    if isinstance(value, bool) or not isinstance(value, Real):
        return None
    number = float(value)
    return number if math.isfinite(number) else None


def state_name(value: Any) -> str:
    """Get an enum name without allowing an invalid state to look measured."""
    name = getattr(value, "name", None)
    if isinstance(name, str) and name in KNOWN_STATES:
        return name
    if isinstance(value, str) and value in KNOWN_STATES:
        return value
    return "UNKNOWN_STATE"


def range_point_indices(value: Any) -> list[int] | None:
    """Convert the reference app's selected point tuple to JSON safely."""
    if not isinstance(value, (tuple, list)) or len(value) != 2:
        return None
    if any(isinstance(item, bool) or not isinstance(item, Integral) for item in value):
        return None
    return [int(value[0]), int(value[1])]


def observation_dict(result: Any, observed_at_unix_micros: int) -> dict[str, Any]:
    """Build one state-preserving JSON observation from a reference result.

    This function deliberately accepts a duck-typed result so its serialization
    and fail-closed behaviour can be tested without importing the vendor SDK.
    """
    app_state = state_name(getattr(result, "app_state", None))
    presence = getattr(result, "presence_result", None)
    breathing = getattr(result, "breathing_result", None)

    presence_detected = bool(getattr(presence, "presence_detected", False))
    distance = finite_number(getattr(presence, "presence_distance", None))
    if not presence_detected:
        distance = None

    rate = None
    if app_state == ESTIMATE_STATE and breathing is not None:
        rate = finite_number(getattr(breathing, "breathing_rate", None))

    return {
        "schema": SCHEMA_VERSION,
        "observed_at_unix_micros": int(observed_at_unix_micros),
        "app_state": app_state,
        "rate_available": rate is not None,
        "breathing_rate_breaths_per_min": rate,
        "presence_detected": presence_detected,
        "presence_distance_m": distance,
        "intra_presence_score": finite_number(
            getattr(presence, "intra_presence_score", None)
        ),
        "inter_presence_score": finite_number(
            getattr(presence, "inter_presence_score", None)
        ),
        "range_point_indices": range_point_indices(
            getattr(result, "distances_being_analyzed", None)
        ),
    }


def write_observation(result: Any, output: TextIO) -> None:
    """Write and flush one JSONL observation."""
    observation = observation_dict(result, time.time_ns() // 1_000)
    output.write(json.dumps(observation, allow_nan=False, separators=(",", ":")) + "\n")
    output.flush()


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--serial-port",
        required=True,
        help="XE125 Enhanced virtual serial port (for example /dev/ttyUSB0)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("-"),
        help="JSONL destination; '-' (the default) means stdout",
    )
    parser.add_argument(
        "--raw-record",
        type=Path,
        help="optional H5 destination for the raw A121 session",
    )
    parser.add_argument(
        "--duration-seconds",
        type=float,
        default=600.0,
        help="stop after this many seconds; must be greater than 0 and no more than 600",
    )
    parser.add_argument("--sensor-id", type=int, default=1)
    parser.add_argument("--start-m", type=float, default=DEFAULT_START_M)
    parser.add_argument("--end-m", type=float, default=DEFAULT_END_M)
    parser.add_argument("--frame-rate", type=float, default=DEFAULT_FRAME_RATE)
    parser.add_argument("--num-distances", type=int, default=DEFAULT_NUM_DISTANCES)
    parser.add_argument(
        "--distance-determination-seconds",
        type=float,
        default=DEFAULT_DISTANCE_DETERMINATION_SECONDS,
    )
    parser.add_argument(
        "--lowest-breathing-rate",
        type=float,
        default=DEFAULT_LOWEST_BREATHING_RATE,
    )
    parser.add_argument(
        "--highest-breathing-rate",
        type=float,
        default=DEFAULT_HIGHEST_BREATHING_RATE,
    )
    parser.add_argument(
        "--time-series-seconds",
        type=float,
        default=DEFAULT_TIME_SERIES_SECONDS,
    )
    return parser


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse and validate operator configuration."""
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.duration_seconds <= 0 or args.duration_seconds > 600:
        parser.error("--duration-seconds must be greater than 0 and no more than 600")
    if args.start_m >= args.end_m:
        parser.error("--end-m must be greater than --start-m")
    if args.frame_rate <= 0:
        parser.error("--frame-rate must be positive")
    if args.num_distances <= 0:
        parser.error("--num-distances must be positive")
    if args.distance_determination_seconds <= 0:
        parser.error("--distance-determination-seconds must be positive")
    if args.lowest_breathing_rate <= 0:
        parser.error("--lowest-breathing-rate must be positive")
    if args.highest_breathing_rate <= args.lowest_breathing_rate:
        parser.error("--highest-breathing-rate must exceed --lowest-breathing-rate")
    if args.time_series_seconds <= 0:
        parser.error("--time-series-seconds must be positive")
    return args


def open_output(path: Path) -> tuple[TextIO, bool]:
    """Open a JSONL destination and state whether the caller owns it."""
    if str(path) == "-":
        return sys.stdout, False
    path.parent.mkdir(parents=True, exist_ok=True)
    return path.open("w", encoding="utf-8", buffering=1), True


def close_quietly(resource: Any, label: str) -> None:
    """Close an SDK resource while keeping the original failure visible."""
    if resource is None:
        return
    close = getattr(resource, "close", None)
    if close is None:
        return
    try:
        close()
    except Exception as exc:  # Optional cleanup must not mask capture failure.
        print(f"xe125-breathing: {label} cleanup failed: {exc}", file=sys.stderr)


def run_capture(args: argparse.Namespace) -> int:
    """Run the vendor reference application until duration or interruption."""
    try:
        from acconeer.exptool import a121
        from acconeer.exptool.a121.algo.breathing import RefApp
        from acconeer.exptool.a121.algo.breathing._ref_app import (
            BreathingProcessorConfig,
            RefAppConfig,
            get_sensor_config,
        )
        from acconeer.exptool.a121.algo.presence import (
            ProcessorConfig as PresenceProcessorConfig,
        )
    except ModuleNotFoundError as exc:
        print(
            "xe125-breathing: missing Acconeer dependency; run with "
            "uv run --with 'acconeer-exptool[app]' ...",
            file=sys.stderr,
        )
        print(f"xe125-breathing: import failed: {exc}", file=sys.stderr)
        return 70

    breathing_config = BreathingProcessorConfig(
        lowest_breathing_rate=args.lowest_breathing_rate,
        highest_breathing_rate=args.highest_breathing_rate,
        time_series_length_s=args.time_series_seconds,
    )
    presence_config = PresenceProcessorConfig(intra_detection_threshold=4.0)
    ref_app_config = RefAppConfig(
        use_presence_processor=True,
        num_distances_to_analyze=args.num_distances,
        distance_determination_duration=args.distance_determination_seconds,
        start_m=args.start_m,
        end_m=args.end_m,
        frame_rate=args.frame_rate,
        breathing_config=breathing_config,
        presence_config=presence_config,
    )
    sensor_config = get_sensor_config(ref_app_config=ref_app_config)

    client = None
    ref_app = None
    recorder = None
    started = False
    output: TextIO | None = None
    owns_output = False
    try:
        output, owns_output = open_output(args.output)
        client = a121.Client.open(serial_port=args.serial_port)
        # Keep the explicit setup call: it lets a setup failure be reported
        # before the reference controller begins its session.
        client.setup_session(sensor_config)
        ref_app = RefApp(
            client=client,
            sensor_id=args.sensor_id,
            ref_app_config=ref_app_config,
        )
        if args.raw_record is not None:
            args.raw_record.parent.mkdir(parents=True, exist_ok=True)
            recorder = a121.H5Recorder(str(args.raw_record), client)
        ref_app.start(recorder=recorder)
        started = True
        print(
            "xe125-breathing: capturing "
            f"serial={args.serial_port} range={args.start_m:.2f}-{args.end_m:.2f}m "
            f"raw_record={args.raw_record if args.raw_record else 'off'}",
            file=sys.stderr,
        )

        deadline = None
        if args.duration_seconds > 0:
            deadline = time.monotonic() + args.duration_seconds
        while deadline is None or time.monotonic() < deadline:
            write_observation(ref_app.get_next(), output)
        return 0
    except KeyboardInterrupt:
        print("xe125-breathing: interrupted", file=sys.stderr)
        return 0
    except Exception as exc:  # vendor errors are not stable across SDK versions
        print(f"xe125-breathing: capture failed: {exc}", file=sys.stderr)
        return 70
    finally:
        if started and ref_app is not None:
            close_quietly(ref_app, "reference application")
        elif recorder is not None:
            close_quietly(recorder, "raw recorder")
        close_quietly(client, "client")
        if owns_output and output is not None:
            close_quietly(output, "JSONL output")


def main(argv: list[str] | None = None) -> int:
    """CLI entry point."""
    return run_capture(parse_args(argv))


if __name__ == "__main__":
    raise SystemExit(main())
