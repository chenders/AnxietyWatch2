-- Revert vitals:retention from pg
SELECT remove_retention_policy('samples', if_exists => TRUE);
