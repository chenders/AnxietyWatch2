-- Verify vitals:noise_event on pg

BEGIN;

SELECT event_id, observed_at, human_heard, cat_scared, press_count,
       provenance, note, evidence, weather
  FROM noise_event WHERE false;

SELECT event_id, reported_at, source, note
  FROM noise_event_corroboration WHERE false;

SELECT event_id, identity, icao_hex, callsign, registration, icao_type,
       airframe_class, fr24_id, feeds, score, reasons, distance_km,
       altitude_ft, noise_index, phase, raw, retain_until
  FROM noise_event_candidate WHERE false;

SELECT event_id, generated_at, verdict, verdict_reason, margin,
       candidate_count, confirmed_hex, confirmed_identity,
       confirmed_registration, confirmed_type, flight_key, method
  FROM noise_attribution WHERE false;

SELECT icao_hex, identity, registration, icao_type, human_heard_count,
       cat_scared_count, first_heard_at, last_heard_at, event_ids
  FROM aircraft_noise_history WHERE false;

-- Synthetic provenance must be structurally unrepresentable.
DO $$
BEGIN
    BEGIN
        INSERT INTO noise_event (event_id, observed_at, human_heard, cat_scared, provenance)
        VALUES ('verify-synthetic', now(), true, false, 'synthetic');
        RAISE EXCEPTION 'noise_event accepted synthetic provenance';
    EXCEPTION WHEN check_violation THEN
        NULL;  -- expected
    END;
END $$;

ROLLBACK;
