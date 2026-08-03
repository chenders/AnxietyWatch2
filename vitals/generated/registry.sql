-- GENERATED FROM registry.toml — DO NOT EDIT
-- Target: PostgreSQL 16 + TimescaleDB. See amendment 01.

CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE TABLE IF NOT EXISTS vitals_type (
    id        INTEGER PRIMARY KEY,
    name      TEXT             NOT NULL UNIQUE,
    unit      TEXT             NOT NULL,
    range_lo  DOUBLE PRECISION NOT NULL,
    range_hi  DOUBLE PRECISION NOT NULL,
    precision DOUBLE PRECISION NOT NULL,
    CHECK (range_lo < range_hi)
);

INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (1, 'hr', 'bpm', 20.0, 250.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (2, 'spo2', 'percent', 50.0, 100.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (3, 'accel', 'g', -16.0, 16.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (4, 'hrv_rmssd', 'ms', 1.0, 500.0, 0.1)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (5, 'rr_interval', 'seconds', 0.2, 3.0, 0.001)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (6, 'bp_systolic', 'mmHg', 50.0, 260.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (7, 'bp_diastolic', 'mmHg', 30.0, 160.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;
INSERT INTO vitals_type (id, name, unit, range_lo, range_hi, precision)
VALUES (8, 'bp_map', 'mmHg', 40.0, 200.0, 1.0)
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, unit = EXCLUDED.unit,
    range_lo = EXCLUDED.range_lo, range_hi = EXCLUDED.range_hi,
    precision = EXCLUDED.precision;

CREATE TABLE IF NOT EXISTS vitals_source (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL UNIQUE,
    transport TEXT    NOT NULL
);

INSERT INTO vitals_source (id, name, transport)
VALUES (0, 'emay', 'ble')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (1, 'polar_h10', 'ble')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (2, 'healthkit', 'push')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (3, 'derived', 'internal')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;
INSERT INTO vitals_source (id, name, transport)
VALUES (6, 'ad_ua651ble', 'ble')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name,
    transport = EXCLUDED.transport;

-- Append-only. No UPDATE, no DELETE; removal is a tombstone row.
-- TIMESTAMPTZ rather than epoch REAL: hypertable partitioning and every
-- hyperfunction expect a real timestamp type. This does not weaken the
-- UTC-only rule — TIMESTAMPTZ stores an absolute instant and the cluster
-- timezone is pinned to UTC.
CREATE TABLE IF NOT EXISTS samples (
    source       INTEGER          NOT NULL REFERENCES vitals_source(id),
    type         INTEGER          NOT NULL REFERENCES vitals_type(id),
    timestamp    TIMESTAMPTZ      NOT NULL,
    value        DOUBLE PRECISION NOT NULL,
    extra        BYTEA,
    hlc_physical BIGINT           NOT NULL,
    hlc_logical  INTEGER          NOT NULL,
    node_id      BYTEA            NOT NULL,
    PRIMARY KEY (source, type, timestamp)
);

-- The REFERENCES clauses above enforce referential integrity. A
-- CHECK (... IN (SELECT ...)) would be redundant and illegal: PostgreSQL
-- and SQLite both prohibit subqueries in CHECK, so such a table fails at
-- CREATE time rather than at insert time.

SELECT create_hypertable('samples', by_range('timestamp', INTERVAL '1 day'),
                         if_not_exists => TRUE);

ALTER TABLE samples SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'source, type',
    timescaledb.compress_orderby   = 'timestamp DESC'
);
SELECT add_compression_policy('samples', INTERVAL '7 days', if_not_exists => TRUE);

-- min/max are carried because an average hides a desaturation nadir, and
-- the nadir is the clinically interesting number.
CREATE MATERIALIZED VIEW IF NOT EXISTS samples_1min
WITH (timescaledb.continuous) AS
SELECT source, type,
       time_bucket(INTERVAL '1 minute', timestamp) AS bucket,
       avg(value) AS value, count(*) AS sample_count,
       min(value) AS min_value, max(value) AS max_value
FROM samples
GROUP BY source, type, bucket
WITH NO DATA;

SELECT add_continuous_aggregate_policy('samples_1min',
    start_offset      => INTERVAL '3 hours',
    end_offset        => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute',
    if_not_exists     => TRUE);
