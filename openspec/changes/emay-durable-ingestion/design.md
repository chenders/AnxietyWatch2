# Design — durable EMAY SleepO2 ingestion

**Status:** Stored-session identity established; production collector implementation pending.

## Existing path

The Pi already runs `vitalsd` as the single writer. A collector sends length-prefixed CBOR
`Request::Append` frames over `/run/vitals/vitalsd.sock`; `vitalsd` converts the Unix-microsecond
wire time into `SystemTime`, appends PostgreSQL binary-COPY rows to its spool, and
`vitals-spool-drain` commits them to the existing `samples` hypertable. The schema primary key is
`(source, type, session_id, seq)`. The panel's existing `panel-vitals-status` publisher reads
PostgreSQL and emits the `vitals_status.json`/`vitals_h10_history.json` projections.

The missing producer is now the remaining implementation: the official device-storage mode provides
the source identity needed for safe durable rows, while the anonymous live stream remains
presentation/journal-only.

## Identity finding

The verified live data notification is exactly:

```
EB 01 05 <pulse> <SpO2> 7F 00 <checksum>
```

The live payload has no counter, device timestamp, or stable record identifier. The connected BlueZ
characteristic `0xFF02` reports `read`, `notify`; it is not an indication. A notification is not an
ATT-confirmed store-and-forward delivery. Repeated live readings are byte-identical to a
retransmission, and a reconnect starts with no sensor-provided continuity marker. The live path
therefore remains display/journal-only until a different accepted identity is available.

the storage slot and device recording start (`YY MM DD hh mm ss`). Its `ED` records are 24-byte
checksummed records with this layout:

```
ED 01 <metric> 01 <slot> <cursor-lo> <cursor-hi> <payload[16]> <checksum>
```

Metric `01` is SpO₂ and metric `02` is pulse; the value is at absolute byte 9 and `0x7f` is the
stored no-reading sentinel. The 16-bit cursor is source data. The capture returned 12,810 records,
all checksum-valid, across seven session slots; paired metric streams echoed identical cursors in
order. The cursor's reserved page bit causes the observed jumps from `0x007f` to `0x0100`; that is
an encoding detail, not a lost-row count.

The durable identity for stored records is therefore the complete session descriptor plus slot for
`session_id`, and the `ED` cursor for `seq`. Both values come from the device and remain unchanged
when an unchanged stored record is downloaded again. The collector must use a deterministic session
encoding so a reconnect or re-download cannot mint a new key for the same device session. The
session start plus the protocol's documented approximately-1-Hz sample ordinal supplies event time;
convert the device's local clock to UTC once at the decoder and retain the raw descriptor metadata.

This does not make the anonymous live `EB` frame durable. A collector-local counter or a timestamp
based on arrival is still forbidden for live numeric rows.

## Collector resolution

T-E1's evidence work is complete for stored-session ingestion. The production collector should:

1. Keep one known-device connection and use the existing FF12/FF01/FF02 transport.
2. Query the device's session descriptors, then download each slot's metric records with the
   observed `9d` commands. Reassemble notifications before parsing; a partial transfer must be
   retryable.
3. Validate every `EC`/`ED` frame and reject/quarantine malformed or out-of-range values. Preserve
   the `0x7f` no-reading state as absent.
4. Derive a deterministic `session_id` from the complete descriptor and slot, use the device
   cursor for `seq`, and derive event time from the device session start plus the documented sample
   ordinal. Store UTC epoch seconds only.
5. Send rows through `vitalsd` → COPY spool → PostgreSQL and make the existing panel projection read
   them. The live `EB` stream may continue to update the bedside display and journal, but it must not
   be assigned a host identity merely to enter `samples`.

No new identity amendment is required for the stored records. This plan does not claim exactly-once
BLE transport; it relies on source-provided keys for idempotent re-downloads and surfaces partial
transfers. If implementation or a later unchanged-session comparison disproves cursor stability,
stop numeric stored ingest and document the contradiction before changing the key.

## Historical data already collected

The old stopgap's journal lines are useful evidence, but they were not assigned durable session and
sequence identity. They must not be bulk-inserted into `samples` by assigning a new session/counter;
that would make their identity appear stronger than the source can support. They can remain in the
journal until an approved replay identity is defined.

## Safe state while implementation is pending

Keep `emay-live.service` as the display-only collector until the storage-mode collector has passed
its local and Pi verification. It continues to publish current status and journal evidence. Do not
deploy the withheld host-counter draft. H10, the database, the spool drainer, the kiosk, and the
Pi-local watcher remain unchanged. The graph must continue to label EMAY's history as unavailable /
not yet PostgreSQL-backed rather than claim durability before T-E2 and T-E3 ship.
