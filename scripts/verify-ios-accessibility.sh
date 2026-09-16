#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
app=clients/apple/AnxietyWatchApp

rg -q 'TabView|NavigationStack|Form' "$app" --glob '*.swift' || {
  echo "accessibility verification failed: iPhone navigation surface is absent" >&2
  exit 1
}
rg -q '\.privacySensitive\(\)' "$app" --glob '*.swift' || {
  echo "accessibility verification failed: app-switcher privacy marking is absent" >&2
  exit 1
}
rg -q '\.accessibility(Label|Value|Hint)\(' "$app" --glob '*.swift' || {
  echo "accessibility verification failed: explicit VoiceOver semantics are absent" >&2
  exit 1
}
if rg -n '\.font\(\.system\(size:|\.frame\(width: [0-9]+, height: [0-9]+' "$app" --glob '*.swift'; then
  echo "accessibility verification failed: fixed-size typography or touch surface found" >&2
  exit 1
fi
rg -q 'Label\(' "$app" --glob '*.swift' || {
  echo "accessibility verification failed: semantic labels are absent" >&2
  exit 1
}
echo "iOS accessibility checks passed"
