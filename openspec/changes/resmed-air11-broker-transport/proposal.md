# Change: Move the AirSense 11 collector behind the Linux BLE broker

## Status

Proposed. This change authorizes a host-side transport migration and static
verification only. It does not authorize pairing, service enablement, adapter
changes, reboot, or live Pi acceptance.

## Why

The AirSense collector currently opens BlueZ directly with `bluer` and pins
itself to `hci0`. That violates the broker ownership boundary and makes the
collector sensitive to runtime adapter aliases. The saved AirSense application
credentials are still valid, but credentials alone cannot repair a missing
BlueZ device object or arbitrate adapter access.

## What changes

Move AirSense GATT connection, bounded target discovery, service and
characteristic resolution, notifications, writes, and disconnect cleanup behind
the versioned `vitals-ble-broker` Unix-socket protocol. Extend the broker client
with typed bounded wrappers for the already-defined discovery and write
operations. Preserve the AirSense application authentication, encryption,
read-only RPC allowlist, journaling, status, source identity, and vitalsd
append behavior.

The collector receives a stable adapter selector from deployment configuration;
`hciN` is never treated as hardware identity. A separate broker-owned discovery
run may materialize the configured ResMed target before the collector starts.
The operator-managed credential file and its `LoadCredential` mapping remain
unchanged.

## In scope

- Broker-client discovery and GATT-write APIs with deadline cancellation.
- AirSense transport migration from direct BlueZ to broker leases and events.
- Stable adapter-selector configuration and broker service dependency.
- Static, protocol, unit, and offline transport tests.
- A mandatory Linux LLVM coverage gate with 100% executable line, function,
  and region coverage for the broker.
- Documentation of the bounded discovery prerequisite and rollback boundary.

## Out of scope

- Creating or rotating AirSense credentials.
- First application pairing or passkey entry.
- Enabling or starting `vitals-air11.service` on the Pi.
- Adapter replacement, `hciN` renaming, overlays, reboot, or database/schema work.
- Changes to AirSense RPC permissions: the production collector remains
  read-only.

## Acceptance boundary

Host tests prove that no AirSense worker path imports or calls BlueZ and that
broker operations are typed, bounded, capability-gated, and cancellable. The
broker's Linux coverage gate must report 100% executable line, function, and
region coverage; uncovered code blocks acceptance. Pi acceptance remains a
separate explicitly authorized command after the target is available and the
broker has been deployed.
