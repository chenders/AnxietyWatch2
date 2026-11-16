# Linux BlueZ broker

## ADDED Requirements

### Requirement: One adapter-wide BlueZ owner

Application-level BlueZ operations for one verified physical adapter SHALL have exactly one owner,
the `vitals-ble-broker`. Source workers SHALL retain protocol state, pure decoding, provenance and
durable ingest, and SHALL NOT open BlueZ sessions, start discovery, or mutate filters directly.

#### Scenario: A worker attempts a direct BlueZ operation

- **GIVEN** the broker is running for the adapter
- **WHEN** a source worker attempts a BlueZ discovery or filter mutation outside the broker
- **THEN** the operation is absent from the worker's call surface
- **AND** the only path to that operation is a broker lease

### Requirement: The broker protocol is versioned, private and bounded

The broker SHALL expose a versioned protocol over a private Unix socket. Every frame SHALL be
bounded, and each worker SHALL identify its kind, adapter selector, device selector and requested
capabilities during handshake. The broker SHALL reject a protocol-version mismatch or unsupported
capability, and SHALL expose no arbitrary D-Bus method call.

#### Scenario: A worker requests an unsupported capability

- **GIVEN** a worker requests a capability its declared worker kind does not support
- **WHEN** it handshakes with the broker
- **THEN** the broker rejects the request naming the unsupported capability
- **AND** it grants no lease

#### Scenario: A request exceeds its frame bound

- **WHEN** a worker sends a frame larger than the declared protocol limit
- **THEN** the broker rejects the frame and closes or resets that request without allocating an unbounded buffer

### Requirement: Backpressure is bounded and reported to the worker

Broker payloads and per-worker queues SHALL be bounded. A slow worker SHALL receive a named
backpressure result and release its lease rather than blocking established streams.

#### Scenario: A worker queue reaches its limit

- **WHEN** a worker cannot accept another frame because its bounded queue is full
- **THEN** the broker reports backpressure by name and releases the worker's lease
- **AND** other established streams continue

### Requirement: Adapter and device operations use bounded, observable leases

The broker SHALL grant discovery and connection admission as typed leases with a deadline and a
cooldown, and SHALL name the source holding each lease. A denial SHALL name the blocker and the
value that caused it.

#### Scenario: An absent optional source holds discovery

- **GIVEN** an optional source whose device is not present
- **WHEN** its lease deadline expires without a connection
- **THEN** the broker releases the lease and reports the source as absent with the deadline it exceeded
- **AND** a BP measurement arriving on kernel autoconnect is still delivered without waiting for that release

#### Scenario: A lease is refused

- **WHEN** a source requests a lease another source holds
- **THEN** the denial names the holding source, the remaining deadline, and the declared priority

### Requirement: BP connections are kernel-created and no broker path can dial them

The BP path SHALL rely on kernel accept-list and autoconnect registration. Neither the broker nor the
BP worker SHALL be able to issue `Device1.Connect` for BP.

#### Scenario: BP arrives without any application connect

- **WHEN** the cuff comes into range
- **THEN** the kernel creates the connection and the BP worker subscribes through the broker
- **AND** no `Device1.Connect` call for BP exists on any reachable path

### Requirement: Raw-frame durability is journaled before the broker acknowledges delivery

For the BP path the broker SHALL journal raw bytes durably before it acknowledges frame delivery as
accepted to the worker, and the worker SHALL perform its source append with `sync: true` and observe
the reply. These are two separate acknowledgements: the synced append does not cover the earlier
broker-to-worker crash gap. This narrows and exposes the ACK-to-journal gap; it does not claim the
kernel ATT confirmation is application durability.

#### Scenario: The broker accepts a BP frame

- **WHEN** the broker receives a BP raw frame it will deliver to the worker
- **THEN** the bytes are durably journaled before the broker acknowledges delivery to the worker

#### Scenario: BP frames are synced before the ingest acknowledgement

- **WHEN** the worker appends a BP frame with `sync: true`
- **THEN** the append is durable before that acknowledgement returns
- **AND** a decoder bug remains replayable from the raw-frame journal

### Requirement: Discovery filter state is restored, and failed restoration is degraded

Discovery SHALL record the owner, adapter generation, previous, requested and restored filters, the
start and end monotonic times, the deadline and the restoration result. A failed filter restoration
SHALL be reported as degraded state, not as a successful timeout.

#### Scenario: Filter restoration fails after discovery

- **GIVEN** a discovery lease has changed the adapter's filter set
- **WHEN** restoration of the previous filters fails
- **THEN** the broker reports degraded state naming the restoration failure
- **AND** the outcome is not recorded as a clean timeout

### Requirement: Adapter generations invalidate stale leases and sessions

The broker SHALL treat BlueZ restart, D-Bus loss, USB replug, controller reset and hotplug as adapter
generation changes. A generation change SHALL invalidate old leases and source sessions, and stale
`Connected` properties or an old lease SHALL NOT be treated as current health.

#### Scenario: The adapter is replugged

- **WHEN** the adapter changes generation while leases are held
- **THEN** the held leases and source sessions are invalidated
- **AND** consumers report unknown naming the generation change as the blocker

### Requirement: Failure is explicit and never blocks an unrelated source

Broker and worker failures SHALL produce explicit unknown or degraded states, and SHALL release
leases so an unrelated source is not blocked. Every failure SHALL be classified with its source,
attempt number, error class and value, and the next attempt's eligibility time.

#### Scenario: The broker dies mid-lease

- **WHEN** the broker process exits while holding a discovery lease
- **THEN** consumers report unknown naming the broker as the blocker
- **AND** no source reports measured-good
- **AND** restarting the broker re-establishes ownership on a new adapter generation

### Requirement: Migration is source-by-source with a tested rollback

Each source SHALL migrate to the broker individually, and SHALL have a rollback that never leaves the
old and new BlueZ owners running concurrently. Wellue SHALL remain disabled and H10 SHALL remain
unaccepted until fresh physical measurements pass its existing gate.

#### Scenario: A migrated source is rolled back

- **WHEN** a source rollback is executed
- **THEN** the previous owner is restored with the new owner stopped first
- **AND** coexistence acceptance records packet gaps, notification latency and reconnects before the
  source is declared healthy
