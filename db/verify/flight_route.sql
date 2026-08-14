-- Verify vitals:flight_route
--
-- Asserts by attempting the violations. Checking that columns exist would pass
-- against a table whose constraints do nothing.
DO $$
DECLARE
    fetched timestamptz;
BEGIN
    -- A real route round-trips.
    INSERT INTO flight_route (callsign, airline_name, origin_iata, destination_iata)
    VALUES ('ASA783', 'Alaska Airlines', 'ATL', 'PDX');

    -- "Asked, and there is no route" must be representable. This is the row
    -- shape that stops a free service being re-queried forever, so if it
    -- cannot be stored the enricher will ask again on every run.
    INSERT INTO flight_route (callsign) VALUES ('XXX999');
    SELECT looked_up_at INTO fetched FROM flight_route WHERE callsign = 'XXX999';
    IF fetched IS NULL THEN
        RAISE EXCEPTION 'looked_up_at defaulted to NULL; retries cannot be paced';
    END IF;

    -- A lowercase callsign must be refused rather than silently stored, or
    -- ASA783 and asa783 become two different routes.
    BEGIN
        INSERT INTO flight_route (callsign) VALUES ('asa783');
        RAISE EXCEPTION 'a lowercase callsign was accepted; the CHECK is gone';
    EXCEPTION WHEN check_violation THEN NULL;
    END;

    -- An empty callsign is not a key. dump1090 reports blank callsigns
    -- constantly, and one reaching this table would collide with the next.
    BEGIN
        INSERT INTO flight_route (callsign) VALUES ('');
        RAISE EXCEPTION 'an empty callsign was accepted; the CHECK is gone';
    EXCEPTION WHEN check_violation THEN NULL;
    END;

    -- One route per callsign.
    BEGIN
        INSERT INTO flight_route (callsign) VALUES ('ASA783');
        RAISE EXCEPTION 'a duplicate callsign was accepted; the key is gone';
    EXCEPTION WHEN unique_violation THEN NULL;
    END;

    RAISE EXCEPTION 'rollback: verification only';
EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'rollback: verification only' THEN RAISE; END IF;
END $$;
