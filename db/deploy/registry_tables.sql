-- Deploy vitals:registry_tables to pg
-- STRUCTURE ONLY. The rows are reference data generated from
-- vitals/registry.toml and reconciled on service start — see amendment 01 §7.
-- Two owners of the same object is the drift this split exists to prevent.
BEGIN;

CREATE TABLE vitals_type (
    id        INTEGER PRIMARY KEY,
    name      TEXT             NOT NULL UNIQUE,
    unit      TEXT             NOT NULL,
    range_lo  DOUBLE PRECISION NOT NULL,
    range_hi  DOUBLE PRECISION NOT NULL,
    precision DOUBLE PRECISION NOT NULL,
    CHECK (range_lo < range_hi)
);

CREATE TABLE vitals_source (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    transport TEXT NOT NULL
);

COMMIT;
