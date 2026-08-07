## ADDED Requirements

### Requirement: Every remaining design commitment has an executable owner

The project SHALL map every normative admin-console design commitment to a queued task, an explicit
external owner, or an operator decision gate. A queued task SHALL declare `Amends:`, `Reads:`, `Do:`,
and `Verify:` fields and SHALL be accepted only by its verification command exiting zero.

#### Scenario: A worker selects the next implementation task

- **WHEN** the conductor selects an unchecked, unblocked task from the roadmap
- **THEN** the task identifies its governing amendments and scoped source files
- **AND** the task states the implementation outcome and an executable acceptance command
- **AND** the conductor can dispatch it without inferring requirements from the design document

#### Scenario: A design commitment is not implemented in the console

- **WHEN** a design commitment belongs to a collector, phone, alarm daemon, or later platform plan
- **THEN** the traceability document names that external owner
- **AND** the commitment is not silently omitted or duplicated with a conflicting console contract

### Requirement: Generation-1 behaviour is an explicit, subordinate reference

The roadmap SHALL preserve a scoped inventory of useful generation-1 admin workflows and map each
one to a generation-2 task owner or an explicit drop. The reference SHALL NOT override an accepted
amendment, the generation-1 reassessment verdicts, or generation-2 security and data-integrity rules.

#### Scenario: A worker implements a successor admin workflow

- **WHEN** a queued task succeeds a generation-1 admin feature
- **THEN** its `Reads:` field identifies the relevant reference row and narrowly scoped legacy source
- **AND** the worker can reproduce useful operator behaviour without porting Flask, old schema
  assumptions, or known security and lifecycle defects

#### Scenario: A legacy feature was intentionally dropped

- **WHEN** the reference marks a generation-1 behaviour as dropped
- **THEN** no successor task silently reintroduces it
- **AND** the decommission ledger records the drop and governing rationale

### Requirement: Dependencies and judgment gates remain explicit

The roadmap SHALL prevent work from being dispatched before its substrate or operator decision
exists. An agent SHALL NOT resolve an operator-judgment gate by inference.

#### Scenario: A task depends on an unresolved architecture contradiction

- **WHEN** implementation depends on the unresolved Timescale hypertable and sample-identity conflict
- **THEN** the task remains marked blocked
- **AND** an amendment decision task precedes implementation
- **AND** no worker changes sample identity or adds timestamp to the key as a silent workaround

#### Scenario: Layout pruning needs a human choice

- **WHEN** all layout directions are available for live use but no determination is recorded
- **THEN** the pruning task remains blocked
- **AND** no agent chooses a direction from mockups or code inspection

### Requirement: Milestone acceptance remains shell-verifiable

Each milestone SHALL end in an acceptance task that covers its milestone-specific criteria and the
cross-cutting accessibility and regression criteria that have become applicable.

#### Scenario: A milestone is reported complete

- **WHEN** the conductor reports M2, M3, M4, M5, or M6 complete
- **THEN** that milestone's acceptance script has been run by the conductor
- **AND** its exit status is zero
- **AND** all prerequisite task verification commands have exited zero

### Requirement: Unknown evidence fails closed throughout the queue

Every implementation task that renders or acts on status evidence SHALL preserve the design's
three-state law: missing, malformed, stale, or never-tested evidence cannot be represented as
successful.

#### Scenario: Evidence is absent or malformed

- **WHEN** a service, backup, notification channel, telemetry exporter, or alarm channel lacks valid
  fresh evidence
- **THEN** the resulting health assertion is `UNKNOWN` with a named reason and threshold
- **AND** the UI does not synthesize a successful value

### Requirement: State-changing operations are bounded and auditable

Every state-changing operation queued by this roadmap SHALL be authenticated, CSRF-protected,
allowlisted, proportionately confirmed, recorded before execution, and written to an immutable audit
trail with actor and timestamp.

#### Scenario: A service operation is requested

- **WHEN** an operator requests restart or stop for a managed unit
- **THEN** the backend validates authentication, CSRF, operation capability, action, unit allowlist,
  and confirmation class before execution
- **AND** an attempt exists before execution begins
- **AND** the final result and audit entry identify actor, target, action, timestamp, and outcome
- **AND** arbitrary commands and systemctl verbs are impossible through the route
