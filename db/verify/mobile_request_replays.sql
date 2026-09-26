-- Verify vitals:mobile_request_replays
SELECT client_id, request_id, request_digest, created_at, expires_at
  FROM mobile_request_replays WHERE FALSE;
