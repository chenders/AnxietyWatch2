## ADDED Requirements

### Requirement: The repository SHALL be published only after every ref it serves is clean
Visibility SHALL change to public only after a fresh clone from GitHub, with every branch and tag, passes the content rules, the residual-phrase check and a secrets scan, and GitHub has confirmed that cached pull-request refs were purged.

#### Scenario: The rewrite is pushed but pull-request refs are not yet purged
- **GIVEN** a clean rewritten history on every GitHub branch
- **AND** no confirmation from GitHub Support that `refs/pull/*` were purged
- **WHEN** publication is considered
- **THEN** publication is blocked, naming the missing confirmation

### Requirement: A workflow that a fork can trigger SHALL NOT run on a self-hosted runner
Any workflow whose triggers include an event other than `push`, `workflow_dispatch` or `schedule` SHALL use only GitHub-hosted runners, and CI SHALL reject a change that violates this, naming the workflow, the job and the trigger.

#### Scenario: A new workflow targets the Pi from pull requests
- **GIVEN** a workflow triggered by `pull_request` with a job on `[self-hosted, …]`
- **WHEN** CI runs the runner gate
- **THEN** the gate fails and names the workflow file, the job and `pull_request`

### Requirement: The rewritten history SHALL satisfy the main ruleset as it is configured
The force-push SHALL either carry verified signatures on every new commit, or be made under a ruleset change the owner made for that push and reversed immediately after. The plan SHALL NOT assume a rule the ruleset does not contain.

#### Scenario: The ruleset requires signatures and the rewrite stripped them
- **GIVEN** `required_signatures` active on `main` with no bypass actor
- **AND** a rewritten mirror whose commits are unsigned
- **WHEN** the push is prepared
- **THEN** it is not attempted until the owner has chosen re-signing or a lifted rule
