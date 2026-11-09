-- Verify vitals:domain_schema
-- Catalog queries must ASSERT, not merely SELECT. A bare
-- `SELECT ... WHERE schema_name = 'domain'` returns zero rows when the schema
-- is absent, and zero rows is not an error -- so psql exits 0 and the verify
-- passes against an empty database. This was the only fail-open script of the
-- 26, and it caused a change to be recorded as deployed when its schema did
-- not exist; the next change then failed with "schema does not exist".
--
-- `1 / count(*)` divides by zero when the object is missing, which is an
-- error. Scripts that name a real relation and its columns (even with
-- `WHERE FALSE`) already fail at parse time and need no such idiom.
SELECT 1 / count(*)
  FROM information_schema.schemata
 WHERE schema_name = 'domain';

SELECT 1 / count(*)
  FROM information_schema.tables
 WHERE table_schema = 'domain' AND table_name = 'records';
