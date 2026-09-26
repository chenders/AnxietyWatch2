-- Revert vitals:mobile_status_beacons from pg
BEGIN;
DROP TABLE IF EXISTS mobile_status_beacons;
COMMIT;
