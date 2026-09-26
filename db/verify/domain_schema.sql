-- Verify vitals:domain_schema
SELECT schema_name
  FROM information_schema.schemata
 WHERE schema_name = 'domain';

SELECT table_schema, table_name
  FROM information_schema.tables
 WHERE table_schema = 'domain' AND table_name = 'records';
