#!/usr/bin/env bash
# Install one allowlisted AirSense artifact on the Pi.
#
# This deliberately does not accept arbitrary source or destination paths.
# Copy to a regular /tmp file first, then install to one fixed destination.
set -Eeuo pipefail

usage() {
    printf 'Usage: PI_HOST=user@host PI_PATH=/remote/project %s <collector|onboard|unit|broker-unit>\n' "$0"
}

artifact=${1:-}
case "$artifact" in
    collector)
        local_path="target/release/resmed-air11-collector"
        remote_path="/usr/local/bin/resmed-air11-collector"
        mode=0755
        reload=0
        ;;
    onboard)
        local_path="target/release/resmed-air11-onboard"
        remote_path="/usr/local/bin/resmed-air11-onboard"
        mode=0755
        reload=0
        ;;
    unit)
        local_path="deploy/vitals-air11.service"
        remote_path="/etc/systemd/system/vitals-air11.service"
        mode=0644
        reload=1
        ;;
    broker-unit)
        local_path="deploy/vitals-ble-broker-air11.service"
        remote_path="/etc/systemd/system/vitals-ble-broker-air11.service"
        mode=0644
        reload=1
        ;;
    -h|--help)
        usage
        exit 0
        ;;
    *)
        usage >&2
        exit 64
        ;;
esac

: "${PI_HOST:?PI_HOST is required}"
: "${PI_PATH:?PI_PATH is required}"
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source_path="$root/$local_path"
[[ -f "$source_path" && ! -L "$source_path" ]] || {
    printf 'install blocked: local artifact is not a regular file: %s\n' "$local_path" >&2
    exit 1
}

remote_tmp="/tmp/aw2-air11-install-${artifact}-$$"
cleanup() {
    ssh "$PI_HOST" "rm -f -- '$remote_tmp'" >/dev/null 2>&1 || true
}
trap cleanup EXIT

scp -- "$source_path" "$PI_HOST:$remote_tmp"
ssh "$PI_HOST" "set -eu
sudo test -f '$remote_tmp'
sudo test ! -L '$remote_tmp'
sudo install -o root -g root -m '$mode' '$remote_tmp' '$remote_path'
$(if ((reload)); then printf 'sudo systemctl daemon-reload\n'; fi)
sudo stat -c 'installed type=%F mode=%a owner=%U group=%G' '$remote_path'
sudo test -f '$remote_path'
sudo test ! -L '$remote_path'
"
