# Tasks — Pi prototype storage degraded mode

Every task is gated by the previous task's shell exit code. Do not tick a task from a worker report.
Run its `Verify:` command directly. This change documents and implements a temporary operating mode;
it does not authorize destructive cleanup or unresolved migrations.

- [ ] **T-P0 — Record baseline and recovery contract**
- [ ] **T-P1 — Create and verify the off-box recovery set**
- [ ] **T-P2 — Enable reversible low-load prototype profile**
- [ ] **T-P3 — Bound drain, query, and resource pressure**
- [ ] **T-P4 — Measure the second-NVMe decision**
- [ ] **T-P5a — Inventory and isolate separate-project data**
- [ ] **T-P5 — Approve any source-specific data reduction**
- [ ] **T-P6 — Return to production profile and verify acceptance**

## T-P0 — Record baseline and recovery contract

**Tier:** implement-standard  
**Host:** pi  
**Context budget:** ~8k tokens  
**Depends:** none  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `openspec/changes/pi5-vitals-core/proposal.md`,
`openspec/changes/pi5-vitals-core/RUNBOOK.md`, `openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/design.md`

**Creates:** `/var/lib/vitals/recovery/baseline-*.json`

**Do:**
1. Record boot history, kernel/journal availability, systemd state, watchdog state, thermal and
   throttle state, filesystem mounts/free space, PostgreSQL readiness/activity, spool count/bytes,
   and active service/resource profiles.
2. Stop optional panel/read workloads before sustained database inspection.
3. Record exact service names and the current spool identity/offset contract.
4. Do not mutate samples, spool contents, offsets, schema, or watchdog settings.

**Verify:** `ssh "$PI_HOST" 'test -s /var/lib/vitals/recovery/baseline-latest.json && jq -e ".host and .services and .spool and .postgres" /var/lib/vitals/recovery/baseline-latest.json >/dev/null'`

**Expect:** exit 0; the baseline names host, services, spool, and PostgreSQL evidence.

## T-P1 — Create and verify the off-box recovery set

**Tier:** implement-complex  
**Host:** either  
**Context budget:** ~10k tokens  
**Depends:** T-P0 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/design.md`,
`deploy/restore-verification/README.md`, `deploy/restore-verification/verify-restore.sh`

**Creates:** `~/minidude-reboot-evidence/latest/recovery/`, an off-box encrypted recovery set

**Do:**
1. Prefer the direct-to-Mac path while Pi free space is at risk: stream PostgreSQL globals, a
   custom-format logical dump, WAL/recovery-boundary evidence, and all pending spool segments over
   SSH. Do not stage the backup archive on the Pi.
2. Encrypt each stream on the Mac and record checksums. Keep the passphrase mode 0600 and outside
   source control.
3. Do not delete, concatenate, rewrite, or advance anything on the Pi.
4. Verify checksum, archive readability, and spool archive readability. Record that this logical
   recovery set is not a physical pgBackRest restore; the physical production gate remains open.

**Verify:** `test -s "$HOME/minidude-reboot-evidence/latest/recovery/database.gpg" && test -s "$HOME/minidude-reboot-evidence/latest/recovery/spool.gpg" && test -s "$HOME/minidude-reboot-evidence/latest/recovery/wal-evidence.gpg" && grep -q 'failures=0' "$HOME/minidude-reboot-evidence/latest/recovery/direct-recovery.log"`

**Expect:** exit 0; the complete logical database, WAL evidence, and pending spool are encrypted and stored off Pi.

## T-P2 — Enable reversible low-load prototype profile

**Tier:** implement-standard  
**Host:** pi  
**Context budget:** ~8k tokens  
**Depends:** T-P1 green  
**Review required:** yes

**Reads:** `openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/design.md`,
`deploy/stopgap/panel-vitals-status.service`, `deploy/vitals-spool-drain.service`

**Creates:** `/etc/vitals/profiles/prototype-degraded.env`, `/var/lib/vitals/recovery/state.json`

**Do:**
1. Stop optional dashboard/read services and install an explicit, root-owned degraded profile.
2. Set a conservative panel interval or disable it; make the active profile visible in status.
3. Keep safety-relevant collection and append-only spool durability unchanged.
4. Make rollback one explicit command and record the backup identifier and operator reason.

**Verify:** `ssh "$PI_HOST" 'jq -e ".state == \"prototype_degraded\" and .backup_id != null and .rollback_command != null" /var/lib/vitals/recovery/state.json >/dev/null && ! systemctl is-active --quiet panel-vitals-status.service'`

**Expect:** exit 0; degraded mode is explicit, tied to a recovery set, reversible, and panel polling is inactive.

## T-P3 — Bound drain, query, and resource pressure

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~12k tokens  
**Depends:** T-P2 green  
**Review required:** yes

**Reads:** `crates/vitals-drain/src/lib.rs`, `crates/vitalsd/src/main.rs`,
`crates/vitals-console/src/api/postgres.rs`, `tools/panel_vitals_status.py`,
`deploy/vitals-spool-drain.service`, `openspec/changes/pi-prototype-storage-degraded-mode/design.md`

**Creates:** bounded service/profile configuration and load evidence

**Do:**
1. Bound drainer batches, concurrency, connection use, and duty cycle without changing identity or
   offset semantics.
2. Remove or isolate full-spool scans from the critical event loop; expose segment age/bytes/count
   without rescanning all files per health tick.
3. Reject unrestricted raw dashboard queries or replace them with bounded/precomputed reads.
4. Measure PostgreSQL CPU, active queries, locks, I/O wait, spool throughput, and backlog before and
   after the profile.

**Verify:** `ssh "$PI_HOST" 'jq -e ".ok == true and .unbounded_raw_queries == false and .spool_backlog_measured == true and .drain_progress_measured == true" /var/lib/vitals/recovery/load-control-latest.json >/dev/null'`

**Expect:** exit 0; resource controls are active and both backlog and drain progress are measured.

## T-P4 — Measure the second-NVMe decision

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~10k tokens  
**Depends:** T-P3 green  
**Review required:** yes

**Reads:** `openspec/changes/pi-prototype-storage-degraded-mode/design.md`,
`openspec/changes/pi5-vitals-core/RUNBOOK.md`, `docs/specs/AMENDMENTS.md`

**Creates:** `/var/lib/vitals/recovery/storage-topology-decision.json`

**Do:**
1. Measure database/WAL latency, spool fsync/append latency, queue depth, I/O wait, free space,
   temperature, power/throttle state, and drain throughput under the bounded profile.
2. Compare the current layout against at least two second-device layouts using measured or clearly
   labelled estimated cost/power/thermal values.
3. Separate storage contention from CPU, memory, query, connection, and filesystem-scan causes.
4. Record `buy`, `defer`, or `insufficient_evidence`, with the measured blocker and rollback/recovery
   implications. Do not purchase or repartition as part of this task.

**Verify:** `ssh "$PI_HOST" 'jq -e ".decision | IN(\"buy\",\"defer\",\"insufficient_evidence\") and .measurements.storage_contention != null and .measurements.power_thermal != null and .alternatives | length >= 2" /var/lib/vitals/recovery/storage-topology-decision.json >/dev/null'`

**Expect:** exit 0; the decision is evidence-based and includes at least two alternatives.

## T-P5a — Inventory and isolate separate-project data

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~10k tokens  
**Depends:** T-P1 green  
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/design.md`,
`db/sqitch.plan`, `db/deploy`, `db/verify`, `vitals/registry.toml`

