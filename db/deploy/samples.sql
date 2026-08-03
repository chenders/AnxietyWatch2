-- Deploy vitals:samples to pg
-- Append-only. No UPDATE, no DELETE; removal is a tombstone row.
--
-- TIMESTAMPTZ rather than the epoch REAL of the original spec: hypertable
-- partitioning and every hyperfunction expect a real timestamp type. This does
-- not weaken the UTC-only rule — TIMESTAMPTZ stores an absolute instant with no
-- timezone attached, and the cluster timezone is pinned to UTC. The C side
-- still holds double epoch seconds and converts once, in the COPY encoder.
BEGIN;

CREATE TABLE samples (
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

-- The drainer stages a COPY batch here, then INSERT ... ON CONFLICT DO NOTHING
-- into samples, because COPY has no ON CONFLICT. Unlogged: it is rebuilt from
-- the spool on every drain and never needs to survive a crash.
CREATE UNLOGGED TABLE samples_staging (LIKE samples INCLUDING DEFAULTS);

COMMIT;
