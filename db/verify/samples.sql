-- Verify vitals:samples
SELECT source, type, timestamp, value, extra, hlc_physical, hlc_logical, node_id
  FROM samples WHERE FALSE;
SELECT source FROM samples_staging WHERE FALSE;

-- timestamp must be TIMESTAMPTZ, not TIMESTAMP. The difference is silent at
-- write time and wrong forever after.
SELECT 1 / count(*) FROM information_schema.columns
 WHERE table_name = 'samples' AND column_name = 'timestamp'
   AND data_type = 'timestamp with time zone';

-- Both foreign keys must exist; without them an unknown source id is accepted.
SELECT 1 / (count(*) / 2) FROM pg_constraint
 WHERE conrelid = 'samples'::regclass AND contype = 'f';
