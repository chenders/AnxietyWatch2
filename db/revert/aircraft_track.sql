-- Revert vitals:aircraft_track from pg

BEGIN;

DROP TABLE IF EXISTS aircraft_track_point;

COMMIT;
