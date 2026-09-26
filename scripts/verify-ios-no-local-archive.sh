#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

if rg -n 'CoreData|NSPersistentContainer|SQLite|GRDB|Realm' clients/apple --glob '*.swift' --glob '*.json' --glob '*.plist'; then
  echo "local archive verification failed: local history database dependency found" >&2
  exit 1
fi
rg -q 'FileOutboxStore' clients/apple/Packages/AWOutbox/Sources/AWOutbox/DurableOutbox.swift || {
  echo "local archive verification failed: temporary outbox implementation is absent" >&2
  exit 1
}
echo "no-local-archive checks passed"
