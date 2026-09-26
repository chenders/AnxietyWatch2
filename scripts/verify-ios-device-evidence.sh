#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if [[ $# -ne 1 ]]; then
  echo "device evidence: FAIL — expected one evidence JSON path" >&2
  exit 1
fi

python3 - "$1" <<'PY'
import json
import re
import sys
from datetime import datetime
from pathlib import Path

path = Path(sys.argv[1])
if not path.is_file():
    raise SystemExit(f"device evidence: FAIL — missing evidence file: {path}")
try:
    document = json.loads(path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as error:
    raise SystemExit(f"device evidence: FAIL — evidence JSON is invalid: {error}") from error

if document.get("schema_version") != 1:
    raise SystemExit("device evidence: FAIL — schema_version must be 1")


def require_utc(value, field):
    if not isinstance(value, str) or not value.endswith("Z"):
        raise SystemExit(f"device evidence: FAIL — {field} must be an RFC3339 UTC instant")
    try:
        datetime.fromisoformat(value[:-1] + "+00:00")
    except ValueError as error:
        raise SystemExit(f"device evidence: FAIL — {field} is not RFC3339: {error}") from error

require_utc(document.get("captured_at"), "captured_at")

phone = document.get("iphone")
if not isinstance(phone, dict):
    raise SystemExit("device evidence: FAIL — iphone object is required")
for field in ("device_label", "os_version", "app_build"):
    if not isinstance(phone.get(field), str) or not phone[field].strip():
        raise SystemExit(f"device evidence: FAIL — iphone.{field} is required")

checks = document.get("checks")
if not isinstance(checks, dict):
    raise SystemExit("device evidence: FAIL — checks object is required")

required_checks = (
    "healthkit_observer_delivery",
    "background_task_drain",
    "force_quit_behavior",
    "locked_after_first_unlock",
    "secure_enclave_background_signing",
    "wrong_host_rejection",
    "expired_certificate_rejection",
    "untrusted_certificate_rejection",
    "replay_rejection",
    "revocation_rejection",
    "clock_failure_rejection",
    "key_rotation_overlap",
    "queued_record_preservation",
    "express_capture_timing",
    "journal_capture_timing",
    "overnight_battery_impact",
    "watch_connectivity_recovery",
)
for name in required_checks:
    check = checks.get(name)
    if not isinstance(check, dict):
        raise SystemExit(f"device evidence: FAIL — missing check: {name}")
    if check.get("status") != "pass":
        status = check.get("status", "missing")
        blocker = check.get("blocker", "no blocker supplied")
        raise SystemExit(f"device evidence: FAIL — {name}={status}; blocker={blocker}")
    require_utc(check.get("observed_at"), f"checks.{name}.observed_at")
    if not isinstance(check.get("evidence"), str) or not check["evidence"].strip():
        raise SystemExit(f"device evidence: FAIL — checks.{name}.evidence is required")

limits = document.get("limits")
if not isinstance(limits, dict):
    raise SystemExit("device evidence: FAIL — limits object is required")
for field in ("healthkit_batch_max", "retry_max", "overnight_battery_budget_percent"):
    value = limits.get(field)
    if not isinstance(value, (int, float)) or isinstance(value, bool) or value <= 0:
        raise SystemExit(f"device evidence: FAIL — limits.{field} must be positive")
if limits["healthkit_batch_max"] > 100:
    raise SystemExit(
        f"device evidence: FAIL — limits.healthkit_batch_max={limits['healthkit_batch_max']} exceeds 100"
    )

# Evidence is committed to the repository, so it must contain observations and timing only—not
# HealthKit values, private filesystem paths, endpoint details, or device identifiers.
raw = json.dumps(document, sort_keys=True).lower()
for forbidden in (
    "healthkit_value",
    "heart_rate",
    "blood_pressure",
    "https://",
    "http://",
    "/users/",
    "private/var/mobile",
    "00008110-",
):
    if forbidden in raw:
        raise SystemExit(f"device evidence: FAIL — forbidden private or measurement material: {forbidden}")
if re.search(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b", raw):
    raise SystemExit("device evidence: FAIL — device or deployment UUID must not be committed")

print("device evidence: PASS — complete iPhone/Watch acceptance evidence is sanitized and bounded")
PY
