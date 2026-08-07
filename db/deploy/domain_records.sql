-- Deploy vitals:domain_records to pg
-- This change does not touch samples, so the spool-empty guard does not apply.
-- Identity is the client-minted UUID (spec §2.3b). Same id + same hash replays
-- as a no-op; same id + different hash is a conflict recorded as rejected,
-- never an overwrite.
BEGIN;

CREATE TABLE domain_records (
    id             UUID        PRIMARY KEY,
    kind           TEXT        NOT NULL,
    content        JSONB       NOT NULL,
    content_sha256 BYTEA       NOT NULL CHECK (octet_length(content_sha256) = 32),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX domain_records_kind_created_idx
    ON domain_records (kind, created_at DESC);

COMMENT ON TABLE domain_records IS
    'Identity is the client-minted UUID (spec §2.3b); same id + same hash is a no-op replay, while same id + different hash is rejected as a conflict and never overwrites.';

COMMIT;
