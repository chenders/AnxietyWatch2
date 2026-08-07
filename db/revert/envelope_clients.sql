-- Revert vitals:envelope_clients from pg
BEGIN;
DROP TABLE IF EXISTS envelope_clients;
COMMIT;
