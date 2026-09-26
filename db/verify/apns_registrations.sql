-- Verify vitals:apns_registrations
SELECT server_instance_id, device_id, client_id, token_ciphertext, token_fingerprint,
       environment, app_identifier, app_version, build, authorization_status,
       first_seen_at, last_seen_at, revoked_at
  FROM apns_device_registrations WHERE FALSE;
SELECT server_instance_id, device_id, client_id, action, occurred_at
  FROM apns_registration_audit WHERE FALSE;
