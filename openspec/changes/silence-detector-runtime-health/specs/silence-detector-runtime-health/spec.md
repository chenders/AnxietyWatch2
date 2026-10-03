## ADDED Requirements

### Requirement: SQLx-compatible silence-detector database configuration
The silence detector SHALL receive a PostgreSQL URL accepted by SQLx and SHALL reject a libpq
keyword connection string before opening a pool.

#### Scenario: Correct local socket configuration starts
- **GIVEN** `DATABASE_URL` is a `postgres://` or `postgresql://` URL targeting the local PostgreSQL
  socket
- **WHEN** the daemon starts
- **THEN** SQLx receives the URL unchanged and the daemon proceeds to pool connection

#### Scenario: Keyword configuration fails closed
- **GIVEN** `DATABASE_URL` is `host=/var/run/postgresql user=vitals dbname=vitals`
- **WHEN** the daemon starts
- **THEN** it exits with configuration status 64 and names the required URL form
- **AND** it does not enter its database retry loop

### Requirement: Stable service health is required for activation
The deployment helper SHALL wait through the post-restart settling window and SHALL accept only
stable successful long-running services; completed `vitals-bp-autoconnect` oneshots MAY be inactive
only after successful completion.

#### Scenario: A service exits during restart settling
- **GIVEN** a selected service is initially restarted successfully
- **WHEN** it enters `activating`, `failed`, or a restart loop during the settling window
- **THEN** activation fails closed and the blocker names the service state, exit status, and restart
  count
- **AND** the existing rollback path runs

#### Scenario: A completed BP autoconnect oneshot is inactive
- **GIVEN** `vitals-bp-autoconnect` completed successfully
- **WHEN** its state is `inactive/dead` with exit status 0
- **THEN** activation does not reject it solely for being inactive

#### Scenario: Healthy services remain stable
- **GIVEN** each long-running target is `active/running` with a successful result and no restart
  count increase
- **WHEN** the settling window completes
- **THEN** activation may proceed to its existing receipt and rollback boundaries
