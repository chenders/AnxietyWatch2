-- Verify vitals:domain_conflicts
SELECT id, record_id, incoming_kind, incoming_schema_version,
       incoming_content, incoming_content_sha256, status, resolution_record_id
  FROM domain.record_conflicts WHERE FALSE;
SELECT id, record_id, conflict_id, event_kind, actor, occurred_at, request_id
  FROM domain.record_audit WHERE FALSE;

SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain.record_conflicts'::regclass AND contype = 'p';
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain.record_conflicts'::regclass
   AND conname = 'record_conflicts_record_id_incoming_content_sha256_key';
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'domain.record_audit'::regclass AND contype = 'p';
