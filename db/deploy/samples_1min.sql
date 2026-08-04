-- Deploy vitals:samples_1min to pg
-- Replaces the hand-maintained rollup table of the original spec. A continuous
-- aggregate survives after the raw data is dropped, which is what makes the
-- retention policy safe.
--
-- min/max are carried deliberately: an average hides a desaturation nadir, and
-- the nadir is the clinically interesting number.
CREATE MATERIALIZED VIEW samples_1min
WITH (timescaledb.continuous) AS
SELECT source, type,
       time_bucket(INTERVAL '1 minute', timestamp) AS bucket,
       avg(value)   AS value,
       count(*)     AS sample_count,
       min(value)   AS min_value,
       max(value)   AS max_value
FROM samples
GROUP BY source, type, bucket
WITH NO DATA;

SELECT add_continuous_aggregate_policy('samples_1min',
    start_offset      => INTERVAL '3 hours',
    end_offset        => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists     => TRUE);
