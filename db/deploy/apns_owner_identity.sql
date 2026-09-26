-- Deploy vitals:apns_owner_identity to pg
-- Bind each registration to the configured operator/user identity for same-user fanout.
BEGIN;
ALTER TABLE apns_device_registrations ADD COLUMN owner_user_id UUID;
CREATE INDEX apns_device_registrations_owner_idx
    ON apns_device_registrations (server_instance_id, owner_user_id, last_seen_at DESC)
    WHERE revoked_at IS NULL;
COMMIT;
