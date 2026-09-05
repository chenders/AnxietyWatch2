# Design — production safety hardening and monitoring evidence

## 1. Evidence model

Each measurement has two independent identities:

- source identity: `(source, type, session_id, seq)` from `vitals/registry.toml` and the approved
  source protocol;
- event time: the source's best estimate, preserved as UTC epoch time and never used as the key.

Each pipeline operation reports an explicit stage. The stages are monotonic for a particular row,
but they are not collapsed into one boolean:

| Stage | Meaning | Required evidence |
|---|---|---|
| received | a source decoder accepted an observation | source-specific decoder result/counter |
| spooled | `vitalsd` appended the encoded row | append receipt and spool byte/segment |
| fsynced | the spool bytes survived an explicit sync | sync receipt, when requested or periodic sync completed |
| drained | a drainer read the row from a sealed segment | segment name and offset |
| committed | PostgreSQL transaction committed the row or its idempotent duplicate | transaction counters and commit time |
| displayed | the read model rendered it | status revision and display timestamp |

A row is never described as durable merely because it is received or visible in a live status file.
A duplicate replay is counted as `duplicate`, not silently counted as a new measurement.

## 2. Spool and drainer

The COPY-binary spool remains the only local durable buffer. `vitalsd` is the single writer. It
publishes a fully initialized segment atomically, syncs data according to the caller's durability
contract, and rotates frequently enough that the active writer segment is not a history boundary.
The initial production bound is one second; it is a latency bound, not a permanent live/history
product split.

The drainer source is a workspace member and is deployed from the same build artifact tested in CI.
It holds one PostgreSQL advisory lock, validates the live schema fingerprint, processes segments in
lexical creation order, loads each segment into a private temporary staging table, and commits the
sample insert plus spool checkpoint in one transaction. It advances no checkpoint on COPY,
PostgreSQL, or local persistence failure.

The drainer emits structured health atomically into the spool directory. The health record contains
schema version, generated-at time, last successful commit time, pending segment count/bytes/oldest
age, rows read, inserted, duplicate, rejected, and a named last error. The record is telemetry, not a
history store. Missing, malformed, stale, or contradictory health is `UNKNOWN`.

Active segments are handled conservatively: the writer rotates on the one-second bound, while the
drainer only removes a segment after it has an exclusive lock and a committed EOF checkpoint. A
locked active segment is reported as pending and never removed. A crash after PostgreSQL commit and
before local cleanup replays safely through the primary key and is reported as duplicate.

## 3. EMAY identity and freshness

The anonymous live `EB` frame has no stable counter, timestamp, or record id. It remains prohibited
from direct durable sample insertion. The collector instead:

1. decodes it for current bedside status and increments received/measurement/no-finger counters;
2. marks that live value `pending`/`not_committed` in the live status document;
3. periodically re-enters the device's confirmed stored-session transfer mode;
4. appends only `EC`/`ED` rows whose descriptor and device cursor provide stable identity;
5. leaves live mode only for the bounded transfer window and records the transfer outcome;
6. returns to live mode and marks stored rows committed only after the database-backed publisher sees
   them.

This is store-and-forward, not invented identity. The device's persistent stored cursor is the
recovery source. If the device does not retain a reading in storage, the system cannot claim it was
durably recorded; the page says so. The status document exposes the age of the latest committed
stored row, the age of the current live frame, the pending transfer state, and any transfer error
independently.

## 4. `vitalsd` health

Configured primary sources appear in health even before their first sample, as `NeverSeen`. Health
includes spool pending state and the last observed drainer health. A stale/missing drainer record,
nonzero pending backlog beyond the configured bound, append failure, or subscriber drop adds a
specific degraded reason containing the blocking value. The heartbeat cannot pulse when the
monitoring window is active and any required evidence is missing.

Errors at periodic sync, rotation, status publication, and drainer processing are never swallowed.
They are logged with operation, stage, path/segment, and value. The daemon continues only when doing
so preserves the stated durability contract; otherwise it rejects new appends and reports the
blocker.

## 5. Panel publisher and admin monitoring view

The bedside panel remains PostgreSQL-authoritative for its durable history. Recent spool/live state
is a separate explicitly labelled pending overlay and is never merged into the durable history
collection. History points retain `(session_id, seq)` identity in the projection so equal values at
different source cursors are not deduplicated by timestamp/value.

A read-only admin monitoring endpoint/page consumes the same status, drainer health, PostgreSQL
projection, and service evidence. It includes:

- current health per configured source with process state, link state, last live time, last committed
  time, and exact blocker;
- a pipeline ledger with received/spooled/fsynced/drained/committed/displayed counts and ages;
- current spool segments, pending rows/bytes, lag age, and last drain error;
- bounded historical charts/tables for the selected UTC window, with explicit gaps and no synthetic
  points;
- a recent event timeline and raw evidence identifiers;
- a fail-closed document banner when its feed is missing, stale, malformed, or internally
  inconsistent.

The page performs GET-only reads. It has no operator controls that could mutate monitoring state.
It is a diagnostic view, not an alarm path.

## 6. Validation framework

The repository runs, on every change:

1. format and clippy with denied warnings;
2. `cargo nextest` with a no-skip assertion;
3. `cargo llvm-cov` with `--fail-under-lines 95` for each core library package and the combined
   core set;
4. RustSec audit and `cargo deny` policy checks;
5. decoder fuzz targets, with a bounded smoke run in CI;
6. focused mutation tests for spool, protocol, state, and publisher logic;
7. Python/JavaScript tests for the publisher and admin page, including malformed/stale feed states;
8. Pi-only integration gates for BlueZ, systemd, PostgreSQL, spool draining, crash replay, and
   hardware continuity.

Coverage is a guard against unexecuted code, not proof of correctness. The acceptance suite also
asserts invariants and failure behavior, and the deployment runbook requires independent evidence at
each pipeline stage.
