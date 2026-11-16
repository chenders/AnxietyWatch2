# Pi 5 Vitals Core

## ADDED Requirements

### Requirement: The Pi owns always-on radio and durable physiological ingest

The Pi SHALL host the always-on radio path and act as the physiological system of record. The iPhone
remains the HealthKit, journaling and viewing client; it SHALL NOT be required for continuous BLE
collection or durable ingestion.

#### Scenario: The phone is unavailable during collection

- **WHEN** the iPhone is powered off while a supported BLE source is producing measurements
- **THEN** the Pi continues collecting and durably spooling those measurements
- **AND** the phone's absence does not interrupt collection

### Requirement: Physiological samples use registry semantics and source provenance

The vitals core SHALL use `vitals/registry.toml` as the source of truth for source and type identity,
canonical units, valid ranges and thresholds. Decoders SHALL reject out-of-range values rather than
clamp them, and samples SHALL retain source identity and provenance through ingest.

#### Scenario: A decoded measurement is outside its declared range

- **WHEN** a decoder receives a measurement outside the registry-declared range
- **THEN** it rejects and quarantines the value with a named reason
- **AND** it does not publish a clamped value as a measurement

### Requirement: The durable spool precedes PostgreSQL ingestion

Collectors SHALL append rows in PostgreSQL binary COPY format to the durable spool, and the drainer
SHALL ingest through PostgreSQL binary COPY without changing source identity. A synced BP append SHALL
be durable before acknowledgement. No unresolved or absent value SHALL be coerced to zero.

#### Scenario: PostgreSQL is temporarily unavailable

- **WHEN** a collector receives a valid measurement while PostgreSQL is unavailable
- **THEN** the measurement is appended to the durable spool with its provenance intact
- **AND** it is available for replay when the database returns

### Requirement: The Pi storage layer has a verified recovery path

The PostgreSQL store SHALL reside on the NVMe-backed Pi system, and pgBackRest SHALL provide a
verified backup and restore path. The Pi core SHALL NOT claim the storage acceptance gate until a
restore verification has completed successfully.

#### Scenario: Restore verification fails

- **WHEN** the scheduled restore verification reports failure
- **THEN** storage recovery is not reported healthy and the blocking value is recorded

### Requirement: System health distinguishes missing evidence from healthy operation

The Pi core SHALL distinguish measured-and-good, measured-and-bad, and not-measured states. A missing
signal SHALL neither reassure nor escalate on its own.

#### Scenario: No recent source sample exists

- **WHEN** a liveness evaluation has no qualifying recent sample
- **THEN** the state is not-measured, not healthy and not unhealthy solely due to absence
