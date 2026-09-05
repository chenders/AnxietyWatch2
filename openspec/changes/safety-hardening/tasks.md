# Tasks — production safety hardening and monitoring evidence

**Status:** T-S1 and T-S2 verified locally; T-S3 is next, and Pi deployment/complete-stack evidence remains open. Work is intentionally staged and each task is verified before the next starts.

**Amends:** `docs/specs/AMENDMENTS.md`; operationally supersedes the stale bounded-overlay
interpretation in `openspec/changes/emay-durable-ingestion/tasks.md` without changing the accepted
source-identity rule.

## T-S1 — Reconcile the core workspace and validation gates

**Tier:** implement-standard · **Host:** either · **Context budget:** ~5k · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `Cargo.toml`, `crates/vitals-core/Cargo.toml`,
`.github/workflows/ci.yml`, `crates/vitals-core/src/spool.rs`, the final T-D5 drainer source,
`deploy/vitals-spool-drain.service`

**Do:** Add the exact drainer source to the workspace, expose only the framing API it needs, add
explicit dependency/license policy, add reproducible coverage/test/fuzz/audit commands, and ensure
CI cannot silently skip a core gate.

**Verify:** `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run --workspace --all-features --no-fail-fast`

## T-S2 — Make spool durability and drain accounting observable

**Tier:** implement-complex · **Host:** either for tests, Pi for runtime · **Context budget:** ~8k ·
**Depends:** T-S1 · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `crates/vitals-core/src/copybin.rs`, `crates/vitals-core/src/spool.rs`,
`crates/vitalsd/src/main.rs`, `crates/vitalsd/src/state.rs`, `crates/vitalsd/src/bin/vitals-cli.rs`,
`crates/vitalsd/tests/socket.rs`, `crates/vitals-proto/src/lib.rs`, `crates/vitals-drain/src/lib.rs`,
`crates/vitals-drain/src/main.rs`, `crates/vitals-drain/src/bin/vitals-spool-fixture.rs`,
`crates/vitals-core/tests/roundtrip.rs`, `db/deploy/samples.sql`

**Do:** Bound active-segment lag, preserve crash-safe checkpoint ordering, report all stage counters,
fail closed on sync/rotation errors, and publish named drainer health. Add outage, partial-file,
duplicate, crash-window, lock, and recovery tests.

**Verify:** `cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo llvm-cov --package vitals-core --package vitals-proto --package vitalsd --package vitals-drain --lib --fail-under-lines 95 --summary-only`

## T-S3 — Reconcile EMAY live freshness with durable stored-session identity

**Tier:** implement-complex · **Host:** either for pure tests, Pi for BlueZ · **Context budget:** ~8k ·
**Depends:** T-S2 · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`,
`openspec/changes/emay-durable-ingestion/emay-identity-evidence.md`, `crates/emay-collector/src/lib.rs`,
`crates/emay-collector/src/main.rs`, `crates/vitals-decode/src/emay.rs`, `crates/vitals-proto/src/lib.rs`,
`vitals/registry.toml`

**Do:** Keep anonymous `EB` frames out of durable numeric rows, perform bounded periodic confirmed
stored-session synchronization, expose transfer/identity/durability states, and add pure tests for
all transitions and failure cases. Never use a host sequence.

**Verify:** `cargo test -p vitals-decode -p emay-collector -p vitals-proto && cargo clippy -p emay-collector --all-targets --all-features -- -D warnings && cargo llvm-cov --package emay-collector --fail-under-lines 95 --summary-only`

## T-S4 — Repair the durable read model and add the admin monitoring page

**Tier:** implement-complex · **Host:** either · **Context budget:** ~8k · **Depends:** T-S2, T-S3 ·
**Review required:** yes

**Reads:** `tools/panel_vitals_status.py`, `vitals/tools/tests/test_panel_vitals_status.py`,
`docs/design/pi-panel/direction-a-clinical/live-vitals.html`,
`docs/design/admin-console/claude-design/a/console.css`,
`docs/design/admin-console/claude-design/a/overview.html`,
`docs/specs/AMENDMENTS.md`

**Do:** Deduplicate only by durable identity, retain pending/live values separately, emit pipeline
health and explicit gaps, and create a read-only monitoring/history page with malformed/stale-feed
fail-closed behavior. Add page tests for current status, pipeline stages, bounded history, and
missing evidence.

**Verify:** `python3 -m pytest -q vitals/tools/tests/test_panel_vitals_status.py && node --test tools/tests/test_admin_monitoring_page.mjs && git diff --check`

## T-S5 — Run the continuous verification and Pi acceptance gates

**Tier:** implement-complex · **Host:** Pi · **Context budget:** ~8k · **Depends:** T-S1…T-S4 ·
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, this change's design/proposal, deployment units, the collector
identity note, `scripts/sync-to-pi.sh`, and the admin/page tests

**Do:** Build the exact checked-out artifacts, deploy atomically without touching unrelated services,
run database/spool crash/replay and live EMAY/H10 continuity checks, inspect all stage telemetry, and
record every command/output in a Pacific-time status update. Stop and report if any evidence is
ambiguous.

**Coverage policy:** In addition to this task's exact Verify command, CI enforces the repository-wide
source contract: every major Rust module/source target reported by `cargo llvm-cov` (including
binaries and deterministic hardware-boundary seams) is at least 90% covered by lines, and the
workspace aggregate is at least 95%. The exact CI command is:

```sh
cargo llvm-cov --workspace --all-features \
  --fail-under-file-lines 90 \
  --fail-under-lines 95 --summary-only
```

The real BlueZ/systemd/radio path still requires Pi acceptance; that does not justify excluding its
source file or binary from coverage.

**Verify:** `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run --workspace --all-features --no-fail-fast && cargo llvm-cov --workspace --all-features --fail-under-lines 95 --summary-only`

## Completion rule

Do not check a task box or call the system production-ready until its Verify command exits zero and
the corresponding evidence is recorded. Hardware verification remains required even when all local
checks pass.
