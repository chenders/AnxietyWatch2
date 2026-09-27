## ADDED Requirements

### Requirement: Consolidated task custody
The project MUST maintain one authoritative task contract for each task held by the
`99-last-specs` change. A migrated task MUST retain its original identifier, provenance, owner,
host, dependencies, amendment references, review requirement, and Verify command.

#### Scenario: T-B1 is moved into the consolidation change
- **WHEN** the Polar H10 hardware-check task is moved from `pi5-vitals-core`
- **THEN** `99-last-specs/tasks.md` is authoritative for T-B1 and the former task list points to it

#### Scenario: A task is present in a source checkout
- **WHEN** a task is discovered in another checkout and selected for consolidation
- **THEN** the migration ledger records the source path, original task id, current status, and
  destination status before the task is edited

### Requirement: Acceptance preservation during migration
Moving a task between OpenSpec changes MUST NOT weaken or silently replace its acceptance contract.
The destination MUST retain the task's executable Verify command, and a task MUST NOT be marked
complete unless that command exits zero.

#### Scenario: A migrated task has not been verified
- **WHEN** T-B1 has been moved but its hardware-check Verify command has not exited zero
- **THEN** T-B1 remains unchecked and is reported as incomplete

#### Scenario: A migrated task is completed
- **WHEN** the destination task's Verify command exits zero and required review is complete
- **THEN** the destination task may be checked off with the evidence retained in the destination

### Requirement: Explicit future migration
Each later task added to this consolidation change MUST preserve its source and MUST have a named
final destination before the consolidation change is archived.

#### Scenario: Another recovered task is added
- **WHEN** another checkout contributes a task for consolidation
- **THEN** the task is added as its own ledger entry and task block without merging its acceptance
  evidence with another task

#### Scenario: The holding change is closed
- **WHEN** all consolidated tasks have either moved to named destination changes or received an
  explicit rejected/deferred disposition
- **THEN** the ledger records that outcome and the holding change may be archived
