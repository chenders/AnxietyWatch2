#!/usr/bin/env python3
"""Fail-closed check for the XE125 placement/reference matrix report."""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

REQUIRED_CASES = {
    "overhead_supine",
    "overhead_side",
    "side_supine",
    "side_side",
    "bedding",
    "deliberate_motion",
    "no_presence",
}


def fail(message: str) -> int:
    print(f"XE125 matrix verify: {message}", file=sys.stderr)
    return 1


def main() -> int:
    if len(sys.argv) != 2:
        return fail("usage: verify_xe125_matrix.py REPORT.json")
    path = Path(sys.argv[1])
    if not path.is_file():
        return fail(f"missing report: {path}")
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return fail(f"cannot read JSON report: {exc}")
    if not isinstance(report, dict):
        return fail("report root must be an object")
    cases = report.get("cases")
    if not isinstance(cases, dict) or not REQUIRED_CASES <= set(cases):
        return fail(f"cases must include {sorted(REQUIRED_CASES)}")
    reference = report.get("reference_breaths_per_minute")
    if not isinstance(reference, (int, float)) or not math.isfinite(reference) or reference <= 0:
        return fail("reference_breaths_per_minute must be a positive finite number")
    for name in REQUIRED_CASES:
        case = cases[name]
        if not isinstance(case, dict) or case.get("status") not in {"measured", "unknown"}:
            return fail(f"{name} must have status measured or unknown")
        duration = case.get("duration_seconds")
        if not isinstance(duration, (int, float)) or not math.isfinite(duration):
            return fail(f"{name} must record a finite duration_seconds")
        if duration <= 0 or duration > 600:
            return fail(f"{name} duration {duration}s is outside the 0–600s hardware-check bound")
        if case["status"] == "unknown":
            if "rate" in case and case["rate"] is not None:
                return fail(f"{name} is unknown but contains a numeric rate")
        else:
            rate = case.get("rate")
            if not isinstance(rate, (int, float)) or not math.isfinite(rate) or rate <= 0:
                return fail(f"{name} measured case lacks a positive finite rate")
    for field in ("unknown_count", "motion_count"):
        value = report.get(field)
        if not isinstance(value, int) or value < 0:
            return fail(f"{field} must be a non-negative integer")
    print("XE125 matrix verify: complete cases, reference, and explicit unknown/motion counts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
