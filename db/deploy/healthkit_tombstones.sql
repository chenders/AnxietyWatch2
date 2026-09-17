-- Deploy vitals:healthkit_tombstones to pg
-- HealthKit deletions are facts, not physical deletes. Keep them separate from samples because a
-- tombstone has no physiological value and must never be coerced into one.
BEGIN;

CREATE TABLE healthkit_sample_tombstones (
    healthkit_sample_uuid UUID        NOT NULL,
    healthkit_type        TEXT        NOT NULL,
    registry_type         INTEGER     NOT NULL REFERENCES vitals_type(id),
    deleted_at            TIMESTAMPTZ NOT NULL,
    query_batch_id        UUID        NOT NULL,
    recorded_at           TIMESTAMPTZ NOT NULL,
    node_id               BYTEA       NOT NULL,
    extra                 BYTEA       NOT NULL,
    PRIMARY KEY (healthkit_sample_uuid, healthkit_type)
);

CREATE INDEX healthkit_tombstones_time_idx
    ON healthkit_sample_tombstones (registry_type, deleted_at DESC);

COMMIT;
