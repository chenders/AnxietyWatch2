# Tasks — GitHub Actions delivery to the Pi

Acceptance is each `Verify:` command's exit code, run by the conductor/operator. A worker report is
not evidence. Do not tick a box until the command has exited 0. The tasks are deliberately ordered:
runner first, then progressively more capable dry runs, then backup, then one minor update, then an
observation decision.

- [x] **T-R0 — Record the deployment boundary and staged rollout contract**
- [x] **T-R1 — Bootstrap and prove the native ARM64 runner**
- [x] **T-R2 — Add the trusted, non-deploying Pi workflow boundary**
- [x] **T-R3 — Build a SHA-addressed artifact and manifest**
- [x] **T-R4 — Implement a no-mutation host dry run**
- [x] **T-R5 — Rehearse failure, health, and rollback paths**
- [x] **T-R6 — Create the pre-deploy backup and recovery receipt**
- [ ] **T-R7 — Coordinate and execute one minor real update**
- [ ] **T-R8 — Observe, document rollback readiness, and retire SSH sync**

## T-R0 — Record the deployment boundary and staged rollout contract

**Tier:** implement-standard  
**Host:** mac  
**Context budget:** ~12k tokens  
**Depends:** none  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `../AnxietyWatch/.github/workflows/deploy.yml`,
`../AnxietyWatch/docker-compose.runners.yml`, `../AnxietyWatch/docs/SERVER_SETUP.md`,
`scripts/sync-to-pi.sh`, `rust-toolchain.toml`, `Cargo.toml`, `deploy/*.service`,
`openspec/changes/pi5-vitals-core/design.md`, `openspec/changes/pi5-vitals-core/RUNBOOK.md`

**Creates:** `openspec/changes/pi-github-actions-deployment/proposal.md`,
`openspec/changes/pi-github-actions-deployment/design.md`,
`openspec/changes/pi-github-actions-deployment/tasks.md`,
`openspec/changes/pi-github-actions-deployment/traceability.md`,
`deploy/github-runner/README.md`, `.github/workflows/pi-runner-smoke.yml`

**Do:**
1. Compare the AnxietyWatch hosted-build/self-hosted-deploy flow with this repository's native
   Rust/systemd/PostgreSQL/spool boundary.
2. Record the Docker-socket, mutable-`latest`, no-backup, and no-rollback differences rather than
   copying them into the Pi path.
3. Record every checked-in service `ExecStart` and every workspace binary; identify missing target
   mappings as blockers.
4. Define the runner, artifact, dry-run, backup, minor-update, observation, and sync-retirement
   gates. Do not claim that the runner is installed or that a real update occurred.

**Verify:** `test -s openspec/changes/pi-github-actions-deployment/proposal.md && test -s openspec/changes/pi-github-actions-deployment/design.md && test -s openspec/changes/pi-github-actions-deployment/tasks.md && test -s deploy/github-runner/README.md && test -s openspec/changes/pi-github-actions-deployment/traceability.md && test -s .github/workflows/pi-runner-smoke.yml && rg -q 'Docker socket|sync-to-pi.sh|dry-run|rollback|backup' openspec/changes/pi-github-actions-deployment/{proposal,design,tasks}.md && git diff --check`

**Expect:** exit 0; the plan exists, names the current limitations, and the smoke workflow explicitly
performs no deployment.

## T-R1 — Bootstrap and prove the native ARM64 runner

**Tier:** human  
**Host:** pi  
**Context budget:** ~10k tokens  
**Depends:** T-R0 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `deploy/github-runner/README.md`,
`.github/workflows/pi-runner-smoke.yml`, `rust-toolchain.toml`,
`openspec/changes/pi-github-actions-deployment/design.md`

**Creates:** a host-only runner installation and a host-only bootstrap receipt; no token or private
network value is committed

**Do:**
1. Create the repository-scoped ARM64 runner under the dedicated unprivileged account, using a
   recorded, checksum-verified runner release and short-lived registration token.
2. Install the pinned Rust toolchain, `rustfmt`, `clippy`, and only required build utilities.
3. Install and enable the constrained systemd runner service. Confirm it has no Docker socket,
   production secrets, or broad sudo rule.
4. Confirm the GitHub runner has exactly the required project labels and is Idle.
5. Dispatch the manual smoke workflow and retain its run URL/commit/runner/toolchain evidence in the
   host receipt without recording credentials.

