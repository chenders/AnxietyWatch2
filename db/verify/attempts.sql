-- Verify vitals:attempts
SELECT id, kind, subject, started_at, expected_duration_s, finished_at,
       outcome, effect, high_water_mark, detail
  FROM attempts WHERE FALSE;

-- The attempt invariants are enforced in the database, not left to callers.
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'attempts'::regclass AND contype = 'p';
SELECT 1 / (count(*) / 3) FROM pg_constraint
 WHERE conrelid = 'attempts'::regclass AND contype = 'c';

-- Attempt history reads newest runs by kind and subject.
SELECT 1 / count(*) FROM pg_indexes
 WHERE tablename = 'attempts'
   AND indexname = 'attempts_kind_subject_started_idx'
   AND indexdef LIKE '%(kind, subject, started_at DESC)%';
