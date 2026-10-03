## ADDED Requirements

### Requirement: Sandboxed activation write boundary
The repository-scoped GitHub runner service SHALL permit the root-owned `vitals-deploy` helper to write only the existing release, binary, console-asset, systemd-unit, and udev-rule target roots required by its manifest, while retaining the runner's unprivileged account and narrow sudo command boundary.

#### Scenario: Protected activation reaches its allowlisted targets
- **GIVEN** a valid SHA-addressed ARM64 manifest, a current backup gate, and a healthy Pi
- **WHEN** the trusted `main` workflow invokes the root-owned deployment helper through the runner
- **THEN** the helper can create the staged release and atomically update its allowlisted live targets
- **AND** the runner service continues to retain `ProtectSystem=full`, `ProtectHome=yes`, and `PrivateTmp=yes`

#### Scenario: Runner cannot bypass the helper
- **GIVEN** the runner is active under `github-vitals`
- **WHEN** the runner account attempts to write a deployment target or invoke an unlisted root command
- **THEN** the direct write or elevation is rejected
- **AND** the spool, PostgreSQL data, boot firmware, and production credentials remain outside the runner write boundary

#### Scenario: Existing safety gates still block activation
- **GIVEN** the runner write boundary is active
- **WHEN** the manifest, backup gate, spool fingerprint, health evidence, or rollback precondition is invalid
- **THEN** `vitals-deploy` rejects or rolls back the activation with a reason naming the blocking value
- **AND** no deployment is reported as successful
