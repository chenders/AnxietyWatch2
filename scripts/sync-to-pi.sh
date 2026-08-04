#!/usr/bin/env bash
# sync-to-pi.sh — push the working tree to the-pi and optionally build+test.
#
# For anything host:pi — BlueZ, Postgres, systemd, or any
# debugging of them. macOS cannot compile that code, let alone run it, and a
# Mach-O binary will not execute on the Pi regardless.
#
# Usage:
#   scripts/sync-to-pi.sh              sync only
#   scripts/sync-to-pi.sh --build      sync, configure, build
#   scripts/sync-to-pi.sh --test       sync, build, run ctest
set -euo pipefail

HOST="${PI_HOST:-$PI_HOST}"
REMOTE="${PI_PATH:-$PI_PATH}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

rsync -az --delete \
  --exclude 'build*' --exclude '.git' --exclude '.idea' --exclude '.DS_Store' \
  "$ROOT/" "$HOST:$REMOTE/"
echo "synced → $HOST:$REMOTE"

case "${1:-}" in
  --build)
    ssh "$HOST" "cd $REMOTE && cargo build --workspace"
    ;;
  --test)
    ssh "$HOST" "cd $REMOTE && cargo build --workspace && cargo test --workspace"
    ;;
esac
