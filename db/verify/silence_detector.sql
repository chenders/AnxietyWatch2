-- Verify vitals:silence_detector
SELECT next_repeat_at, transition_id, declared_absent, muted_until
  FROM status_observations WHERE FALSE;
SELECT subject, transition_id, suppressed_by_mute
  FROM notification_events WHERE FALSE;
