-- Verify vitals:mobile_credentials
SELECT enrollment_id, server_instance_id, token_hash, scopes, expires_at,
       consumed_at FROM enrollment_artifacts WHERE FALSE;
SELECT client_id, key_id, public_key_fingerprint, scopes, expires_at,
       last_used_at, revoked_at FROM mobile_credentials WHERE FALSE;
SELECT client_id, key_id, action, actor, occurred_at
  FROM credential_audit WHERE FALSE;
