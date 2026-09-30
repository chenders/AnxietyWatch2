## ADDED Requirements

### Requirement: The AirSense worker SHALL use only the BLE broker

The production AirSense worker SHALL communicate with Bluetooth only through a
versioned `BrokerClient` lease. It SHALL not import `bluer`, open BlueZ/D-Bus,
use an `hciN` adapter name, or perform direct discovery, connect, resolve,
subscribe, write, or disconnect operations.

#### Scenario: A broker-backed session is opened

- **GIVEN** valid external AirSense credentials and a configured stable adapter
  selector
- **WHEN** the worker starts a connection cycle
- **THEN** it authenticates as `Source::AirSense`
- **AND** acquires only the declared Connect/Resolve/Subscribe/Write
  capabilities
- **AND** performs all BLE operations through the broker lease

#### Scenario: A direct BlueZ path is present

- **GIVEN** a collector source file or service contains a direct `bluer` call,
  `hciN` identity, or BlueZ discovery/connect operation
- **WHEN** the static transport gate runs
- **THEN** the gate fails
- **AND** the broker-owned path is not accepted as complete

### Requirement: Broker operations SHALL be bounded and cancellable

The broker client SHALL expose typed bounded wrappers for target discovery and
GATT writes. Each wrapper SHALL correlate the operation ID, enforce a caller
limit, and send a cancellation request after deadline expiry. Payload sizes and
capabilities SHALL remain bounded and explicit.

#### Scenario: A write deadline expires

- **GIVEN** a broker write does not complete before its configured deadline
- **WHEN** the client reaches the deadline
- **THEN** it requests cancellation for that operation ID
- **AND** returns a named deadline or cancellation failure
- **AND** it does not leave the lease waiting on an unobserved write

#### Scenario: A worker requests an undeclared operation

- **GIVEN** an AirSense handshake or lease omits Write or Discover capability
- **WHEN** the worker requests that operation
- **THEN** the broker rejects it with a named capability blocker
- **AND** no BlueZ operation is attempted

### Requirement: AirSense GATT resolution SHALL remain exact

The worker SHALL resolve only the documented FD56 service and exact TX/RX
characteristic UUIDs through the broker, subscribe only to RX, and write only
bounded FIG packets to TX. It SHALL preserve existing FIG reassembly and
application response correlation.

#### Scenario: Broker frames contain AirSense responses and notifications

- **GIVEN** the broker delivers coalesced or split RX frames
- **WHEN** the worker consumes broker events
- **THEN** it feeds the payloads to the existing bounded FIG reassembler
- **AND** it distinguishes response packets from stream notifications
- **AND** it emits no physiological value from malformed or unrelated frames

#### Scenario: The broker reports a device or generation failure

- **GIVEN** the broker reports a disconnect, unknown handle, expired lease, or
  adapter-generation change
- **WHEN** the worker handles the event
- **THEN** it marks the AirSense link degraded/unknown with the named blocker
- **AND** it tears down the session and retries only through a new bounded lease

### Requirement: Stable adapter selection SHALL survive runtime alias changes

Deployment SHALL pass the broker's validated stable selector and SHALL not use
`hci0`, `hci1`, or any other runtime adapter alias as identity. The broker SHALL
refuse missing or ambiguous selector matches.

#### Scenario: The adapter runtime alias changes

- **GIVEN** the verified controller moves from one `hciN` alias to another
- **WHEN** the worker reconnects
- **THEN** it uses the unchanged stable selector
- **AND** the broker resolves the current runtime alias
- **AND** the worker does not pin or guess an `hciN` name

#### Scenario: The stable selector is missing or ambiguous

- **GIVEN** no controller matches, or more than one controller matches, the
  configured selector
- **WHEN** the broker handshake or lease is attempted
- **THEN** it fails closed with the candidate/match value
- **AND** the worker publishes no numeric AirSense data

### Requirement: Discovery SHALL be a separate bounded broker-owned operation

