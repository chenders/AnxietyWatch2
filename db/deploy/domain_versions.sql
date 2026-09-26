-- Deploy vitals:domain_versions to pg
-- Add immutable version metadata and the versioned relationship surface.
BEGIN;

ALTER TABLE domain.records
    ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1
        CHECK (schema_version > 0),
    ADD COLUMN recorded_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- The API always supplies the iPhone capture time. The migration default only
-- keeps direct SQL fixtures and pre-T-M5b rows structurally valid.
ALTER TABLE domain.records
    ALTER COLUMN recorded_at SET NOT NULL,
    ADD COLUMN supersedes_id UUID REFERENCES domain.records(id),
    ADD COLUMN tombstone_target_id UUID REFERENCES domain.records(id),
    ADD COLUMN actor TEXT NOT NULL DEFAULT 'client',
    ADD CONSTRAINT domain_records_not_self_version
        CHECK (supersedes_id IS NULL OR supersedes_id <> id),
    ADD CONSTRAINT domain_records_not_self_tombstone
        CHECK (tombstone_target_id IS NULL OR tombstone_target_id <> id),
    ADD CONSTRAINT domain_records_tombstone_not_version
        CHECK (tombstone_target_id IS NULL OR supersedes_id IS NULL);

CREATE INDEX domain_records_supersedes_idx
    ON domain.records (supersedes_id)
    WHERE supersedes_id IS NOT NULL;
CREATE INDEX domain_records_tombstone_idx
    ON domain.records (tombstone_target_id)
    WHERE tombstone_target_id IS NOT NULL;
CREATE INDEX domain_records_recorded_idx
    ON domain.records (kind, recorded_at DESC);

-- Relationship rows retain client identity and their own immutable history.
-- Common relation kinds cover the legacy domain shape without making the
-- canonical record envelope depend on provider or serial IDs.
CREATE TABLE domain.record_links (
    id                  UUID PRIMARY KEY,
    relation_kind       TEXT NOT NULL CHECK (relation_kind IN (
        'journal_song', 'journal_tag', 'medication_dose',
        'medication_follow_up', 'prescription_medication',
        'prescription_pharmacy', 'profile_context', 'therapy_schedule',
        'therapy_conflict', 'journal_lab', 'song_tag', 'prompt_selection'
    )),
    source_record_id    UUID NOT NULL REFERENCES domain.records(id),
    target_record_id    UUID NOT NULL REFERENCES domain.records(id),
    schema_version      INTEGER NOT NULL DEFAULT 1 CHECK (schema_version > 0),
    content             JSONB NOT NULL DEFAULT '{}'::jsonb,
    content_sha256     BYTEA NOT NULL CHECK (octet_length(content_sha256) = 32),
    recorded_at         TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    supersedes_id       UUID REFERENCES domain.record_links(id),
    tombstone_target_id UUID REFERENCES domain.record_links(id),
    actor               TEXT NOT NULL DEFAULT 'client',
    CHECK (source_record_id <> target_record_id),
    CHECK (supersedes_id IS NULL OR supersedes_id <> id),
    CHECK (tombstone_target_id IS NULL OR tombstone_target_id <> id),
    CHECK (tombstone_target_id IS NULL OR supersedes_id IS NULL)
);

CREATE INDEX domain_record_links_source_idx
    ON domain.record_links (source_record_id, relation_kind, recorded_at DESC);
CREATE INDEX domain_record_links_target_idx
    ON domain.record_links (target_record_id, relation_kind, recorded_at DESC);

COMMENT ON TABLE domain.records IS
    'Append-only client-minted domain records. Edits use a new id and supersedes_id; deletions use a tombstone_target_id.';
COMMENT ON TABLE domain.record_links IS
    'Append-only client-minted relationships for journal, song, medication, profile, therapy, lab, tag, and prompt-selection records.';

COMMIT;
