-- Revert vitals:telemetry from pg
BEGIN;
DROP TABLE IF EXISTS telemetry_config;
COMMIT;
