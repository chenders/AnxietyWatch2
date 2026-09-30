# ResMed AirSense 11 admin and kiosk console

## ADDED Requirements

### Requirement: The console SHALL expose only a safe AirSense projection

The Rust console SHALL expose `GET /api/air11` as a bounded server-side
projection of AirSense status, service state, external credential readiness,
sanitized recent service logs, and non-secret audit state. It SHALL not grant
the browser direct access to runtime, credential, journal, or database paths.

#### Scenario: The status file contains a current blocker

- **GIVEN** `/run/vitals-air11/status.json` contains a degraded link and a
  blocker
- **WHEN** an allowed private-network browser requests `/api/air11`
- **THEN** the response includes the separate link state, measurement state,
  counters, host receipt, and named blocker
- **AND** it does not include a raw target address, serial, client id, master
  key, session key, or raw payload

#### Scenario: The status file is absent or malformed

- **GIVEN** the status file is missing, oversized, or invalid JSON
- **WHEN** the endpoint is read
- **THEN** it returns an explicit not-observed/unknown status and named blocker
- **AND** it does not infer healthy or measured state from old database rows

### Requirement: Credential values SHALL remain outside the browser and database

The console SHALL inspect the external AirSense credential file only for
bounded presence, permissions, and shape. It SHALL return readiness facts but
never accept, return, log, store, or display the AirSense credential values.

#### Scenario: External credentials are ready

- **GIVEN** the operator-managed file is a regular owner-only file with a
  bounded client id, explicit Linux target/mapping, and valid 32-byte key shape
- **WHEN** the AirSense page reads credential readiness
- **THEN** it reports `ready`, owner-only permissions, and presence booleans
- **AND** the response contains none of the client id, target, or key values

#### Scenario: Credential material is absent or unsafe

- **GIVEN** the file is missing, too permissive, malformed, oversized, or has an
  invalid target/key shape
- **WHEN** the page reads credential readiness or attempts Start
- **THEN** the page names the credential blocker
- **AND** Start is refused without accepting a replacement key in HTTP

### Requirement: The console SHALL support private-LAN access without public exposure

The console SHALL bind to a LAN-capable address and the AirSense API SHALL
admit only peers in a configured private-network CIDR allowlist. Invalid or
empty allowlists SHALL fail closed. A missing peer identity SHALL be rejected.

#### Scenario: An allowed LAN peer reads the monitor

- **GIVEN** the peer address belongs to a configured local CIDR
- **WHEN** it requests the AirSense admin or kiosk feed
- **THEN** the safe status projection is served

#### Scenario: A peer is outside the allowlist

- **GIVEN** the peer address is outside the configured local CIDRs or no peer
  identity is available
- **WHEN** it requests the AirSense API
- **THEN** the request is rejected with a named network blocker
- **AND** no status or credential projection is returned

### Requirement: Opt-in SHALL be explicit and non-secret

The admin page SHALL offer explicit AirSense opt-in/out state backed by a
singleton PostgreSQL configuration row. This state SHALL contain no AirSense
credential or device identifier. Opt-out SHALL not silently stop an active
collector.

#### Scenario: The operator opts in

- **GIVEN** the operator is authorized and the config/audit tables are available
- **WHEN** the operator confirms Opt in
- **THEN** the singleton records `opted_in=true`
- **AND** an append-only audit row records the fixed action and outcome
- **AND** no service is paired, scanned, or started by the opt-in operation

#### Scenario: The operator attempts to opt out while active

- **GIVEN** the config says opted-in and systemd reports the AirSense unit active
- **WHEN** the operator confirms Opt out
- **THEN** the action is rejected with `active_state=active`
- **AND** the opt-in row remains true

### Requirement: Lifecycle actions SHALL be fixed, authorized, and auditable

The API SHALL permit only opt-in, opt-out, credential-check, and start/stop/
restart of `vitals-air11.service`. Mutations SHALL require the service policy,
external console-admin capability, per-process action token, and exact
confirmation for state-changing actions. No arbitrary command, unit, path, or
AirSense RPC SHALL be accepted.

#### Scenario: The operator starts an opted-in ready collector

- **GIVEN** opt-in is true, external credentials are ready, authorization is
  valid, and the operator confirms Start
- **WHEN** the fixed action is submitted
- **THEN** the backend audits the request and invokes only
  `systemctl start vitals-air11.service`
- **AND** it audits the fixed action result
- **AND** it never pairs, scans, or sends an AirSense therapy/control RPC

#### Scenario: Start preconditions are not met

- **GIVEN** opt-in is false/unknown or credentials are not ready
- **WHEN** Start is requested
- **THEN** the backend rejects it with the blocking value and expected state
- **AND** it does not invoke systemd

