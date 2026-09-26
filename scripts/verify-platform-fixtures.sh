#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

python3 - <<'PY'
import json
from pathlib import Path

registry = json.loads(Path("vitals/generated/registry.json").read_text())
healthkit = registry["source"]["healthkit"]
required_types = {"hr", "hrv_rmssd", "sleep_stage"}
provided = set(healthkit["provides"])
missing_types = required_types - provided
if missing_types:
    raise SystemExit(f"platform fixtures blocked: healthkit registry missing {sorted(missing_types)}")

fixture_dir = Path("clients/apple/Fixtures/GEN1")
quantity = json.loads((fixture_dir / "healthkit-quantity-sample.json").read_text())
sleep = json.loads((fixture_dir / "healthkit-sleep-category.json").read_text())
replay = json.loads((fixture_dir / "healthkit-replay.json").read_text())

required_common = {
    "healthkit_sample_uuid", "healthkit_type", "start_at", "end_at", "source_revision",
    "metadata", "query_batch_id", "recorded_at",
}
for name, fixture in (("quantity", quantity), ("sleep", sleep), ("replay", replay)):
    missing = required_common - fixture.keys()
    if missing:
        raise SystemExit(f"platform fixtures blocked: {name} missing {sorted(missing)}")
    if not fixture["source_revision"]:
        raise SystemExit(f"platform fixtures blocked: {name} has no source revision")

if quantity["healthkit_type"] != "HKQuantityTypeIdentifierHeartRate":
    raise SystemExit("platform fixtures blocked: quantity fixture type is not heart rate")
if "original_value" not in quantity or "original_unit" not in quantity:
    raise SystemExit("platform fixtures blocked: quantity fixture lacks original value/unit")
if sleep["healthkit_type"] != "HKCategoryTypeIdentifierSleepAnalysis":
    raise SystemExit("platform fixtures blocked: sleep fixture type is not sleep analysis")
if "original_category" not in sleep:
    raise SystemExit("platform fixtures blocked: sleep fixture lacks original category")

identity_fields = ("healthkit_sample_uuid", "healthkit_type", "start_at", "end_at", "query_batch_id")
if any(quantity[field] != replay[field] for field in identity_fields):
    raise SystemExit("platform fixtures blocked: replay identity does not match original")

print("platform-fixtures: PASS — registry mappings, stages, provenance, and replay identity")
PY
