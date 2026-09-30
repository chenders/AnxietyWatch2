# Design — operator-authorized AirSense 11 onboarding

## Boundary

Onboarding is a separate Linux/Pi operation from
`resmed-air11-collector`. It may use BlueZ discovery and the AirSense
application-level first-pair flow, but it is never a generic Bluetooth or RPC
console. The steady-state collector continues to accept only its existing
saved credential and fixed observation RPCs.

The operation must be explicitly selected and confirmed by the operator. A
finite default and hard upper bound are required so a forgotten command cannot
leave discovery or pairing active indefinitely. The exact duration and upper
bound are implementation-task decisions and must be visible in the confirmation
phrase.

## Discovery

1. Verify the intended adapter exists and record its pre-operation discovery
   state; do not create, power, replace, or reconfigure an adapter.
2. Start a bounded BlueZ discovery on that adapter, filtered to the documented
   AirSense service UUID `0000fd56-0000-1000-8000-00805f9b34fb` and the ResMed
   advertisement identity where available.
3. Collect candidate addresses only in memory. Require exactly one candidate,
   or require an explicit operator-selected candidate when more than one
   legitimate AirSense is visible. A CoreBluetooth UUID is not converted or
   guessed into a MAC.
4. Stop discovery in `finally`-equivalent cleanup and restore the prior
   discovery state. Failure to restore state is a named failed result.
5. If application credentials already exist, write only the normalized Linux
   address mapping to the external credential file using an owner-only,
   atomic replacement. Never write the address to Git, PostgreSQL, journal,
   ordinary logs, or browser responses.

The discovery result is not itself proof of authentication or clinical data.
The acceptance harness must still prove the authenticated stream and database
path.

## First application pairing when credentials are absent

If no valid reusable external credentials exist, onboarding connects to the
operator-selected FD56 target and runs the documented
`StartKeyExchange`/`ConfirmKeyExchange` sequence. The four-digit passkey is
read interactively from the physical device and is never accepted as a command
line argument, environment value, log field, status field, journal field, or
browser value. The resulting `clientId` and `masterPairKey` are written only to
the external credential file through a private atomic update. This is the
required first-pair path for a new target; it is not repeated when the saved
credentials are valid.

Pairing mode has no generic method escape hatch. It cannot send therapy,
settings, reset, erase, OTA, clock-setting, or arbitrary RPCs. A failed or
cancelled operation must not replace a previously valid credential.

If the operator already has valid credentials, discovery mode alone resolves
the Linux address and the existing reconnect authentication supplies the live
stream session.

## Failure and cleanup

Every failure names its phase and value: adapter, discovery duration,
candidate count, target selection, passkey input, GATT service resolution,
application proof, credential update, or cleanup. Secrets are excluded from all
errors. The operation exits nonzero unless cleanup and the requested phase are
both confirmed.

No onboarding operation starts `vitals-air11.service`, enables a unit, changes
routes, edits `/boot/firmware`, changes EzShare, or changes H10/EMAY. The
operator may run the existing acceptance command only after onboarding has
completed and the normal physical prerequisites are present.

## Production boundary retained

The Rust production collector remains a read-only observation client. Its
final send boundary continues to reject all methods outside
`RequestSession`, `CheckSessionIntegrity`, `GetVersion`, `GetDateTime`, and
`StartStream`. Onboarding is a separate explicitly invoked capability and does
not widen that collector boundary or the admin console into an arbitrary RPC
surface.
