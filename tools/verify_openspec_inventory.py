#!/usr/bin/env python3
"""Verify completeness and safety properties of the metadata-only corpus inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
from typing import Any

REQUIRED = {
    "stable_id",
    "source_path",
    "kind",
    "category",
    "purpose",
    "authority",
    "provenance",
    "lifecycle_claim",
    "evidence_state",
    "owner",
    "reviewer",
    "related_openspec_change",
    "sensitivity_class",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--inventory", required=True, type=Path)
    return parser.parse_args()


def fail(message: str) -> int:
    print(f"OpenSpec inventory invalid: {message}", file=sys.stderr)
    return 1


def stable_id(relative: str) -> str:
    return f"doc-{hashlib.sha256(relative.encode('utf-8')).hexdigest()[:16]}"


def paths_under(root: Path) -> list[Path]:
    if root.is_dir() and not root.is_symlink():
        return [root, *sorted(root.rglob("*"), key=lambda path: path.as_posix())]
    return [root]


def main() -> int:
    args = parse_args()
    try:
        manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
        inventory = json.loads(args.inventory.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return fail(f"input unreadable or invalid JSON: {error}")
    root = Path(manifest["repository_root"])
    scope_roots = [root / item["path"] for item in manifest["scope_roots"]]
    records = inventory.get("records")
    if not isinstance(records, list) or not records:
        return fail("records must be a non-empty list")
    if inventory.get("task_id") != "T-OCPA2":
        return fail(f"task_id={inventory.get('task_id')}")
    by_path: dict[str, dict[str, Any]] = {}
    by_id: set[str] = set()
    for record in records:
        if not isinstance(record, dict) or not REQUIRED <= record.keys():
            return fail("every record must contain the complete metadata contract")
        path = record["source_path"]
        if not isinstance(path, str) or path.startswith("/") or ".." in Path(path).parts:
            return fail(f"unsafe source_path={path}")
        if path in by_path:
            return fail(f"duplicate source_path={path}")
        if record["stable_id"] != stable_id(path):
            return fail(f"stable_id mismatch for {path}")
        if record["stable_id"] in by_id:
            return fail(f"duplicate stable_id={record['stable_id']}")
        if record["evidence_state"] not in {"not-measured", "historical-only", "measured-good", "measured-bad"}:
            return fail(f"unsupported evidence_state for {path}")
        if record["sensitivity_class"] != "public-path-metadata-only":
            return fail(f"unsafe sensitivity_class for {path}")
        by_path[path] = record
        by_id.add(record["stable_id"])
    expected: set[str] = set()
    for scope_root in scope_roots:
        if not scope_root.exists() and not scope_root.is_symlink():
            return fail(f"scope root missing: {scope_root}")
        expected.update(path.relative_to(root).as_posix() for path in paths_under(scope_root))
    missing = sorted(expected - by_path.keys())
    extra = sorted(by_path.keys() - expected)
    if missing:
        return fail(f"missing records, first paths: {', '.join(missing[:5])}")
    if extra:
        return fail(f"records outside declared scope, first paths: {', '.join(extra[:5])}")
    serialized = json.dumps(inventory).lower()
    for forbidden in ("secret", "credential", "private key", "password"):
        if forbidden in serialized:
            return fail(f"inventory contains restricted term: {forbidden}")
    print(f"PASS OpenSpec inventory: {len(records)} complete metadata-only records")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