If the target is not materialized in BlueZ, discovery SHALL be requested through
the broker with a finite target and deadline. It SHALL not be an implicit,
indefinite collector scan. Discovery SHALL restore prior adapter filter state or
return a named cleanup failure.

#### Scenario: The saved target is not materialized

- **GIVEN** credentials exist but the broker cannot materialize the target
- **WHEN** an explicitly authorized bounded discovery runs
- **THEN** the broker discovers only for the configured finite window
- **AND** the target must be observed unambiguously before connection proceeds
- **AND** the collector does not perform direct BlueZ discovery

#### Scenario: Discovery cannot establish the target

- **GIVEN** the target is not advertised, candidates are ambiguous, or filter
  restoration fails
- **WHEN** the discovery window ends
- **THEN** the operation exits nonzero with the phase and blocking value
- **AND** the collector remains degraded/unknown
- **AND** no credential is replaced and no service is enabled

### Requirement: Credentials and read-only AirSense semantics SHALL be preserved

The worker SHALL continue loading the existing private credential through
systemd's private credential mapping and SHALL retain the current application
challenge, session encryption, read-only RPC allowlist, journal-before-decode,
status, source identity, and vitalsd append behavior.

#### Scenario: Existing credentials are valid

- **GIVEN** the external mode-0600 credential file contains a valid client ID,
  target, and master key
- **WHEN** broker transport connects and AirSense authenticates
- **THEN** the worker uses those credentials without rewriting them
- **AND** no key, session secret, or full target address appears in logs/status
- **AND** the fixed read-only stream path remains unchanged

#### Scenario: The BLE broker is unavailable

- **GIVEN** the broker socket, lease, or generation is unavailable
- **WHEN** the worker attempts collection
- **THEN** status remains degraded/unknown with the broker blocker
- **AND** no direct-BlueZ fallback starts
- **AND** no numeric sample is fabricated or marked live

### Requirement: Deployment SHALL retain the explicit activation gate

The systemd unit SHALL depend on the broker service and stable selector while
remaining disabled until separate physical/governance approval. Static
verification SHALL not start, enable, pair, or reboot the Pi.

#### Scenario: Host verification runs

- **GIVEN** the broker transport code and unit are changed
- **WHEN** the OpenSpec verification command runs
- **THEN** it proves formatting, tests, lint, public-information checks, and
  absence of direct worker BlueZ access
- **AND** it does not claim live AirSense or H10 acceptance

### Requirement: The BLE broker SHALL have complete executable-code coverage

The broker's executable Rust code, including policy, protocol, client, server,
capability, cancellation, and Linux-owner decision seams, SHALL have 100%
line, function, and region coverage under LLVM source-based coverage. Tests
SHALL exercise success, rejection, timeout, cancellation, cleanup, generation,
backpressure, malformed input, and every named failure path. The coverage gate
SHALL run on Linux with all broker features and SHALL fail closed if an
executable line, function, or region is uncovered.

#### Scenario: A broker change leaves an untested path

- **GIVEN** a broker change introduces an executable line, function, or region
  without a test
- **WHEN** the broker coverage command runs
- **THEN** it exits nonzero and reports the uncovered item
- **AND** the change is not accepted or deployed

#### Scenario: A broker failure path is covered

- **GIVEN** a test injects a malformed frame, capability rejection, lease
  expiry, operation timeout, cancellation failure, BlueZ owner failure, worker
  disconnect, or adapter-generation change
- **WHEN** the broker handles that case
- **THEN** the test observes the named blocker and cleanup result
- **AND** the coverage report counts the exercised path

#### Scenario: Coverage runs without a physical radio

- **GIVEN** the host has no AirSense, H10, or Bluetooth controller
- **WHEN** the broker coverage gate runs
- **THEN** deterministic fake-owner and protocol seams exercise the broker
  logic without mocking a collector as a broker
- **AND** the gate does not claim physical BlueZ or Pi acceptance
