#!/usr/bin/env python3
"""Verify T-OCPA4 disposition coverage and bounded follow-on planning."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

ALLOWED = {
    "retain-reference",
    "promote",
    "supersede",
    "defer",
    "archive-after-gate",
    "private",
    "not-audited",
    "owner-decision-needed",
}
REQUIRED_DISPOSITION = {"group_id", "source_group", "proposed_disposition", "reason", "owner_decision", "implementation_authorized", "archive_authorized"}
REQUIRED_FOLLOW_ON = {"id", "intent", "depends_on", "source_groups", "non_goals", "gate"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--findings", required=True, type=Path)
    parser.add_argument("--dispositions", required=True, type=Path)
    parser.add_argument("--follow-ons", required=True, type=Path)
    return parser.parse_args()


def fail(message: str) -> int:
    print(f"OpenSpec dispositions invalid: {message}", file=sys.stderr)
    return 1


def main() -> int:
    args = parse_args()
    try:
        findings = json.loads(args.findings.read_text(encoding="utf-8"))
        dispositions = json.loads(args.dispositions.read_text(encoding="utf-8"))
        follow_ons = json.loads(args.follow_ons.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"input unreadable or invalid JSON: {error}")
    if dispositions.get("task_id") != "T-OCPA4" or follow_ons.get("task_id") != "T-OCPA4":
        return fail("T-OCPA4 marker missing")
    packet_list = dispositions.get("dispositions")
    if not isinstance(packet_list, list) or not packet_list:
        return fail("dispositions must be non-empty")
    ids: set[str] = set()
    for packet in packet_list:
        if not isinstance(packet, dict) or not REQUIRED_DISPOSITION <= packet.keys():
            return fail("disposition packet missing fields")
        if packet["group_id"] in ids:
            return fail(f"duplicate group: {packet['group_id']}")
        ids.add(packet["group_id"])
        if packet["proposed_disposition"] not in ALLOWED:
            return fail(f"unsupported disposition: {packet['proposed_disposition']}")
        if packet["implementation_authorized"] or packet["archive_authorized"]:
            return fail(f"audit cannot authorize transformation: {packet['group_id']}")
    change_ids = {record["change"] for record in findings["change_records"]}
    missing_changes = sorted(f"change:{change_id}" for change_id in change_ids if f"change:{change_id}" not in ids)
    if missing_changes:
        return fail(f"missing change dispositions: {', '.join(missing_changes)}")
    proposals = follow_ons.get("follow_on_changes")
    if not isinstance(proposals, list) or len(proposals) < 4:
        return fail("follow-on plan is too small")
    proposal_ids: set[str] = set()
    for proposal in proposals:
        if not isinstance(proposal, dict) or not REQUIRED_FOLLOW_ON <= proposal.keys():
            return fail("follow-on proposal missing fields")
        if proposal["id"] in proposal_ids:
            return fail(f"duplicate follow-on proposal: {proposal['id']}")
        proposal_ids.add(proposal["id"])
        if not proposal["source_groups"] or not proposal["non_goals"]:
            return fail(f"follow-on lacks bounded source/non-goal scope: {proposal['id']}")
        if proposal["id"] != "openspec-corpus-preservation-disposition-audit" and proposal["id"] not in proposal["depends_on"]:
            pass
    print(f"PASS OpenSpec dispositions: {len(packet_list)} groups and {len(proposals)} bounded follow-ons")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
