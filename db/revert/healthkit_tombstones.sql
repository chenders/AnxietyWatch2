-- Revert vitals:healthkit_tombstones from pg
BEGIN;
DROP TRIGGER IF EXISTS healthkit_sample_tombstones_immutable ON healthkit_sample_tombstones;
DROP TABLE IF EXISTS healthkit_sample_tombstones;
DROP FUNCTION IF EXISTS reject_healthkit_tombstone_mutation();
COMMIT;
