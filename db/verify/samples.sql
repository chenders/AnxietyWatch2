-- Verify vitals:samples
SELECT source, type, session_id, seq, timestamp, value, extra, node_id, batch_id
  FROM samples WHERE FALSE;
SELECT segment, offset_bytes, drained, fingerprint, last_advanced
  FROM spool_status WHERE FALSE;

-- Identity must be (source, type, session_id, seq). If the primary key is on
-- timestamp instead, RR intervals arriving in one packet collide and are
-- silently discarded — the defect this schema exists to prevent.
SELECT 1 / count(*) FROM pg_index i
  JOIN pg_class c ON c.oid = i.indexrelid
 WHERE i.indrelid = 'samples'::regclass AND i.indisprimary
   AND (SELECT count(*) FROM unnest(i.indkey) k
         WHERE k = (SELECT attnum FROM pg_attribute
                     WHERE attrelid = 'samples'::regclass AND attname = 'seq')) = 1;

-- HLC columns must be absent (amendment 04 §8).
SELECT 1 / (1 - count(*)) FROM information_schema.columns
 WHERE table_name = 'samples' AND column_name IN ('hlc_physical','hlc_logical');

-- No shared staging table: the drainer uses a private TEMP one (amendment 04 §6).
SELECT 1 / (1 - count(*)) FROM information_schema.tables
 WHERE table_name = 'samples_staging';

-- timestamp must be TIMESTAMPTZ, not TIMESTAMP. The difference is silent at
-- write time and wrong forever after.
SELECT 1 / count(*) FROM information_schema.columns
 WHERE table_name = 'samples' AND column_name = 'timestamp'
   AND data_type = 'timestamp with time zone';

-- Both foreign keys must exist; without them an unknown source id is accepted.
SELECT 1 / (count(*) / 2) FROM pg_constraint
 WHERE conrelid = 'samples'::regclass AND contype = 'f';
