-- Deploy vitals:email_channel to pg
-- The app password is ciphertext only; plaintext is never a column.
BEGIN;

CREATE TABLE email_channel_config (
    singleton           BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    enabled             BOOLEAN NOT NULL DEFAULT FALSE,
    from_address        TEXT,
    destination_address TEXT,
    password_nonce      BYTEA,
    password_ciphertext BYTEA,
    password_last_four  TEXT CHECK (password_last_four IS NULL OR length(password_last_four) = 4),
    last_test_succeeded BOOLEAN,
    last_test_at        TIMESTAMPTZ,
    last_test_detail    TEXT
);

COMMENT ON TABLE email_channel_config IS
    'Gmail STARTTLS configuration; password is encrypted at rest and only its last four characters are displayed.';

COMMIT;
