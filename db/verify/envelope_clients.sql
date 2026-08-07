-- Verify vitals:envelope_clients
SELECT client_id, secret, created_at, last_seen_at
  FROM envelope_clients WHERE FALSE;

-- Client identity is unique and the credential cannot be omitted.
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'envelope_clients'::regclass AND contype = 'p';
SELECT 1 / count(*) FROM information_schema.columns
 WHERE table_name = 'envelope_clients' AND column_name = 'secret'
   AND data_type = 'bytea' AND is_nullable = 'NO';
