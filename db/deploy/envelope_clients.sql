-- Deploy vitals:envelope_clients to pg
-- This change does not touch samples, so the spool-empty guard does not apply.
-- Register-once/update-many envelope credentials. The secret is the libsodium
-- secretbox key and is never logged.
BEGIN;

CREATE TABLE envelope_clients (
    client_id    UUID        PRIMARY KEY,
    secret       BYTEA       NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ
);

COMMENT ON TABLE envelope_clients IS
    'Register-once/update-many envelope credentials; secret is the libsodium secretbox key and is never logged.';

COMMENT ON COLUMN envelope_clients.secret IS
    'libsodium secretbox key; never log this value.';

COMMIT;
