-- Revert vitals:aircraft_enrichment from pg
BEGIN;
DROP INDEX IF EXISTS aircraft_pending_lookup_idx;
ALTER TABLE aircraft DROP COLUMN IF EXISTS looked_up_at;
COMMIT;
