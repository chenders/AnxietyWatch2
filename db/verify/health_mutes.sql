-- Verify vitals:health_mutes on pg
BEGIN;

SELECT id, source_id, muted_by, muted_until, created_at FROM health_mutes WHERE FALSE;

ROLLBACK;