**Creates:** `/var/lib/vitals/recovery/separate-project-data-inventory.json`,
`/var/lib/vitals/recovery/separate-project-cleanup-manifest.json`

**Do:**
1. Inventory all tables, columns, foreign keys, row counts, provenance/source/type identities,
   timestamps, and code/schema references related to planes/noise and CNS depression warnings.
2. Include those records in the already-created recovery set; do not create a filtered backup.
3. Classify every candidate as `project_exclusive`, `shared`, or `uncertain`. Table names alone do
   not establish exclusivity.
4. For exclusive non-`samples` records, describe the separate-project export and exact cleanup
   predicate without executing destructive cleanup in this task.
5. For `samples` candidates, record the approved tombstone path; never issue `DELETE` or `UPDATE`
   against `samples`. Retain shared and uncertain data.

**Verify:** `ssh "$PI_HOST" 'jq -e ".ok == true and (.candidates | type) == \"array\" and (.candidates | all(.classification == \"project_exclusive\" or .classification == \"shared\" or .classification == \"uncertain\")) and .samples_delete_attempted == false" /var/lib/vitals/recovery/separate-project-data-inventory.json >/dev/null'`

**Expect:** exit 0; the inventory proves the backup scope and classifies candidates without deleting data.

## T-P5 — Approve any source-specific data reduction

**Tier:** human  
**Host:** pi  
**Context budget:** ~6k tokens  
**Depends:** T-P3 green and T-P4 green  
**Review required:** yes

**Reads:** `vitals/registry.toml`, `docs/specs/AMENDMENTS.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/design.md`

**Creates:** `/var/lib/vitals/recovery/data-reduction-decision.json`

**Do:**
1. The operator names each affected source/type, the reason, expected pending/latency behavior,
   whether raw recovery remains available, and the rollback command.
2. Keep BP, EMAY, and safety-relevant classes lossless unless a new accepted amendment explicitly
   changes that rule.
3. If no reduction is needed, record `none` rather than inventing a policy.

**Verify:** `ssh "$PI_HOST" 'jq -e ".operator_approved == true and (.affected | type) == \"array\" and .rollback_command != null" /var/lib/vitals/recovery/data-reduction-decision.json >/dev/null'`

**Expect:** exit 0; an operator—not an implicit default—approved the exact scope or approved no reduction.

## T-P6 — Return to production profile and verify acceptance

**Tier:** implement-complex  
**Host:** pi  
**Context budget:** ~10k tokens  
**Depends:** T-P4 green, T-P5a green, and T-P5 green  
**Review required:** yes

**Reads:** `openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`,
`openspec/changes/pi-prototype-storage-degraded-mode/design.md`,
`docs/specs/AMENDMENTS.md`, `deploy/restore-verification/verify-restore.sh`

**Creates:** `/var/lib/vitals/recovery/production-acceptance.json`

**Do:**
1. Disable the prototype profile and enable the last verified production profile.
2. Verify watchdog, resource priorities, backup/restore status, schema/ledger state, drain progress,
   and absence of unexplained spool gaps.
3. Leave unresolved schema drift blocked; do not mark production accepted because the host merely
   rebooted or because a backup file exists.
4. Record remaining blockers and the selected storage topology.

**Verify:** `ssh "$PI_HOST" 'jq -e ".state == \"production\" and .backup_verified == true and .restore_verified == true and .schema_reconciled == true and .spool_gaps_explained == true" /var/lib/vitals/recovery/production-acceptance.json >/dev/null'`

**Expect:** exit 0; production acceptance is explicit and all required recovery/load/schema gates are green.
