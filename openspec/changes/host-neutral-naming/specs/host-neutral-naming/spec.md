## ADDED Requirements

### Requirement: Reusable host interfaces SHALL use hardware-neutral names
Canonical names for reusable Linux monitoring-host helpers, configuration variables, and
active plan/display directories SHALL describe their function rather than require a
particular board. Actual operating-system, architecture, and dependency requirements SHALL
remain explicit. Hardware-neutral naming SHALL NOT be presented as platform validation.

#### Scenario: A reusable helper has a legacy Pi name
- **WHEN** the inventory establishes that the helper is not board-specific
- **THEN** its canonical name uses host-neutral terminology
- **AND** existing supported callers continue to work through a documented compatibility path
- **AND** no Raspberry Pi requirement is inferred from the canonical interface

### Requirement: Renaming SHALL preserve safety and caller compatibility
The migration SHALL preserve artifact destination allowlists, regular-file and owner/mode
checks, `/dev/null` character-device verification, finite acceptance durations, explicit
operator authorization, credential handling, and opt-in service lifecycle boundaries.
Legacy variable aliases SHALL reject conflicting old/new values with a named blocker.
Compatibility wrappers SHALL delegate to the same guarded implementation, not duplicate it.

#### Scenario: Old and new host variables disagree
- **WHEN** a caller supplies conflicting legacy and canonical host/path variables
- **THEN** the helper exits nonzero before any remote operation
- **AND** identifies the conflicting variable names without exposing credentials

#### Scenario: An existing installer caller uses the old script name
- **WHEN** the legacy entry point is invoked
- **THEN** it reaches the canonical allowlisted implementation
- **AND** retains all destination, file-type, permission, and device-node safeguards

### Requirement: Active references SHALL remain resolvable without falsifying history
Moves SHALL update current documentation, hooks, CI, tests, and task read lists together.
Existing exact Verify commands SHALL be preserved through compatibility paths or changed only
through the repository's explicit Verify-change decision procedure. Accepted amendment text
and historical hardware evidence SHALL NOT be rewritten as if measured on other hardware.

#### Scenario: A task points to a moved planning directory
- **WHEN** the active directory receives a neutral name
- **THEN** current links and read lists resolve
- **AND** task IDs, checkbox state, and acceptance evidence remain intact
- **AND** any legacy Verify command still works or has a separately justified recorded change

#### Scenario: A reference describes Raspberry Pi firmware or measured hardware
- **WHEN** the inventory classifies it as genuinely board-specific or historical
- **THEN** its hardware identity is retained and documented as an intentional exception
- **AND** the generic software is not described as requiring that board
