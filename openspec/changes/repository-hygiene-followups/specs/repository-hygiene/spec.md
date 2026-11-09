## ADDED Requirements

### Requirement: The tracked tree SHALL contain no health data about a real person
Test fixtures, research notes, evidence ledgers and application placeholders SHALL NOT carry a real person's physiological readings, clinical history, medication regimen, or personal device and account identifiers. Where a fixture's value lies in a captured byte layout, the layout SHALL be kept and the readings substituted, and the fixture's provenance note SHALL say which.

#### Scenario: A decoder fixture captured from the owner
- **GIVEN** a frame captured from a device worn by the owner
- **WHEN** it is committed as a test fixture
- **THEN** flags, lengths, exponents, status and checksums are kept
- **AND** the readings are replaced by generated values that decode to what the test asserts
- **AND** the note does not call the substituted values real

#### Scenario: An application placeholder that looks like a regimen
- **GIVEN** a surface whose data source does not exist yet
- **WHEN** it renders
- **THEN** it shows an explicit not-available state
- **AND** never an invented list rendered like real data

### Requirement: Removal SHALL include history when the audience widens
Deleting sensitive material at the tip SHALL NOT be reported as removing it. Before the repository's audience widens, material deleted from the tip SHALL also be removed from every reachable ref, by the owner, because history rewriting of `main` is the owner's decision.

#### Scenario: A screenshot deleted in a later commit
- **GIVEN** a file removed from the tip but present in earlier commits on `main`
- **WHEN** the repository is about to be shared more widely
- **THEN** the owner decides on a history rewrite
- **AND** the decision, either way, is recorded

### Requirement: Every tracked unit SHALL have a planning owner or a live caller
Code, tools and design artifacts SHALL be owned by an openspec change or amendment, or be invoked by CI, a hook, a deploy unit or a Verify line. A unit with neither SHALL be removed or given an owner, not left to look current.

#### Scenario: An emulator no change owns
- **GIVEN** a firmware tree that no change references and no CI job builds
- **WHEN** the inventory finds it
- **THEN** the owner decides to own it in a change or remove it
