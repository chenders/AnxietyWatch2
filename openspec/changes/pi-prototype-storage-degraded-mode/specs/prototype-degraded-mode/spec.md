# Prototype storage degraded mode

## ADDED Requirements

### Requirement: Production baseline and degraded mode are named, separate states

The system SHALL expose a named production baseline and a separately named prototype degraded mode,
and SHALL never silently treat one as the other. Degraded mode SHALL be visible in health output for
as long as it is active.

#### Scenario: Degraded mode is entered

- **WHEN** the operator activates degraded mode
- **THEN** health output names the mode as degraded and the time it was entered
- **AND** production-baseline checks refuse to report green against the degraded configuration

### Requirement: An off-Pi recovery set is verified before any policy change

Before any ingest or drain policy is changed for degraded mode, the system SHALL produce and verify an
encrypted off-Pi recovery set. When Pi free space is constrained, recovery artifacts SHALL stream to
the Mac rather than stage an archive on the Pi.

#### Scenario: Free space is constrained

- **GIVEN** Pi free space is below the recovery-staging threshold
- **WHEN** the recovery set is produced
- **THEN** artifacts stream off-Pi and no archive is staged on the Pi's filesystem
- **AND** the verify step confirms the off-Pi set before any policy change is applied

### Requirement: Degraded mode preserves safety measurements and spool integrity

Degraded mode MAY reduce optional dashboard polling, cap drainer concurrency, and defer non-safety
writes while retaining their authenticated raw records in the spool. It MUST preserve BP, EMAY and
other safety-relevant measurements losslessly.

#### Scenario: A non-safety write is deferred

- **WHEN** degraded mode defers a non-safety write
- **THEN** the authenticated raw record remains in the spool
- **AND** no offset is advanced past it
- **AND** health names the deferred class and the pending count

### Requirement: No load-control measure may destroy or skip stored evidence

Degraded mode MUST NOT delete, concatenate, rewrite or manually skip spool segments, advance offsets,
update or delete rows from `samples`, coerce or clamp physiological measurements, run unresolved
migrations, or disable watchdog protection as a load-control measure.

#### Scenario: A forbidden load-control is attempted

- **WHEN** an operator attempts to delete a spool segment or advance an offset to free space
- **THEN** the action is refused and the blocker is named with the attempted value

### Requirement: Return to production is gated on evidence, not elapsed time

Returning to the production baseline SHALL require the gates and tests named by this change —
recovery-set currency, drain progress, resource bounds, and schema/Sqitch reconciliation — and
SHALL NOT be claimed by an observation period alone.

#### Scenario: An observation period is offered as production return

- **WHEN** the only evidence presented is that degraded mode has been running for some elapsed time
- **THEN** production return is refused and the missing gate is named
