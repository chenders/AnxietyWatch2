-- Deploy vitals:aircraft_warning to pg
-- Durable warning decisions and their human reviews. A warning is not a noise
-- event: it records what the model predicted and whether lights fired; review
-- records whether the prediction matched what the operator later perceived.

BEGIN;

CREATE TABLE aircraft_warning (
    warning_id       text PRIMARY KEY,
    decided_at       timestamptz NOT NULL,
    fired            boolean NOT NULL,
    suppressed_by    text CHECK (suppressed_by IS NULL OR suppressed_by IN
                                  ('quiet_hours', 'cooldown', 'disarmed')),
    test              boolean NOT NULL DEFAULT false,
    icao_hex          text CHECK (icao_hex IS NULL OR icao_hex ~ '^[0-9A-F]{6}$'),
    registration      text,
    callsign          text,
    aircraft_type     text,
    operator_kind     text,
    loudness_band     text NOT NULL CHECK (loudness_band IN
                         ('VERY LOUD', 'LOUD', 'AUDIBLE', 'FAINT', 'UNKNOWN', 'UNLOCATED')),
    notice_seconds    double precision,
    late              boolean NOT NULL DEFAULT false,
    hold_seconds      double precision CHECK (hold_seconds IS NULL OR hold_seconds >= 0),
    prediction        jsonb,
    matched_pass      jsonb,
    recovered_from    text,
    raw               jsonb NOT NULL,
    CHECK ((fired AND suppressed_by IS NULL) OR
           (NOT fired AND suppressed_by IS NOT NULL))
);

CREATE INDEX aircraft_warning_decided_idx ON aircraft_warning (decided_at DESC);
CREATE INDEX aircraft_warning_hex_idx ON aircraft_warning (icao_hex, decided_at DESC)
    WHERE icao_hex IS NOT NULL;

CREATE TABLE aircraft_warning_review (
    warning_id     text NOT NULL REFERENCES aircraft_warning (warning_id),
    reviewed_at    timestamptz NOT NULL,
    verdict        text NOT NULL CHECK (verdict IN
                       ('accurate', 'inaccurate', 'unknown', 'expired_unreviewed')),
    note           text,
    provenance     text NOT NULL DEFAULT 'admin_operator',
    PRIMARY KEY (warning_id, reviewed_at)
);

CREATE INDEX aircraft_warning_review_latest_idx
    ON aircraft_warning_review (warning_id, reviewed_at DESC);

CREATE VIEW aircraft_warning_with_latest_review AS
SELECT warning.*,
       review.reviewed_at,
       review.verdict AS review_verdict,
       review.note AS review_note
FROM aircraft_warning warning
LEFT JOIN LATERAL (
    SELECT reviewed_at, verdict, note
    FROM aircraft_warning_review
    WHERE warning_id = warning.warning_id
    ORDER BY reviewed_at DESC
    LIMIT 1
) review ON true;

COMMENT ON TABLE aircraft_warning IS
    'Every aircraft warning decision, fired or explicitly suppressed, including recovered historical journal warnings.';
COMMENT ON TABLE aircraft_warning_review IS
    'Append-only warning review lifecycle: human accurate/inaccurate/unknown, or system expired_unreviewed after the three-hour review window. Unknown and expired contribute nothing to training.';

COMMIT;
