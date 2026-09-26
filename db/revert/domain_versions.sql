-- Revert vitals:domain_versions from pg
BEGIN;

DROP TABLE IF EXISTS domain.record_links;
DROP INDEX IF EXISTS domain.domain_records_recorded_idx;
DROP INDEX IF EXISTS domain.domain_records_tombstone_idx;
DROP INDEX IF EXISTS domain.domain_records_supersedes_idx;

ALTER TABLE domain.records
    DROP CONSTRAINT IF EXISTS domain_records_tombstone_not_version,
    DROP CONSTRAINT IF EXISTS domain_records_not_self_tombstone,
    DROP CONSTRAINT IF EXISTS domain_records_not_self_version,
    DROP COLUMN IF EXISTS actor,
    DROP COLUMN IF EXISTS tombstone_target_id,
    DROP COLUMN IF EXISTS supersedes_id,
    DROP COLUMN IF EXISTS recorded_at,
    DROP COLUMN IF EXISTS schema_version;

COMMIT;
