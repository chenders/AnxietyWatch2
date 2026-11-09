-- Verify vitals:samples_1min
SELECT 1 / count(*) FROM timescaledb_information.continuous_aggregates
 WHERE view_name = 'samples_1min';
SELECT 1 / count(*) FROM timescaledb_information.jobs
 WHERE proc_name = 'policy_refresh_continuous_aggregate';
-- min/max must be present: without them the rollup cannot answer "what was the
-- lowest SpO2 that minute", which is the only question that matters.
SELECT bucket, value, sample_count, min_value, max_value FROM samples_1min WHERE FALSE;
