-- Revert vitals:status_observations from pg
BEGIN;
DROP TABLE IF EXISTS status_observations;
COMMIT;
