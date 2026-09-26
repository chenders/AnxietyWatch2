-- Deploy vitals:operator_overrides to pg
BEGIN;

CREATE TABLE source_overrides (
    source_id INTEGER PRIMARY KEY REFERENCES vitals_source(id),
    display_name TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE type_overrides (
    type_id INTEGER PRIMARY KEY REFERENCES vitals_type(id),
    display_name TEXT,
    unit TEXT,
    precision DOUBLE PRECISION,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMIT;
