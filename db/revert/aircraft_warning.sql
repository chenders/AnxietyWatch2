-- Revert vitals:aircraft_warning from pg
BEGIN;
DROP VIEW aircraft_warning_with_latest_review;
DROP TABLE aircraft_warning_review;
DROP TABLE aircraft_warning;
COMMIT;
