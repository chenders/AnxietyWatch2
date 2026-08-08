-- Revert vitals:source_declarations from pg
BEGIN;

DROP TABLE source_declarations;

COMMIT;
