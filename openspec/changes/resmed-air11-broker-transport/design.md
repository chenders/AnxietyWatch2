# Design — broker-owned AirSense 11 BLE transport

## Boundary

The `vitals-ble-broker` process remains the only application-level BlueZ/D-Bus
owner. The AirSense collector retains protocol state, application crypto,
allowlisted RPC construction, FIG reassembly, raw journaling, status, and
vitalsd append logic, but has no `bluer::Session`, adapter, device,
characteristic, discovery, or direct disconnect capability.

The implementation spans the broker/client crate and the AirSense collector
crate. The broker revision containing the stable-controller selector and typed
operations is a prerequisite; this change does not duplicate or fork that
owner.

The broker boundary is deliberately extraction-ready. Product names,
credentials, AirSense RPCs, physiological types, and bedside status remain in
AnxietyWatch2 integration crates. The reusable boundary consists of bounded
versioned messages, opaque worker/device selectors, capability-gated leases,
operation deadlines/cancellation, typed GATT operations, generation changes,
and redacted errors. A future extraction may publish those pieces without
publishing this product's collectors or health data model; that extraction is a
separate OpenSpec change and is not allowed to weaken this migration's safety
gates.

## Lease and operation sequence

1. Load the existing private credential through systemd's `LoadCredential`.
2. Open a `BrokerClient` handshake as `Source::AirSense` using the configured
   stable adapter selector and normalized target address.
3. Acquire one bounded connect lease with `Connect`, `Resolve`, `Subscribe`, and
   `Write` capabilities. The capability set is explicit and no generic D-Bus
   operation exists.
4. If the broker reports the target device is not materialized, use a separate
   bounded broker discovery lease/operation. Discovery must require the target
   address, preserve and restore the adapter's prior filter, and fail closed on
   timeout, ambiguity, or cleanup failure. The steady-state collector does not
   scan indefinitely.
5. Connect the target through the broker, resolve the exact FD56 service and
   TX/RX UUIDs, and subscribe to RX notifications. The broker returns opaque
   typed handles; the worker never sees BlueZ object paths.
6. Send FIG packets with the broker's bounded write operation and consume
   `Frame` events from the same lease. Application responses and stream
   notifications continue through the existing AirSense correlation and crypto
   layers.
7. On disconnect, cancellation, broker generation change, or worker shutdown,
   release the lease. Broker cleanup owns the BlueZ disconnect and subscription
   teardown; the collector does not issue a direct BlueZ call.

The broker client adds `discover_target_bounded` and `write_bounded` wrappers
for the existing wire operations. Both wrappers correlate operation IDs,
return named broker errors, and send cancellation after their deadlines. Writes
retain a bounded payload and never expose arbitrary D-Bus methods.

## Concurrency and event ordering

The client reader remains single-owner. A write waits only for its correlated
broker operation result; subsequent response and notification frames are
consumed by the existing `next_event` path. The transport must retain the
bounded FIG reassembler across coalesced broker frames and distinguish broker
`DeviceState`/`Disconnected` events from AirSense payloads. It must not assume
that a connected state is equivalent to authenticated or measured state.

## Configuration and deployment

The service receives a stable selector string (for example, the broker's
validated USB bus plus VID/PID selector) rather than `--adapter hci0`. The
collector passes that selector to the broker client. The unit starts after and
requires the broker socket service, while retaining its existing private
credential, vitalsd, memory, and filesystem restrictions. The unit remains
installed-but-disabled until the physical acceptance gate authorizes activation.

The target address remains in the private credential file and is not logged,
placed in status, or embedded in the repository. Existing credentials are
application credentials; they do not themselves materialize a BlueZ device.
If the target is absent, onboarding/rediscovery is a finite broker-owned
operation and must complete before steady-state collection.

## Failure and rollback

Every broker failure names the phase and blocker value/limit without exposing
credentials or full device identifiers. A lease timeout, failed cancellation,
unknown handle, generation change, notification end, or failed release makes
AirSense degraded/unknown and prevents numeric publication where durability or
session identity is uncertain.

Rollback is source-scoped: stop the broker client, verify lease and subscription
release, then restore the prior approved collector only if that owner is
explicitly authorized. The direct BlueZ AirSense unit must never run alongside
the broker client.

## Future library boundary

The reusable broker surface must not require a device-specific enum, a
physiology registry, an application credential format, a product status file,
or a BlueZ object path. The Linux BlueZ implementation is an optional backend
behind typed adapter/GATT seams. Product integration supplies worker identity,
priority policy, stable hardware selector configuration, and source-specific
application protocol above that seam. The public package must preserve bounded
payloads, no generic D-Bus calls, fail-closed capability checks, lease cleanup,
and deterministic tests.

The separate `ble-broker-library-extraction` OpenSpec change defines packaging,
API stability, licensing, documentation, and publication after the operational
migration is accepted.

## Verification

- Broker client unit/protocol tests cover capability gates, operation
  correlation, bounded payloads, deadlines, cancellation, and target discovery.
- The broker runs under LLVM source-based coverage on Linux with all features
  and all targets. The gate requires 100% executable line, function, and region
  coverage for broker source; any uncovered item is a failing acceptance result.
  Fake-owner and protocol seams cover deterministic failure paths without
  pretending that a host test is physical BlueZ evidence.
- AirSense transport tests use a fake broker frame/event stream and retain the
  existing read-only RPC, FIG, crypto, reconnect, journal, and status tests.
- Static checks prove no `bluer`/BlueZ import remains in the AirSense worker and
  the unit contains no `hciN` identity.
- Linux/Pi evidence is separate from host tests and is not claimed by this
  change's host verification.
