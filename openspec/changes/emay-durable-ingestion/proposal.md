# Proposal — durable EMAY SleepO2 ingestion

**Status:** Device-storage identity, durable collector implementation, panel history, and T-E4 activation verification are complete.

## Why

The registry defines EMAY (`VitalsSource::Emay`, source id 0). Before this change, the deployed
`emay-spike` wrote only a current JSON status object and journal lines, so it never published
samples to `vitalsd`, the COPY spool, or PostgreSQL. The bedside graph therefore lost EMAY history
on reload and the database had no EMAY rows. The approved durable collector now uses device storage
for numeric rows and keeps anonymous live frames display-only.

## What has been established

- The production durable collector is `crates/emay-collector/` and is deployed as
  `vitals-emay.service` on pi-host.
- The verified real-time EMAY measurement payload is `EB 01 05 <pulse> <SpO2> 7F 00 <checksum>`;
  it carries no sensor counter, timestamp, or stable record identifier. The device also emits the
  exact all-sentinel `EB 01 07 7F 7F 7F 00 70` no-reading transition, which is link evidence only.
- BlueZ reports the data characteristic `0xFF02` as `read`, `notify`, not `indicate`; there is no
  ATT confirmation boundary that could make host delivery order a safe identity.
- The accepted identity amendments prohibit ingesting the streaming, payload-anonymous, unconfirmed
  `EB` source under a host delivery sequence. Reconnect/replay would admit indistinguishable duplicates.
- The official app's storage mode returns `EC` session descriptors and 24-byte `ED` records. Each
  `ED` record carries a metric selector, storage slot, and device-provided 16-bit cursor; `EC`
  carries the device recording start. All 12,810 captured `ED` records passed their checksum, and
  both metric streams echoed the same cursor sets.

## Proposed resolution

The official storage mode is the durable source. The collector downloads the device's
stored sessions, decodes the `EC`/`ED` records, derives `session_id` from the complete session
descriptor plus slot, and uses the device's `ED` cursor for `seq`. It then sends the resulting
SpO₂ and pulse rows through the existing `vitalsd` → COPY spool → PostgreSQL path with no schema
change. The live `EB` stream remains available for current display and health evidence, but its
anonymous frames must not be assigned a host sequence merely to make them durable.

The stored cursor is source-provided, so no new at-least-once exception is needed for stored
records. The collector must still make partial downloads retryable and must not claim exactly-once
transport semantics. If a later test shows the cursor is not stable when a session is unchanged,
stop numeric ingest and create a deliberate amendment rather than silently changing the key.

## Non-goals

- Do not deploy a collector using `SessionMinter` plus host delivery `seq`.
- Do not backfill old journal lines into `samples` with invented identity.
- Do not add a parallel history file, database table, spool, active scan, BP change, or reboot.
- Do not interrupt the healthy H10/EMAY live operation.
