#!/usr/bin/env bash
# Replace the root deployment helper only through an explicit operator gate.
set -euo pipefail

if [[ "${1:-}" == "--self-test" ]]; then
    grep -Fq 'VITALS_HELPER_INSTALL_CONFIRM' "$0"
    grep -Fq 'sha256sum' "$0"
    grep -Fq 'ARM aarch64' "$0"
    grep -Fq 'github-deploy-gate.json' "$0"
    printf '%s\n' 'install-deploy-helper: self-test passed'
    exit 0
fi

if (( $# != 2 )); then
    printf '%s\n' 'helper install blocked: source path and expected SHA-256 are required' >&2
    exit 1
fi
if [[ "${VITALS_HELPER_INSTALL_CONFIRM:-}" != "1" ]]; then
    printf '%s\n' 'helper install blocked: VITALS_HELPER_INSTALL_CONFIRM=1 is required' >&2
    exit 1
fi
if (( EUID != 0 )); then
    printf 'helper install blocked: uid=%s, expected uid=0\n' "$EUID" >&2
    exit 1
fi

source=$1
expected=${2,,}
gate=${VITALS_BACKUP_GATE:-/var/lib/vitals/recovery/github-deploy-gate.json}
receipt=${VITALS_HELPER_RECEIPT:-/var/lib/vitals/recovery/vitals-deploy-helper-latest.json}
target=/usr/local/bin/vitals-deploy
recovery=/var/lib/vitals/recovery

[[ -f "$source" && ! -L "$source" ]] || {
    printf 'helper install blocked: source is missing or symlinked: %s\n' "$source" >&2
    exit 1
}
[[ "$expected" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'helper install blocked: expected SHA-256 is invalid: %s\n' "$expected" >&2
    exit 1
}
file_description=$(file -b "$source")
case "$file_description" in
    *'ELF 64-bit'*'ARM aarch64'*) ;;
    *)
        printf 'helper install blocked: source architecture is not ARM64 ELF: %s\n' "$file_description" >&2
        exit 1
        ;;
esac
actual=$(sha256sum "$source" | awk '{print $1}')
[[ "$actual" == "$expected" ]] || {
    printf 'helper install blocked: SHA-256 actual=%s expected=%s\n' "$actual" "$expected" >&2
    exit 1
}
[[ -f "$gate" && ! -L "$gate" ]] || {
    printf 'helper install blocked: backup gate is missing or symlinked: %s\n' "$gate" >&2
    exit 1
}
gate_uid=$(stat -c '%u' "$gate")
gate_mode=$(stat -c '%a' "$gate")
gate_mode_value=$((8#$gate_mode))
[[ "$gate_uid" == 0 && $((gate_mode_value & 022)) == 0 ]] || {
    printf 'helper install blocked: backup gate owner uid=%s mode=%s is not root-only\n' "$gate_uid" "$gate_mode" >&2
    exit 1
}
jq -e '
    .backup_verified == true and
    .restore_verified == true and
    .off_pi == true and
    (.spool_fingerprint | type == "string" and length > 0) and
    (.rollback_release | type == "string" and length > 0) and
    (.encryption_key_location | type == "string" and length > 0)
' "$gate" >/dev/null || {
    printf 'helper install blocked: backup gate lacks verified off-Pi recovery evidence: %s\n' "$gate" >&2
    exit 1
}
[[ -f "$target" && ! -L "$target" ]] || {
    printf 'helper install blocked: existing helper is missing or symlinked: %s\n' "$target" >&2
    exit 1
}
[[ ! -L "$receipt" ]] || {
    printf 'helper install blocked: receipt path is symlinked: %s\n' "$receipt" >&2
    exit 1
}

install -d -o root -g root -m 0750 "$recovery"
backup=$(mktemp "$recovery/vitals-deploy-helper-backup-XXXXXX")
tmp_target=$(mktemp /usr/local/bin/.vitals-deploy.new.XXXXXX)
receipt_tmp=$(mktemp "${receipt}.tmp.XXXXXX")
swapped=false
cleanup() {
    rc=$?
    if [[ "$swapped" == true && "$rc" -ne 0 ]]; then
        if ! install -o root -g root -m 0755 "$backup" "$target"; then
            printf 'helper install rollback blocked: could not restore %s\n' "$target" >&2
            rc=1
        fi
    fi
    rm -f "$tmp_target" "$receipt_tmp"
    exit "$rc"
}
trap cleanup EXIT
install -o root -g root -m 0755 "$target" "$backup"
install -o root -g root -m 0755 "$source" "$tmp_target"
jq -n \
    --arg helper "$target" \
    --arg sha256 "$actual" \
    --arg backup "$backup" \
    --arg installed_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{helper: $helper, sha256: $sha256, backup: $backup, installed_at: $installed_at, verified: true}' \
    > "$receipt_tmp"
chown root:root "$receipt_tmp"
chmod 600 "$receipt_tmp"
mv -f "$tmp_target" "$target"
swapped=true
mv -f "$receipt_tmp" "$receipt"
swapped=false
printf 'helper install: verified %s\n' "$receipt"
