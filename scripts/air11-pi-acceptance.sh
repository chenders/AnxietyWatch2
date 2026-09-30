#!/usr/bin/env bash
# Run a bounded, read-only AirSense acceptance check on the Pi.
#
# This script never pairs, scans, changes an adapter, enables a unit, edits an
# overlay, changes a route, or sends an AirSense RPC. It starts/restarts only
# the already-installed collector after an interactive operator confirmation,
# then stops it again when this script started it.

set -Eeuo pipefail
umask 077

readonly AIR11_UNIT="vitals-air11.service"
readonly AIR11_STATUS="/run/vitals-air11/status.json"
readonly H10_UNIT="vitals-h10.service"
readonly EMAY_UNIT="vitals-emay.service"
readonly EZSHARE_UNIT="hms-cpap.service"
readonly MIN_DURATION=20
readonly MAX_DURATION=3600

usage() {
    cat <<'EOF'
Usage:
  scripts/air11-pi-acceptance.sh --duration SECONDS --require-postgres
  [--require-h10] [--require-emay] [--require-ezshare-unchanged]

Runs a finite, read-only AirSense acceptance window. SECONDS must be between
20 and 3600 so the run contains an authenticated stream and a restart check.
The command asks for an interactive operator confirmation before it starts or
restarts vitals-air11.service. It never pairs, scans, enables a unit, changes
an adapter, edits /boot/firmware, changes a route, or sends a control RPC.
Optional --require-* flags add independent host-health gates.
EOF
}

die() {
    printf 'AirSense Pi acceptance blocked: %s\n' "$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "required command unavailable: $1"
}

duration=""
require_postgres=0
require_h10=0
require_emay=0
require_ezshare=0

while (($# > 0)); do
    case "$1" in
        --duration)
            (($# >= 2)) || die "--duration missing a value; expected ${MIN_DURATION}..${MAX_DURATION} seconds"
            duration=$2
            shift 2
            ;;
        --require-postgres)
            require_postgres=1
            shift
            ;;
        --require-h10)
            require_h10=1
            shift
            ;;
        --require-emay)
            require_emay=1
            shift
            ;;
        --require-ezshare-unchanged)
            require_ezshare=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            # Do not echo an unknown argument: an operator must not be able to
            # put a secret into an error line by passing it as an option value.
            die "unknown option; expected --duration and a supported --require-* gate"
            ;;
    esac
done

