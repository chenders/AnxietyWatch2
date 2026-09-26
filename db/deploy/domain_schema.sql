-- Deploy vitals:domain_schema to pg
-- Move the existing generic envelope table out of public and into the
-- dedicated domain schema selected by T-M5a.
BEGIN;

CREATE SCHEMA IF NOT EXISTS domain;

ALTER TABLE IF EXISTS public.domain_records SET SCHEMA domain;
ALTER TABLE IF EXISTS domain.domain_records RENAME TO records;

COMMENT ON SCHEMA domain IS
    'Authoritative versioned user-domain records; physiological samples remain outside this schema.';

COMMIT;
