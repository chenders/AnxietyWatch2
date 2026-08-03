-- Revert vitals:extensions from pg
-- Deliberately does NOT drop timescaledb: dropping it would destroy every
-- hypertable in the database, which is not a schema revert but data loss.
-- Reverting this change is a no-op; remove the extension by hand if you mean it.
BEGIN;
COMMIT;
