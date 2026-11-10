#!/usr/bin/env bash
# Verify the live APNs evidence ledger without treating provider acceptance as delivery.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE="${1:-$ROOT/artifacts/ios-stage7-apns-evidence.json}"
PI_SSH="${PI_SSH:-${PI_HOST:?remote-apns blocked: neither PI_SSH nor PI_HOST is set (e.g. PI_HOST=user@host)}}"

[[ -f "$EVIDENCE" ]] || { echo "remote-apns: missing evidence file: $EVIDENCE" >&2; exit 1; }

python3 - "$EVIDENCE" <<'PY'
import json, pathlib, sys

path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text())
required = ("targeted_send", "all_active_fanout", "user_fanout", "zero_recipient",
            "invalid_token", "revoked_token", "duplicate_message", "minimal_payload",
            "outcome_separation")
missing = [name for name in required if not isinstance(data.get(name), dict)]
if missing:
    raise SystemExit(f"remote-apns: missing evidence sections: {','.join(missing)}")

send = data["targeted_send"]
message_id = send.get("message_id")
if send.get("status") != "passed" or send.get("provider_status") != 200 or not message_id:
    raise SystemExit("remote-apns: targeted provider acceptance evidence is incomplete")
if not data["outcome_separation"].get("acknowledged_observed"):
    raise SystemExit("remote-apns: acknowledgement evidence is incomplete")

sensitive = {"health", "healthkit", "journal", "medication", "credential", "enrollment_material", "token"}
def walk(value):
    if isinstance(value, dict):
        for key, child in value.items():
            if key.lower() in sensitive:
                raise SystemExit(f"remote-apns: sensitive payload evidence key present: {key}")
            walk(child)
    elif isinstance(value, list):
        for child in value:
            walk(child)
walk(data)

not_measured = [name for name in required if data[name].get("status") != "passed"]
if not_measured:
    raise SystemExit(
        "remote-apns: acceptance cases are not complete: " + ",".join(not_measured)
    )
print(message_id)
PY

MESSAGE_ID="$(python3 - "$EVIDENCE" <<'PY'
import json, sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["targeted_send"]["message_id"])
PY
)"

[[ "$MESSAGE_ID" =~ ^[0-9a-fA-F-]{36}$ ]] || {
    echo "remote-apns: message_id is not a UUID" >&2
    exit 1
}

# The query is intentionally read-only and returns only ledger state, never a token or payload.
LIVE="$(ssh -o BatchMode=yes -o ConnectTimeout=5 "$PI_SSH" \
  "sudo -u postgres psql -d vitals -Atc \"SELECT (EXISTS (SELECT 1 FROM apns_messages WHERE message_id = '$MESSAGE_ID'))::int || ':' || (EXISTS (SELECT 1 FROM apns_delivery_attempts WHERE message_id = '$MESSAGE_ID' AND provider_status = 200 AND state = 'accepted'))::int || ':' || (EXISTS (SELECT 1 FROM apns_client_outcomes WHERE message_id = '$MESSAGE_ID' AND outcome = 'acknowledged'))::int;\"")"

[[ "$LIVE" == "1:1:1" ]] || {
    echo "remote-apns: live ledger does not contain message/provider/acknowledgement evidence: $LIVE" >&2
    exit 1
}

if rg -n --hidden --glob '!target/**' --glob '!*.xcresult/**' \
    'BEGIN (EC|RSA|OPENSSH|PRIVATE) KEY' \
    "$ROOT" >/tmp/vitals-apns-secret-scan.$$ 2>/dev/null; then
    cat /tmp/vitals-apns-secret-scan.$$ >&2
    rm -f /tmp/vitals-apns-secret-scan.$$
    echo "remote-apns: secret material detected in the working tree" >&2
    exit 1
fi
rm -f /tmp/vitals-apns-secret-scan.$$
echo "remote-apns: PASS — live targeted acceptance, acknowledgement, and complete case evidence verified"
