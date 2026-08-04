-- Revert vitals:registry_tables from pg
BEGIN;
DROP TABLE IF EXISTS vitals_source;
DROP TABLE IF EXISTS vitals_type;
COMMIT;
