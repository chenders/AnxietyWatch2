#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if [ "$#" -ne 1 ]; then
  echo "APNs verification failed: one evidence JSON path is required" >&2
  exit 1
fi
evidence_path=$1

swift test --package-path clients/apple/Packages/AWTransport

rg -q 'URLSessionAPNsTransport' clients/apple/AnxietyWatchApp/AnxietyWatchApp.swift
rg -q 'UNUserNotificationCenterDelegate' clients/apple/AnxietyWatchApp/AnxietyWatchApp.swift
rg -q 'aps-environment' clients/apple/AnxietyWatchApp/AnxietyWatchApp.entitlements
rg -q 'APNsRegistrationRequest' clients/apple/Packages/AWTransport/Sources/AWTransport/APNsTransport.swift
rg -q 'APNsOutcomeRequest' clients/apple/Packages/AWTransport/Sources/AWTransport/APNsTransport.swift
rg -q 'remote payload contains unsupported field' clients/apple/Packages/AWTransport/Sources/AWTransport/APNsTransport.swift
for fixture in apns-registration-request.json apns-registration-response.json apns-outcome-request.json apns-outcome-response.json; do
  test -f "api/fixtures/client-v1/$fixture" || {
    echo "APNs verification failed: missing contract fixture $fixture" >&2
    exit 1
  }
done

if [ ! -f "$evidence_path" ]; then
  echo "APNs verification blocked: $evidence_path is missing; physical-device evidence is required" >&2
  exit 1
fi

APNS_EVIDENCE_PATH="$evidence_path" python3 - <<'PY'
import json
import os
from pathlib import Path

path = Path(os.environ["APNS_EVIDENCE_PATH"])
data = json.loads(path.read_text())
required = {
    "schema_version", "captured_at", "device", "targeted_send", "all_active_fanout",
    "zero_recipient", "invalid_token", "revoked_token", "duplicate_message",
    "minimal_payload", "outcome_separation",
}
missing = sorted(required - data.keys())
if missing:
    raise SystemExit(f"APNs verification failed: evidence missing fields {', '.join(missing)}")
if data["schema_version"] != 1:
    raise SystemExit(f"APNs verification failed: unsupported evidence schema {data['schema_version']}")
for key in ("targeted_send", "all_active_fanout", "zero_recipient", "invalid_token", "revoked_token", "duplicate_message", "minimal_payload", "outcome_separation"):
    check = data[key]
    if check.get("status") != "passed":
        raise SystemExit(f"APNs verification blocked: {key}={check.get('status')!r}; physical/server evidence is incomplete")
if data["outcome_separation"].get("apns_accepted_equals_displayed"):
    raise SystemExit("APNs verification failed: provider acceptance was incorrectly treated as display")
print("APNs physical-device and delivery evidence passed")
PY
