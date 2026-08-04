-- Verify vitals:samples_compression
SELECT 1 / count(*) FROM timescaledb_information.hypertables
 WHERE hypertable_name = 'samples' AND compression_enabled;
SELECT 1 / count(*) FROM timescaledb_information.jobs
 WHERE proc_name = 'policy_compression' AND hypertable_name = 'samples';
