-- Deploy vitals:attempts to pg
-- This change does not touch samples, so the spool-empty guard does not apply.
-- A row exists before its work begins. A killed process leaves outcome NULL,
-- which the liveness evaluator degrades to UNKNOWN past expected_duration_s.
-- Effect distinguishes created/updated/replayed/rejected counts because a no-op
-- replay success is not evidence of progress.
BEGIN;

CREATE TABLE attempts (
    id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    kind                TEXT        NOT NULL,
    subject             TEXT        NOT NULL,
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expected_duration_s INTEGER     NOT NULL CHECK (expected_duration_s > 0),
    finished_at         TIMESTAMPTZ,
    outcome             TEXT CHECK (outcome IN ('succeeded', 'failed', 'cancelled')),
    effect              JSONB,
    high_water_mark     TIMESTAMPTZ,
    detail              TEXT,
    CHECK ((finished_at IS NULL) = (outcome IS NULL))
);

CREATE INDEX attempts_kind_subject_started_idx
    ON attempts (kind, subject, started_at DESC);

COMMENT ON TABLE attempts IS
    'A row exists before work begins; a killed process leaves outcome NULL and degrades to UNKNOWN after expected_duration_s. Effect distinguishes created/updated/replayed/rejected because no-op replay success is not progress.';

COMMIT;
