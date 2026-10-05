# Tasks — silence-detector live activation

**Reads:** `docs/specs/AMENDMENTS.md` · `crates/vitals-console/src/bin/silence-detector.rs` · `deploy/silence-detector.service` · `db/deploy/silence_detector.sql` · `scripts/verify-sqitch-change.sh` · `crates/vitals-deploy/src/main.rs` · `deploy/github-runner/github-vitals-deploy.sudoers`

**Amends:** `silence-detector-runtime-health` — production schema/helper activation was explicitly outside that change's scope.

Acceptance is each `Verify:` command's exit code. Do not tick a box before running its command.

- [x] **T-LA-1 — Fail closed on missing detector schema** · implement-standard
- [x] **T-LA-2 — Add backup-gated detector schema procedure** · implement-complex
- [x] **T-LA-3 — Add operator-only root helper replacement procedure** · implement-complex
- [x] **T-LA-4 — Apply schema and refresh helper on the Pi** · human · Pi
- [x] **T-LA-5 — Verify persistence and cycles, then enable explicitly** · human · Pi

## T-LA-1 — Fail closed on missing detector schema

**Do:** Probe all detector columns/tables after connecting; exit status 70 with an actionable
schema-object message when any is absent.

**Verify:** `cargo test -p vitals-console --bin silence-detector && cargo fmt --all -- --check && rg -q 'schema validation failed' crates/vitals-console/src/bin/silence-detector.rs`

## T-LA-2 — Add backup-gated detector schema procedure

**Do:** Add a root-only explicit-confirmation script that validates the backup gate, refuses partial
schemas, applies only `db/deploy/silence_detector.sql`, verifies the objects, and writes a protected
receipt without touching `samples`.

**Verify:** `scripts/apply-silence-detector-schema.sh --self-test && scripts/verify-sqitch-change.sh silence_detector && rg -q 'samples' scripts/apply-silence-detector-schema.sh`

## T-LA-3 — Add operator-only root helper replacement procedure

**Do:** Add a script requiring explicit confirmation, ARM64 ELF validation, expected SHA-256, and an
atomic root-owned replacement with a rollback copy; keep the GitHub sudoers rule unchanged.

**Verify:** `scripts/install-deploy-helper.sh --self-test && rg -q 'github-vitals ALL' deploy/github-runner/github-vitals-deploy.sudoers && rg -q 'sha256' scripts/install-deploy-helper.sh`

## T-LA-4 — Apply schema and refresh helper on the Pi

**Do:** With operator approval, transfer the checked-in guarded procedures to the Pi, run both using
the existing backup gate, verify receipts, and leave `silence-detector` disabled until T-LA-5 passes.

**Verify:** `ssh "$PI_HOST" 'set -e; sudo jq -e ".schema == \"silence_detector\" and .verified == true" /var/lib/vitals/recovery/silence-detector-schema-latest.json >/dev/null; sudo jq -e ".verified == true" /var/lib/vitals/recovery/vitals-deploy-helper-latest.json >/dev/null; test "$(sudo systemctl is-enabled silence-detector)" = disabled'`

## T-LA-5 — Verify persistence and cycles, then enable explicitly

**Do:** Transfer `verify-silence-detector-runtime.sh` to the Pi, start only the disabled service,
observe three 30-second cycles, verify persisted status and notification evidence, verify structured
health and unchanged `NRestarts`, then enable only as the final operator-approved action.

**Verify:** `scripts/verify-silence-detector-runtime.sh --self-test && ssh "$PI_HOST" 'set -e; VITALS_ENABLE_CONFIRM=1 sudo -n --preserve-env=VITALS_ENABLE_CONFIRM /usr/local/lib/vitals/ops/verify-silence-detector-runtime.sh --enable; sudo systemctl is-enabled silence-detector; sudo systemctl is-active silence-detector; sudo jq -e ".verified == true and .enabled == true and .cycles == 3 and .baseline_restarts == .final_restarts and .status_recent > 0 and .events_recent > 0" /var/lib/vitals/recovery/silence-detector-runtime-latest.json >/dev/null; test "$(sudo sha256sum /usr/local/bin/silence-detector | awk '{print \$1}')" = "$(sudo jq -r .binary_sha256 /var/lib/vitals/recovery/silence-detector-runtime-latest.json)"'`
