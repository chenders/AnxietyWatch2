-- Deploy vitals:samples_compression to pg
-- Columnar compression gives 90-95% reduction on cold chunks. Segment by
-- (source, type) because every query filters on both; order by timestamp DESC
-- because reads are overwhelmingly "most recent first".
ALTER TABLE samples SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'source, type',
    timescaledb.compress_orderby   = 'timestamp DESC'
);

-- 7 days: long enough that a night under review is still uncompressed and
-- cheap to scan, short enough that storage stays flat.
SELECT add_compression_policy('samples', INTERVAL '7 days', if_not_exists => TRUE);
