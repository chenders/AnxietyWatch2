#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
watch=clients/apple/AnxietyWatchWatch

rg -q '^import WatchConnectivity$' "$watch"/*.swift || {
  echo "watch-boundary: FAIL — WatchConnectivity relay is absent" >&2
  exit 1
}
rg -q 'WatchRelayStore|WatchRelayQueue' "$watch"/*.swift || {
  echo "watch-boundary: FAIL — durable watch queue is absent" >&2
  exit 1
}
if rg -n 'Keychain|SecureEnclave|ClientOrigin|URLSession|api/v1|Bearer|UserDefaults|SwiftData|CoreData' \
  "$watch" --glob '*.swift'; then
  echo "watch-boundary: FAIL — watch contains phone/Pi credential, network, or archive code" >&2
  exit 1
fi
rg -q 'transferUserInfo' "$watch"/*.swift || {
  echo "watch-boundary: FAIL — disconnected delivery path is absent" >&2
  exit 1
}
rg -q 'sendMessage' "$watch"/*.swift || {
  echo "watch-boundary: FAIL — reachable delivery path is absent" >&2
  exit 1
}
printf 'watch-boundary: PASS — durable queue, reachable/disconnected relay, and no Pi credential verified\n'
