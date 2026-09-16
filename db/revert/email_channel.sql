-- Revert vitals:email_channel from pg
BEGIN;
DROP TABLE IF EXISTS email_channel_config;
COMMIT;
