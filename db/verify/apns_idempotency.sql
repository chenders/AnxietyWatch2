-- Verify vitals:apns_idempotency
SELECT idempotency_key FROM apns_messages LIMIT 1;
