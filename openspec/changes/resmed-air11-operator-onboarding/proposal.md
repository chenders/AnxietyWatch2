# Change: Operator-authorized ResMed AirSense 11 onboarding and discovery

## Status

Accepted — owner approved 2026-09-11. This change updates the physical
onboarding boundary only; a live run still requires the implementation task's
Verify command and a separate bounded operator confirmation.

## Decision requested

Remove the categorical prohibition on AirSense BLE discovery and first-pairing
from the physical onboarding path. Permit a separately invoked, operator-
authorized onboarding operation to:

1. discover the AirSense 11 FD56 service on the intended Linux adapter and
   recover its Linux address; and
2. when no reusable external credentials exist, complete the documented
   AirSense application pairing flow using the passkey shown on the device;
   valid existing credentials do not need to be paired again.

The existing production therapy collector remains read-only after onboarding.
This proposal does **not** permit therapy/settings/reset/erase/OTA RPCs, route
or firmware changes, adapter replacement, or public exposure of credentials.

## Why

The current AirSense change assumes that an external setup has already supplied
both application credentials and a Linux MAC address. The prior Python client
successfully connected using a macOS CoreBluetooth UUID, which proves the
protocol path but does not give BlueZ the Linux address. The current blanket
no-scan/no-pair wording makes the documented onboarding path impossible when
that mapping is absent, even though a bounded operator-observed discovery can
resolve it.

The distinction is now explicit:

- **onboarding** may discover the target and must perform first application
  pairing when reusable credentials are absent;
- **steady-state collection** uses saved credentials and the resolved target;
- **therapy/control operations** remain forbidden.

## What changes

- Add a separate onboarding command/workflow, not a generic RPC shell.
- Replace the blanket no-pair/no-scan physical wording with an explicit,
  operator-confirmed finite discovery/pairing operation.
- Filter discovery to the AirSense FD56 service and require an unambiguous
  target before writing `linux_address` to the external credential file.
- Keep passkeys and keys interactive and memory-only; never log, echo to a
  file, store in PostgreSQL, or place them in Git.
- Restore the adapter's prior discovery state on every exit and leave H10,
  EMAY, EzShare, routes, overlays, and firmware untouched.
- Keep the existing production RPC allowlist and all ingestion, timestamp,
  null, identity, journal, and database boundaries unchanged.

## Out of scope

- A general Bluetooth browser, indefinite discovery, arbitrary GATT writes, or
  arbitrary AirSense RPC methods.
- Therapy transitions, settings, reset, erase, OTA, clock synchronization, or
  credential export/rotation by an unattended process.
- Changes to the existing collector's read-only stream behavior.

## Approval effect

Approval authorizes implementation of the onboarding workflow described here.
It does not authorize service activation or live acceptance. Physical discovery
and first pairing remain separate bounded operations with command exit-code
evidence; the operator confirmation is collected by the onboarding command.
