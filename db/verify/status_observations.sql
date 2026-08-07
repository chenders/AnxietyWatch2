-- Verify vitals:status_observations
SELECT subject, state, reason, reported_at, changed_at
  FROM status_observations WHERE FALSE;

-- Each subject has exactly one current observation.
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'status_observations'::regclass AND contype = 'p';

-- Freshness and value-change time are distinct stored TIMESTAMPTZ fields.
SELECT 1 / (count(*) / 2) FROM information_schema.columns
 WHERE table_name = 'status_observations'
   AND column_name IN ('reported_at', 'changed_at')
   AND data_type = 'timestamp with time zone'
   AND is_nullable = 'NO';
