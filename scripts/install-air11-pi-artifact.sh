#!/usr/bin/env bash
# Install one allowlisted Pi artifact.
#
# This deliberately does not accept arbitrary source or destination paths.
# Copy to a regular /tmp file first, then install to one fixed destination.
#
# The allowlist is the mechanism, so a new destination is added here
# deliberately and reviewed rather than worked around with an inline scp. It
# started as AirSense-only and the filename still says so; `registry` was added
# when the deployed registry.toml was found to be three months stale and missing
# four of nine sources, including the one that wrote almost every row.
set -Eeuo pipefail

usage() {
    printf 'Usage: PI_HOST=user@host PI_PATH=/remote/project %s <collector|onboard|unit|registry>\n' "$0"
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
    # Reference data, not code: the single source of truth for source ids,
    # type ids, units and ranges. The cold export embeds a copy in every
    # monthly bundle so an archive can be read without project code, which
    # means a stale file here silently produces archives that cannot describe
    # their own contents.
    registry)
        local_path="vitals/registry.toml"
        remote_path="/usr/local/share/vitals/registry.toml"
        mode=0644
        reload=0
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

remote_dir=$(ssh "$PI_HOST" 'mktemp -d -p /tmp aw2-air11-install.XXXXXX')
remote_tmp="$remote_dir/$artifact"
cleanup() {
    ssh "$PI_HOST" "rm -rf -- '$remote_dir'" >/dev/null 2>&1 || true
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
sudo test \"\$(stat -c '%a:%U:%G' '$remote_path')\" = '${mode#0}:root:root'
sudo test -c /dev/null
"
