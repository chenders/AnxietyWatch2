#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
python3 - <<'PY'
import json
from pathlib import Path

fixture_dir = Path("api/fixtures/client-v1")
required = {
    "health-beacon-request.json",
    "health-beacon-background-delivery.json",
    "health-beacon-reboot.json",
    "health-beacon-no-wake.json",
    "health-beacon-locked.json",
    "health-beacon-denied-type.json",
    "health-beacon-unknown.json",
    "health-beacon-stale.json",
}
actual = {path.name for path in fixture_dir.glob("health-beacon*.json")}
missing = required - actual
if missing:
    raise SystemExit(f"phone beacon fixture verification failed: missing {sorted(missing)}")

required_keys = {
    "beacon_id", "asserted_at", "changed_at", "app_version", "registry_sha256",
    "outbox_count", "oldest_outbox_age_s", "last_server_ack_at", "types",
}
for path in sorted(fixture_dir.glob("health-beacon*.json")):
    body = json.loads(path.read_text())
    if set(body) != required_keys:
        raise SystemExit(f"phone beacon fixture verification failed: {path.name} has unexpected fields")
    if not isinstance(body["types"], list) or not body["types"]:
        raise SystemExit(f"phone beacon fixture verification failed: {path.name} has no type facts")
    for fact in body["types"]:
        if not {"healthkit_type", "configured", "authorization_request_outcome",
                "observer_registration_outcome", "background_delivery_outcome",
                "last_query_outcome", "last_query_at", "last_matching_sample_at",
                "last_pi_ack_at"} == set(fact):
            raise SystemExit(f"phone beacon fixture verification failed: {path.name} has malformed type facts")
        if any(key in fact for key in ("value", "samples", "aggregate", "trend")):
            raise SystemExit(f"phone beacon fixture verification failed: {path.name} contains measurement data")

print(f"phone beacon fixtures verified: {len(actual)} sanitized operational cases")
PY
