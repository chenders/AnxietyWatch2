-- Deploy vitals:status_observations to pg
-- This change does not touch samples, so the spool-empty guard does not apply.
-- reported_at is the last affirmative assertion; changed_at is the last value
-- change. Freshness is stored, never derived (Home Assistant precedent, spec
-- §8b).
BEGIN;

CREATE TABLE status_observations (
    subject     TEXT        PRIMARY KEY,
    state       TEXT        NOT NULL,
    reason      TEXT,
    reported_at TIMESTAMPTZ NOT NULL,
    changed_at  TIMESTAMPTZ NOT NULL
);

COMMENT ON TABLE status_observations IS
    'reported_at is the last affirmative assertion and changed_at the last value change; freshness is stored, never derived (Home Assistant precedent, spec §8b).';

COMMIT;
