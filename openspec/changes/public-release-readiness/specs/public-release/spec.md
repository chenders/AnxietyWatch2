## ADDED Requirements

### Requirement: The repository SHALL be published only after every ref it serves is clean
Visibility SHALL change to public only after a fresh clone from GitHub, with every branch and tag, passes the content rules, the residual-phrase check and a secrets scan, and GitHub has confirmed that cached pull-request refs were purged.

#### Scenario: The rewrite is pushed but pull-request refs are not yet purged
- **GIVEN** a clean rewritten history on every GitHub branch
- **AND** no confirmation from GitHub Support that `refs/pull/*` were purged
- **WHEN** publication is considered
- **THEN** publication is blocked, naming the missing confirmation

### Requirement: A fork SHALL NOT be able to reach the self-hosted runner
Because a fork PR runs its own workflow files on `pull_request`, the runner SHALL either be removed from the repository or every external contributor's run SHALL require approval, and the runner SHALL be offline whenever the repository is public without that policy in force.

#### Scenario: Visibility changes before the approval policy can be set
- **GIVEN** a private repository with a self-hosted runner
- **WHEN** visibility changes to public
- **THEN** the runner is offline until the fork-PR approval policy reads all external contributors

### Requirement: Default-branch workflows a fork can trigger SHALL NOT use the self-hosted runner
As defence in depth, any workflow on the default branch whose triggers include an event other than `push`, `workflow_dispatch` or `schedule` SHALL use only GitHub-hosted runners and SHALL NOT call a remote reusable workflow; CI SHALL reject a violation, naming the workflow, the job and the trigger.

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

### Requirement: GitHub-side data SHALL be clean before publication
Actions runs, artifacts and caches created before the rewrite SHALL be deleted, PR and issue text including review-comment diff hunks SHALL contain no removed value, and every sampled pre-rewrite commit SHA SHALL answer 404.

#### Scenario: A deploy log names the host
- **GIVEN** a workflow run whose log contains the Pi's hostname
- **WHEN** publication is considered
- **THEN** publication is blocked until the run is deleted
