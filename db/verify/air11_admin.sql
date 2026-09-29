-- Verify vitals:air11_admin
SELECT singleton, opted_in, created_at, updated_at
  FROM air11_integration_config WHERE FALSE;
SELECT id, action, outcome, detail, created_at
  FROM air11_admin_audit WHERE FALSE;

-- The config is a singleton and starts with an explicit not-opted-in state.
SELECT 1 / count(*) FROM air11_integration_config
 WHERE singleton = TRUE AND opted_in = FALSE;
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'air11_integration_config'::regclass AND contype = 'p';

-- The audit vocabulary and append ordering are enforced by the database.
SELECT 1 / (count(*) / 2) FROM pg_constraint
 WHERE conrelid = 'air11_admin_audit'::regclass AND contype = 'c';
SELECT 1 / count(*) FROM pg_indexes
 WHERE tablename = 'air11_admin_audit'
   AND indexname = 'air11_admin_audit_created_idx'
   AND indexdef LIKE '%(created_at DESC, id DESC)%';
SELECT 1 / count(*) FROM pg_trigger
 WHERE tgrelid = 'air11_admin_audit'::regclass
   AND tgname = 'air11_admin_audit_immutable'
   AND NOT tgisinternal;
