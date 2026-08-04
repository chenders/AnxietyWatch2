-- Verify vitals:extensions
-- Divide-by-zero is the sqitch idiom: a missing row raises, and a raise fails
-- the verify. Asserting presence is the whole point of a verify script.
SELECT 1 / count(*) FROM pg_extension WHERE extname = 'timescaledb';
SELECT 1 / count(*) FROM pg_extension WHERE extname = 'timescaledb_toolkit';
SELECT 1 / count(*) FROM pg_extension WHERE extname = 'pg_stat_statements';
SELECT 1 / count(*) FROM pg_extension WHERE extname = 'pg_cron';
