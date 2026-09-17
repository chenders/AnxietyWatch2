-- Deploy vitals:mobile_request_replays to pg
-- Durable request admission prevents replay of an authenticated HTTP request.
BEGIN;

CREATE TABLE mobile_request_replays (
    client_id    UUID NOT NULL,
    request_id   UUID NOT NULL,
    request_digest BYTEA NOT NULL CHECK (octet_length(request_digest) = 32),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (client_id, request_id)
);
CREATE INDEX mobile_request_replays_expiry_idx
    ON mobile_request_replays (expires_at);

COMMENT ON TABLE mobile_request_replays IS
    'Durable RFC 9421 request admissions; rows remain at least 24 hours before bounded cleanup.';

COMMIT;
