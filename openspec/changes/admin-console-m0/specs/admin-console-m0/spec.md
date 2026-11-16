# admin-console-m0 — foundations

## ADDED Requirements

### Requirement: Liveness is one pure function with three outcomes

The console SHALL evaluate source liveness in a single pure function that returns measured-good,
measured-bad, or not-measured, and SHALL never let a missing observation render as the good state.

#### Scenario: A source has no recent observation

- **GIVEN** a source whose last observation is older than its declared cadence expectation
- **WHEN** the liveness evaluator runs
- **THEN** it returns not-measured, carrying the age and the expectation that failed
- **AND** no caller renders that result as measured-good

### Requirement: The M0 HTTP surface is Rust and serves the domain-record envelope

The console SHALL serve the M0 read surface from the Rust axum service consuming `vitals-proto`, and
SHALL carry the §2.3b domain-record class in the §8b envelope. Bind and authentication SHALL default
to `127.0.0.1` for M0.

#### Scenario: M0 serves the envelope on the loopback default

- **WHEN** the M0 service starts without an explicit bind address
- **THEN** it binds `127.0.0.1` only
- **AND** the envelope it returns carries the domain-record class alongside the sample payload

### Requirement: Per-source cadence lives in the registry, not in console code

The console SHALL read per-source cadence from the generated registry types and SHALL NOT hardcode a
cadence, unit, or range for any source.

#### Scenario: A cadence changes in the registry

- **WHEN** a source's cadence value is changed in `vitals/registry.toml` and the registry is regenerated
- **THEN** the liveness expectation changes with it and no console source file is edited
