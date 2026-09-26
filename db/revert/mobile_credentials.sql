-- Revert vitals:mobile_credentials from pg
BEGIN;
DROP TABLE IF EXISTS credential_audit;
DROP TABLE IF EXISTS mobile_credentials;
DROP TABLE IF EXISTS enrollment_artifacts;
COMMIT;
