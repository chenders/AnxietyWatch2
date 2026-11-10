## ADDED Requirements

### Requirement: Sensitive material SHALL be removed from history, not only from the tip
Material that violates the repository's PII and PHI rules SHALL be removed from every commit reachable from any pushed ref, because a clone delivers all of them. Deleting it in a later commit SHALL NOT be reported as removing it.

#### Scenario: A screenshot added in one commit and deleted in a later one
- **GIVEN** a file present in an earlier commit of `main` and absent at the tip
- **WHEN** the repository is cloned
- **THEN** the clone contains the file
- **AND** the file counts as present until history is rewritten

### Requirement: A history rewrite SHALL be proven before it is published
A rewrite SHALL be performed on a mirror, never in a working checkout, and SHALL NOT be pushed until it is shown that no blob in any ref fails the content rules, no commit contains a removed path, and the final tree of `main` is unchanged.

#### Scenario: The rewrite would alter current code
- **GIVEN** a replacement pattern that also matches content in `main`'s final tree
- **WHEN** the rewritten `main` tree hash is compared with the original
- **THEN** the hashes differ
- **AND** the push is blocked until the pattern is narrowed or the tip is fixed first

### Requirement: The values being removed SHALL NOT be written into the repository
The manifest of paths and text to remove SHALL be kept outside the repository. Plans, specs and commit messages SHALL describe sensitive values, never quote them.

#### Scenario: Writing the purge plan
- **GIVEN** a list of identifiers to replace
- **WHEN** the plan is committed
- **THEN** it names their kind and count only
- **AND** the list itself lives outside the tree

### Requirement: Every commit a pull request introduces SHALL pass the content and path gates
Because merges preserve branch commits, the gates SHALL check each commit a pull request adds, not only its head.

#### Scenario: Add-then-delete inside one pull request
- **GIVEN** a branch that adds a screenshot and deletes it two commits later
- **WHEN** CI runs on the pull request
- **THEN** the commit that added it fails the path gate
