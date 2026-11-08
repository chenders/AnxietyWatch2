-- Verify vitals:samples_hypertable
SELECT 1 / count(*) FROM timescaledb_information.hypertables
 WHERE hypertable_name = 'samples';