**Verify:** `ssh "$PI_HOST" 'set -e; id github-vitals; systemctl is-active --quiet github-vitals-runner; test "$(uname -m)" = aarch64; sudo -u github-vitals -H env RUSTUP_HOME=/var/lib/github-vitals/.rustup CARGO_HOME=/var/lib/github-vitals/.cargo PATH=/var/lib/github-vitals/.cargo/bin:/usr/local/bin:/usr/bin:/bin rustup run 1.97.1 cargo --version; test ! -e /var/run/docker.sock || sudo -u github-vitals test ! -r /var/run/docker.sock; sudo test -s /var/lib/github-vitals/bootstrap-receipt.json'`

**Expect:** exit 0; an unprivileged ARM64 runner is active, has the pinned toolchain, cannot use the
Docker socket, and has a bootstrap receipt. A green job alone is insufficient.

## T-R2 — Add the trusted, non-deploying Pi workflow boundary

**Tier:** implement-standard  
**Host:** mac  
**Context budget:** ~10k tokens  
**Depends:** T-R1 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `openspec/changes/pi-github-actions-deployment/design.md`,
`.github/workflows/pi-runner-smoke.yml`, `AGENTS.md`, `rust-toolchain.toml`

**Creates:** `.github/workflows/pi-deploy.yml` and its workflow verification script or equivalent

**Do:**
1. Add a workflow that is unreachable from pull requests, runs only after hosted CI, uses a protected
   environment and serialized concurrency, and has `contents: read` only.
2. Add a build-only/manual path before any activation step; require the Pi labels and pinned SHA.
3. Use `persist-credentials: false`, fail on dirty/mismatched checkout, and write a summary that
   distinguishes `built`, `dry_run`, `activated`, and `rolled_back`.
4. Keep the deploy job blocked until T-R4 and T-R5 provide green dry-run/fault receipts. Do not add
   a root shell, Docker socket, production secret, or database migration command.

**Verify:** `python3 - <<'PY'
from pathlib import Path
p = Path('.github/workflows/pi-deploy.yml').read_text()
for text in ('workflow_dispatch', 'self-hosted', 'ARM64', 'vitals-pi', 'contents: read', 'persist-credentials: false', 'dry_run', 'production'):
    assert text in p, text
assert 'pull_request' not in p
assert 'docker.sock:' not in p
assert '/var/run/docker.sock' in p
PY
 git diff --check`

**Expect:** exit 0; the workflow can build or dry-run only, has no PR path, and cannot gain deploy
privilege by a Docker mount or broad token.

## T-R3 — Build a SHA-addressed artifact and manifest

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~16k tokens  
**Depends:** T-R2 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `Cargo.toml`, `rust-toolchain.toml`, `deploy/*.service`,
`deploy/*.rules`, `scripts/sync-to-pi.sh`, `openspec/changes/pi-github-actions-deployment/design.md`,
`openspec/changes/pi5-vitals-core/tasks.md` § global constraints and deployment tasks

**Creates:** the checked-in deployment helper/tooling, unit-to-binary target inventory, tests, and a
staged artifact manifest; exact paths are selected in the task implementation

**Do:**
1. Build the explicit target set natively with the pinned toolchain and run formatting, workspace
   tests, and clippy at the repository's deny levels.
2. Resolve every `ExecStart` to a built artifact or record it as a named blocker. Do not deploy a
   library in place of a binary and do not silently omit a collector or silence detector.
3. Stage under a per-SHA directory outside the checkout. Generate a versioned manifest with source
   SHA, tree hash, architecture, toolchain, target files, modes, SHA-256 checksums, and current
   release identity. Reject paths, symlinks, secrets, and unexpected files.
4. Add unit tests for manifest parsing, path traversal, checksum mismatch, missing target, duplicate
   target, SHA mismatch, and deterministic output. Keep the helper out of decoder/core crates.

**Verify:** `set -e; SOURCE_SHA="${GITHUB_SHA:-$(git rev-parse HEAD)}"; OUTPUT_DIR="${RUNNER_TEMP:-/tmp}"; export SOURCE_SHA; cargo test -p vitals-deploy && cargo clippy -p vitals-deploy --all-targets -- -D warnings && cargo fmt --all -- --check && cargo build --release -p vitalsd -p vitals-drain -p vitals-console -p vitals-collectors --bins && cargo run -p vitals-deploy -- manifest --source "$SOURCE_SHA" --dry-run --output "$OUTPUT_DIR/vitals-manifest.json" && jq -e '.source_sha == env.SOURCE_SHA and .architecture == "aarch64" and (.files | length) > 0 and (.blockers | type) == "array"' "$OUTPUT_DIR/vitals-manifest.json" >/dev/null`

**Verify correction:** Cargo does not build multiple auto-discovered binary targets when invoked with only a package selector; `--bins` is required so the H10 and BP `ExecStart` targets are actually present for manifest validation.

