-- Verify vitals:domain_versions
SELECT id, kind, schema_version, content, content_sha256, recorded_at,
       created_at, updated_at, supersedes_id, tombstone_target_id, actor
  FROM domain.records WHERE FALSE;

SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain.records'::regclass
   AND conname = 'domain_records_not_self_version';
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain.records'::regclass
   AND conname = 'domain_records_tombstone_not_version';

SELECT table_schema, table_name
  FROM information_schema.tables
 WHERE table_schema = 'domain' AND table_name = 'record_links';
