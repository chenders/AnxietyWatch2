#!/usr/bin/env bash
# sync-to-pi.sh — push the working tree to minidude and optionally build+test.
#
# For anything host:pi — BlueZ, epoll, systemd, sd_notify, getrandom(2), or any
# debugging of them. macOS cannot compile that code, let alone run it, and a
# Mach-O binary will not execute on the Pi regardless.
#
# Usage:
#   scripts/sync-to-pi.sh              sync only
#   scripts/sync-to-pi.sh --build      sync, configure, build
#   scripts/sync-to-pi.sh --test       sync, build, run ctest
set -euo pipefail

HOST="${PI_HOST:-chris@minidude}"
REMOTE="${PI_PATH:-/home/chris/AnxietyWatch2}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

rsync -az --delete \
  --exclude 'build*' --exclude '.git' --exclude '.idea' --exclude '.DS_Store' \
  "$ROOT/" "$HOST:$REMOTE/"
echo "synced → $HOST:$REMOTE"

case "${1:-}" in
  --build)
    ssh "$HOST" "cd $REMOTE && cmake -B build -DCMAKE_BUILD_TYPE=Debug -G Ninja >/dev/null && cmake --build build"
    ;;
  --test)
    ssh "$HOST" "cd $REMOTE && cmake -B build -DCMAKE_BUILD_TYPE=Debug -G Ninja >/dev/null && cmake --build build && ctest --test-dir build --output-on-failure"
    ;;
esac
