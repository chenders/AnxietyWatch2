# Design — durable EMAY SleepO2 ingestion

**Status:** Blocked pending an identity decision required by the accepted amendments.

## Existing path

The Pi already runs `vitalsd` as the single writer. A collector sends length-prefixed CBOR
`Request::Append` frames over `/run/vitals/vitalsd.sock`; `vitalsd` converts the Unix-microsecond
wire time into `SystemTime`, appends PostgreSQL binary-COPY rows to its spool, and
`vitals-spool-drain` commits them to the existing `samples` hypertable. The schema primary key is
`(source, type, session_id, seq)`. The panel's existing `panel-vitals-status` publisher reads
PostgreSQL and emits the `vitals_status.json`/`vitals_h10_history.json` projections.

The missing producer is not enough to implement the change safely: EMAY's identity capability must
be established first.

## Identity finding

The verified data notification is exactly:

```
EB 01 05 <pulse> <SpO2> 7F 00 <checksum>
```

The payload has no counter, device timestamp, or stable record identifier. The connected BlueZ
characteristic `0xFF02` reports `read`, `notify`; it is not an indication. A notification is not an
ATT-confirmed store-and-forward delivery. Repeated readings are byte-identical to a retransmission,
and a reconnect starts with no sensor-provided continuity marker.

Amendment 05 §1 and Amendment 06 §1.2 require `seq` to be stable across retransmission. Delivery
order is permitted for the confirmed store-and-forward A&D cuff because the confirmation makes
redelivery impossible; it is not permitted for an unconfirmed streaming source. Therefore a
`SessionMinter` plus a collector-local frame counter is **not** a valid EMAY implementation under the
current design. It would make the database accept duplicates after replay/reconnect and would make
same-value frames impossible to distinguish from duplicates.

## Required resolution before collector work

The preferred, minimal path is a protocol/firmware investigation for a stable EMAY identity:

1. Check vendor protocol documentation or an available alternate EMAY mode for a per-record counter,
   monotonic device tick, or stored-record export.
2. Verify whether any alternate data characteristic is an indication/store-and-forward channel.
3. Do not issue unverified commands against the overnight device while H10/EMAY collection is
   running; no active scan or broad reset is justified by this question.
4. If stable identity is found, implement a Linux collector that uses it, validates with generated
   registry enums/ranges, and appends through `vitalsd` with the existing streaming or confirmed
   durability policy as appropriate. Then add the existing PostgreSQL-backed EMAY history to the
   panel projection.

If no identity exists, the owner must explicitly choose a new exception. The least disruptive
exception would be an append-only at-least-once EMAY observation contract with an explicit duplicate
indicator/handling policy and a review of how duplicates affect graphing and CNS consumers. A new
raw/event table is technically possible but violates the no-second-history-store preference and is
not proposed without approval.

## Historical data already collected

The old stopgap's journal lines are useful evidence, but they were not assigned durable session and
sequence identity. They must not be bulk-inserted into `samples` by assigning a new session/counter;
that would make their identity appear stronger than the source can support. They can remain in the
journal until an approved replay identity is defined.

## Safe state while blocked

Keep `emay-live.service` as the display-only collector. It continues to publish current status and
journal evidence. Do not deploy the uncommitted delivery-index draft. H10, the database, the spool
drainer, the kiosk, and the Pi-local watcher remain unchanged. The graph must continue to label
EMAY's history as unavailable/live-buffer-only rather than claim PostgreSQL durability.
