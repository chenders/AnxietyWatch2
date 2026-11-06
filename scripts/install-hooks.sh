#!/usr/bin/env bash
# install-hooks.sh — point git at .githooks/ so the pre-push gates run.
# Run once per clone, and once per worktree is NOT needed (core.hooksPath is
# repo-wide and worktrees share it).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
git -C "$ROOT" config core.hooksPath .githooks
chmod +x "$ROOT/.githooks/"* 2>/dev/null || true
echo "hooks installed: $(git -C "$ROOT" config core.hooksPath)"
echo "gates: public-info · no-push-to-main · secrets · capture artifacts ·"
echo "       PHI and device identifiers · comparison-package · build junk ·"
echo "       generated-files-current · cargo test+clippy+fuzz · iOS static gates ·"
echo "       agent provenance · Verify-command tampering"
echo "secrets, capture artifacts and PHI are scanned per commit, not on the net"
echo "diff — an add-then-redact branch no longer passes."
