## ADDED Requirements

### Requirement: Supported XE125 bench acquisition
The bench acquisition tool MUST use a firmware and Exploration Tool combination that supports the
A121 sensor on the XM125 module, and MUST select the XE125 Enhanced serial port when connecting over
USB. A bench failure MUST be reported as a non-zero process result with a reason; it MUST NOT look
like a quiet capture.

#### Scenario: The board is connected through the supported serial port
- **WHEN** a compatible XM125 module is connected and the operator starts a capture on its Enhanced
  serial port
- **THEN** the tool establishes an A121 session or reports the connection error explicitly

#### Scenario: No board is available
- **WHEN** the requested serial port cannot be opened or the module does not report an A121 sensor
- **THEN** the tool exits non-zero and names the unavailable port or incompatible sensor

### Requirement: State-preserving breathing estimates
The software MUST preserve the Acconeer breathing reference application's state and MUST emit a
numeric respiratory-rate estimate only when a finite estimate is available while the application is
in `ESTIMATE_BREATHING_RATE`. `NO_PRESENCE_DETECTED`, `INTRA_PRESENCE_DETECTED`, and
`DETERMINE_DISTANCE_ESTIMATE` MUST remain explicitly non-estimate states.

#### Scenario: A stationary person is being analysed
- **WHEN** the reference application reports `ESTIMATE_BREATHING_RATE` and a finite rate
- **THEN** the output contains the rate in breaths per minute and the selected range context

#### Scenario: The person moves or disappears
- **WHEN** the application reports `INTRA_PRESENCE_DETECTED` or `NO_PRESENCE_DETECTED`
- **THEN** the output contains no respiratory-rate number and identifies the state

#### Scenario: The target distance is still being learned
- **WHEN** the application reports `DETERMINE_DISTANCE_ESTIMATE`
- **THEN** the output contains no respiratory-rate number and identifies that determination is in
  progress

### Requirement: Canonical rate and range validation
The registry MUST define `respiratory_rate` as the canonical `breaths_per_minute` measurement type.
Every durable ingest path MUST reject non-finite or out-of-registry estimates with a named reason;
it MUST never clamp an estimate into range. The initial engineering range is 1.0–120.0
breaths/minute and is not a statement of clinical normality.

#### Scenario: A finite in-range estimate is produced
- **WHEN** the estimate is finite and within the registry range
- **THEN** the value is eligible for a `RespiratoryRate` sample in canonical units

#### Scenario: An invalid estimate is produced
- **WHEN** the estimate is NaN, infinite, or outside the registry range
- **THEN** the value is quarantined with the reason and no numeric sample is emitted

### Requirement: Explicit unknown and quality state
Every emitted observation MUST include enough state to distinguish measured data from not measured
data. Missing, stale, motion-interrupted, or absent radar evidence MUST NOT be represented as zero
and MUST NOT be treated as reassuring evidence.

#### Scenario: The reference application has no valid rate
- **WHEN** a frame is received without a valid rate
- **THEN** the output serialises the rate as absent and includes the application state

#### Scenario: A consumer reads a non-estimate observation
- **WHEN** a consumer receives an observation whose state is not `ESTIMATE_BREATHING_RATE`
- **THEN** it treats the respiratory rate as unknown rather than healthy or alarming by default

### Requirement: Provenance and source identity
Durable samples MUST identify the source as `acconeer_xe125` and the type as `respiratory_rate`. The
source and type MUST be represented by generated registry enums at Rust call sites. Host arrival time
MUST NOT be used as the ingest identity. A production streaming collector MUST use a frame identity
or counter supplied by the module/protocol; until that is demonstrated stable across retransmission,
bench observations MUST remain non-durable evidence.

#### Scenario: A bench observation is written
- **WHEN** the bench runner emits an observation
- **THEN** it is clearly marked as bench evidence and does not claim a stable durable sample identity

#### Scenario: A production frame has no stable upstream identity
- **WHEN** a production collector cannot obtain a retransmission-stable frame identity
- **THEN** it refuses durable ingestion and reports the identity blocker

### Requirement: Replayable raw evidence
The acquisition path MUST support saving raw A121 sessions before or alongside processing so that
algorithm changes and decoder defects can be replayed without the person being present. The raw
recording MUST include the configuration and firmware context needed to interpret it.

#### Scenario: Processing fails after acquisition
- **WHEN** the processing layer fails after a raw frame was acquired
- **THEN** the raw recording remains available and the failure is reported separately from absence of
  breathing

### Requirement: Safety promotion is validation-gated
XE125 respiratory rate MUST remain outside the CNS escalation and clearing path until it passes a
positions, bedding, motion, no-presence, and failure cases. A successful software write or a numeric
spectral peak alone MUST NOT satisfy this gate.

#### Scenario: The validation gate is not complete
- **WHEN** a radar rate is available before the validation evidence is accepted
- **THEN** the system may record it as experimental data but MUST NOT let it escalate or clear an
  alarm tier

#### Scenario: The validation gate has accepted the source
- **WHEN** the documented validation evidence is accepted by the project owner
- **THEN** a separate change MAY promote the source according to an explicit alert-policy decision
