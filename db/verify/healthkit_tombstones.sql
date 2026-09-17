-- Verify vitals:healthkit_tombstones
BEGIN;

SELECT healthkit_sample_uuid, healthkit_type, registry_type, deleted_at,
       query_batch_id, recorded_at, node_id, extra
  FROM healthkit_sample_tombstones
 WHERE FALSE;

ROLLBACK;
