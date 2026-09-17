-- Revert vitals:healthkit_tombstones from pg
BEGIN;
DROP TABLE healthkit_sample_tombstones;
COMMIT;
