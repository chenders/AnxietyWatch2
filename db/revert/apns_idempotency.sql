-- Revert vitals:apns_idempotency from pg
BEGIN;
DROP INDEX IF EXISTS apns_messages_idempotency_idx;
ALTER TABLE apns_messages DROP COLUMN IF EXISTS idempotency_key;
COMMIT;
