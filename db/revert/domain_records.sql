-- Revert vitals:domain_records from pg
BEGIN;
DROP TABLE IF EXISTS domain_records;
COMMIT;
