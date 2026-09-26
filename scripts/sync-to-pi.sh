#!/usr/bin/env bash
# sync-to-pi.sh — push the working tree to the Pi and optionally build+test.
#
# For anything host:pi — BlueZ, Postgres, systemd, or any
# debugging of them. macOS cannot compile that code, let alone run it, and a
# Mach-O binary will not execute on the Pi regardless.
#
# Requires PI_HOST and PI_PATH to be set (e.g. in a .local file or shell profile).
# Device-specific values are not committed; see .gitignore for *.local.
#
# Usage:
#   PI_HOST=user@hostname PI_PATH=/home/user/AnxietyWatch2 scripts/sync-to-pi.sh
#   scripts/sync-to-pi.sh              sync only
#   scripts/sync-to-pi.sh --build      sync, configure, build
#   scripts/sync-to-pi.sh --test       sync, build, run cargo test
set -euo pipefail

if [[ -z "${PI_HOST:-}" || -z "${PI_PATH:-}" ]]; then
  echo "error: PI_HOST and PI_PATH must be set (e.g. PI_HOST=user@hostname PI_PATH=/home/user/AnxietyWatch2)" >&2
  exit 1
fi
HOST="$PI_HOST"
REMOTE="$PI_PATH"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

rsync -az --delete \
  --exclude 'build*' --exclude 'target' --exclude 'fuzz/target' --exclude '.git' --exclude '.idea' --exclude '.DS_Store' \
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
