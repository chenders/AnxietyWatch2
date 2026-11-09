## ADDED Requirements

### Requirement: A backup copy SHALL be recoverable, not merely present
A copy of an encrypted repository SHALL be counted as a copy only where the key that decrypts it survives the loss of the device holding that copy. Counting a repository and its own decryption key as independent copies SHALL be treated as counting one.

#### Scenario: A copy whose key shares its device is not counted
- **GIVEN** an encrypted repository and its plaintext passphrase on one filesystem
- **WHEN** independent copies are counted
- **THEN** the recoverable count is zero, not one
- **AND** the gap is recorded rather than raised as urgent, because the threat model is a single-user LAN prototype

### Requirement: A scheduled durability job SHALL NOT run against an unsynchronised clock
Any timer whose output is labelled or aged by wall-clock time SHALL be ordered after time synchronisation.

#### Scenario: A cold boot with a dead RTC
- **GIVEN** the real-time clock reports 1970-01-01 at boot
- **AND** a backup timer is `Persistent=true`
- **WHEN** the unit is ordered after time synchronisation
- **THEN** it does not fire until the clock is correct
- **AND** no backup label sorts before every genuine one

### Requirement: A monitor SHALL have a reader
A check that writes an alert artifact SHALL have a consumer that surfaces it. An alert path whose only other reference is the statement that clears it SHALL be treated as absent.

#### Scenario: A restore verification fails and nobody is told
- **GIVEN** restore verification fails and writes an alert file
- **WHEN** no process reads that path
- **THEN** the monitor is not a monitor
- **AND** the failure can persist for the full staleness window unobserved

### Requirement: A table documented as append-only SHALL be enforced as append-only
Where immutability is described as enforced by the database, the triggers or constraints that reject mutation SHALL exist, and their absence SHALL fail verification.

#### Scenario: Immutability claimed but unenforced
- **GIVEN** an audit table whose documentation states it is append-only
- **AND** the rejecting triggers do not exist
- **WHEN** the change's verify script runs
- **THEN** it fails and names the missing functions and triggers

### Requirement: A verify script SHALL assert structure, not mutable state
A sqitch verify script SHALL assert the presence and shape of schema objects. It SHALL NOT assert a seed value that legitimate operation changes.

#### Scenario: A seed value that has legitimately changed
- **GIVEN** a verify script asserting a configuration row still holds its initial value
- **AND** the operator has since changed that value through the supported path
- **WHEN** verification runs against a correctly deployed change
- **THEN** it must pass, because the schema is correct
- **AND** a script that fails here is asserting the wrong thing

### Requirement: A deployed collector SHALL be rebuildable from tracked source
Every binary that writes to the sample store SHALL be built from source in this repository and installed through the allowlisting installer.

#### Scenario: A collector binary with no source
- **GIVEN** a running collector whose binary is built by no tracked source
- **WHEN** it needs a fix
- **THEN** it cannot be patched or rebuilt, and the data path it feeds is unmaintainable

#### Scenario: A collector whose device is absent
- **GIVEN** a collector whose device only advertises while in use
- **WHEN** the device is absent
- **THEN** it reports absence as a steady named state and backs off
- **AND** it does not crash-loop while reporting active
