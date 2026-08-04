#!/usr/bin/env bash
# install-hooks.sh — point git at .githooks/ so the pre-push gates run.
# Run once per clone, and once per worktree is NOT needed (core.hooksPath is
# repo-wide and worktrees share it).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
git -C "$ROOT" config core.hooksPath .githooks
chmod +x "$ROOT/.githooks/"* 2>/dev/null || true
echo "hooks installed: $(git -C "$ROOT" config core.hooksPath)"
echo "gates: no-push-to-main · secrets · capture artifacts · build junk ·"
echo "       generated-files-current · build+ctest · agent provenance ·"
echo "       Verify-command tampering"
