-- Revert vitals:apns_owner_identity from pg
BEGIN;
DROP INDEX IF EXISTS apns_device_registrations_owner_idx;
ALTER TABLE apns_device_registrations DROP COLUMN IF EXISTS owner_user_id;
COMMIT;
