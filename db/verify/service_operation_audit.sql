-- Verify vitals:service_operation_audit
SELECT id, occurred_at, actor, target, action, confirmation_class,
       attempt_id, outcome, detail
  FROM service_operation_audit
 WHERE FALSE;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conrelid = 'service_operation_audit'::regclass
           AND contype = 'p'
    ) THEN
        RAISE EXCEPTION 'service_operation_audit primary key is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conrelid = 'service_operation_audit'::regclass
           AND contype = 'u'
           AND conname LIKE '%attempt_id%'
    ) THEN
        RAISE EXCEPTION 'service_operation_audit attempt uniqueness is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1
          FROM pg_trigger
         WHERE tgrelid = 'service_operation_audit'::regclass
           AND tgname = 'service_operation_audit_immutable'
           AND NOT tgisinternal
    ) THEN
        RAISE EXCEPTION 'service_operation_audit append-only trigger is missing';
    END IF;
END;
$$;
