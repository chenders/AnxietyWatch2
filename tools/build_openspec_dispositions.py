#!/usr/bin/env python3
"""Build owner-decision packets and bounded follow-on plan for T-OCPA4."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--findings", required=True, type=Path)
    parser.add_argument("--dispositions", required=True, type=Path)
    parser.add_argument("--follow-ons", required=True, type=Path)
    return parser.parse_args()


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    args = parse_args()
    findings = load(args.findings)
    observed = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    groups = [
        ("change:99-last-specs", "99-last-specs", "owner-decision-needed", "Empty holding change; retain its migration provenance until a destination is selected."),
        ("change:emay-durable-ingestion", "emay-durable-ingestion", "owner-decision-needed", "Behavioral durable-ingestion plan with a structural validation finding; do not classify as spec-free without review."),
        ("change:openspec-corpus-preservation-disposition-audit", "openspec-corpus-preservation-disposition-audit", "retain-reference", "Active audit evidence; close only after owner dispositions are recorded."),
        ("change:pi5-vitals-core", "pi5-vitals-core", "owner-decision-needed", "Behavioral platform plan with a structural validation finding and historical task claims."),
        ("change:project-knowledge-architecture", "project-knowledge-architecture", "retain-reference", "Valid project-level routing proposal; do not merge it with the preservation audit silently."),
        ("change:repository-recovery-and-lifecycle", "repository-recovery-and-lifecycle", "owner-decision-needed", "Accepted policy records coexist with an unimplemented governance proposal and no tasks artifact."),
        ("change:safety-hardening", "safety-hardening", "owner-decision-needed", "Behavioral safety work with a structural validation finding; preserve safety intent and evidence."),
        ("change:xe125-respiratory-rate", "xe125-respiratory-rate", "retain-reference", "Structurally valid active change; retain its open tasks and experimental promotion gate."),
        ("slice:root-guidance", "AGENTS.md, CLAUDE.md, README.md, PI-KICKOFF.md", "retain-reference", "Authority and orientation records; do not duplicate their rules into a weaker copy."),
        ("slice:normative-specs", "docs/specs", "retain-reference", "Decision and amendment history; authoritative conflicts require explicit disposition."),
        ("slice:architecture-design", "docs/architecture and docs/design", "owner-decision-needed", "Retain design evidence; reconcile the panel-direction conflict before promoting behavior."),
        ("slice:research", "docs/research", "owner-decision-needed", "Research remains reference material unless an owner adopts a conclusion."),
        ("slice:hardware-integration", "docs/hardware and docs/integration", "owner-decision-needed", "Promote only measured acceptance gates and interfaces into bounded changes."),
        ("slice:operations", "docs/ops", "owner-decision-needed", "Keep runbooks as operational records; link implementation tasks where required."),
        ("slice:reviews", "docs/reviews", "retain-reference", "Preserve review provenance, comments, dissent, and dispositions."),
        ("slice:setup-logs", "docs/setup-logs", "retain-reference", "Historical execution evidence; do not treat it as current acceptance."),
        ("slice:status-updates", "docs/status-updates", "retain-reference", "Historical progress evidence; current state requires fresh verification."),
    ]
    disposition_payload = {
        "disposition_id": "openspec-dispositions",
        "change_id": findings["change_id"],
        "task_id": "T-OCPA4",
        "observed_at_utc": observed,
        "source_findings": args.findings.as_posix(),
        "authority": ["AGENTS.md", "docs/specs/AMENDMENTS.md", "openspec/changes/repository-recovery-and-lifecycle/reviews/governance/owner-decision-2026-09-06.md"],
        "owner_decision_state": "required-before-transformation",
        "dispositions": [
            {
                "group_id": group_id,
                "source_group": source_group,
                "proposed_disposition": disposition,
                "reason": reason,
                "owner_decision": "needed",
                "implementation_authorized": False,
                "archive_authorized": False,
            }
            for group_id, source_group, disposition, reason in groups
        ],
    }
    follow_ons = {
        "plan_id": "openspec-follow-on-plan",
        "change_id": findings["change_id"],
        "task_id": "T-OCPA4",
        "observed_at_utc": observed,
        "owner_decision_state": "proposed-only",
        "follow_on_changes": [
            {
                "id": "openspec-authoring-contract-foundation",
                "intent": "Define project traceability, lifecycle, evidence, Reads/Amends, and new-task Verify conventions.",
                "depends_on": ["openspec-corpus-preservation-disposition-audit"],
                "source_groups": ["change:repository-recovery-and-lifecycle", "slice:normative-specs", "slice:reviews"],
                "non_goals": ["changing existing Verify lines", "activating enforcement", "private-store or deployment work"],
                "gate": "owner approval plus required review before implementation",
            },
            {
                "id": "bedside-display-responsive-evidence",
                "intent": "Own responsive layout and the complete feed-to-physical-screen acceptance chain.",
                "depends_on": ["openspec-corpus-preservation-disposition-audit", "project-knowledge-architecture"],
                "source_groups": ["slice:architecture-design", "slice:hardware-integration", "slice:operations"],
                "non_goals": ["making the panel an alarm dependency", "inferring physical acceptance from process health", "rewriting display source documents in the audit"],
                "gate": "display/hardware review and independent physical evidence",
            },
            {
                "id": "durability-read-model-evidence",
                "intent": "Specify received, spooled, durable, drained, committed, published, live, stale, and unknown states.",
                "depends_on": ["openspec-corpus-preservation-disposition-audit"],
                "source_groups": ["change:emay-durable-ingestion", "change:safety-hardening", "slice:operations"],
                "non_goals": ["claiming historical evidence is current", "changing storage identity outside accepted amendments"],
                "gate": "domain/safety review and fresh executable evidence",
            },
            {
                "id": "legacy-change-migration",
                "intent": "Map retained legacy task plans to requirements, scenarios, tasks, and evidence through companion changes.",
                "depends_on": ["openspec-corpus-preservation-disposition-audit", "openspec-authoring-contract-foundation"],
                "source_groups": ["change:pi5-vitals-core", "change:emay-durable-ingestion", "change:safety-hardening", "change:xe125-respiratory-rate"],
                "non_goals": ["rewriting historical artifacts", "silently repairing Verify contracts", "inferring owner acceptance"],
                "gate": "per-change owner disposition and specialist review",
            },
            {
                "id": "canonical-capability-publication",
                "intent": "Publish approved current capabilities under openspec/specs only after authority and evidence reconciliation.",
                "depends_on": ["openspec-corpus-preservation-disposition-audit", "openspec-authoring-contract-foundation"],
                "source_groups": ["slice:normative-specs", "change:project-knowledge-architecture"],
                "non_goals": ["creating canonical specs merely to satisfy validation", "archiving active work", "changing amendments silently"],
                "gate": "explicit owner approval and reviewed publication change",
            },
            {
                "id": "archive-and-closure-reconciliation",
                "intent": "Archive only changes whose current evidence, Verify results, review, owner decision, and regressions are reconciled.",
                "depends_on": ["legacy-change-migration", "canonical-capability-publication"],
                "source_groups": ["slice:reviews", "slice:status-updates", "slice:setup-logs"],
                "non_goals": ["using archive to hide validation failures", "deleting historical evidence", "treating archive as acceptance"],
                "gate": "all required task Verify commands exit 0 and owner closure is recorded",
            },
        ],
    }
    args.dispositions.parent.mkdir(parents=True, exist_ok=True)
    args.follow_ons.parent.mkdir(parents=True, exist_ok=True)
    args.dispositions.write_text(json.dumps(disposition_payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    args.follow_ons.write_text(json.dumps(follow_ons, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {len(groups)} disposition packets and {len(follow_ons['follow_on_changes'])} follow-on proposals")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