[[ -n "$duration" ]] || die "duration missing; expected --duration ${MIN_DURATION}..${MAX_DURATION}"
[[ "$duration" =~ ^[0-9]+$ ]] || die "duration=$duration is invalid; expected integer seconds"
duration=$((10#$duration))
((duration >= MIN_DURATION && duration <= MAX_DURATION)) || \
    die "duration=$duration is outside the bounded range ${MIN_DURATION}..${MAX_DURATION} seconds"

for command_name in \
    systemctl getent awk python3 timeout ip stat cmp date sleep seq mktemp mkdir cp rm tr head psql sudo; do
    require_command "$command_name"
done

if [[ ! -t 0 || ! -t 1 ]]; then
    die "operator authorization required: run this bounded command from a terminal"
fi
confirmation_token="AIR11-LIVE-${duration}"
printf '%s\n' \
    "This will run the read-only AirSense collector for ${duration} seconds," \
    "restart its session once and verify the selected host-health gates." \
    "It will not pair, scan, change an adapter, enable a unit, or edit firmware." \
    "Type ${confirmation_token} to authorize this exact bounded run: " >&2
IFS= read -r confirmation || die "operator authorization failed: confirmation input was unavailable"
[[ "$confirmation" == "$confirmation_token" ]] || \
    die "operator authorization failed: confirmation did not match ${confirmation_token}"

if ! work_dir=$(mktemp -d "${TMPDIR:-/tmp}/air11-acceptance.XXXXXXXX"); then
    die "temporary evidence directory creation failed: bounded private workspace unavailable"
fi
status_dir="$work_dir/status"
mkdir -m 700 "$status_dir" || die "temporary evidence directory creation failed: status workspace unavailable"
started_by_us=0

cleanup() {
    local exit_code=$?
    trap - EXIT
    if ((started_by_us == 1)); then
        if ! timeout 20 systemctl stop "$AIR11_UNIT" >/dev/null 2>&1; then
            printf 'AirSense Pi acceptance cleanup blocked: unit stop state is unknown\n' >&2
            exit_code=1
        fi
    fi
    rm -rf -- "$work_dir" || exit_code=1
    exit "$exit_code"
}
trap cleanup EXIT
trap 'die "operator interrupted: bounded acceptance has no result"' INT TERM

if ! vitals_home=$(getent passwd vitals | awk -F: 'NR == 1 { print $6; exit }'); then
    die "vitals account lookup failed: passwd database query returned nonzero"
fi
[[ -n "$vitals_home" ]] || die "vitals account lookup failed: home directory is empty"
credentials_path="$vitals_home/.air11/air11-credentials.json"
if ! sudo test -f "$credentials_path"; then
    die "credentials gate failed: operator credential file is absent or unreadable at the vitals home .air11 path"
fi

# Validate only the shape and permissions. The master key is read but never
# printed, copied, put in an argument, or included in a diagnostic. The
# credentials directory is private to the vitals account, so the validator
# runs through sudo even when the operator invokes this script unprivileged.
if ! target=$(sudo python3 - "$credentials_path" 2>"$work_dir/credentials.err" <<'PY'
import json
import os
import re
import stat
import sys

path = sys.argv[1]
metadata = os.lstat(path)
if not stat.S_ISREG(metadata.st_mode):
    raise ValueError("credentials file is not a regular non-symlink file")
if metadata.st_size > 16 * 1024:
    raise ValueError("credentials file exceeds 16384-byte bound")
if metadata.st_mode & 0o077 or not metadata.st_mode & stat.S_IRUSR:
    raise ValueError("credentials file is not owner-readable and private")
with open(path, encoding="utf-8") as handle:
    document = json.load(handle)
if not isinstance(document, dict):
    raise ValueError("credentials root is not an object")

client_id = document.get("client_id", document.get("clientId"))
if not isinstance(client_id, str) or not client_id or len(client_id) > 128:
    raise ValueError("client id is missing or outside the 1..128-byte bound")

master_key = next(
    (document.get(name) for name in ("master_pair_key", "masterPairKey", "master_key", "master_key_hex")
     if document.get(name) is not None),
    None,
)
if (not isinstance(master_key, str)
        or re.fullmatch(r"[0-9a-fA-F]{64}", master_key) is None
        or not any(character != "0" for character in master_key)):
    raise ValueError("master pair key is not exactly 32 non-zero bytes of hexadecimal")

linux_address = next(
    (document.get(name) for name in ("linux_address", "linuxAddress", "linux_mac", "linuxMac")
     if document.get(name) is not None),
    None,
)
address = linux_address if linux_address is not None else next(
    (document.get(name) for name in ("address", "device_address", "target")
     if document.get(name) is not None),
    None,
)
if not isinstance(address, str) or not address.strip():
    raise ValueError("target address is absent")
address = address.strip()

if re.fullmatch(r"(?:[0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}", address):
    normalized = address.upper()
elif re.fullmatch(r"(?:[0-9a-fA-F]{2}-){5}[0-9a-fA-F]{2}", address):
    normalized = address.replace("-", ":").upper()
elif re.fullmatch(r"[0-9a-fA-F]{12}", address):
    normalized = ":".join(address[index:index + 2] for index in range(0, 12, 2)).upper()
else:
    raise ValueError("target is not an explicit six-octet Linux address")
all_zero = ":".join("00" for _ in range(6))
if normalized == all_zero:
    raise ValueError("target is the all-zero address")
print(normalized)
PY
); then
    die "credentials gate failed: private JSON, key length, client-id, or explicit Linux target validation failed"
fi
[[ "$target" =~ ^([0-9A-F]{2}:){5}[0-9A-F]{2}$ ]] || \
    die "credentials gate failed: normalized target is not a six-octet uppercase MAC"

unit_state() {
    systemctl is-active "$1" 2>/dev/null || true
}

require_active() {
    local unit=$1
    local label=$2
    local state
    state=$(unit_state "$unit")
    [[ "$state" == "active" ]] || \
        die "$label health gate failed: unit_state=${state:-unknown}; expected active"
}

if ((require_h10 == 1)); then
    require_active "$H10_UNIT" "H10"
fi
if ((require_emay == 1)); then
    require_active "$EMAY_UNIT" "EMAY"
fi
if ((require_ezshare == 1)); then
    require_active "$EZSHARE_UNIT" "EzShare"
    route_before="$work_dir/route.before"
    if ! ip route show table main >"$route_before" 2>/dev/null; then
        die "EzShare route gate failed: could not capture the main route table before the run"
    fi
fi

psql_scalar() {
    local sql=$1
    local output
    # PostgreSQL peer authentication belongs to the vitals role; using it for
    # both root and unprivileged operator invocations avoids a hidden role
    # dependency in the acceptance command.
    local -a psql_prefix=(sudo -u vitals)
    if ! output=$(timeout 15 "${psql_prefix[@]}" psql --no-psqlrc --quiet --no-align --tuples-only \
        --no-password --set=ON_ERROR_STOP=1 --command "$sql" 2>"$work_dir/psql.err"); then
        return 1
    fi
    output=$(printf '%s' "$output" | tr -d '[:space:]')
    printf '%s' "$output"
}

require_h10_measurement() {
    local schema_state value
    schema_state=$(psql_scalar "SELECT CASE WHEN to_regclass('public.samples') IS NULL THEN 'missing' ELSE 'ready' END;") || \
        die "H10 value gate failed: PostgreSQL health query did not complete"
    [[ "$schema_state" == "ready" ]] || \
        die "H10 value gate failed: public.samples=$schema_state; expected ready"
    value=$(psql_scalar "SELECT count(*) FROM samples AS s JOIN vitals_source AS source ON source.id = s.source JOIN vitals_type AS type ON type.id = s.type WHERE source.name = 'polar_h10' AND type.name IN ('hr', 'rr_interval') AND s.timestamp >= to_timestamp(${start_epoch} - 5) AND s.timestamp <= now() + interval '5 seconds' AND s.value >= type.range_lo AND s.value <= type.range_hi;") || \
        die "H10 value gate failed: fresh validated-row query did not complete"
    [[ "$value" =~ ^[0-9]+$ ]] || \
        die "H10 value gate failed: validated-row count=$value; expected a numeric count"
    ((value >= 1)) || \
        die "H10 value gate failed: fresh validated rows=$value; expected at least one HR or RR interval row"
}

if ((require_postgres == 1)); then
    if ! schema_state=$(psql_scalar "SELECT CASE WHEN to_regclass('public.samples') IS NULL THEN 'missing' ELSE 'ready' END;"); then
        die "PostgreSQL gate failed: health query did not complete"
    fi
    [[ "$schema_state" == "ready" ]] || \
        die "PostgreSQL gate failed: public.samples=$schema_state; expected ready"
fi

if ! start_epoch=$(date +%s); then
    die "host-time gate failed: could not read the host epoch seconds"
fi
[[ "$start_epoch" =~ ^[0-9]+$ ]] || die "host-time gate failed: epoch value is not numeric"

snapshot_status() {
    local label=$1
    [[ -r "$AIR11_STATUS" ]] || \
        die "status gate failed: atomic status file is absent before snapshot=$label"
    cp -- "$AIR11_STATUS" "$status_dir/${label}.json" || \
        die "status gate failed: could not copy snapshot=$label"
}

fail_fast_status() {
    local label=$1
    if ! python3 - "$AIR11_STATUS" "$label" <<'PY'
import json
import sys

path, label = sys.argv[1:]
with open(path, encoding="utf-8") as handle:
    status = json.load(handle)
received = status.get("received_envelopes", 0)
decoded = status.get("decoded_frames", 0)
quarantined = status.get("quarantined", 0)
published = status.get("published_samples", 0)
if (
    isinstance(received, int)
    and isinstance(decoded, int)
    and isinstance(quarantined, int)
    and isinstance(published, int)
    and received > 0
    and decoded > 0
    and quarantined > 0
    and published == 0
):
    blocker = status.get("blocker")
    raise SystemExit(
        f"snapshot={label} received_envelopes={received} decoded_frames={decoded} "
        f"quarantined={quarantined} published_samples={published}; blocker={blocker!r}"
    )
PY
    then
        reason=$(python3 - "$AIR11_STATUS" "$label" <<'PY'
import json
import sys

path, label = sys.argv[1:]
with open(path, encoding="utf-8") as handle:
    status = json.load(handle)
print(
    f"snapshot={label} received_envelopes={status.get('received_envelopes')} "
    f"decoded_frames={status.get('decoded_frames')} quarantined={status.get('quarantined')} "
    f"published_samples={status.get('published_samples')}; blocker={status.get('blocker')!r}"
)
PY
        )
        die "status gate failed fast: $reason"
    fi
}

wait_for_air11_active() {
    local attempt state
    for attempt in $(seq 1 20); do
        state=$(unit_state "$AIR11_UNIT")
        [[ "$state" == "active" ]] && return 0
        sleep 1
    done
    state=$(unit_state "$AIR11_UNIT")
    die "AirSense service gate failed: unit_state=${state:-unknown}; expected active within 20 seconds"
}

wait_for_air11_measured() {
    local remaining=$1
    local deadline=$((SECONDS + remaining))
    while :; do
        if [[ -r "$AIR11_STATUS" ]] && python3 - "$AIR11_STATUS" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    status = json.load(handle)
if (
    status.get("link_state") == "streaming"
    and status.get("measurement_state") == "measured"
    and isinstance(status.get("published_samples"), int)
    and status["published_samples"] > 0
):
    raise SystemExit(0)
raise SystemExit(1)
PY
        then
            return 0
        fi
        ((SECONDS >= deadline)) && return 0
        sleep 1
    done
}

initial_air11_state=$(unit_state "$AIR11_UNIT")
if [[ "$initial_air11_state" == "active" ]]; then
    if ! timeout 20 systemctl restart "$AIR11_UNIT" >/dev/null 2>&1; then
        die "AirSense restart gate failed: initial authenticated session could not be restarted"
    fi
else
    started_by_us=1
    if ! timeout 20 systemctl start "$AIR11_UNIT" >/dev/null 2>&1; then
        die "AirSense start gate failed: unit_state=$initial_air11_state; expected start success"
    fi
fi
wait_for_air11_active
sleep 2
snapshot_status "after-start"
fail_fast_status "after-start"

half_duration=$((duration / 2))
sleep "$half_duration"
if ! timeout 20 systemctl restart "$AIR11_UNIT" >/dev/null 2>&1; then
    die "AirSense reconnect gate failed: bounded service restart returned nonzero"
fi
wait_for_air11_active
sleep 2
snapshot_status "after-restart"
fail_fast_status "after-restart"

remaining_duration=$((duration - half_duration))
wait_for_air11_measured "$remaining_duration"
snapshot_status "final"

target_file="$work_dir/target.txt"
printf '%s\n' "$target" >"$target_file"
chmod 600 "$target_file"
if ! python3 - "$status_dir" "$target_file" <<'PY' 2>"$work_dir/status.err"
import json
import sys
from pathlib import Path

status_dir = Path(sys.argv[1])
target = Path(sys.argv[2]).read_text(encoding="utf-8").strip()
paths = sorted(status_dir.glob("*.json"))
if len(paths) != 3:
    raise SystemExit(f"status snapshots={len(paths)}; expected 3")

allowed = {
    "link_state",
    "measurement_state",
    "session_id",
    "reconnect_count",
    "retry_count",
    "received_envelopes",
    "decoded_frames",
    "published_samples",
    "unavailable",
    "duplicates",
    "quarantined",
    "dropped",
    "durability_unknown",
    "last_host_receipt_unix_micros",
    "last_device_start_time",
    "last_event_data_id",
    "blocker",
    "event_blocker",
    "event_notifications",
    "event_records",
    "event_subscription_state",
    "event_truncated_notifications",
    "therapy_blocker",
    "therapy_finalized_host_unix_micros",
    "therapy_session_id",
    "therapy_start_host_unix_micros",
    "therapy_state",
    "therapy_stop_host_unix_micros",
}
streaming = False
measured = False
envelopes = False
decoded = False
published = False
host_time = False
device_time = False
unavailable = False
sessions = set()
for path in paths:
    raw = path.read_text(encoding="utf-8")
    for forbidden in (
        "masterPairKey",
        "master_pair_key",
        "sessionKey",
        "session_key",
        "master key",
        "session key",
    ):
        if forbidden in raw or target.lower() in raw.lower():
            raise SystemExit(f"snapshot={path.name} contains forbidden private material")
    try:
        status = json.loads(raw)
    except json.JSONDecodeError as error:
        raise SystemExit(f"snapshot={path.name} is invalid JSON") from error
    if not isinstance(status, dict):
        raise SystemExit(f"snapshot={path.name} root is not an object")
    unknown = set(status) - allowed
    if unknown:
        raise SystemExit(f"snapshot={path.name} contains unexpected fields")
    if status.get("link_state") == "streaming":
        streaming = True
    if status.get("measurement_state") == "measured":
        measured = True
    if isinstance(status.get("received_envelopes"), int) and status["received_envelopes"] > 0:
        envelopes = True
    if isinstance(status.get("decoded_frames"), int) and status["decoded_frames"] > 0:
        decoded = True
    if isinstance(status.get("published_samples"), int) and status["published_samples"] > 0:
        published = True
    if isinstance(status.get("last_host_receipt_unix_micros"), int) and status["last_host_receipt_unix_micros"] > 0:
        host_time = True
    if isinstance(status.get("last_device_start_time"), str) and status["last_device_start_time"]:
        device_time = True
    if isinstance(status.get("unavailable"), int) and status["unavailable"] > 0:
        unavailable = True
    if status.get("durability_unknown") is True:
        raise SystemExit(f"snapshot={path.name} has durability_unknown=true")
    session_id = status.get("session_id")
    if isinstance(session_id, int) and not isinstance(session_id, bool):
        sessions.add(session_id)

final = json.loads(paths[-1].read_text(encoding="utf-8"))
if final.get("link_state") != "streaming":
    raise SystemExit(f"final link_state={final.get('link_state')}; expected streaming")
if final.get("measurement_state") != "measured":
    raise SystemExit(
        f"final measurement_state={final.get('measurement_state')}; expected measured"
    )
if final.get("blocker") is not None:
    raise SystemExit("final measured state has a blocker")
if not streaming:
    raise SystemExit("no snapshot reached link_state=streaming")
if not measured:
    raise SystemExit("no snapshot reached measurement_state=measured")
if not envelopes:
    raise SystemExit("received_envelopes=0; expected a journaled stream envelope")
if not decoded:
    raise SystemExit("decoded_frames=0; expected a decoded StreamData frame")
if not published:
    raise SystemExit("published_samples=0; expected a vitalsd acknowledgement")
if not host_time:
    raise SystemExit("host receipt timestamp is absent or non-positive")
if not device_time:
    raise SystemExit("raw device start time is absent")
if not unavailable:
    raise SystemExit("unavailable counter=0; expected at least one preserved null/unavailable value")
if len(sessions) < 2:
    raise SystemExit(f"distinct session_ids={len(sessions)}; expected at least 2 across restart")
PY
then
    status_reason=$(head -n 1 "$work_dir/status.err" | tr -cd '[:print:]' || true)
    die "status/reconnect/null gate failed: ${status_reason:-evidence was unavailable}"
fi

sample_rows=0
distinct_types=0
metadata_rows=0
distinct_sessions=0
if ((require_postgres == 1)); then
window_predicate="source = (SELECT id FROM vitals_source WHERE name = 'resmed_air11') AND timestamp >= to_timestamp(${start_epoch} - 5) AND timestamp <= now() + interval '5 seconds'"
query_count() {
    local sql=$1
    local value
    value=$(psql_scalar "SELECT count(*) FROM samples WHERE ${window_predicate} AND ${sql};") || return 1
    [[ "$value" =~ ^[0-9]+$ ]] || return 1
    printf '%s' "$value"
}

# The collector is allowed to finish its non-sync append and the existing
# drainer is allowed a short, bounded grace period. No direct write is made by
# this script.
db_deadline=$(( $(date +%s) + 30 ))
sample_rows=0
distinct_types=0
metadata_rows=0
distinct_sessions=0
while :; do
    sample_rows=$(query_count "TRUE") || die "PostgreSQL evidence failed: source/type count query did not complete"
    distinct_types=$(psql_scalar "SELECT count(DISTINCT type) FROM samples WHERE ${window_predicate};") || \
        die "PostgreSQL evidence failed: distinct type query did not complete"
    metadata_rows=$(query_count "convert_from(extra, 'UTF8') LIKE '%clock_quality%' AND convert_from(extra, 'UTF8') LIKE '%untrusted%'") || \
        die "PostgreSQL evidence failed: host-time metadata query did not complete"
    distinct_sessions=$(psql_scalar "SELECT count(DISTINCT session_id) FROM samples WHERE ${window_predicate};") || \
        die "PostgreSQL evidence failed: session identity query did not complete"
    [[ "$distinct_types" =~ ^[0-9]+$ && "$distinct_sessions" =~ ^[0-9]+$ ]] || \
        die "PostgreSQL evidence failed: count query returned a non-numeric result"
    if ((sample_rows >= 1 && distinct_types >= 1 && metadata_rows >= 1 && distinct_sessions >= 2)); then
        break
    fi
    now_epoch=$(date +%s)
    if ((now_epoch >= db_deadline)); then
        die "PostgreSQL evidence incomplete: source_rows=$sample_rows types=$distinct_types metadata_rows=$metadata_rows sessions=$distinct_sessions; expected rows>=1 types>=1 metadata>=1 sessions>=2"
    fi
    sleep 3
done

fi
if ((require_h10 == 1)); then
    require_active "$H10_UNIT" "H10"
    require_h10_measurement
fi
if ((require_emay == 1)); then
    require_active "$EMAY_UNIT" "EMAY"
fi
if ((require_ezshare == 1)); then
    require_active "$EZSHARE_UNIT" "EzShare"
    route_after="$work_dir/route.after"
    if ! ip route show table main >"$route_after" 2>/dev/null; then
        die "EzShare route gate failed: could not capture the main route table after the run"
    fi
    cmp -s "$route_before" "$route_after" || \
        die "EzShare route gate failed: main route snapshot changed; expected unchanged"
fi

printf 'AirSense Pi acceptance passed: duration=%s seconds, source_rows=%s, types=%s, metadata_rows=%s, sessions=%s\n' \
    "$duration" "$sample_rows" "$distinct_types" "$metadata_rows" "$distinct_sessions"
