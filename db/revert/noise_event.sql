-- Revert vitals:noise_event from pg

BEGIN;

DROP VIEW IF EXISTS aircraft_noise_history;
DROP TABLE IF EXISTS noise_attribution;
DROP TABLE IF EXISTS noise_event_candidate;
DROP TABLE IF EXISTS noise_event_corroboration;
DROP TABLE IF EXISTS noise_event;

COMMIT;
