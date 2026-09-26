-- Deploy vitals:apns_idempotency to pg
BEGIN;
ALTER TABLE apns_messages ADD COLUMN idempotency_key UUID;
CREATE UNIQUE INDEX apns_messages_idempotency_idx
    ON apns_messages (server_instance_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
COMMIT;
