-- Deploy vitals:telemetry to pg
-- Telemetry is opt-in and stores only encrypted license-key material.
BEGIN;

CREATE TABLE telemetry_config (
    singleton           BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    enabled             BOOLEAN NOT NULL DEFAULT FALSE,
    region              TEXT NOT NULL DEFAULT 'us' CHECK (region IN ('us', 'eu')),
    key_nonce           BYTEA,
    key_ciphertext      BYTEA,
    key_last_four       TEXT CHECK (key_last_four IS NULL OR length(key_last_four) BETWEEN 1 AND 4),
    last_test_succeeded BOOLEAN,
    last_test_at        TIMESTAMPTZ,
    last_test_detail    TEXT,
    dropped_count       BIGINT NOT NULL DEFAULT 0 CHECK (dropped_count >= 0)
);

COMMENT ON TABLE telemetry_config IS
    'Opt-in operational OTLP export; license key is encrypted and shown only as presence plus last four.';

COMMIT;
