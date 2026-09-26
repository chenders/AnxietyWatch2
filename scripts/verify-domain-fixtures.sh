#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

python3 - <<'PY'
import json
from pathlib import Path

fixture_dir = Path("api/fixtures/domain-v1")
paths = sorted(fixture_dir.glob("*.json"))
if not paths:
    raise SystemExit("domain fixture verification failed: no fixtures found")

for path in paths:
    with path.open(encoding="utf-8") as handle:
        fixture = json.load(handle)
    if "record" not in fixture:
        if "relation_kind" in fixture:
            required = ("id", "source_record_id", "target_record_id", "recorded_at")
        else:
            required = ("record_id", "incoming_kind", "status")
        if not all(fixture.get(key) for key in required):
            raise SystemExit(f"domain fixture verification failed: {path} non-record shape")
        continue

    record = fixture["record"]
    if record.get("schema_version") != 1:
        raise SystemExit(f"domain fixture verification failed: {path} schema_version")
    if not record.get("id") or not record.get("kind"):
        raise SystemExit(f"domain fixture verification failed: {path} identity")
    if not record.get("recorded_at", "").endswith("Z"):
        raise SystemExit(f"domain fixture verification failed: {path} timestamp is not UTC")
    content = record.get("content")
    if not isinstance(content, dict):
        raise SystemExit(f"domain fixture verification failed: {path} content is not an object")
    if record.get("supersedes_id") and record.get("tombstone_target_id"):
        raise SystemExit(f"domain fixture verification failed: {path} version and tombstone combined")

journal = json.loads((fixture_dir / "journal-command.json").read_text(encoding="utf-8"))
content = journal["record"]["content"]
if not content["subject_at"].endswith("Z"):
    raise SystemExit("domain fixture verification failed: journal subject_at is not UTC")
if content["note"] is not None or content["song_id"] is not None:
    raise SystemExit("domain fixture verification failed: journal absence is not explicit")

version = json.loads((fixture_dir / "version-command.json").read_text(encoding="utf-8"))
if not version["record"]["supersedes_id"]:
    raise SystemExit("domain fixture verification failed: version has no supersedes_id")
tombstone = json.loads((fixture_dir / "tombstone-command.json").read_text(encoding="utf-8"))
if not tombstone["record"]["tombstone_target_id"]:
    raise SystemExit("domain fixture verification failed: tombstone has no target")

print(f"verified {len(paths)} domain-v1 fixtures")
PY
