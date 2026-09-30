# Amendment 12 — Deterministic BlueZ backend test seam

**Status:** Accepted by the owner during the AirSense broker migration.

## Decision

The broker's Linux BlueZ owner must place its hardware-facing operations behind a
small typed backend/transport seam. The production implementation remains the
real `bluer`/D-Bus implementation. Deterministic tests may provide a stateful
fake backend to exercise the production owner policy and operation orchestration.

The seam must cover, without generic D-Bus escape hatches:

- adapter health and generation handling;
- discovery start, event handling, timeout, and prior-filter restoration;
- kernel-device observation;
- connect deadline and disconnect cleanup;
- exact service/characteristic resolution;
- notification subscription and stream termination;
- serialized writes and write failures;
- BlueZ error classification and fail-closed conversion.

Actual BlueZ/Pi integration remains a separate acceptance gate. A fake backend is
not evidence that a physical controller, BlueZ daemon, adapter, or target device
is available.

## Reason

The original coverage requirement includes all broker executable lines, functions,
and regions, but a concrete owner that constructs `bluer` objects directly cannot
exercise its failure branches deterministically without a live and failure-
injectable D-Bus environment. Testing a replacement owner would cover the mock,
not the production dispatch. A typed backend seam preserves the application-level
BlueZ ownership boundary while making the 100% LLVM gate reproducible in CI.

## Constraints

- Stable controller selection remains outside and before the backend seam.
- Runtime `hciN` aliases remain evidence only, never hardware identity.
- The broker remains the only application-level BlueZ owner.
- No production path is excluded from coverage to satisfy the threshold.
- No pairing, service activation, reboot, or physical acceptance is authorized by
  this amendment.
