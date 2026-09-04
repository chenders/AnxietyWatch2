# Proposal — durable EMAY SleepO2 ingestion

**Status:** Blocked pending an identity decision required by the accepted amendments.

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
- The accepted identity amendments prohibit ingesting a streaming, payload-anonymous, unconfirmed
  source under a host delivery sequence. Reconnect/replay would admit indistinguishable duplicates.

## Proposed resolution

First investigate whether a documented EMAY command, firmware mode, or alternate characteristic
provides a stable sensor record identity. If one exists, add the production collector through the
existing `vitalsd` → COPY spool → PostgreSQL path with no schema change.

If the device cannot provide stable identity, a deliberate new amendment is required before any
numeric EMAY values enter `samples`. That amendment must choose and specify one of:

- an explicitly at-least-once, non-idempotent EMAY observation contract with duplicate handling; or
- a separate append-only raw-arrival/event table, which conflicts with the user's no-second-history-
  store preference and therefore needs explicit approval.

## Non-goals until resolved

- Do not deploy a collector using `SessionMinter` plus host delivery `seq`.
- Do not backfill old journal lines into `samples` with invented identity.
- Do not add a parallel history file, database table, spool, active scan, BP change, or reboot.
- Do not interrupt the healthy H10/EMAY live operation.
