# Tasks — silence-detector runtime and deployment health

Acceptance is each `Verify:` command's exit code. Do not tick a box before running its command.

**Amended by:** `silence-detector-live-activation` — its T-LA-5 performed the operator-gated
runtime acceptance this change deferred. That change's header already points here; this is the
reciprocal pointer, so a worker correctly scoped to this change can see it. Without it, the
superseding decision is invisible to anyone obeying the `Reads:` rule — the failure mode
`AGENTS.md`'s amendment section exists to prevent.

- [x] **T-SD-1 — Correct the SQLx URL contract and artifact modes** · implement-standard
- [x] **T-SD-2 — Require stable structured service health** · implement-complex
- [x] **T-SD-5 — Restore downloaded nonbinary artifact modes** · implement-standard
- [x] **T-SD-6 — Escape SQLx URL percent signs for systemd** · implement-standard
- [x] **T-SD-7 — Record stable per-service activation evidence** · implement-complex
- [ ] **T-SD-3 — Verify the corrected daemon on the Pi before enabling it** · human · Pi · blocked: superseded gate, see note
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

**Not measured — and do not tick this on `silence-detector-live-activation` T-LA-5's
evidence.** The runtime acceptance described under **Do:** was performed on 2026-10-04 under
T-LA-5: three 30-second cycles, 8 persisted status observations, 8 notification events,
`NRestarts=0` unchanged, `enabled`, `active/running`, `Result=success`, `ExecMainStatus=0`,
receipt `/var/lib/vitals/recovery/silence-detector-runtime-latest.json`, detector built from
`8b169e3`.

That is **not this task's Verify command.** T-LA-5 asserts on
`silence-detector-runtime-latest.json`; this command additionally gates on
`github-deploy-latest.json`, the GitHub Actions activation receipt. They are different
artifacts, so the two are not the same verification.

Worse, re-running this command today would pass for the wrong reason.
`docs/deploy/pi-github-actions-observation.md` records `github-deploy-latest.json` for release
`bbc4cf8` as `activated=true, health_ok=true` — and the same document states that green receipt
"did not contain the new per-service `services` evidence: the workflow invoked the root-owned
helper installed before that release, so the strengthened helper was not actually the live
activation controller." The live detector was built from `8b169e3` and installed by the
operator-gated path. `rollback_completed` is a real field on `ActivationReceipt`
(`crates/vitals-deploy/src/main.rs`), so the `jq` does not fail for a missing key — it
silently succeeds against a receipt describing a superseded release while `is-active` reports
the newer binary. That scores *not measured by this workflow* as *measured and good*, which is
what "Unknown is not OK" forbids.

**To close it honestly:** run one more GitHub Actions activation of current `main` through the
refreshed root helper (`vitals-deploy-helper-latest.json`, verified 2026-10-04). That rewrites
`github-deploy-latest.json` with per-service evidence for the live detector, after which this
command becomes a true gate and whoever ran it may tick the box. Requires Pi access and an
activation window.

## T-SD-4 — Record the incident and final acceptance evidence

**Do:** Correct the deployment observation document with the executable-mode failure, the SQLx URL
failure, the service-health race, and the operator-controlled resolution. Keep the emergency sync
path and recovery artifacts.

**Verify:** `test -s docs/deploy/pi-github-actions-observation.md && rg -q 'silence-detector|relative URL|203/EXEC|restart' docs/deploy/pi-github-actions-observation.md && git diff --check`