**Expect:** exit 0; the artifact is reproducible, SHA-addressed, complete for the reconciled target
set, and no live path is changed. If `vitals-deploy` is not the selected package name, update this
Verify deliberately as a separate decision before implementation; never weaken the checks.

## T-R4 — Implement a no-mutation host dry run

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~16k tokens  
**Depends:** T-R3 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `openspec/changes/pi-github-actions-deployment/design.md`,
`deploy/*.service`, `deploy/restore-verification/README.md`, `deploy/vitals-spool-drain.service`,
`crates/vitals-core/src/spool.rs`, the T-R3 manifest/helper files

**Creates:** dry-run command, host-side dry-run receipt, and tests proving zero mutation

**Do:**
1. Validate a staged release into a disposable root, including ownership/modes, unit references,
   `systemd-analyze verify`, dependency order, and current release discovery.
2. Snapshot hashes/metadata for live binaries, units, PostgreSQL schema identity, spool count/bytes/
   fingerprint, active service state, and runner authority before the run.
3. Run the exact workflow command with `--dry-run`; emit `would_change` and `blocked` entries with
   the value that caused each decision. Do not call `systemctl restart`, alter symlinks, touch the
   spool, run Sqitch, or write production databases.
4. Compare the after snapshot to the before snapshot and fail if any live state changed.

**Verify:** `ssh "$PI_HOST" 'set -e; before=$(mktemp); after=$(mktemp); /usr/local/bin/vitals-deploy snapshot --output "$before"; sudo /usr/local/bin/vitals-deploy apply --manifest /var/lib/vitals/deploy/test-manifest.json --dry-run --receipt /var/lib/vitals/recovery/last-dry-run.json; /usr/local/bin/vitals-deploy snapshot --output "$after"; cmp <(jq -S "del(.observed_at)" "$before") <(jq -S "del(.observed_at)" "$after"); sudo jq -e ".mode == \"dry-run\" and .activated == false and (.blocked | type) == \"array\"" /var/lib/vitals/recovery/last-dry-run.json >/dev/null'`

**Verify correction:** The recovery directory is root-only on the Pi; `sudo` is required for the read-only helper and receipt assertion. The helper's dry-run still forbids activation and production mutation.

**Expect:** exit 0; the full deployment plan is exercised, a receipt exists, and production state is
byte-for-byte unchanged apart from explicitly excluded observation timestamps.

## T-R5 — Rehearse failure, health, and rollback paths

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~18k tokens  
**Depends:** T-R4 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `openspec/changes/pi-github-actions-deployment/design.md`,
T-R3/T-R4 helper and test files, `deploy/*.service`, `deploy/restore-verification/verify-restore.sh`,
`openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`

**Creates:** fault-injection tests and retained rehearsal receipts under the host recovery directory

**Do:**
1. Exercise missing artifact, path traversal, checksum mismatch, wrong architecture, dirty source,
   interrupted staging, invalid unit, failed service start, stale health, and changed spool identity.
2. Prove preflight failures do not change the current release or services.
3. Prove a post-activation health failure restores the prior release and reports the failed unit,
   health document, age, or value; no rollback may erase spool data or run a database write.
4. Prove a concurrent invocation is rejected with the deployment lock/value named.

**Verify:** `ssh "$PI_HOST" 'set -e; sudo /usr/local/bin/vitals-deploy test-faults --manifest /var/lib/vitals/deploy/test-manifest.json --receipt /var/lib/vitals/recovery/rollback-rehearsal.json; sudo jq -e ".ok == true and .preflight_mutations == 0 and .rollback_completed == true and (.cases | length) >= 8 and (.cases | all(.reason != null and .exit_code != null))" /var/lib/vitals/recovery/rollback-rehearsal.json >/dev/null'`

**Verify correction:** The rehearsal receipt is root-owned under the protected recovery directory, so both the helper and read-only assertion run through `sudo`; no production service or database action is added.

**Expect:** exit 0; every injected fault is observable, fail-closed or rolled back, and leaves the
previous release/data boundary intact.

## T-R6 — Create the pre-deploy backup and recovery receipt

**Tier:** implement-complex  
**Host:** either  
**Context budget:** ~12k tokens  
**Depends:** T-R5 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `deploy/restore-verification/README.md`,
`deploy/restore-verification/verify-restore.sh`, `openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-github-actions-deployment/design.md`

**Creates:** an encrypted off-Pi recovery set and `/var/lib/vitals/recovery/github-deploy-gate.json`

**Do:**
1. Create the normal encrypted pgBackRest backup and record its identity/checksum/status.
2. Run the existing non-vacuous restore verification and retain its result; do not accept a file
   merely because it exists.
