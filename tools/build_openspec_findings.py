#!/usr/bin/env python3
"""Build the lifecycle, validation, and discrepancy ledger for T-OCPA3."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
from typing import Any

CHECKBOX = re.compile(r"^- \[([ xX])\]", re.MULTILINE)
VERIFY = re.compile(r"\*\*Verify:\*\*")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inventory", required=True, type=Path)
    parser.add_argument("--findings", required=True, type=Path)
    return parser.parse_args()


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def change_ids(inventory: dict[str, Any]) -> list[str]:
    ids: set[str] = set()
    for record in inventory["records"]:
        path = record["source_path"]
        if path.startswith("openspec/changes/"):
            remainder = path.removeprefix("openspec/changes/")
            change = remainder.split("/", 1)[0]
            if change and change != "archive":
                ids.add(change)
    return sorted(ids)


def task_record(root: Path, change: str) -> dict[str, Any]:
    path = root / "openspec" / "changes" / change / "tasks.md"
    if not path.exists():
        return {
            "change": change,
            "tasks_path": f"openspec/changes/{change}/tasks.md",
            "tasks_present": False,
            "checked_tasks": None,
            "open_tasks": None,
            "verify_contracts": None,
            "lifecycle_claim": "not-measured",
        }
    text = path.read_text(encoding="utf-8")
    boxes = CHECKBOX.findall(text)
    checked = sum(mark.lower() == "x" for mark in boxes)
    return {
        "change": change,
        "tasks_path": f"openspec/changes/{change}/tasks.md",
        "tasks_present": True,
        "checked_tasks": checked,
        "open_tasks": len(boxes) - checked,
        "verify_contracts": len(VERIFY.findall(text)),
        "lifecycle_claim": "task-checkbox-claim-only",
    }


def main() -> int:
    args = parse_args()
    inventory = load(args.inventory)
    root = Path(inventory["source_manifest"]).resolve().parent.parent.parent
    validation_path = root / "docs/knowledge/openspec-validation.json"
    validation = load(validation_path)
    exit_code = int((root / "docs/knowledge/openspec-validation.exit").read_text(encoding="utf-8").strip())
    version = (root / "docs/knowledge/openspec-version.txt").read_text(encoding="utf-8").strip()
    list_specs = (root / "docs/knowledge/openspec-list-specs.txt").read_text(encoding="utf-8").strip()
    doctor = load(root / "docs/knowledge/openspec-doctor.json")

    changes = [task_record(root, change) for change in change_ids(inventory)]
    failed = [item["id"] for item in validation.get("items", []) if not item.get("valid", False)]
    passed = [item["id"] for item in validation.get("items", []) if item.get("valid", False)]
    findings = {
        "findings_id": "openspec-findings",
        "change_id": "openspec-corpus-preservation-disposition-audit",
        "task_id": "T-OCPA3",
        "observed_at_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "evidence_state_rule": "not-measured is distinct from measured-good and measured-bad; lifecycle and evidence are separate",
        "tooling": {
            "openspec_version": version,
            "doctor": doctor,
            "canonical_specs_output": list_specs,
            "canonical_specs_state": "not-measured/none-found by CLI output",
        },
        "structural_validation": {
            "command": "openspec validate --all --strict --no-interactive --json --report full",
            "exit_code": exit_code,
            "raw_output": "docs/knowledge/openspec-validation.json",
            "passed_changes": passed,
            "failed_changes": failed,
            "interpretation": "structural OpenSpec evidence only; does not prove implementation, owner acceptance, Verify success, runtime, hardware, or physical display behavior",
        },
        "change_records": changes,
        "owner_disposition": {
            "per_change_disposition_records": "not-found",
            "governance_policy": "owner-accepted policy; implementation and enforcement remain separate",
            "authoring_guidance": "owner-approved guidance; not proof of implemented enforcement",
        },
        "discrepancies": [
            {
                "id": "validation-no-delta-emay",
                "state": "measured-bad-structural-only",
                "source_paths": ["openspec/changes/emay-durable-ingestion"],
                "claim": "latest strict validator reports no recognized spec delta",
                "disposition": "owner decision needed; do not automatically add specs or set skip_specs",
            },
            {
                "id": "validation-no-delta-pi5",
                "state": "measured-bad-structural-only",
                "source_paths": ["openspec/changes/pi5-vitals-core"],
                "claim": "latest strict validator reports no recognized spec delta",
                "disposition": "owner decision needed; do not automatically add specs or set skip_specs",
            },
            {
                "id": "validation-no-delta-repository-governance",
                "state": "measured-bad-structural-only",
                "source_paths": ["openspec/changes/repository-recovery-and-lifecycle"],
                "claim": "latest strict validator reports no recognized spec delta",
                "disposition": "owner decision needed; preserve owner policy records and decide whether this is spec-free",
            },
            {
                "id": "validation-no-delta-safety",
                "state": "measured-bad-structural-only",
                "source_paths": ["openspec/changes/safety-hardening"],
                "claim": "latest strict validator reports no recognized spec delta",
                "disposition": "owner decision needed; this change contains behavioral safety work and cannot be classified by validator output alone",
            },
            {
                "id": "no-canonical-specs",
                "state": "measured-good-cli-observation",
                "source_paths": ["openspec/specs"],
                "claim": "openspec list --specs reports no specs found",
                "disposition": "do not create canonical specs until owner disposition and authority review",
            },
            {
                "id": "panel-direction-documentation-conflict",
                "state": "not-measured-unresolved",
                "source_paths": [
                    "docs/design/CLAUDE-DESIGN-SOURCES.md",
                    "docs/design/pi-panel/direction-a-clinical/README.md",
                    "docs/design/PROMPT-pi-panel.md"
                ],
                "claim": "one source says all three panel directions are ported; the Direction A README says B and C are not built",
                "disposition": "preserve both claims and create a separate reconciliation decision or change",
            },
            {
                "id": "authoring-guidance-version-drift",
                "state": "measured-bad-version-drift-only",
                "source_paths": [
                    "openspec/changes/repository-recovery-and-lifecycle/reviews/openspec-authoring-guidance.md",
                    "docs/knowledge/openspec-version.txt"
                ],
                "claim": "approved guidance records OpenSpec 1.6.0 while the installed CLI is 1.13.2",
                "disposition": "re-probe version-specific claims; do not rewrite approved guidance silently",
            },
            {
                "id": "repository-governance-tasks-missing",
                "state": "measured-bad-structural-only",
                "source_paths": ["openspec/changes/repository-recovery-and-lifecycle/tasks.md"],
                "claim": "the repository-governance change has no tasks.md in the current corpus",
                "disposition": "record as missing planning artifact; do not infer task completion",
            }
        ],
        "current_operational_evidence": {
            "state": "not-measured",
            "claims": [
                "no current physical bedside-display acceptance is established by this audit",
                "no current hardware acceptance is established by this audit",
                "no task Verify command was rerun by this findings builder"
            ]
        }
    }
    args.findings.parent.mkdir(parents=True, exist_ok=True)
    args.findings.write_text(json.dumps(findings, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote findings for {len(changes)} changes and {len(findings['discrepancies'])} discrepancies")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
