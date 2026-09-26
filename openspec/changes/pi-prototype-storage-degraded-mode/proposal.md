# Change: pi-prototype-storage-degraded-mode — reversible load reduction with a production return path

## Why

The prototype Pi has demonstrated host instability while PostgreSQL, dashboard reads, spool
scanning, and draining are active together. Continuing to collect evidence requires a temporary
resource-constrained mode, but changing production safeguards without a recovery boundary could
silently lose physiological data or make the eventual production architecture ambiguous.

This change records the exact transition, the evidence that must exist before it, the reversible
prototype limits, and the production design we intend to return to. It also makes a second NVMe a
measured cost/stability decision rather than an assumed cure.

## What changes

- Establish a named **production baseline** and a separately named **prototype degraded mode**.
- Produce and verify an encrypted, off-Pi recovery set before changing ingest or drain policy;
  when Pi free space is constrained, stream the logical recovery artifacts directly to the Mac rather
  than staging a backup archive on `minidude`.
- Preserve raw spool segments and append-only evidence while reducing optional read and write load.
- Add bounded, observable PostgreSQL/drainer/resource controls; no silent discard or manual offset
  advancement is permitted.
- Define the decision evidence for purchasing and deploying a second NVMe device.
- Preserve all database data for the separate planes/noise project and CNS depression-warning project
  in the recovery set, then inventory and isolate only data proven to belong exclusively to those
  projects before any authorized cleanup.
- Define the gates and tests required to return to production behavior.

## Production end goal

The production system remains lossless for safety-relevant measurements, uses PostgreSQL/TimescaleDB
with the accepted identity and migration contracts, keeps the COPY spool as a durable recovery
boundary, and isolates safety-path processes from database, dashboard, backup, and analysis load.
It has:

- verified off-box encrypted backups and restore tests;
- bounded connection, query, drain, memory, disk, and WAL resources;
- no O(spool-files) health scan on the daemon's critical event loop;
- batched, backpressured drain behavior with measurable progress;
- precomputed/limited dashboard reads rather than unrestricted raw scans;
- explicit schema/Sqitch/Timescale reconciliation; and
- a storage topology selected from measurements, which may include a second NVMe for backup,
  spool, or database isolation if its benefit justifies cost, power, thermal, and maintenance risk.

## Prototype degraded mode

Degraded mode is temporary and must be visible in health output. It may:

- stop or greatly reduce optional dashboard/database polling;
- cap drainer concurrency, batch size, and duty cycle;
- defer non-safety writes while retaining their authenticated raw records in the spool;
- reduce or pause explicitly classified non-safety/high-rate database materialization only after the
  recovery set is verified and the operator accepts the affected data class;
- preserve BP, EMAY, and other safety-relevant measurements losslessly.

It must not:

- delete, concatenate, rewrite, or manually skip spool segments;
- manually advance offsets;
- update or delete rows from `samples`;
- coerce, clamp, or silently discard physiological measurements;
- run unresolved migrations; or
- disable watchdog protection as a load-control measure.

## Scope

### In scope

Backup and recovery evidence, safe host load reduction, spool/drainer protection, dashboard query
isolation, storage-topology measurement, explicit prototype operating limits, and production-return
acceptance criteria.

### Out of scope

Buying hardware without the decision gate, changing the samples identity/schema, resolving the
Timescale/Sqitch contradiction by workaround, or claiming clinical production readiness. Cleanup of
candidate planes/noise or CNS-only data is a later gated operation, not an assumption made from a
table name or a broad query.

## Hard gates

1. No degraded ingest/materialization policy is enabled until an encrypted recovery set is copied
   off `minidude`, checksummed, and its restore/readability is verified.
2. The recovery set includes pending spool data; a PostgreSQL backup alone is insufficient.
3. No safety-relevant source is reduced without an explicit operator decision naming the source,
   reason, expected loss/latency behavior, and rollback command.
4. A second NVMe is purchased or installed only after the topology decision task reports measured
   contention and a cost/power/thermal comparison.
5. Production mode is not declared restored until every required Verify command in this change exits
   0 and the schema/migration state is reconciled deliberately.

## Non-goals

This change does not make APNs an outage monitor, replace email/independent host monitoring, or
make a backup trusted merely because a file was copied. A backup is evidence only after checksum
and restore/readability verification.
