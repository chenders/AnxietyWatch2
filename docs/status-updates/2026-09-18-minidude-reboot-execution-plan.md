# Pi host reboot execution plan

**Status:** ready for power restoration
**Host:** `pi-host` / `pi-host`
**Prepared:** 2026-09-20

## My understanding of the instruction

You have unplugged `pi-host`. Before you reconnect power, I must have the local side ready so that
connection attempts begin immediately. Once SSH becomes available, I should autonomously continue
through the planned recovery work—diagnosing the reboot/watchdog/PostgreSQL/spool problem, preserving
evidence, making the verified backup, and applying the reversible prototype load controls—without
random pauses or waiting for you to direct each routine step.

I should continue until the planned work is complete, the host goes down again, or an operation is
actually destructive, unsafe, unavailable, or requires an explicit operator decision. A host failure
is not a reason to stop the plan permanently: I should resume observation/reconnection attempts.

The direct-to-Mac path is now prepared in `/tmp/pi-host-direct-recovery.sh`. It encrypts each
stream locally and explicitly records that WAL metadata is not a physical WAL archive; the
production pgBackRest physical-backup/restore gate remains separate.

You also asked that when you ask whether I understand, I state my understanding explicitly rather
than replying only with a short acknowledgement.

## Prepared before power restoration

- The OpenSpec is written at:
  - `openspec/changes/pi-prototype-storage-degraded-mode/proposal.md`
  - `openspec/changes/pi-prototype-storage-degraded-mode/design.md`
  - `openspec/changes/pi-prototype-storage-degraded-mode/tasks.md`
  - `openspec/changes/pi-prototype-storage-degraded-mode/traceability.md`
- The local watcher has been syntax-checked and started.
- Watcher log: `/tmp/pi-host-after-reboot-watcher.log`
- Watcher PID: recorded in `/tmp/pi-host-after-reboot-watcher.pid` (currently `60900`).
- Local evidence root: `~/pi-host-reboot-evidence/`
- SSH polling target: `chris@pi-host`, every two seconds.
- No compilation, migration, schema change, sample deletion, spool deletion, offset change, or
  watchdog change is part of the first connection action.

## First connection actions

When SSH responds, the watcher immediately:

1. Captures boot history, prior/current journal tails, failed units, uptime/memory, mounts/free
   space, temperature/throttle state, watchdog state, PostgreSQL activity/settings, and relevant
   service state.
2. Writes the restricted evidence locally and on the Pi.
3. Stops the known optional/read-heavy `panel-vitals-status.service`, the spool drainer, and all
   known ingest services after the first evidence capture. Incoming records are intentionally not a
   priority during recovery.
4. Leaves PostgreSQL running only long enough to capture and stream the recovery set; existing
   database and spool contents remain untouched until backup verification.

## Autonomous continuation after the first capture

I will continue in this order without waiting for another routine prompt:

1. Preserve and inspect all relevant previous-boot, kernel, NVMe, PostgreSQL, filesystem, thermal,
   power, and watchdog evidence.
2. Verify the actual service, schema, spool, and backup configuration; do not trust stale plans over
   observed state.
3. Create a fresh encrypted recovery set directly on this Mac when possible: stream PostgreSQL
   globals, a custom-format database dump, WAL/recovery-boundary evidence, and the pending spool
   segments over SSH. This avoids staging a backup archive on `pi-host`.
4. Verify checksums, archive readability, and the logical dump/spool read tests before
   relaxing ingest or materialization behavior.
5. Measure and isolate query amplification, connection churn, spool scans, drain pressure, memory,
   I/O, and CPU causes.
6. Keep all ingest paths stopped during recovery; no new records need to be retained while the
   database is being backed up and reduced.
7. Apply only reversible, observable prototype controls after the backup: optional read reduction,
   bounded database work, and the explicitly authorized source-specific cleanup path.
8. Measure whether a second NVMe would materially improve the failure mode. It will not be assumed to
   help merely because the database is large; cost, power, thermal, PCIe, endurance, and recovery
   implications will be included.
8. Preserve all planes/noise and CNS depression-warning data in the unfiltered recovery set.
9. Inventory those candidates after backup verification by actual provenance and dependencies. Only
   data proven exclusive to the spun-off project may receive a later cleanup decision; shared or
   uncertain data stays. `samples` remains append-only and cannot be deleted or updated.
11. Record any source-specific data reduction as an explicit operator decision, never as an implicit
   or silent discard policy.

## Deliberate stop conditions

These are not random pauses:

- `pi-host` is unreachable or reboots, in which case the watcher/reconnection plan continues;
- a backup destination or encryption/recovery prerequisite is missing;
- a command would delete data, alter `samples`, rewrite/skip spool offsets, repartition, modify
  `/boot/firmware`, apply `dtoverlay`, or restore destructively into a live database;
- an unresolved schema/migration contradiction requires an amendment or explicit operator decision;
- a proposed change could reduce safety-relevant measurements without an explicit named policy;
- a data-removal predicate cannot prove that the records are exclusively planes/noise or CNS-only.

## Ready instruction

The local watcher and evidence path are in place, and the watcher is actively polling SSH. **You can plug `pi-host` in now.**
