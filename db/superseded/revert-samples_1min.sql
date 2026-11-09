-- Revert vitals:samples_1min from pg
SELECT remove_continuous_aggregate_policy('samples_1min', if_not_exists => TRUE);
DROP MATERIALIZED VIEW IF EXISTS samples_1min;
