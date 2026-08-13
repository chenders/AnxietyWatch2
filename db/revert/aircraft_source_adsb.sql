-- Revert vitals:aircraft_source_adsb from pg
BEGIN;
UPDATE aircraft SET source = 'manual' WHERE source = 'adsb';
ALTER TABLE aircraft DROP CONSTRAINT aircraft_source_check;
ALTER TABLE aircraft ADD CONSTRAINT aircraft_source_check
    CHECK (source IN ('roster', 'adsbdb', 'fr24', 'manual'));
COMMIT;
