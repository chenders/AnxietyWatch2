-- Deploy vitals:domain_conflicts to pg
-- Preserve same-ID/different-content conflicts and append-only audit evidence.
BEGIN;

CREATE TABLE domain.record_conflicts (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    record_id             UUID NOT NULL REFERENCES domain.records(id),
    incoming_kind         TEXT NOT NULL,
    incoming_schema_version INTEGER NOT NULL CHECK (incoming_schema_version > 0),
    incoming_content      JSONB NOT NULL,
    incoming_content_sha256 BYTEA NOT NULL CHECK (octet_length(incoming_content_sha256) = 32),
    incoming_supersedes_id UUID REFERENCES domain.records(id),
    incoming_tombstone_target_id UUID REFERENCES domain.records(id),
    actor                 TEXT NOT NULL,
    detected_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    status                TEXT NOT NULL DEFAULT 'open'
                              CHECK (status IN ('open', 'resolved', 'dismissed')),
    resolution_record_id  UUID REFERENCES domain.records(id),
    resolved_at           TIMESTAMPTZ,
    resolution_detail     JSONB,
    UNIQUE (record_id, incoming_content_sha256),
    CHECK ((status = 'open' AND resolved_at IS NULL)
        OR (status IN ('resolved', 'dismissed') AND resolved_at IS NOT NULL)),
    CHECK ((status = 'resolved') = (resolution_record_id IS NOT NULL))
);

CREATE INDEX domain_record_conflicts_open_idx
    ON domain.record_conflicts (record_id, detected_at DESC)
    WHERE status = 'open';

CREATE TABLE domain.record_audit (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    record_id         UUID REFERENCES domain.records(id),
    conflict_id       BIGINT REFERENCES domain.record_conflicts(id),
    event_kind        TEXT NOT NULL CHECK (event_kind IN (
        'created', 'replayed', 'conflict', 'tombstoned', 'versioned',
        'link_created', 'link_replayed', 'link_conflict', 'prompt_selected'
    )),
    actor             TEXT NOT NULL,
    occurred_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    request_id        UUID,
    detail            JSONB NOT NULL DEFAULT '{}'::jsonb,
    CHECK (record_id IS NOT NULL OR conflict_id IS NOT NULL)
);

CREATE INDEX domain_record_audit_record_idx
    ON domain.record_audit (record_id, occurred_at DESC)
    WHERE record_id IS NOT NULL;
CREATE INDEX domain_record_audit_conflict_idx
    ON domain.record_audit (conflict_id, occurred_at DESC)
    WHERE conflict_id IS NOT NULL;

COMMENT ON TABLE domain.record_conflicts IS
    'Same-ID/different-content submissions are preserved here and never overwrite the accepted record.';
COMMENT ON TABLE domain.record_audit IS
    'Append-only actor/time evidence for domain writes, relationships, conflicts, tombstones, and prompt selection.';

COMMIT;