3. Record pending spool segment count/bytes/fingerprint and the recovery boundary. Preserve raw
   spool data; do not drain, concatenate, delete, or advance offsets to make the gate green.
4. Copy recovery evidence off Pi and record the backup age, restore result, schema identity, and
   rollback release. Keep encryption material outside GitHub and source control.

**Verify:** `ssh "$PI_HOST" 'set -e; sudo jq -e ".backup_verified == true and .restore_verified == true and .off_pi == true and .spool_fingerprint != null and .rollback_release != null and .encryption_key_location != null" /var/lib/vitals/recovery/github-deploy-gate.json >/dev/null'`

**Verify correction:** The recovery gate is intentionally root-owned and stored under the protected recovery directory; the assertion uses `sudo` for read-only access.

**Expect:** exit 0; the first real deployment has a fresh, encrypted, off-Pi, restore-verified gate
and a known previous release. A logical copy without restore evidence fails this task.

## T-R7 — Coordinate and execute one minor real update

**Tier:** human  
**Host:** pi  
**Context budget:** ~16k tokens  
**Depends:** T-R6 green and operator-approved maintenance window  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `openspec/changes/pi-github-actions-deployment/design.md`,
`deploy/github-runner/README.md`, T-R3/T-R4/T-R5/T-R6 receipts, the selected service unit, and its
health contract

**Creates:** a real deployment receipt and retained rollback instructions

**Do:**
1. Choose one small update that does not change schema, Bluetooth, `/boot/firmware`, collector
   identity, spool format, or alarm behavior. Record the exact source SHA and changed targets.
2. Confirm CI, dry-run, rollback rehearsal, backup/restore gate, current health, and operator
   presence immediately before dispatch. No unknown state is acceptable.
3. Dispatch the trusted main workflow; require the protected environment approval; observe the local
   Pi journal and health surfaces while the helper stages, activates, and verifies.
4. If any health or data-path check fails, stop and use the recorded manual rollback command. Do not
   improvise a second deployment to repair the first.
5. Record before/after release, changed services, health evidence, spool evidence, elapsed time, and
   whether rollback was needed. Keep the old release installed.

**Verify:** `ssh "$PI_HOST" 'set -e; jq -e ".activated == true and .source_sha != null and .preflight_ok == true and .health_ok == true and .spool_unchanged == true and .backup_gate == true and .rollback_command != null" /var/lib/vitals/recovery/github-deploy-latest.json >/dev/null; systemctl is-active --quiet vitalsd; test -S /run/vitals/vitalsd.sock'`

**Expect:** exit 0; the selected minor update is active, the safety daemon/socket is healthy, spool
identity was preserved, backup evidence was consumed, and rollback remains available.

## T-R8 — Observe, document rollback readiness, and retire SSH sync

**Tier:** implement-standard  
**Host:** mac  
**Context budget:** ~14k tokens  
**Depends:** T-R7 green plus the agreed observation period  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `scripts/sync-to-pi.sh`, all T-R1–T-R7 receipts,
`openspec/changes/pi5-vitals-core/tasks.md`, `openspec/changes/pi5-vitals-core/RUNBOOK.md`,
`openspec/changes/pi-github-actions-deployment/design.md`

**Creates:** observation report, updated runbooks/task Verify commands, and either a retired wrapper
or the deliberate decision to retain `sync-to-pi.sh`

**Do:**
1. Observe multiple trusted main deliveries and scheduled idle periods; review runner availability,
   artifact identity, health evidence, spool/drainer behavior, resource pressure, and journal errors.
2. Rehearse manual rollback again without a GitHub job and record its exit code.
3. Migrate task contracts that still depend on rsync/SSH to the staged native path, or explicitly
   retain the script for a documented emergency-only use.
4. Remove the old script only if the operator approves and the observation/rollback evidence is
   complete. Otherwise leave it intact and record the blocker; convenience is not a retirement gate.

**Verify:** `test -s docs/deploy/pi-github-actions-observation.md && rg -q 'observation|rollback' docs/deploy/pi-github-actions-observation.md && if rg -q 'scripts/sync-to-pi.sh' openspec/changes/pi5-vitals-core/tasks.md openspec/changes/pi5-vitals-core/RUNBOOK.md; then test -s scripts/sync-to-pi.sh && rg -q 'emergency' docs/deploy/pi-github-actions-observation.md; else test ! -e scripts/sync-to-pi.sh; fi`

**Expect:** exit 0; the observation period, rollback rehearsal, and final sync-script decision are
recorded. The script may remain if its emergency role is still justified.
