-- Verify vitals:source_declarations on pg
BEGIN;

SELECT source_id, is_configured, updated_at FROM source_declarations WHERE FALSE;

ROLLBACK;
