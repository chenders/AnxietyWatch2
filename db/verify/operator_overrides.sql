-- Verify vitals:operator_overrides on pg
BEGIN;

SELECT source_id, display_name, updated_at FROM source_overrides WHERE FALSE;
SELECT type_id, display_name, unit, precision, updated_at FROM type_overrides WHERE FALSE;

ROLLBACK;
