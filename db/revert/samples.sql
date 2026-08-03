-- Revert vitals:samples from pg
BEGIN;
DROP TABLE IF EXISTS samples_staging;
DROP TABLE IF EXISTS samples;
COMMIT;
