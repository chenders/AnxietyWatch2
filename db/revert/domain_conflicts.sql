-- Revert vitals:domain_conflicts from pg
BEGIN;

DROP TABLE IF EXISTS domain.record_audit;
DROP TABLE IF EXISTS domain.record_conflicts;

COMMIT;
