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

### Requirement: Runtime executable and service-state acceptance
The deployment manifest SHALL assign mode `0755` to every deployed service binary independent of the runner process umask, and activation SHALL reject a post-restart snapshot containing a failed or unknown service state.

#### Scenario: Restricted runner umask does not make a service binary unexecutable
- **GIVEN** the runner uses a restrictive umask while building and downloading an artifact
- **WHEN** the manifest is generated and the staged artifact is restored for activation
- **THEN** each binary destination has mode `0755`
- **AND** the service user can execute its selected binary

#### Scenario: A selected service fails after restart
- **GIVEN** activation has installed the release and restarted its selected services
- **WHEN** the post-activation snapshot reports a service state beginning with `failed` or `unknown:`
- **THEN** the helper records activation as unsuccessful and attempts rollback
- **AND** the receipt names the failed or unknown state rather than reporting health success
