-- Deploy vitals:silence_detector to pg
-- Durable detector state prevents a process restart from becoming a duplicate alarm.
BEGIN;

ALTER TABLE status_observations
    ADD COLUMN next_repeat_at TIMESTAMPTZ,
    ADD COLUMN transition_id TEXT NOT NULL DEFAULT '',
    ADD COLUMN declared_absent BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN muted_until TIMESTAMPTZ;

CREATE TABLE notification_events (
    id                 BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    subject            TEXT        NOT NULL,
    transition_id      TEXT        NOT NULL,
    state              TEXT        NOT NULL,
    reason             TEXT        NOT NULL,
    emitted_at         TIMESTAMPTZ NOT NULL,
    suppressed_by_mute BOOLEAN     NOT NULL DEFAULT FALSE
);

CREATE INDEX notification_events_subject_transition_idx
    ON notification_events (subject, transition_id, emitted_at DESC);

COMMENT ON TABLE notification_events IS
    'Detector events retain transition identity so outbound channels can coalesce repeats without losing audit evidence.';

GRANT SELECT, INSERT ON notification_events TO vitals;
GRANT USAGE, SELECT ON SEQUENCE notification_events_id_seq TO vitals;

COMMIT;
