# Proposal — Polar H10 physical acceptance

## Status

Deferred. The H10 sensor is currently unavailable and cannot be physically exercised. This is a
separate follow-up to `linux-bluez-broker`; no H10 acceptance is claimed there or here.

## Why

The Linux broker transport is implemented, but a running service, a stale `Connected` property, a
connection attempt, or synthetic/emulator data cannot prove fresh HR/RR measurements. The sensor
must be available for a bounded physical run with packet and ingest evidence.

## Scope

- Verify the already-approved H10 identity without pairing or changing bonds.
- Run the H10 collector through the broker with Wellue disabled.
- Capture bounded BlueZ/broker/collector evidence for connection, GATT resolution, subscription,
  notification timing, reconnect, and cancellation behavior.
- Require fresh, nonzero HR and RR data and source/session/sequence evidence in durable ingest.
- Test the H10 path after broker and `bluetoothd` restart without overlapping BlueZ owners.
- Record a rollback and leave H10 stopped if the physical gate fails.

## Out of scope

- Changes to the Linux broker transport or BP kernel-autoconnect path.
- Pairing, removing bonds, adapter resets, firmware or `/boot/firmware` changes.
- Enabling Wellue or claiming Wellue coexistence.
- Treating emulator, stale, or historical H10 data as physical acceptance.

## Entry gate

An operator must confirm that the H10 is physically available and authorize a finite canary window.
Until then H10 remains `unknown`, its collector remains stopped, and the current broker change is
accepted without H10 physical evidence.

## Acceptance evidence

The follow-up is complete only when the command exits and evidence bundle show:

1. one broker owner and no overlapping legacy BlueZ owner;
2. stable controller identity and current runtime alias;
3. fresh nonzero HR/RR notifications in a bounded `btmon`/journald trace;
4. notification gap, reconnect, and cancellation outcomes;
5. source-specific session/sequence identity and durable `vitalsd` acknowledgement; and
6. clean recovery or an explicitly named degraded/unknown result after restart.