#### Scenario: An unauthorized or arbitrary action is submitted

- **GIVEN** the admin capability/action token is missing or the request names an
  unsupported action/unit/verb
- **WHEN** the request reaches the API
- **THEN** it is rejected before any mutation or systemd call
- **AND** the rejection does not reveal credential material

#### Scenario: Systemd action result cannot be audited

- **GIVEN** a fixed systemd action completes but its result audit append fails
- **WHEN** the API responds
- **THEN** it reports the action outcome as unknown
- **AND** it does not claim that the service state is confirmed

### Requirement: Status, blockers, and logs SHALL be obvious and truthful

The admin page SHALL distinguish link, measurement, process, configuration, and
health states. It SHALL promote important blockers and display the last status,
bounded counters, and recent sanitized service journal entries. `UNKNOWN` SHALL
not use the healthy treatment, and `NOT CONFIGURED` SHALL remain distinct.

#### Scenario: The collector reports durability uncertainty

- **GIVEN** status has `durability_unknown=true` or an append/journal blocker
- **WHEN** the admin or kiosk page renders
- **THEN** the blocker is promoted above routine facts with an explicit warning
- **AND** the page says the journal is retained/uncertain rather than durable

#### Scenario: The unit has an important journal error

- **GIVEN** the selected service journal contains a priority error entry
- **WHEN** the console reads the journal
- **THEN** the page displays the bounded sanitized message as an error
- **AND** the error is promoted in the attention area and remains visible in the
  recent-log list
- **AND** addresses, key-shaped values, raw JSON, and oversized text are not
  exposed

#### Scenario: The console feed becomes stale

- **GIVEN** the page has a prior successful response and later refreshes fail
- **WHEN** the feed exceeds its refresh expectation
- **THEN** a page-level unknown/stale warning appears
- **AND** the last known status and error reason remain visible
- **AND** no old value is relabeled as current healthy evidence

### Requirement: The admin page SHALL render through all three console directions

The AirSense screen SHALL use one backend view-model and render it through
Service Mode, Preflight, and Flowpath without direction-specific data logic.
Each direction SHALL retain the same state semantics, blocker precedence,
credential boundary, controls, logs, and audit facts.

#### Scenario: The operator switches layout direction

- **GIVEN** the AirSense screen is loaded with a selected direction
- **WHEN** the operator changes the runtime direction
- **THEN** the same status facts and blockers are rendered in the new layout
- **AND** no direction invents a measurement or hides an error

### Requirement: The dedicated kiosk SHALL be read-only and monitorable

The console SHALL provide `/console/air11-kiosk.html` as a dedicated monitor
using the same safe API. It SHALL show large current link, measurement, process,
credential-readiness, freshness, counter, blocker, and log facts. It SHALL not
show lifecycle controls, credential inputs, or device-control operations.

#### Scenario: Kiosk receives measured streaming evidence

- **GIVEN** the safe API reports opted-in, `streaming`, and `measured`
- **WHEN** the kiosk refreshes
- **THEN** it shows a prominent read-only streaming verdict, host receipt age,
  counters, and current sanitized logs
- **AND** it does not display device time as the authoritative timestamp

#### Scenario: Kiosk receives no current evidence

- **GIVEN** the API is unavailable, stale, disconnected, or only authenticated
  without a usable measurement
- **WHEN** the kiosk renders
- **THEN** it shows `UNKNOWN` or `NOT CONFIGURED` with the named reason
- **AND** it never substitutes zero or a healthy measurement

### Requirement: The physiology dashboard SHALL keep operational state separate

The existing physiology dashboard MAY link to the AirSense kiosk, but SHALL not
merge AirSense process/error/configuration state into physiological measurements
or infer clinical meaning from collector connectivity.

#### Scenario: An operator opens the existing dashboard

- **GIVEN** the physiology dashboard is loaded
- **WHEN** the operator wants AirSense machine evidence
- **THEN** a link leads to the dedicated AirSense monitor
- **AND** the existing physiological screen retains its own source/type and null
  semantics

### Requirement: Verification SHALL cover the safety boundary

The change SHALL have host tests and static checks for safe projection,
credential non-leakage, CIDR handling, journal sanitization, fixed action
allowlisting, direction rendering, kiosk syntax, database migration structure,
and public-information hygiene.

#### Scenario: The offline verification suite runs

- **GIVEN** no Pi, pairing, credentials, or live database is available
- **WHEN** the task's Verify command runs
- **THEN** it exercises the host-safe tests and exits nonzero on a failed check
- **AND** it does not attempt pairing, scanning, service activation, route
  changes, adapter changes, or firmware changes
