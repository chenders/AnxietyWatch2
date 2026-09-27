#!/usr/bin/env bash
# Verify the Linux XE125 collector contract after the collector exists.
set -euo pipefail

cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
test -x target/release/xe125-collector || {
  echo "XE125 collector verify: release collector binary is missing" >&2
  exit 1
}
test -x scripts/xe125-live-socket-check.sh || {
  echo "XE125 collector verify: live socket check is missing" >&2
  exit 1
}
timeout --signal=INT --kill-after=5 10m scripts/xe125-live-socket-check.sh
