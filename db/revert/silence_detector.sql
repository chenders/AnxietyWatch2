-- Revert vitals:silence_detector from pg
BEGIN;
DROP TABLE IF EXISTS notification_events;
ALTER TABLE status_observations
    DROP COLUMN IF EXISTS next_repeat_at,
    DROP COLUMN IF EXISTS transition_id,
    DROP COLUMN IF EXISTS declared_absent,
    DROP COLUMN IF EXISTS muted_until;
COMMIT;
