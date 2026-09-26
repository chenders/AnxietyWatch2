-- Revert vitals:service_operation_audit
BEGIN;
DROP TABLE IF EXISTS service_operation_audit;
DROP FUNCTION IF EXISTS reject_service_operation_audit_mutation();
COMMIT;
