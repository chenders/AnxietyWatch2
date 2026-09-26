-- Revert vitals:operator_overrides from pg
BEGIN;

DROP TABLE type_overrides;
DROP TABLE source_overrides;

COMMIT;
