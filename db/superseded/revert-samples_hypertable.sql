-- Revert vitals:samples_hypertable from pg
-- A hypertable cannot be converted back to a plain table in place. Reverting
-- means recreating the table, which discards its rows — so this refuses rather
-- than destroying data silently. Revert `samples` instead if that is the intent.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM timescaledb_information.hypertables
                WHERE hypertable_name = 'samples')
       AND EXISTS (SELECT 1 FROM samples LIMIT 1) THEN
        RAISE EXCEPTION
          'refusing to revert samples_hypertable: samples has rows. '
          'Reverting would discard them. Revert the samples change instead.';
    END IF;
END $$;
