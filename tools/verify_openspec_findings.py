#!/usr/bin/env python3
"""Verify the T-OCPA3 lifecycle, validation, and discrepancy ledger."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any

REQUIRED_DISCREPANCY = {"id", "state", "source_paths", "claim", "disposition"}
REQUIRED_CHANGE = {
    "change",
    "tasks_path",
    "tasks_present",
    "checked_tasks",
    "open_tasks",
    "verify_contracts",
    "lifecycle_claim",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", required=True, type=Path)
    parser.add_argument("--findings", required=True, type=Path)
    return parser.parse_args()


def fail(message: str) -> int:
    print(f"OpenSpec findings invalid: {message}", file=sys.stderr)
    return 1


def main() -> int:
    args = parse_args()
    try:
        inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
        findings = json.loads(args.findings.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"input unreadable or invalid JSON: {error}")
    root = Path(inventory["source_manifest"]).resolve().parent.parent.parent
    if findings.get("task_id") != "T-OCPA3":
        return fail(f"task_id={findings.get('task_id')}")
    structural = findings.get("structural_validation")
    if not isinstance(structural, dict):
        return fail("structural_validation missing")
    if structural.get("command") != "openspec validate --all --strict --no-interactive --json --report full":
        return fail("structural validation command is not the measured command")
    if structural.get("exit_code") != 1:
        return fail(f"expected measured validation exit 1, got {structural.get('exit_code')}")
    raw_output = root / structural["raw_output"]
    if not raw_output.exists():
        return fail(f"raw validation output missing: {raw_output}")
    changes = findings.get("change_records")
    if not isinstance(changes, list) or not changes:
        return fail("change_records must be non-empty")
    for change in changes:
        if not isinstance(change, dict) or not REQUIRED_CHANGE <= change.keys():
            return fail("change record missing required fields")
        if change["tasks_present"] and change["verify_contracts"] is None:
            return fail(f"tasks record missing verify count: {change['change']}")
        if not change["tasks_present"] and change["lifecycle_claim"] != "not-measured":
            return fail(f"missing tasks must remain not-measured: {change['change']}")
    discrepancies = findings.get("discrepancies")
    if not isinstance(discrepancies, list) or len(discrepancies) < 5:
        return fail("discrepancy register is incomplete")
    ids: set[str] = set()
    for discrepancy in discrepancies:
        if not isinstance(discrepancy, dict) or not REQUIRED_DISCREPANCY <= discrepancy.keys():
            return fail("discrepancy missing required fields")
        if discrepancy["id"] in ids:
            return fail(f"duplicate discrepancy id: {discrepancy['id']}")
        ids.add(discrepancy["id"])
        if not discrepancy["source_paths"]:
            return fail(f"discrepancy has no source paths: {discrepancy['id']}")
    if findings.get("current_operational_evidence", {}).get("state") != "not-measured":
        return fail("current operational evidence must remain not-measured")
    print(f"PASS OpenSpec findings: {len(changes)} changes, {len(discrepancies)} discrepancies, validation exit 1 recorded")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
