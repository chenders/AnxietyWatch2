## ADDED Requirements

### Requirement: A stored sample's type SHALL resolve through a per-source map, not through its raw id
`samples.type` is a global id namespace that two writers populated under different schemes, so the id alone does not determine meaning. Every reader SHALL resolve `(source, type, timestamp)` through `sample_type_resolution` to a canonical type name and the unit the value was written in. A reader SHALL NOT join `vitals_type` on `samples.type` alone.

#### Scenario: A colliding id resolves to the writer's meaning
- **GIVEN** a stored row with `source = 5` and `type = 10`
- **AND** `vitals_type` declares id 10 as `resting_hr` in `bpm`
- **WHEN** the row is read through `samples_resolved`
- **THEN** `canonical_name` is `patient_flow`
- **AND** `stored_unit` is `L/s`
- **AND** `resolved` is true

#### Scenario: The same id resolves differently for a different source
- **GIVEN** a stored row with `source = 2` and `type = 10`
- **WHEN** the row is read through `samples_resolved`
- **THEN** `canonical_name` is `resting_hr`
- **AND** the source-5 row at the same id is unaffected

#### Scenario: An unmapped pair is reported unknown, never assumed
- **GIVEN** a stored row whose `(source, type)` has no resolution entry
- **WHEN** the row is read through `samples_resolved`
- **THEN** `resolved` is false
- **AND** `canonical_name` is null
- **AND** the row is still returned, because absence of a mapping is the third state and not grounds for hiding a measurement

#### Scenario: A reader that bypasses the map fails the build
- **GIVEN** a change that adds `JOIN vitals_type ON vitals_type.id = s.type`
- **WHEN** CI runs
- **THEN** the build fails, because a confidently wrong answer must be unrepresentable rather than discouraged

### Requirement: The resolution map SHALL be derived from each row's own attestation
A mapping SHALL record what evidence established it, how many rows carry that evidence, and the total rows affected. A mapping SHALL NOT be hand-written where the rows attest their own meaning.

#### Scenario: A fully attested mapping
- **GIVEN** all 110,651,486 source-5 rows carry `extra.signal`
- **AND** the `type → signal` relation is one-to-one with zero deviations
- **WHEN** the map is populated
- **THEN** each source-5 entry records `attested_by = 'extra.signal'`
- **AND** `attested_rows` equals `total_rows`

#### Scenario: A weakly attested mapping is labelled as such
- **GIVEN** source 4's rows carry no signal field in `extra`
- **WHEN** its mapping is populated
- **THEN** `attested_rows` is 0
- **AND** `attested_by` names the declaration-level evidence rather than claiming row-level attestation

### Requirement: A type id's declared meaning SHALL NOT be overwritten while rows depend on it
Reconciling the lookup tables SHALL fail, naming the id, both names, both units and the affected row count, unless every affected `(source, stored_type)` pair already resolves and the recorded unit and range match what is stored.

#### Scenario: Relabelling is refused without coverage
- **GIVEN** `vitals_type` holds id 10 as `patient_flow` in `L/s` with rows present
- **AND** no resolution entry covers `(5, 10)`
- **WHEN** the generated reference data is applied
- **THEN** the statement raises and names id 10, `patient_flow`, `L/s`, `resting_hr`, `bpm` and the row count
- **AND** `vitals_type` is unchanged

#### Scenario: Relabelling proceeds once every affected pair resolves
- **GIVEN** every `(source, type)` pair present in `samples` has a resolution entry whose stored unit and range match `vitals_type`
- **WHEN** the generated reference data is applied
- **THEN** it succeeds
- **AND** reading any historical row through `samples_resolved` yields the same canonical name as before

### Requirement: A value outside its declared range SHALL be quarantined by predicate, not removed or coerced
Rows failing a declared range SHALL be excluded at read time through a recorded predicate carrying a stated reason. They SHALL NOT be deleted, updated, or clamped into range.

#### Scenario: A sentinel leaking as a number is quarantined
- **GIVEN** 277 source-4 `respiratory_rate` rows between 0.2 and 0.9 against a declared minimum of 1.0
- **AND** no accumulation at exactly 1.0, showing the range check was absent rather than clamping
- **WHEN** the quarantine is declared
- **THEN** a predicate entry records the reason and the row count
- **AND** all 52,059 rows remain present and readable
