-- Revert vitals:air11_admin from pg
BEGIN;
DROP TABLE IF EXISTS air11_admin_audit;
DROP TABLE IF EXISTS air11_integration_config;
DROP FUNCTION IF EXISTS reject_air11_admin_audit_mutation();
COMMIT;
