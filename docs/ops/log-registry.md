# Operational log registry

**Updated:** 2026-09-05 11:24 PDT  
**Audit scope:** T-S2 (`vitals-core`, `vitalsd`, `vitals-proto`, `vitals-drain`, and the
`spool_status` schema)

This is the inventory of operational evidence, not a claim that a running process is healthy.
Every row must identify the producer, the event, the durable location, the retention/reader, and
what remains unknown. A missing or stale record is `UNKNOWN`, never `OK`.

## Stage vocabulary

The stages are intentionally separate:

| Stage | Meaning | Minimum fields |
|---|---|---|
| `received` | `vitalsd` accepted a request from a collector | source, type, session, seq range, count, request id, received time |
| `spooled` | bytes were appended to a named segment | segment, offset/byte range, count, byte count, result/error |
| `fsynced` | the relevant spool bytes reached stable storage | segment, durable offset, count, sync reason (`explicit`/`periodic`), result/error |
| `drained` | the drainer read a segment suffix and loaded its private staging table | segment, start/end offset, rows read, framing/fingerprint, result/error |
| `committed` | the PostgreSQL transaction committed and `spool_status` advanced | segment, committed offset, inserted, duplicate, rejected, commit time |
| `published` | `vitalsd` enqueued a sample for a subscriber | subscriber id, source/type/session/seq, queue depth, result |
| `displayed` | a UI renderer consumed and rendered a value | feed generation, durable identity, display time, pending/durable state |

`published` is not `displayed`; a socket write or queue admission cannot prove that a panel or
browser rendered the value.

## Registry

| Producer / owner | Evidence required | Current evidence at this audit | Gap / required action | Location / reader |
|---|---|---|---|---|
| `vitals-core::Spool` / vitalsd | append result, segment name, byte growth, explicit and periodic fsync result, rotation result | `append` returns `Result`; `sync=true` is represented in the reply; the local implementation now emits stage/error events and exposes whether unsynced bytes existed | Deployed Pi still runs the pre-telemetry artifact; stage counters and journal verification remain a deployment gate | journald via vitalsd; `vitalsd` health; drainer snapshot |
| `vitalsd` ingest boundary | received → spooled → fsynced ledger; malformed-frame and client disconnect reason; bounded subscriber drops | Local implementation emits `event=vitalsd.*` records with client/request ids, stage/result, and named details; it reads `.drain-health.json` and logs blocker transitions | Deployed Pi still emits the old unstructured lines and has no health snapshot; subscriber `published` is still not UI `displayed` | journald; Unix-socket `Health` response |
| `vitals-proto` | machine-readable error class and enough context for caller logging | `ErrorKind` and error detail exist; malformed CBOR/frame errors are returned | Pure wire crate must not log; every caller must log the returned class, peer id, and frame/request context | caller journal; wire `Health`/`Error` response |
| `vitals-spool-drain` | startup/lock/schema validation, pending bytes/age, per-segment drained/committed result, inserted/duplicate/rejected counters, last successful commit, last named error | Local implementation publishes an atomically replaced `.drain-health.json`, reports active-segment pending bytes, and emits `event=drainer.*` records; tests cover JSON publication and active-segment inspection | Deployed Pi still runs the hourly pre-telemetry artifact and has no `.drain-health.json`; exact artifact deployment and crash/outage checks remain open | `/var/lib/vitals/spool/.drain-health.json`; journald; vitalsd health reader |
| `spool_status` / PostgreSQL owner | transactional segment offset and fingerprint; commit evidence must agree with local offset | `spool_status` has `segment`, `offset_bytes`, `drained`, `fingerprint`, `last_advanced`; unique primary key is present | No explicit database-side stage/audit event for a failed transaction; drainer must expose commit counters and named DB errors without advancing status | PostgreSQL `spool_status`; drainer snapshot |
| active segment | pending rows must be visible even while the writer holds its lock | the drainer skips a locked active segment during normal processing | Pending calculation must include the active segment without opening it; an old/locked segment must not be reported as empty | drainer snapshot; vitalsd health |
| schema/fingerprint guard | migration mismatch must name expected/actual fingerprint and first changed field | `SchemaFingerprint`, live layout query, and named mismatch errors exist | Ensure every mismatch is emitted to journal and health; no silent retry that looks healthy | drainer journal and snapshot; sqitch operator output |
| subscriber / panel boundary | queue admission and drops must be distinct from UI display | bounded `try_send` and per-subscriber drop counters exist | `vitalsd` can prove only `published`; panel/browser `displayed` evidence is outside T-S2 and must be registered in T-S4 | vitalsd health; panel/admin feed ledger |

## Out-of-scope rows that must not be marked healthy

The following are required for the complete production registry but are not in T-S2's `Reads:` list.
They remain explicitly unaudited until their owning task permits reading and changing them:

- EMAY live/stored-session collector: raw-frame, connection, decode, identity, synchronization,
  append-reply, and `sync=true` evidence (T-S3).
- H10 collector and CNS detector: source timestamp/sequence, gap, evaluation, replay/warm-up, and
  subscriber-backpressure evidence (T-S3/T-S5).
- BlueZ/kernel/systemd: service-resolution, bonded-but-unencrypted, restart/OOM, and link-state
  evidence (T-S5).
- PostgreSQL server configuration and logs: connection, lock, checkpoint, statement/error, and
  backup/restore evidence (T-S5).
- Panel publisher/merger/HTTP/kiosk and admin console: feed generation, stale/malformed input,
  durable-versus-pending rendering, bounded history, and actual display evidence (T-S4/T-S5).
- CI/deployment tools: immutable build identity, command exit status, artifact digest, deployment
  target, and rollback evidence (T-S5).

An entry in this section is a blocker, not a permission to infer health from `systemctl is-active`,
a green page, or a fresh process log.

## Registry maintenance rule

When a producer, filename, JSON schema, event name, retention policy, or reader changes, update this
file in the same commit and add a Pacific-time status update recording the verification command and
exit code. If a source cannot produce an event, record `UNKNOWN` and the named blocker rather than
inventing a substitute timestamp or sequence.
