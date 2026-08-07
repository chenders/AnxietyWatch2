-- Verify vitals:domain_records
SELECT id, kind, content, content_sha256, created_at, updated_at
  FROM domain_records WHERE FALSE;

-- Client-minted UUID identity and the 32-byte SHA-256 invariant are enforced.
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain_records'::regclass AND contype = 'p';
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain_records'::regclass AND contype = 'c';

-- Domain history reads newest records by kind.
SELECT 1 / count(*) FROM pg_indexes
 WHERE tablename = 'domain_records'
   AND indexname = 'domain_records_kind_created_idx'
   AND indexdef LIKE '%(kind, created_at DESC)%';
