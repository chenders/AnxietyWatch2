#!/usr/bin/env bash
# Verify the transport-neutral XE125 frame model task contract.
set -euo pipefail

test -f fuzz/fuzz_targets/fuzz_xe125_frame.rs || {
  echo "XE125 frame verify: fuzz target is missing" >&2
  exit 1
}
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fuzz run fuzz_xe125_frame -- -max_total_time=60
