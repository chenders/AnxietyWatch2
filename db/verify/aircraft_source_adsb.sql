-- Verify vitals:aircraft_source_adsb
--
-- Asserts by attempting the insert that used to fail. A column-list check would
-- pass against the broken constraint.
DO $$
BEGIN
    INSERT INTO aircraft (icao_hex, source) VALUES ('FFFFFD', 'adsb');
    -- And an unknown source must still be refused: widening the list must not
    -- have turned the guard off altogether.
    BEGIN
        INSERT INTO aircraft (icao_hex, source) VALUES ('FFFFFC', 'guesswork');
        RAISE EXCEPTION 'an arbitrary source was accepted; the CHECK is gone';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    RAISE EXCEPTION 'rollback: verification only';
EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'rollback: verification only' THEN RAISE; END IF;
END $$;
