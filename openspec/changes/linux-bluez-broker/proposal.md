# Proposal — Linux BlueZ broker and collector access rewrite

## Why

The current Linux Bluetooth implementation has no adapter-wide application owner. Multiple collectors and stopgap tools independently open BlueZ sessions, start discovery, connect devices, mutate filters, retry, and in one path reset the adapter. The recent incident in which an absent Wellue device held discovery while BP waited for kernel autoconnect is strong evidence of a shared-resource arbitration defect, not merely independent device failures.

The repository investigation is recorded in:

- `docs/reviews/bluetooth-rewrite/LIBRARIAN.md`
- `docs/reviews/bluetooth-rewrite/COLLECTOR-AUDIT.md`
- `docs/reviews/bluetooth-rewrite/ARBITRATION-ARCHITECTURE.md`
- `docs/reviews/bluetooth-rewrite/PI-BLUEZ-PLATFORM.md`
- `docs/reviews/bluetooth-rewrite/RELIABILITY-TEST-PLAN.md`
- `docs/reviews/bluetooth-rewrite/PUBLIC-PRIOR-ART.md`

## What

Introduce a Linux-only `vitals-ble-broker` service/library that is the sole application owner of BlueZ operations for one verified physical adapter. Source workers retain protocol state, pure decoding, provenance, and durable ingest, but lose direct access to BlueZ.

The broker will provide typed, bounded leases and operations for discovery, connection admission, service resolution, GATT subscription, and writes. It will enforce source priorities, deadlines, cooldowns, cancellation, backpressure, adapter generations, and observable ownership. A separate narrowly privileged kernel-policy boundary will retain BP accept-list/autoconnect registration; the BP worker and broker must not initiate `Device1.Connect` for BP.

The rewrite will be incremental and reversible. Wellue remains disabled. H10 remains unaccepted until fresh physical measurements pass its existing gate. AirSense therapy lifecycle state remains a separate concern.

## Scope

- One broker per verified physical Linux Bluetooth adapter.
- A pure lease/scheduler state machine with deterministic tests.
- A versioned private Unix-socket broker protocol with bounded frames and worker capabilities.
- One BlueZ/D-Bus access boundary in the broker.
- Stable adapter identity resolution and adapter-generation handling.
- BP kernel-autoconnect observation, raw-frame durability, and `sync: true` preservation.
- Source-by-source migration for BP, H10, SleepO2, AirSense, and any later authorized Wellue path.
- Fake BlueZ contract tests plus real Linux/Pi integration gates.
- Systemd, permissions, health reporting, and rollback ownership.

## Out of scope

- Re-enabling Wellue.
- Applying `dtoverlay=disable-bt`, changing `/boot/firmware`, repartitioning, pairing devices, or other physical/runtime changes in this planning change.
- Claiming H10 acceptance from an emulator, a connected property, or stale data.
- Rewriting source decoders or physiological protocol logic except where a transport seam is required.
- Replacing BlueZ with raw HCI/GATT code.
- Introducing Home Assistant, Bleak, or Python Bluetooth dependencies into the appliance.
- iPhone CoreBluetooth or the optional iPhone mobility relay, except for documenting that the Linux broker remains authoritative for Pi-side sources.

## Success criteria

1. Every application-level BlueZ operation has one explicit broker or kernel-policy owner; direct worker call sites are absent or named as approved exceptions.
2. A discovery request has one owner, a bounded lease, filter snapshot/restoration evidence, a deadline, and a named denial/blocker.
3. BP can receive a kernel-created connection and subscribe without any broker or worker path capable of issuing `Device1.Connect` for BP.
4. Broker and worker failures produce explicit unknown/degraded states and release leases without blocking unrelated sources.
5. The absent-Wellue plus BP-measurement integration test passes deterministically, proving that an optional source cannot hold discovery indefinitely or prevent BP delivery.
6. Migration has a tested source-level rollback that never runs old and new BlueZ owners concurrently.
7. Linux/Pi coexistence acceptance records packet gaps, notification latency, reconnects, keepalive misses, and durability outcomes before any source is declared healthy.

## Non-goals

This change does not promise that one physical controller can carry every source concurrently. If measured capacity is insufficient, the design permits a second verified adapter with a second broker; it does not hide loss with retries or interpolation.
