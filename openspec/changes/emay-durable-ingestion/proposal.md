# Proposal — durable EMAY SleepO2 ingestion

**Status:** The device-storage identity decision is complete; production collector implementation is pending.

## Why

The registry defines EMAY (`VitalsSource::Emay`, source id 0) and the Pi receives valid SleepO2
frames continuously, but the deployed `emay-spike` writes only a current JSON status object and
journal lines. It never publishes samples to `vitalsd`, the COPY spool, or PostgreSQL. The bedside
graph therefore loses EMAY history on reload and the database has no EMAY rows.

## What has been established

- No local branch or worktree contains a durable EMAY collector.
- The verified real-time EMAY payload is `EB 01 05 <pulse> <SpO2> 7F 00 <checksum>`: it carries no
  sensor counter, timestamp, or stable record identifier.
- BlueZ reports the data characteristic `0xFF02` as `read`, `notify`, not `indicate`; there is no
  ATT confirmation boundary that could make host delivery order a safe identity.
- The accepted identity amendments prohibit ingesting the streaming, payload-anonymous, unconfirmed
  `EB` source under a host delivery sequence. Reconnect/replay would admit indistinguishable duplicates.
- The official app's storage mode returns `EC` session descriptors and 24-byte `ED` records. Each
  `ED` record carries a metric selector, storage slot, and device-provided 16-bit cursor; `EC`
  carries the device recording start. All 12,810 captured `ED` records passed their checksum, and
  both metric streams echoed the same cursor sets.

## Proposed resolution

Use the official storage mode as the durable source. The collector must download the device's
stored sessions, decode the `EC`/`ED` records, derive `session_id` from the complete session
descriptor plus slot, and use the device's `ED` cursor for `seq`. It then sends the resulting
SpO₂ and pulse rows through the existing `vitalsd` → COPY spool → PostgreSQL path with no schema
change. The live `EB` stream remains available for current display and health evidence, but its
anonymous frames must not be assigned a host sequence merely to make them durable.

The stored cursor is source-provided, so no new at-least-once exception is needed for stored
records. The collector must still make partial downloads retryable and must not claim exactly-once
transport semantics. If a later test shows the cursor is not stable when a session is unchanged,
stop numeric ingest and create a deliberate amendment rather than silently changing the key.

## Non-goals until resolved

- Do not deploy a collector using `SessionMinter` plus host delivery `seq`.
- Do not backfill old journal lines into `samples` with invented identity.
- Do not add a parallel history file, database table, spool, active scan, BP change, or reboot.
- Do not interrupt the healthy H10/EMAY live operation.
