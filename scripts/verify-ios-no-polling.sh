#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if rg -n 'Timer|DispatchSourceTimer|while[[:space:]]+true|NWConnection|AVAudioSession|CLLocationManager' \
  clients/apple/AnxietyWatchApp clients/apple/Packages \
  --glob '*.swift' --glob '!**/Tests/**'; then
  echo "polling verification failed: prohibited timer, permanent socket, audio, location, or BLE keep-alive found" >&2
  exit 1
fi
rg -q 'public actor SyncEngine' clients/apple/Packages/AWTransport/Sources/AWTransport/SyncEngine.swift || {
  echo "polling verification failed: single-owner trigger-driven sync engine is absent" >&2
  exit 1
}
echo "iOS no-polling checks passed"
