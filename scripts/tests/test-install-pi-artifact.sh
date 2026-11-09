#!/usr/bin/env bash
# The artifact installer's allowlist is a security property, and nothing tested
# it. AGENTS.md records why it exists: an ad-hoc install once used /dev/null as
# a copy destination, which is a character device, not a file. The allowlist is
# the thing that makes that unrepresentable — so it needs a test that fails if
# the allowlist is widened by accident.
#
# These cases never contact a host. A valid artifact name reaches the
# `PI_HOST` requirement and stops there; an invalid one is rejected before it.
# The difference in blocker is what proves membership.
set -Eeuo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
script="$root/scripts/install-air11-pi-artifact.sh"
failures=0

fail() {
    printf 'install-pi-artifact test FAIL: %s\n' "$1" >&2
    failures=$((failures + 1))
}

[[ -x $script ]] || { fail "installer is not executable: $script"; exit 1; }

# Every allowlisted name must get past argument parsing and stop on the missing
# host, not on the usage error.
for artifact in collector onboard unit registry; do
    output=$(env -u PI_HOST -u PI_PATH "$script" "$artifact" 2>&1 || true)
    status=$(env -u PI_HOST -u PI_PATH "$script" "$artifact" >/dev/null 2>&1; echo $?)
    case "$output" in
        *PI_HOST*) ;;
        *) fail "allowlisted artifact '$artifact' did not reach the PI_HOST check: $output" ;;
    esac
    (( status != 64 )) || fail "allowlisted artifact '$artifact' was rejected as unknown"
done

# Anything not on the allowlist exits 64, including names that look like paths.
# A destination must never be derivable from the argument.
for artifact in bogus '' '/dev/null' '../../etc/passwd' 'collector unit' 'REGISTRY'; do
    status=$(env -u PI_HOST -u PI_PATH "$script" "$artifact" >/dev/null 2>&1; echo $?)
    (( status == 64 )) || fail "non-allowlisted artifact '$artifact' exited $status, want 64"
done

# The remote verification must keep asserting that /dev/null survived. This is
# the specific check AGENTS.md demands after any Pi deployment command.
grep -Fq 'test -c /dev/null' "$script" \
    || fail 'installer no longer verifies /dev/null is a character device'

# No allowlist entry may target a device node or a directory.
while IFS= read -r dest; do
    case "$dest" in
        /dev/*) fail "allowlist targets a device path: $dest" ;;
        */) fail "allowlist targets a directory: $dest" ;;
    esac
done < <(grep -oE 'remote_path="[^"]+"' "$script" | sed 's/remote_path="//;s/"$//')

# Every allowlisted local path must exist in the tree, or the installer would
# fail at deploy time rather than at review time.
while IFS= read -r src; do
    [[ -f "$root/$src" ]] || fail "allowlisted local path is missing: $src"
done < <(grep -oE 'local_path="[^"]+"' "$script" | sed 's/local_path="//;s/"$//' | grep -v '^target/')

if (( failures )); then
    printf 'install-pi-artifact: %d failure(s)\n' "$failures" >&2
    exit 1
fi
printf '%s\n' 'install-pi-artifact: allowlist contract holds'
