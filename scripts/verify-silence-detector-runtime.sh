#!/usr/bin/env bash
# Verify persistence and stable runtime before the final explicit enablement.
set -euo pipefail

if [[ "${1:-}" == "--self-test" ]]; then
    grep -Fq 'NRestarts' "$0"
    grep -Fq 'notification_events' "$0"
    grep -Fq 'binary_sha256' "$0"
    grep -Fq 'VITALS_ENABLE_CONFIRM' "$0"
    grep -Fq 'sleep 30' "$0"
    printf '%s\n' 'verify-silence-detector-runtime: self-test passed'
    exit 0
fi

if [[ "${1:-}" != "--enable" || $# != 1 ]]; then
    printf '%s\n' 'runtime verification blocked: use --enable as the final explicit action' >&2
    exit 1
fi
if [[ "${VITALS_ENABLE_CONFIRM:-}" != "1" ]]; then
    printf '%s\n' 'runtime verification blocked: VITALS_ENABLE_CONFIRM=1 is required' >&2
    exit 1
fi
if (( EUID != 0 )); then
    printf 'runtime verification blocked: uid=%s, expected uid=0\n' "$EUID" >&2
    exit 1
fi

unit=silence-detector.service
binary=/usr/local/bin/silence-detector
receipt=${VITALS_RUNTIME_RECEIPT:-/var/lib/vitals/recovery/silence-detector-runtime-latest.json}
cycles=3
interval_s=30

initial_enabled=$(systemctl is-enabled "$unit" 2>/dev/null || true)
[[ "$initial_enabled" == "disabled" ]] || {
    printf 'runtime verification blocked: initial enablement=%s, expected disabled\n' "$initial_enabled" >&2
    exit 1
}
[[ -f "$binary" && ! -L "$binary" ]] || {
    printf 'runtime verification blocked: detector binary is missing or symlinked: %s\n' "$binary" >&2
    exit 1
}
binary_sha256=$(sha256sum "$binary" | awk '{print $1}')
start_epoch=$(date +%s)
enabled_by_script=false
cleanup() {
    rc=$?
    if [[ "$rc" -ne 0 ]]; then
        current_enabled=$(systemctl is-enabled "$unit" 2>/dev/null || true)
        if [[ "$enabled_by_script" == true || "$current_enabled" == "enabled" ]]; then
            if ! systemctl disable "$unit" >/dev/null 2>&1 || [[ "$(systemctl is-enabled "$unit" 2>/dev/null || true)" != "disabled" ]]; then
                printf 'runtime verification rollback blocked: could not disable %s\n' "$unit" >&2
                rc=1
            fi
        fi
        if ! systemctl stop "$unit" >/dev/null 2>&1 || [[ "$(systemctl is-active "$unit" 2>/dev/null || true)" != "inactive" ]]; then
            printf 'runtime verification rollback blocked: could not stop %s\n' "$unit" >&2
            rc=1
        fi
    fi
    exit "$rc"
}
trap cleanup EXIT
systemctl start "$unit"

read_service_state() {
    service_state=(
        "$(systemctl show "$unit" -p ActiveState --value)"
        "$(systemctl show "$unit" -p SubState --value)"
        "$(systemctl show "$unit" -p Result --value)"
        "$(systemctl show "$unit" -p ExecMainStatus --value)"
        "$(systemctl show "$unit" -p NRestarts --value)"
    )
    [[ "${#service_state[@]}" -eq 5 && -n "${service_state[4]}" ]] || {
        printf '%s\n' 'runtime verification blocked: systemd evidence is incomplete' >&2
        return 1
    }
    [[ "${service_state[0]}" == active && "${service_state[1]}" == running &&
       "${service_state[2]}" == success && "${service_state[3]}" == 0 ]] || {
        printf 'runtime verification blocked: state=%s sub_state=%s result=%s exit=%s restarts=%s\n' \
            "${service_state[0]}" "${service_state[1]}" "${service_state[2]}" "${service_state[3]}" "${service_state[4]}" >&2
        return 1
    }
}
read_service_state
baseline_restarts=${service_state[4]}
for ((cycle = 1; cycle <= cycles; cycle++)); do
    sleep 30
    read_service_state
    [[ "${service_state[4]}" == "$baseline_restarts" ]] || {
        printf 'runtime verification blocked: NRestarts changed from %s to %s during cycle %s\n' \
            "$baseline_restarts" "${service_state[4]}" "$cycle" >&2
        exit 1
    }
done

status_recent=$(runuser -u postgres -- psql --dbname vitals --tuples-only --no-align \
    --command "SELECT count(*) FROM status_observations WHERE reported_at >= to_timestamp($start_epoch)")
events_recent=$(runuser -u postgres -- psql --dbname vitals --tuples-only --no-align \
    --command "SELECT count(*) FROM notification_events WHERE emitted_at >= to_timestamp($start_epoch)")
[[ "$status_recent" =~ ^[1-9][0-9]*$ ]] || {
    printf 'runtime verification blocked: no status_observations persisted since epoch %s\n' "$start_epoch" >&2
    exit 1
}
[[ "$events_recent" =~ ^[1-9][0-9]*$ ]] || {
    printf 'runtime verification blocked: no notification_events persisted since epoch %s\n' "$start_epoch" >&2
    exit 1
}
[[ ! -L "$receipt" ]] || {
    printf 'runtime verification blocked: receipt path is symlinked: %s\n' "$receipt" >&2
    exit 1
}
install -d -o root -g root -m 0750 "$(dirname -- "$receipt")"
tmp_receipt=$(mktemp "${receipt}.tmp.XXXXXX")
trap 'rm -f "$tmp_receipt"' EXIT
jq -n \
    --arg unit "$unit" \
    --arg binary_sha256 "$binary_sha256" \
    --argjson cycles "$cycles" \
    --argjson interval_s "$interval_s" \
    --arg baseline_restarts "$baseline_restarts" \
    --arg final_restarts "${service_state[4]}" \
    --arg status_recent "$status_recent" \
    --arg events_recent "$events_recent" \
    --arg verified_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{unit: $unit, binary_sha256: $binary_sha256, enabled: true, cycles: $cycles, interval_s: $interval_s, baseline_restarts: ($baseline_restarts | tonumber), final_restarts: ($final_restarts | tonumber), status_recent: ($status_recent | tonumber), events_recent: ($events_recent | tonumber), verified_at: $verified_at, verified: true}' \
    > "$tmp_receipt"
chown root:root "$tmp_receipt"
chmod 600 "$tmp_receipt"
systemctl enable "$unit" >/dev/null
enabled_by_script=true
final_enabled=$(systemctl is-enabled "$unit")
[[ "$final_enabled" == enabled ]] || {
    printf 'runtime verification blocked: final enablement=%s, expected enabled\n' "$final_enabled" >&2
    exit 1
}
mv -f "$tmp_receipt" "$receipt"
trap - EXIT
printf 'runtime verification: verified %s\n' "$receipt"
