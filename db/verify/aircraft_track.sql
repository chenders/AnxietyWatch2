-- Verify vitals:aircraft_track on pg

BEGIN;

SELECT at, provenance, icao_hex, fr24_id, callsign, registration,
       lat, lon, alt_ft, speed_kt, track_deg, vertical_rate_fpm,
       rssi_dbm, distance_km, bearing_deg, retain_until
  FROM aircraft_track_point WHERE false;

-- FR24 points must carry a deletion deadline; receiver points must not.
DO $$
BEGIN
    BEGIN
        INSERT INTO aircraft_track_point (at, provenance, lat, lon)
        VALUES (now(), 'fr24', 45.5, -122.6);
        RAISE EXCEPTION 'aircraft_track_point accepted an fr24 row with no retain_until';
    EXCEPTION WHEN check_violation THEN
        NULL;  -- expected
    END;
END $$;

ROLLBACK;
