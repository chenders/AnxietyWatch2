-- Revert vitals:mobile_request_replays from pg
BEGIN;
DROP TABLE IF EXISTS mobile_request_replays;
COMMIT;
