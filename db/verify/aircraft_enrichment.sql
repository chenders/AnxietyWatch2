-- Verify vitals:aircraft_enrichment
SELECT looked_up_at FROM aircraft WHERE FALSE;

-- A miss must be recordable: the column has to accept a timestamp on a row with
-- no type at all, which is precisely the "asked, found nothing" case.
DO $$
BEGIN
    INSERT INTO aircraft (icao_hex, source, looked_up_at)
    VALUES ('FFFFFE', 'adsbdb', now());
    RAISE EXCEPTION 'rollback: verification only';
EXCEPTION WHEN raise_exception THEN
    IF SQLERRM <> 'rollback: verification only' THEN RAISE; END IF;
END $$;
