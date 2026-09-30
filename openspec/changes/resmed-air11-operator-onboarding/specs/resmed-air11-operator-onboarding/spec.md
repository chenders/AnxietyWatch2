# ResMed AirSense 11 operator onboarding and discovery

## ADDED Requirements

### Requirement: Onboarding SHALL be a separate operator-authorized capability

The system SHALL expose onboarding separately from steady-state collection. A
caller SHALL choose discovery or first application pairing explicitly and SHALL
confirm the exact finite operation before any BlueZ discovery or AirSense
pairing write.

#### Scenario: No operator confirmation is supplied

- **GIVEN** onboarding is invoked without a matching duration-specific
  confirmation
- **WHEN** the command reaches its physical-operation boundary
- **THEN** it exits nonzero before discovery or GATT writes
- **AND** it names the missing or mismatched authorization without revealing
  credentials

### Requirement: Discovery SHALL find and validate the Linux target

Discovery SHALL use the intended existing adapter, a finite duration, and the
AirSense FD56 service filter. It SHALL require an unambiguous target and SHALL
not convert or guess a Linux MAC from a CoreBluetooth UUID.

#### Scenario: Exactly one AirSense candidate is discovered

- **GIVEN** bounded discovery reports exactly one candidate advertising the
  AirSense service
- **WHEN** the operator confirms that candidate
- **THEN** onboarding normalizes its six-octet Linux address
- **AND** it may update only the external credential mapping atomically
- **AND** it does not start the collector or send an application RPC

#### Scenario: Discovery reports no eligible candidates

- **GIVEN** bounded discovery reports zero eligible candidates
- **WHEN** onboarding evaluates the result
- **THEN** it exits nonzero with the candidate count and selection blocker

#### Scenario: Discovery reports multiple candidates

- **GIVEN** bounded discovery reports multiple eligible candidates
- **WHEN** the operator supplies an exact address selection
- **THEN** onboarding accepts it only when that address is one of the eligible
  candidates
- **AND** it does not choose an address by proximity, arrival order, UUID
  truncation, or guessing

### Requirement: Discovery cleanup SHALL restore adapter state

Onboarding SHALL stop its own discovery operation on success, failure,
interruption, and timeout. It SHALL report a cleanup failure as a failed result
and SHALL not leave discovery running intentionally.

#### Scenario: Discovery is interrupted

- **GIVEN** the operator interrupts or the discovery deadline expires
- **WHEN** cleanup runs
- **THEN** the adapter's prior discovery state is restored or the command exits
  nonzero with a named cleanup blocker
- **AND** no collector service or route is changed

### Requirement: First application pairing SHALL be explicit and bounded

When reusable external credentials are absent, onboarding SHALL perform the
documented AirSense first-pair messages, accept the physical passkey
interactively, and persist only the resulting external reusable credentials.
When valid credentials already exist, onboarding SHALL not repeat first
pairing. Pairing SHALL not be available through the production collector or a
generic RPC argument.

#### Scenario: Pairing succeeds

- **GIVEN** an operator-selected AirSense target and a passkey read from the
  device
- **WHEN** `StartKeyExchange` and `ConfirmKeyExchange` complete successfully
- **THEN** onboarding writes the resulting client id and master pair key only
  to the private external credential file
- **AND** it never writes either value to logs, status, PostgreSQL, Git, or a
  command argument

#### Scenario: Pairing proof fails

- **GIVEN** the passkey or SRP proof is rejected
- **WHEN** onboarding receives the failure
- **THEN** it exits nonzero with a phase-level blocker
- **AND** it preserves any previously valid credential without replacement
- **AND** it sends no therapy, settings, reset, erase, OTA, or arbitrary RPC

### Requirement: Onboarding SHALL preserve the existing read-only collector

The steady-state collector SHALL retain its existing final RPC allowlist,
credential secrecy, registry semantics, host-time policy, journal/vitalsd path,
and reconnect behavior. Onboarding SHALL not add a generic method escape hatch.

#### Scenario: The collector starts after discovery

- **GIVEN** onboarding has resolved a private Linux address and the existing
  credential is ready
- **WHEN** the collector is later started through the normal fixed workflow
- **THEN** it uses only the existing read-only observation RPC set
- **AND** onboarding evidence does not imply measured physiology or database
  acceptance

### Requirement: Onboarding SHALL leave unrelated sources and host state alone

Onboarding SHALL not change H10, EMAY, EzShare, PostgreSQL schemas, routes,
firmware, overlays, adapter identity, or service enablement.

#### Scenario: Onboarding completes

- **GIVEN** discovery or pairing succeeds
- **WHEN** onboarding exits
- **THEN** H10/EMAY/EzShare service state and the main route are unchanged
- **AND** `vitals-air11.service` remains stopped and disabled unless a separate
  operator action authorizes otherwise

### Requirement: Onboarding evidence SHALL be private and bounded

Onboarding SHALL keep addresses, passkeys, client ids, master keys, and raw
BLE material out of Git and ordinary logs. Tests SHALL cover authorization,
bounds, candidate selection, cleanup, credential secrecy, and the negative RPC
allowlist.

#### Scenario: Host-safe verification runs

- **GIVEN** no physical device or credential is available
- **WHEN** host verification runs
- **THEN** it exercises only synthetic discovery/pairing seams and exits
  nonzero for unsafe or unbounded behavior
- **AND** it does not initiate a real scan, pairing attempt, service activation,
  route change, or firmware change
