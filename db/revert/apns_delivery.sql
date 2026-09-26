-- Revert vitals:apns_delivery from pg
BEGIN;
DROP TABLE IF EXISTS apns_client_outcomes;
DROP TABLE IF EXISTS apns_delivery_attempts;
DROP TABLE IF EXISTS apns_messages;
COMMIT;
