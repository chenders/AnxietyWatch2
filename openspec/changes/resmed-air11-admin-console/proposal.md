# Change: ResMed AirSense 11 admin and kiosk console

## Status

Proposed. This change adds an operator-facing Rust console surface for the
already-defined read-only AirSense 11 collector. It does not authorize pairing,
service activation on a physical host, or any AirSense therapy operation.

## Why

The collector now publishes an operator-safe status document, but an operator
has no dedicated way to see its link state, measurement state, blockers,
reconnect/durability counters, or recent service errors. The existing console
also has no AirSense opt-in workflow or fixed lifecycle controls. A separate
read-only kiosk view is useful for monitoring without putting machine state into
the physiology dashboard.

## What changes

- Add a Rust API projection for AirSense status, external credential readiness,
  service state, sanitized recent journal entries, and non-secret audit history.
- Store only AirSense opt-in intent and fixed-action audit records in
  PostgreSQL; keep the master pair key in the existing external credential
  file.
- Add explicit opt-in, start, stop, restart, and credential-check operations.
  They are fixed actions, require a private-LAN request, separate console-admin
  authorization, a per-process action token, and proportionate confirmation.
- Add an AirSense screen to every existing console direction and a dedicated
  read-only kiosk page. Link to the kiosk from the physiology dashboard without
  merging operational state into physiological measurements.
- Bind the console for private-LAN access with a configurable CIDR allowlist,
  add a narrowly scoped systemd authorization artifact, and document setup,
  monitoring, logs, kiosk URLs, troubleshooting, and bounded acceptance.

## In scope

The AirSense admin projection, non-secret configuration/audit schema, fixed
collector lifecycle requests, LAN policy, sanitized service journal display,
three-direction admin rendering, dedicated kiosk rendering, tests, deployment
artifacts, and user documentation.

## Out of scope

- Pairing, scanning, passkey collection, credential generation or read-back,
  therapy controls, settings, reset, erase, OTA, arbitrary RPC, or device-clock
  synchronization.
- Direct browser access to runtime/status/credential files, direct PostgreSQL
  writes from the collector, a second sample/history store, or a general
  remote shell/log browser.
- Changing H10, EMAY, EzShare, vitalsd, adapter routes, overlays, firmware, or
  the physical T-A11-6 acceptance gate.

## Impact

The console gains a private-LAN status endpoint and fixed operator actions. A
separate console-admin capability is provisioned outside the page; its absence
leaves the status and logs readable while mutations remain disabled. The
AirSense key is not moved into PostgreSQL or any browser payload. Existing
physiology dashboard semantics and source ownership remain unchanged.
