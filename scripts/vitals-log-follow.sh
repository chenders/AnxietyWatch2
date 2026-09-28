#!/usr/bin/env bash
# Follow the combined AnxietyWatch2 service journal on the Pi.
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  exec sudo -- "$0" "$@"
fi

since="${VITALS_LOG_SINCE:-15 minutes ago}"
units=(
  -u vitalsd.service
  -u vitals-spool-drain.service
  -u vitals-spool-seal.service
  -u vitals-console.service
  -u vitals-h10.service
  -u vitals-bp.service
  -u vitals-bp-autoconnect.service
  -u github-vitals-runner.service
)

printf 'Following AnxietyWatch2 services since %s (UTC); press Ctrl-C to stop.\n' "$since" >&2
exec journalctl \
  --follow \
  --boot \
  --since "$since" \
  --utc \
  --no-pager \
  --output=short-iso-precise \
  "${units[@]}"
