-- Verify vitals:aircraft_pass_detail
--
-- Asserts by attempting the violations. Checking that columns exist would pass
-- against constraints that do nothing.
DO $$
BEGIN
    -- A fully detailed row round-trips.
    INSERT INTO aircraft_pass
        (at, provenance, loudness_band, icao_hex, bearing_deg, speed_kt,
         track_deg, vertical_rate_fpm, rssi_dbm, emitter_category,
         airframe_class, operator_source, cpa_ground_m, cpa_slant_m,
         cpa_altitude_ft, eta_seconds, ambient_dba, noise_index)
    VALUES (now(), 'adsb', 'LOUD', 'FFFFF1', 167.4, 96, 78.0, 0, -18.3, 'A1',
            'light', 'ROSTER', 480.0, 600.0, 1382, 42, 40.0, 62.5);

    -- A pre-recalibration row must still be representable: no ambient, and a
    -- score on the superseded scale. Rejecting these would make the migration
    -- unable to coexist with the 700+ rows already in the table.
    INSERT INTO aircraft_pass (at, provenance, loudness_band, icao_hex)
    VALUES (now(), 'adsb', 'UNKNOWN', 'FFFFF2');

    -- An airframe class the model cannot produce is a publisher typo, and must
    -- fail loudly rather than become a class no query will ever match.
    BEGIN
        INSERT INTO aircraft_pass (at, provenance, loudness_band, airframe_class)
        VALUES (now(), 'adsb', 'UNKNOWN', 'helicopter');
        RAISE EXCEPTION 'an unknown airframe_class was accepted; the CHECK is gone';
    EXCEPTION WHEN check_violation THEN NULL;
    END;

    -- Same for operator provenance: ROSTER and LOOKUP mean different things
    -- and a third value would silently blur them.
    BEGIN
        INSERT INTO aircraft_pass (at, provenance, loudness_band, operator_source)
        VALUES (now(), 'adsb', 'UNKNOWN', 'guess');
        RAISE EXCEPTION 'an unknown operator_source was accepted; the CHECK is gone';
    EXCEPTION WHEN check_violation THEN NULL;
    END;

    RAISE EXCEPTION 'rollback: verification only';
EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'rollback: verification only' THEN RAISE; END IF;
END $$;
