## ADDED Requirements

### Requirement: Missing detector schema fails closed

The daemon SHALL validate the detector schema after connecting and SHALL exit with status 70 when
any required column or table is absent, naming the schema blocker.

#### Scenario: Required schema is absent
- **GIVEN** `status_observations.next_repeat_at` or `notification_events` is absent
- **WHEN** the daemon starts
- **THEN** it exits with status 70 and names the missing schema object
- **AND** systemd health cannot report a stable successful process

#### Scenario: Required schema is present
- **GIVEN** all detector columns and `notification_events` exist
- **WHEN** the daemon starts
- **THEN** it proceeds to its 30-second evaluation loop

## Requirement: Schema activation is explicit and bounded

The operator schema script SHALL require the verified backup gate and explicit confirmation, SHALL
apply only the checked-in `silence_detector` deploy SQL, SHALL verify the resulting objects, and SHALL
write a root-owned receipt. It SHALL not modify `samples`.

#### Scenario: Backup or confirmation is absent
- **GIVEN** either gate evidence or explicit confirmation is absent
- **WHEN** the script runs
- **THEN** it exits nonzero before PostgreSQL mutation and names the blocker

#### Scenario: Detector migration succeeds
- **GIVEN** the base table exists and no partial detector schema exists
- **WHEN** the script applies the migration
- **THEN** all detector objects verify and the receipt records success

## Requirement: Root helper replacement is operator-controlled

The helper installation procedure SHALL require explicit confirmation, an ARM64 ELF, an expected
SHA-256, and an atomic root-owned replacement while retaining a rollback copy. It SHALL not broaden
the GitHub runner sudoers command boundary.

## Requirement: Production acceptance precedes enablement

The service SHALL remain disabled until persistence, stable 30-second cycles, structured health, and
restart-count evidence pass. Enablement SHALL be an explicit final operator action.
