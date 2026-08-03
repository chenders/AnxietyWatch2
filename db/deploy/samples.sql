-- Deploy vitals:samples to pg
--
-- Identity is (source, type, session_id, seq) — NOT timestamp.
--
-- Several RR intervals arrive in one BLE notification. Keying on packet arrival
-- time makes them collide, so ON CONFLICT DO NOTHING silently discards real
-- measurements while every health metric reports normal. Deriving beat times by
-- accumulating durations backward fixes the *estimate* but not the *identity*: a
-- retransmitted payload anchored to a later arrival gets entirely new timestamps
-- and inserts again as a duplicate. See amendment 04 §1 and
-- docs/reviews/SYNTHESIS.md A2.
--
-- So `seq` is a monotonic per-session index — from the cumulative RR sum for
-- interval sources — unaffected by arrival jitter or retransmission. `timestamp`
-- carries best-estimate event time for querying, gap-fill and detection, and is
-- allowed to be an estimate precisely because it is not load-bearing.
--
-- TIMESTAMPTZ rather than epoch REAL: hypertable partitioning and every
-- hyperfunction expect a real timestamp type. This does not weaken the UTC-only
-- rule — TIMESTAMPTZ stores an absolute instant and the cluster timezone is
-- pinned to UTC.
--
-- hlc_physical / hlc_logical are gone (amendment 04 §8): replication progress is
-- a byte offset in the spool, and no two writers share a key because sources
-- partition the key space by construction. node_id stays — it is provenance.
--
-- There is no shared staging table. The drainer creates a private TEMP table per
-- transaction (amendment 04 §6); a shared UNLOGGED one lets two drainers
-- interleave COPY and commit a partial batch while the other's rows are lost.
BEGIN;

CREATE TABLE samples (
    source       INTEGER          NOT NULL REFERENCES vitals_source(id),
    type         INTEGER          NOT NULL REFERENCES vitals_type(id),
    session_id   BIGINT           NOT NULL,
    seq          BIGINT           NOT NULL,
    timestamp    TIMESTAMPTZ      NOT NULL,
    value        DOUBLE PRECISION NOT NULL,
    extra        BYTEA,
    node_id      BYTEA            NOT NULL,
    -- NULL means the row did not arrive through the signed ingest path. That is
    -- the detection mechanism, not an error state to hide (amendment 02 §2.2).
    batch_id     BIGINT,
    PRIMARY KEY (source, type, session_id, seq)
);

-- The primary key is identity; every read path queries by time. Without this,
-- gap-fill and chart queries scan whole chunks.
CREATE INDEX samples_time_idx ON samples (source, type, timestamp DESC);

-- Drain progress. The offset advances in the same transaction as the rows it
-- accounts for, and a migration touching samples must guard on this being empty
-- (amendment 04 §7) — binary COPY freezes column count and order, so migrating
-- with a non-empty spool orphans buffered rows with no downgrade path.
CREATE TABLE spool_status (
    segment       TEXT        PRIMARY KEY,
    offset_bytes  BIGINT      NOT NULL DEFAULT 0,
    drained       BOOLEAN     NOT NULL DEFAULT FALSE,
    fingerprint   BYTEA       NOT NULL,
    last_advanced TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMIT;
