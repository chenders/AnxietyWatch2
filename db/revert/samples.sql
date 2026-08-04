-- Revert vitals:samples from pg
BEGIN;
DROP TABLE IF EXISTS spool_status;
DROP TABLE IF EXISTS samples;
COMMIT;
