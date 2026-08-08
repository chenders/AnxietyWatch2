-- Deploy vitals:source_declarations to pg
BEGIN;

CREATE TABLE source_declarations (
    source_id INTEGER PRIMARY KEY REFERENCES vitals_source(id),
    is_configured BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMIT;
