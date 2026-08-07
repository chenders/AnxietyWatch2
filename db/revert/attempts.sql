-- Revert vitals:attempts from pg
BEGIN;
DROP TABLE IF EXISTS attempts;
COMMIT;
