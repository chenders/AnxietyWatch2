# Tasks — silence-detector runtime and deployment health

Acceptance is each `Verify:` command's exit code. Do not tick a box before running its command.

- [x] **T-SD-1 — Correct the SQLx URL contract and artifact modes** · implement-standard
- [x] **T-SD-2 — Require stable structured service health** · implement-complex
- [x] **T-SD-5 — Restore downloaded nonbinary artifact modes** · implement-standard
- [x] **T-SD-6 — Escape SQLx URL percent signs for systemd** · implement-standard
- [x] **T-SD-7 — Record stable per-service activation evidence** · implement-complex
- [ ] **T-SD-3 — Verify the corrected daemon on the Pi before enabling it** · human · Pi
- [x] **T-SD-4 — Record the incident and final acceptance evidence** · docs

## T-SD-1 — Correct the SQLx URL contract and artifact modes

**Do:** Use a PostgreSQL URL for the Unix socket, reject unsupported `DATABASE_URL` syntax at
startup, and normalize systemd units to `0644` while retaining executable binary mode `0755`.

**Verify:** `cargo test -p vitals-console --bin silence-detector && cargo test -p vitals-deploy && rg -q '^Environment=DATABASE_URL=postgresql://' deploy/silence-detector.service`

## T-SD-2 — Require stable structured service health

**Do:** Capture systemd state, result, exit status, and restart count; wait after restart; reject
unstable long-running services; preserve the completed BP oneshot exception; and name health
blockers in activation failures.

**Verify:** `cargo test -p vitals-deploy && cargo clippy -p vitals-deploy --all-targets -- -D warnings && cargo fmt --all -- --check`

## T-SD-5 — Restore downloaded nonbinary artifact modes

**Do:** Restore mode `0644` for downloaded systemd-unit and udev-rule artifacts after GitHub artifact extraction, so the manifest's mode contract is satisfied before activation.

**Verify:** `rg -q 'path.*units.*rules' .github/workflows/pi-deploy.yml && rg -q 'chmod 644' .github/workflows/pi-deploy.yml`

## T-SD-6 — Escape SQLx URL percent signs for systemd

**Do:** Double the percent signs in the unit's URL so systemd passes SQLx the encoded Unix-socket
path instead of interpreting `%2F` as a unit specifier.

**Verify:** `rg -q 'host=%%2Fvar%%2Frun%%2Fpostgresql' deploy/silence-detector.service && rg -q 'systemd.*specifier' deploy/silence-detector.service`

## T-SD-7 — Record stable per-service activation evidence

**Do:** Require three consecutive two-second stable service observations after the initial settling
window, reject any restart-count increase or unhealthy state, and include state, result, exit status,
restart count, and the derived health result for every service in activation receipts.

**Verify:** `cargo test -p vitals-deploy && rg -q 'services: Vec<ReceiptService>' crates/vitals-deploy/src/main.rs && rg -q 'service_states_are_stable' crates/vitals-deploy/src/main.rs`

## T-SD-3 — Verify the corrected daemon on the Pi before enabling it

**Do:** After the reviewed release is installed, start only `silence-detector`, confirm it remains
active through multiple 30-second cycles, confirm its restart count does not increase, and verify
status/notification persistence. Then enable it only with operator approval.

**Verify:** `ssh "$PI_HOST" 'sudo systemctl is-enabled silence-detector && sudo systemctl is-active silence-detector && sudo systemctl show silence-detector -p ActiveState -p SubState -p Result -p ExecMainStatus -p NRestarts && sudo jq -e ".activated == true and .health_ok == true and .rollback_completed == false" /var/lib/vitals/recovery/github-deploy-latest.json >/dev/null'`

## T-SD-4 — Record the incident and final acceptance evidence

**Do:** Correct the deployment observation document with the executable-mode failure, the SQLx URL
failure, the service-health race, and the operator-controlled resolution. Keep the emergency sync
path and recovery artifacts.

**Verify:** `test -s docs/deploy/pi-github-actions-observation.md && rg -q 'silence-detector|relative URL|203/EXEC|restart' docs/deploy/pi-github-actions-observation.md && git diff --check`
