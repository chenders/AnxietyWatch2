#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
source_file=clients/apple/Packages/AWOutbox/Sources/AWOutbox/DurableOutbox.swift

rg -q 'completeUntilFirstUserAuthentication' "$source_file" || {
  echo "storage verification failed: outbox Data Protection class is absent" >&2
  exit 1
}
rg -q 'isExcludedFromBackup' "$source_file" || {
  echo "storage verification failed: outbox backup exclusion is absent" >&2
  exit 1
}
rg -q 'options: \.atomic' "$source_file" || {
  echo "storage verification failed: outbox atomic commit is absent" >&2
  exit 1
}
if rg -n 'UserDefaults|NSUbiquitousKeyValueStore' clients/apple/Packages/AWOutbox; then
  echo "storage verification failed: insecure preference storage found in outbox code" >&2
  exit 1
fi
echo "iOS storage security checks passed"
