-- Verify vitals:healthkit_tombstones
BEGIN;

SELECT healthkit_sample_uuid, healthkit_type, registry_type, deleted_at,
       query_batch_id, recorded_at, node_id, extra
  FROM healthkit_sample_tombstones
 WHERE FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_trigger
         WHERE tgrelid = 'healthkit_sample_tombstones'::regclass
           AND tgname = 'healthkit_sample_tombstones_immutable'
    ) THEN
        RAISE EXCEPTION 'healthkit tombstone append-only trigger is missing';
    END IF;
END;
$$;

ROLLBACK;
