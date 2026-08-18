-- Revert vitals:noise_analysis_helpers from pg

BEGIN;

DROP VIEW IF EXISTS aircraft_track_gaps;
DROP VIEW IF EXISTS noise_event_audibility;

COMMIT;
