# Change: Read-only ResMed AirSense 11 BLE therapy streaming

## Status

Proposed. The protocol and short live stream have been demonstrated on a physical
AirSense 11; this proposal does not authorize pairing, deployment, activation, or
an overnight run.

## Why

The existing ResMed BLE research handoff incorrectly concludes that AirSense 11
BLE is only a provisioning path. An authenticated BLE session has now delivered
live flow, pressure, ventilation, leak, respiratory-rate, and related therapy
signals. AnxietyWatch2 needs a native Linux collector so those observations can
reach the existing `vitalsd` → COPY spool → PostgreSQL path without introducing a
second store or unsafe device controls.

## What Changes

Add a separate, read-only AirSense 11 BLE stream capability. It will reimplement
the observed FIG framing, SRP-derived session authentication, AES-CBC envelope,
JSON-RPC response handling, and `StreamData` decoding in Rust. The collector will
use only externally provisioned credentials, normalize a usable target to a
BlueZ MAC address, request the fixed therapy stream, stamp rows with host receipt
time, and preserve the untrusted device timestamp as provenance. Source-backed
sequence identity, nullable physiology, reconnect behavior, bounded journaling,
status, and systemd confinement will be explicit contracts.

## Capabilities

- `resmed-air11-ble-stream`

## In Scope

- Registry-backed AirSense source and therapy metric types.
- Portable hostile-input decoders, sanitized replay fixtures, and fuzz targets.
- Linux BlueZ transport, reconnect/authentication, fixed stream subscription, and
  the existing `Request::Append` socket path.
- A strict allowlist that cannot send therapy, settings, erase, pairing, or OTA
  control RPCs from the production collector.
- Raw StreamData journaling, operator-safe status, service confinement, and
  offline/replay acceptance evidence.

## Out of Scope

- First pairing, passkey collection, credential generation, or credential
  rotation by the collector. Those remain an explicitly authorized, external
  operation; credentials stay outside Git at `~/.air11/air11-credentials.json`.
- `Set`, therapy transitions, reset, erase, OTA, arbitrary `Get`, device-clock
  synchronization, historical spool extraction, or CAN control.
- Replacing the EzShare SD/EDF path. EzShare remains the independent durable and
  archive fallback described in `docs/integration/hms-cpap-ezshare-integration.md`.
- Alarm decisions based solely on a live BLE stream or on unavailable signals.

## Impact

No `samples` schema change is required: the existing key
`(source, type, session_id, seq)` and `vitalsd` append protocol are reused. The
registry reference data must be regenerated before rows can drain. Deployment
and physical acceptance remain separate gates and must not disturb EzShare, H10,
EMAY, PostgreSQL, or the existing spool drainer.
