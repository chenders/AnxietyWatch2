-- Revert vitals:health_mutes from pg
BEGIN;

DROP TABLE health_mutes;

COMMIT;
