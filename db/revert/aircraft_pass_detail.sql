-- Revert vitals:aircraft_pass_detail from pg
BEGIN;
ALTER TABLE aircraft_pass
    DROP CONSTRAINT IF EXISTS aircraft_pass_airframe_class_known,
    DROP CONSTRAINT IF EXISTS aircraft_pass_operator_source_known,
    DROP COLUMN IF EXISTS bearing_deg,
    DROP COLUMN IF EXISTS speed_kt,
    DROP COLUMN IF EXISTS track_deg,
    DROP COLUMN IF EXISTS vertical_rate_fpm,
    DROP COLUMN IF EXISTS rssi_dbm,
    DROP COLUMN IF EXISTS emitter_category,
    DROP COLUMN IF EXISTS airframe_class,
    DROP COLUMN IF EXISTS operator_source,
    DROP COLUMN IF EXISTS cpa_ground_m,
    DROP COLUMN IF EXISTS cpa_slant_m,
    DROP COLUMN IF EXISTS cpa_altitude_ft,
    DROP COLUMN IF EXISTS eta_seconds,
    DROP COLUMN IF EXISTS ambient_dba;
COMMIT;
