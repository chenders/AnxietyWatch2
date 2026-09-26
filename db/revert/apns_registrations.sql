-- Revert vitals:apns_registrations from pg
BEGIN;
DROP TABLE IF EXISTS apns_registration_audit;
DROP TABLE IF EXISTS apns_device_registrations;
COMMIT;
