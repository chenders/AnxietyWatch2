-- Verify vitals:registry_tables
SELECT id, name, unit, range_lo, range_hi, precision FROM vitals_type WHERE FALSE;
SELECT id, name, transport                            FROM vitals_source WHERE FALSE;
-- The range CHECK is load-bearing: an inverted range would admit every value.
SELECT 1 / count(*) FROM pg_constraint
 WHERE conrelid = 'vitals_type'::regclass AND contype = 'c';
