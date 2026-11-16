# admin-console-m0 — foundations

## ADDED Requirements

### Requirement: Liveness is one pure function with three outcomes

The console SHALL evaluate source liveness in a single pure function that returns measured-good,
measured-bad, or not-measured, and SHALL never let a missing observation render as the good state.

#### Scenario: A source produces no qualifying evidence at all

- **GIVEN** a source with no attempt evidence, no sample evidence, and no transport contact
- **WHEN** the liveness evaluator runs
- **THEN** it returns not-measured
- **AND** no caller renders that result as measured-good

### Requirement: Sample staleness is judged against declared arrival bounds, never cadence

Liveness SHALL judge sample staleness against the per-source `max_sample_arrival_s` and
`sample_silence_failure_s` declared in `vitals/registry.toml`, and SHALL NOT derive a sample-arrival
verdict from `expected_cadence_s`. A source with no declared arrival bound SHALL get no
sample-derived verdict at all rather than falling back to the nearest available number.

#### Scenario: A source declares no arrival bound

- **GIVEN** a source with `expected_cadence_s` declared and no `max_sample_arrival_s`
- **WHEN** the sample channel is evaluated
- **THEN** the sample channel is silent and produces no verdict
- **AND** no verdict is derived from `expected_cadence_s`

#### Scenario: Measured sample silence past a declared bound while not idle

- **GIVEN** a source declaring `sample_silence_failure_s` whose activity is `Expected` or `Unknown`
  (not observed `Idle`)
- **WHEN** the newest stored sample is older than that bound
- **THEN** the source is measured-bad and the reason names the age and the bound it exceeded

### Requirement: Idle suppresses a sample-silence verdict only when idle is observed

The evaluator SHALL treat the activity state as `Expected`, `Idle` or `Unknown`. Only an observed
`Idle` SHALL suppress a declared sample-silence bound (`sample_silence_failure_s`). `Unknown` SHALL
NOT suppress a sample-silence bound, and SHALL fall back to the registry declaration for that source.
An observed `Idle` SHALL NOT suppress a transport-link silence bound (`max_link_silence_s`); that is
the only bound permitted to escalate while a source is idle.

#### Scenario: Activity state is unknown

- **GIVEN** a source declaring `sample_silence_failure_s` that reports no activity
- **WHEN** its sample-silence bound is exceeded
- **THEN** the sample-silence verdict fires from the declared bound
- **AND** the unknown activity state does not veto it

#### Scenario: Observed Idle suppresses sample silence only

- **GIVEN** a source declaring `sample_silence_failure_s` whose activity is observed `Idle`
- **WHEN** the newest stored sample is older than that bound
- **THEN** the sample-silence verdict is suppressed
- **AND** a declared `max_link_silence_s` may still escalate if its own bound is exceeded

### Requirement: A future-dated timestamp is rejected, never clamped

A sample or contact timestamp more than one second ahead of `now` SHALL be rejected and named, not
clamped to a plausible age, and SHALL NOT be absorbed into a healthy state.

#### Scenario: A clock fault produces a future timestamp

- **WHEN** a stored sample carries a timestamp more than one second ahead of `now`
- **THEN** the value is rejected with the clock fault named
- **AND** the source is not reported healthy on that value

### Requirement: The M0 HTTP surface is Rust and serves the domain-record envelope

The console SHALL serve the M0 read surface from the Rust axum service consuming `vitals-proto`, and
SHALL carry the §2.3b domain-record class in the §8b envelope. M0 SHALL bind the loopback address
`127.0.0.1` only, and SHALL NOT be reachable on a non-loopback interface.

#### Scenario: M0 serves the envelope on the loopback default

- **WHEN** the M0 service starts without an explicit bind address
- **THEN** it binds `127.0.0.1` only and refuses a non-loopback listener
- **AND** the envelope it returns carries the domain-record class alongside the sample payload

### Requirement: Authentication follows amendment 09, not a bind address

The M0 HTTP surface SHALL NOT treat its bind address as an authentication decision. The binding
authentication requirement for client-facing HTTP remains amendment 09, and M0 SHALL NOT claim an
authentication posture amendment 09 governs and this change does not implement.

#### Scenario: M0 is described as authenticated because it binds loopback

- **WHEN** a reader treats the loopback bind as satisfying client-facing authentication
- **THEN** the claim is rejected and amendment 09 is named as the governing requirement

### Requirement: Per-source cadence lives in the registry, not in console code

The console SHALL read per-source cadence and the declared liveness bounds from the generated
registry types and SHALL NOT hardcode a cadence, bound, unit, or range for any source.

#### Scenario: A cadence changes in the registry

- **WHEN** a source's cadence value is changed in `vitals/registry.toml` and the registry is regenerated
- **THEN** the liveness expectation changes with it and no console source file is edited
