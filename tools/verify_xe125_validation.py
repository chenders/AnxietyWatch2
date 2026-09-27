#!/usr/bin/env python3
"""Fail-closed check for the XE125 validation and promotion decision."""
from __future__ import annotations

import json
import sys
from pathlib import Path


def fail(message: str) -> int:
    print(f"XE125 validation verify: {message}", file=sys.stderr)
    return 1


def main() -> int:
    if len(sys.argv) != 2:
        return fail("usage: verify_xe125_validation.py REPORT.json")
    path = Path(sys.argv[1])
    if not path.is_file():
        return fail(f"missing report: {path}")
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return fail(f"cannot read JSON report: {exc}")
    if not isinstance(report, dict):
        return fail("report root must be an object")
    if report.get("owner_accepted") is not True:
        return fail("owner_accepted must be true")
    if report.get("matrix_complete") is not True:
        return fail("matrix_complete must be true")
    evidence = report.get("evidence")
    if (
        not isinstance(evidence, list)
        or not evidence
        or not all(isinstance(x, str) and x for x in evidence)
    ):
        return fail("evidence must be a non-empty list of paths or identifiers")
    decision = report.get("decision")
    if decision not in {"accepted", "experimental-only"}:
        return fail("decision must be accepted or experimental-only")
    if report.get("alarm_promotion", False) is not False:
        return fail("alarm_promotion must remain false in this change")
    print(
        f"XE125 validation verify: {decision}, {len(evidence)} evidence items, "
        "alarm path isolated"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
