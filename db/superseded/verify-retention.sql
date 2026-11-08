-- Verify vitals:retention
SELECT 1 / count(*) FROM timescaledb_information.jobs
 WHERE proc_name = 'policy_retention' AND hypertable_name = 'samples';
-- The rollup must exist before raw data is allowed to expire.
SELECT 1 / count(*) FROM timescaledb_information.continuous_aggregates
 WHERE view_name = 'samples_1min';
