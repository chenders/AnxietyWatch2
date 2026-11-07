## ADDED Requirements

### Requirement: The mandated pre-PR reviewer SHALL be dispatchable against the tree that exists
The local reviewer that `AGENTS.md` requires before opening a pull request SHALL declare a trigger scope naming only directories present in the repository, SHALL reference only instruction files present in the repository, and SHALL assert the invariants of the implementation language actually in use.

#### Scenario: A decoder change triggers the reviewer
- **GIVEN** a diff that modifies a pure decoder under `crates/`
- **WHEN** the pre-PR dispatch decision is made from the reviewer's declared scope
- **THEN** the scope matches the diff and the reviewer is dispatched
- **AND** every instruction file it tells its reader to open resolves to a tracked path

#### Scenario: A docs-only change does not trigger the reviewer
- **GIVEN** a diff touching only Markdown, YAML, and shell under `docs/`, `.github/` and `scripts/`
- **WHEN** the pre-PR dispatch decision is made
- **THEN** the reviewer is not dispatched
- **AND** the reason recorded in the pull request cites the reviewer's own declared scope, not a scope attributed to `AGENTS.md`

#### Scenario: A scope naming an absent directory is a failure, not a skip
- **GIVEN** the reviewer's declared scope names a directory that is not present in the repository
- **WHEN** the review-gate check runs
- **THEN** the check exits nonzero and names the absent directory
- **AND** the condition is not reported as "no review required"

#### Scenario: The checklist matches the implementation language
- **GIVEN** the repository's decoders are Rust under `crates/` with `fuzz/` targets
- **WHEN** the reviewer's checklist is read
- **THEN** it asserts the Rust invariants this repository states — `get()` over indexing, no `unwrap()`/`expect()`/`panic!` outside tests, `no_std` and crate-boundary limits, `Option`/`Result` over a bool plus out-parameter
- **AND** it does not assert C-only constructs that the tree cannot contain

### Requirement: Tracked shell SHALL be statically analysed by a CI gate
Every shell file tracked in the repository, including the repository hooks, SHALL pass `shellcheck` at severity `warning` or stricter, enforced by a CI job whose exit code gates the pull request.

#### Scenario: A fail-open word-splitting guard is rejected
- **GIVEN** a tracked script iterates with `for e in $(ls -A "$dir")` inside a size or safety guard
- **WHEN** the shell analysis gate runs
- **THEN** the gate exits nonzero and names the file and line
- **AND** the pull request cannot merge on that commit

#### Scenario: A clean tree passes
- **GIVEN** every tracked shell file is clean at the configured severity
- **WHEN** the shell analysis gate runs
- **THEN** the gate exits zero
- **AND** the set of files analysed is derived from `git ls-files`, so a newly added script is covered without editing the workflow

#### Scenario: An unanalysable file is not silently skipped
- **GIVEN** a tracked shell file cannot be parsed by the analyser
- **WHEN** the gate runs
- **THEN** the gate exits nonzero and names the file
- **AND** the outcome is not reported as success
