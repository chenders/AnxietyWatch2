-- Verify vitals:aircraft
--
-- Structure, then the invariants that actually matter. A column list alone
-- would pass against a schema that had lost every guard below.

SELECT icao_hex, registration, icao_type, model, operator_key, operator_name,
       operator_kind, retired, retired_reason, source, first_seen, last_seen,
       updated_at
  FROM aircraft WHERE FALSE;

SELECT at, provenance, icao_hex, callsign, registration, icao_type, fr24_id,
       distance_km, altitude_ft, phase, noise_index, loudness_band, notable,
       operator_state, reason, retain_until
  FROM aircraft_pass WHERE FALSE;

-- aircraft_pass must be a hypertable. Without this it is an ordinary table that
-- will grow unpartitioned until queries over a season stop returning.
SELECT 1 / count(*) FROM timescaledb_information.hypertables
 WHERE hypertable_name = 'aircraft_pass';

-- An unscored aircraft must be representable, and must never be storable as a
-- quiet one. Both halves are checked: the NULL case must be accepted, and the
-- mismatched case must be rejected.
DO $$
BEGIN
    INSERT INTO aircraft_pass (at, provenance, icao_hex, noise_index, loudness_band)
    VALUES ('2000-01-01Z', 'adsb', 'ABCDEF', NULL, 'UNKNOWN');

    BEGIN
        INSERT INTO aircraft_pass (at, provenance, icao_hex, noise_index, loudness_band)
        VALUES ('2000-01-02Z', 'adsb', 'ABCDEF', NULL, 'FAINT');
        RAISE EXCEPTION 'an unscored pass was accepted as FAINT: the guard that '
                        'keeps "not measured" from reading as "quiet" is gone';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    -- FR24 rows must carry a deletion deadline; receiver rows must not.
    BEGIN
        INSERT INTO aircraft_pass (at, provenance, icao_hex, loudness_band, retain_until)
        VALUES ('2000-01-03Z', 'fr24', 'ABCDEF', 'UNKNOWN', NULL);
        RAISE EXCEPTION 'an FR24 pass was accepted with no retain_until: the '
                        'vendor retention rule is unenforceable';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    -- A retired airframe must say why.
    BEGIN
        INSERT INTO aircraft (icao_hex, source, retired, retired_reason)
        VALUES ('A9ED9D', 'roster', true, NULL);
        RAISE EXCEPTION 'a retired airframe was accepted with no reason given';
    EXCEPTION WHEN check_violation THEN
        NULL;
    END;

    RAISE EXCEPTION 'rollback: verification only';
EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'rollback: verification only' THEN
        RAISE;
    END IF;
END $$;
