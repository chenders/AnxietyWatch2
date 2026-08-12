-- Revert vitals:aircraft from pg

BEGIN;

DROP TABLE IF EXISTS aircraft_pass;
DROP TABLE IF EXISTS aircraft;

COMMIT;
