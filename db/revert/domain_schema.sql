-- Revert vitals:domain_schema from pg
BEGIN;

ALTER TABLE IF EXISTS domain.records RENAME TO domain_records;
ALTER TABLE IF EXISTS domain.domain_records SET SCHEMA public;
DROP SCHEMA IF EXISTS domain;

COMMIT;
