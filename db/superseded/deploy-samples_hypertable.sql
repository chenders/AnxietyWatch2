-- Deploy vitals:samples_hypertable to pg
-- One-day chunks: a night of sampling lands in one or two chunks, which keeps
-- overnight queries to a single partition and makes compression boundaries fall
-- on a natural unit.
-- No BEGIN/COMMIT: create_hypertable manages its own transaction.
SELECT create_hypertable('samples', by_range('timestamp', INTERVAL '1 day'),
                         migrate_data => TRUE, if_not_exists => TRUE);
