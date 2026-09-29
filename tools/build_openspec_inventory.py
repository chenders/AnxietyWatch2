#!/usr/bin/env python3
"""Build a metadata-only inventory for the bounded OpenSpec corpus scope."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--inventory", required=True, type=Path)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def relative_paths(root: Path) -> list[Path]:
    if root.is_dir() and not root.is_symlink():
        return [root, *sorted(root.rglob("*"), key=lambda path: path.as_posix())]
    return [root]


def classify(relative: str) -> tuple[str, str, str, str]:
    if relative in {"AGENTS.md", "CLAUDE.md"}:
        return "normative-decision", "project authority and worker rules", "binding", "current"
    if relative == "README.md":
        return "project-reference", "project orientation", "reference", "current"
    if relative == "PI-KICKOFF.md":
        return "project-reference", "project kickoff context", "reference", "historical-or-current"
    if relative == "docs/specs/AMENDMENTS.md":
        return "normative-decision", "accepted amendment index", "binding", "current"
    if relative.startswith("docs/specs/"):
        lifecycle = "draft" if "/DRAFT-" in relative or relative.startswith("docs/specs/DRAFT-") else "review-required"
        return "normative-decision", "specification or amendment record", "normative-reference", lifecycle
    if relative.startswith("openspec/changes/archive/"):
        return "openspec-history", "archived OpenSpec change", "historical", "archived"
    if relative.startswith("openspec/changes/"):
        change = relative.split("/", 3)[2] if len(relative.split("/", 3)) > 2 else None
        return "openspec-planning", f"OpenSpec change contract: {change or 'unknown'}", "proposed-contract", "change-lifecycle-unmeasured"
    if relative == "openspec/config.yaml" or relative.startswith("openspec/"):
        return "openspec-governance", "OpenSpec configuration or governance record", "process-reference", "current-or-unmeasured"
    directory_categories = {
        "architecture": ("architecture-design", "architecture record"),
        "design": ("architecture-design", "design and interaction record"),
        "research": ("research-evidence", "research or exploratory evidence"),
        "hardware": ("hardware-integration", "hardware record or evidence"),
        "integration": ("hardware-integration", "integration record or evidence"),
        "ops": ("operations", "operational procedure or record"),
        "reviews": ("review-decision", "review or decision evidence"),
        "setup-logs": ("status-history", "setup or execution history"),
        "status-updates": ("status-history", "historical progress record"),
    }
    parts = relative.split("/")
    if len(parts) >= 2 and parts[0] == "docs" and parts[1] in directory_categories:
        category, purpose = directory_categories[parts[1]]
        return category, purpose, "evidence-reference", "historical-or-unmeasured"
    return "unclassified", "in-scope material requiring review", "unknown", "not-measured"


def stable_id(relative: str) -> str:
    digest = hashlib.sha256(relative.encode("utf-8")).hexdigest()
    return f"doc-{digest[:16]}"


def entry(repository_root: Path, path: Path) -> dict[str, Any]:
    relative = path.relative_to(repository_root).as_posix()
    category, purpose, authority, lifecycle = classify(relative)
    is_directory = path.is_dir() and not path.is_symlink()
    size = None if is_directory else path.stat().st_size
    return {
        "stable_id": stable_id(relative),
        "source_path": relative,
        "kind": "directory" if is_directory else "file",
        "category": category,
        "purpose": purpose,
        "authority": authority,
        "provenance": "repository path; metadata-only inventory; content claim not measured",
        "lifecycle_claim": lifecycle,
        "evidence_state": "not-measured",
        "owner": None,
        "reviewer": None,
        "related_openspec_change": relative.split("/", 3)[2] if relative.startswith("openspec/changes/") and len(relative.split("/", 3)) > 2 else None,
        "sensitivity_class": "public-path-metadata-only",
        "size_bytes": size,
    }


def main() -> int:
    args = parse_args()
    manifest_path = args.manifest.resolve()
    inventory_path = args.inventory.resolve()
    manifest = read_json(manifest_path)
    repository_root = Path(manifest["repository_root"]).resolve()
    roots = [repository_root / item["path"] for item in manifest["scope_roots"]]
    records: list[dict[str, Any]] = []
    for root in roots:
        records.extend(entry(repository_root, path) for path in relative_paths(root))
    records.sort(key=lambda item: item["source_path"])
    payload = {
        "inventory_id": "openspec-corpus-inventory",
        "change_id": manifest["change_id"],
        "task_id": "T-OCPA2",
        "observed_at_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "source_manifest": manifest_path.relative_to(repository_root).as_posix(),
        "inventory_mode": "metadata-only; no source content copied",
        "record_count": len(records),
        "records": records,
    }
    inventory_path.parent.mkdir(parents=True, exist_ok=True)
    inventory_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {len(records)} metadata records to {inventory_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
