-- Deploy vitals:health_mutes to pg
BEGIN;

CREATE TABLE health_mutes (
    id SERIAL PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES vitals_source(id),
    muted_by TEXT NOT NULL,
    muted_until TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT health_mutes_future CHECK (muted_until > created_at)
);

CREATE INDEX health_mutes_source_id_idx ON health_mutes (source_id);
CREATE INDEX health_mutes_active_idx ON health_mutes (source_id, muted_until);

COMMIT;
