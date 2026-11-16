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

### Requirement: Every discovery and connection grant is a bounded, named lease

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

### Requirement: Raw-frame durability is preserved across the rewrite

The broker and its workers SHALL keep the existing raw-frame journal written before decode and SHALL
preserve `sync: true` for the BP path, so a decoder bug remains replayable and an unrecoverable frame
is not lost to an asynchronous write.

#### Scenario: BP frames are synced before the acknowledgement

- **WHEN** a BP frame is appended with `sync: true`
- **THEN** the append is durable before the acknowledgement returns

### Requirement: Failure is explicit and never blocks an unrelated source

Broker and worker failures SHALL produce explicit unknown or degraded states, and SHALL release
leases so an unrelated source is not blocked. Adapter generations SHALL be observable so a replaced
adapter is distinguishable from a silent failure.

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
