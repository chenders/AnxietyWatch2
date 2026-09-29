#!/usr/bin/env python3
"""Verify the bounded, public-safe preservation baseline for T-OCPA1."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

REQUIRED_TOP_LEVEL = {
    "change_id",
    "task_id",
    "observed_at_utc",
    "repository_root",
    "branch",
    "working_tree_note",
    "source_mutation_policy",
    "scope_roots",
    "excluded_or_not_audited",
    "created_paths",
}
REQUIRED_SCOPE_FIELDS = {"path", "role", "handling"}
ALLOWED_HANDLING = {"public-metadata-only", "source-reference", "authority-reference"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True, type=Path)
    return parser.parse_args()


def fail(message: str) -> int:
    print(f"preservation baseline invalid: {message}", file=sys.stderr)
    return 1


def main() -> int:
    args = parse_args()
    try:
        payload = json.loads(args.manifest.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"manifest unreadable or invalid JSON: {error}")

    if not isinstance(payload, dict):
        return fail("manifest root must be an object")
    missing = REQUIRED_TOP_LEVEL - payload.keys()
    if missing:
        return fail(f"missing fields: {', '.join(sorted(missing))}")
    if payload["change_id"] != "openspec-corpus-preservation-disposition-audit":
        return fail(f"change_id={payload['change_id']}")
    if payload["task_id"] != "T-OCPA1":
        return fail(f"task_id={payload['task_id']}")
    if not isinstance(payload["observed_at_utc"], str) or not payload["observed_at_utc"].endswith("Z"):
        return fail("observed_at_utc must be a UTC timestamp ending in Z")
    if payload["source_mutation_policy"] != "read-only; do not rewrite, move, delete, or rename source artifacts":
        return fail("source_mutation_policy does not require read-only source handling")
    scope_roots = payload["scope_roots"]
    if not isinstance(scope_roots, list) or not scope_roots:
        return fail("scope_roots must be a non-empty list")
    repository_root = Path(payload["repository_root"])
    for entry in scope_roots:
        if not isinstance(entry, dict) or not REQUIRED_SCOPE_FIELDS <= entry.keys():
            return fail("every scope root needs path, role, and handling")
        if entry["handling"] not in ALLOWED_HANDLING:
            return fail(f"unsupported handling for {entry.get('path')}: {entry.get('handling')}")
        path = repository_root / entry["path"]
        if not path.exists() and not path.is_symlink():
            return fail(f"scope root missing: {entry['path']}")
    excluded = payload["excluded_or_not_audited"]
    if not isinstance(excluded, list) or not excluded:
        return fail("excluded_or_not_audited must explicitly name boundaries")
    created_paths = payload["created_paths"]
    if not isinstance(created_paths, list) or not created_paths:
        return fail("created_paths must list the audit outputs")
    source_paths = {entry["path"] for entry in scope_roots}
    overlap = sorted(source_paths.intersection(created_paths))
    if overlap:
        return fail(f"audit output overlaps source root: {', '.join(overlap)}")
    if "secrets" in json.dumps(payload).lower() or "private key" in json.dumps(payload).lower():
        return fail("manifest must not disclose secret material")
    print(
        f"PASS preservation baseline: {len(scope_roots)} roots, "
        f"{len(excluded)} explicit boundaries, {len(created_paths)} outputs; source policy is read-only"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
