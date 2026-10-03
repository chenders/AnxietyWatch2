# Tasks: Stable spool identity for activation

- [x] T-SPOOL-1 — Implement content-based spool fingerprint and test the restart boundary · implement-complex · host
- [x] T-SPOOL-2 — Install the helper, recreate the recovery gate, and run the gated deployment · human · Pi

## T-SPOOL-1 — Implement content-based spool fingerprint and test the restart boundary

**Depends:** none · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `proposal.md`, `design.md`,
`crates/vitals-deploy/src/main.rs`, `openspec/changes/github-runner-activation-boundary/specs/github-runner-activation-boundary/spec.md`.

**Modifies:** `crates/vitals-deploy/src/main.rs`.

**Do:** Exclude time-derived segment filenames from the spool identity while retaining file count,
total bytes, per-file size, and per-file content. Add a test proving a rename is stable and content
changes are rejected. Do not weaken the preflight backup-gate comparison.

**Verify:** `cargo test -p vitals-deploy && cargo clippy -p vitals-deploy --all-targets -- -D warnings && cargo fmt --all -- --check && openspec validate github-runner-spool-rotation-acceptance --type change --strict --no-interactive && git diff --check`

## T-SPOOL-2 — Install the helper, recreate the recovery gate, and run the gated deployment

**Depends:** T-SPOOL-1 · **Review required:** yes

**Reads:** `proposal.md`, `design.md`, `tasks.md`, `deploy/github-runner/README.md`,
`openspec/changes/pi-github-actions-deployment/tasks.md` T-R6/T-R7, and the current failure receipt.

**Do:** After review and merge, install the reviewed `vitals-deploy` helper through the protected
host procedure, stop only the approved spool-seal activity, recreate and checksum an encrypted,
restore-verified, off-Pi recovery set using the reviewed fingerprint, dispatch the exact main
workflow with activation approval, and restart the spool-seal timer after the result. If any gate
fails, leave activation failed closed and retain the receipt; do not bypass the spool comparison.

**Verify:** `ssh "$PI_HOST" 'set -e; sudo jq -e ".activated == true and .source_sha != null and .preflight_ok == true and .health_ok == true and .spool_unchanged == true and .backup_gate == true and .rollback_command != null" /var/lib/vitals/recovery/github-deploy-latest.json >/dev/null; systemctl is-active --quiet vitalsd; sudo test -S /run/vitals/vitalsd.sock'`
