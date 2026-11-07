## ADDED Requirements

### Requirement: An amendment citation SHALL resolve to exactly one amendment
Every citation of the form `Amendment NN` in a tracked file SHALL name a number present in
`docs/specs/AMENDMENTS.md`, and SHALL identify which amendment it means whenever that number is
shared by a suffixed sibling in the index. A gate SHALL enforce both conditions by exit code, in
the pre-push hook and in CI.

#### Scenario: A bare citation of a colliding number is rejected
- **GIVEN** the index contains both `10` and `10-A11`
- **AND** a tracked file says `Added Amendment 10: AirSense live acceptance gates only one path`
- **WHEN** the amendment-reference gate runs
- **THEN** the gate exits nonzero
- **AND** it names both candidate amendments and their titles
- **AND** it names the file and line

#### Scenario: Context that identifies the amendment is accepted
- **GIVEN** the index contains both `10` and `10-A11`
- **AND** a tracked file says `Amendment 10 permits first-draft private-LAN HTTP`
- **WHEN** the gate runs
- **THEN** the citation passes, because the surrounding text carries the amendment's title words
- **AND** a link to the amendment's file is equally sufficient

#### Scenario: A suffixed citation is already unambiguous
- **GIVEN** a tracked file cites `Amendment 10-A11`
- **WHEN** the gate runs
- **THEN** the citation passes without additional identification
- **AND** it is not read as a citation of `Amendment 10`

#### Scenario: A bare citation of a non-colliding number stays bare
- **GIVEN** the index contains `12` and no suffixed sibling of it
- **AND** a tracked file says `See Amendment 12 for background`
- **WHEN** the gate runs
- **THEN** the citation passes
- **AND** the gate does not require a title or link, because a bare number is the established form
  for cross-references and a rule demanding otherwise would flag 111 call sites

#### Scenario: A citation naming no indexed amendment is rejected
- **GIVEN** a tracked file cites `Amendment 13` while the index has no row for `13`
- **WHEN** the gate runs
- **THEN** the gate exits nonzero
- **AND** it reports that the index has no row for that number, and lists the numbers it does have

#### Scenario: An index pointing at an unreadable amendment fails rather than passes
- **GIVEN** the index lists an amendment whose document is absent or undecodable
- **WHEN** the gate runs
- **THEN** citations of that amendment exit nonzero
- **AND** the message states that the citation could not be checked, which is not the same as being
  correct
- **AND** if no amendment document is readable at all, the gate exits 2 rather than reporting clean

### Requirement: A citation attributing a requirement to an amendment SHALL be reported when the amendment does not discuss it
The gate SHALL report, without changing its exit status, every citation that asserts the cited
amendment requires, provides, forbids or permits something while sharing fewer than two distinctive
words with that amendment's text. Distinctiveness SHALL be computed from the corpus and SHALL NOT
depend on the number of amendments present.

#### Scenario: A requirement attributed to an unrelated amendment is reported
- **GIVEN** `crates/vitals-console/src/api/air11.rs` says an endpoint must not finalize a therapy
  session because `Amendment 12` "does not yet provide a durable therapy-session record"
- **AND** Amendment 12 is *Provider-aware song search and provider-independent identity*
- **WHEN** the gate runs
- **THEN** the citation appears in the advisory list with Amendment 12's indexed title
- **AND** the exit status is unchanged by its presence

#### Scenario: A terse citation is not treated as a defect
- **GIVEN** a task header reads `**Amends:** Amendment 05`
- **WHEN** the gate runs
- **THEN** the advisory message states that a terse citation is one of the possible explanations
- **AND** the gate does not exit nonzero, because vocabulary cannot distinguish a terse correct
  citation from a wrong one

#### Scenario: Distinctiveness does not depend on corpus size
- **GIVEN** a word appears in two amendments out of eight, and in neither of their titles
- **WHEN** the corpus is built
- **THEN** that word is excluded from both amendments' distinctive vocabulary
- **AND** a word appearing in exactly one amendment is included
- **AND** a word in an amendment's own title is included even if it is widely used

#### Scenario: A supported requirement is not reported
- **GIVEN** a tracked file says `the provider catalogue song search identity required by Amendment 12`
- **WHEN** the gate runs
- **THEN** the citation does not appear in the advisory list

### Requirement: The pre-push hook SHALL run every gate CI requires
`.githooks/pre-push` SHALL fail on any condition that would fail `.github/workflows/ci.yml`, so its
success message cannot contradict CI's verdict.

#### Scenario: A rustfmt-dirty tree cannot push green
- **GIVEN** a tracked Rust file that `cargo fmt --all --check` would reformat
- **WHEN** the pre-push hook runs
- **THEN** the hook exits nonzero and names the file
- **AND** it does not print that all gates passed
