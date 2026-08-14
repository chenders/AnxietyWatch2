-- Revert vitals:flight_route from pg
BEGIN;
DROP TABLE IF EXISTS flight_route;
COMMIT;
