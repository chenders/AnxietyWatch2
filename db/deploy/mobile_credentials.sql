-- Deploy vitals:mobile_credentials to pg
-- Private keys and enrollment plaintext never enter PostgreSQL.
BEGIN;

CREATE TABLE enrollment_artifacts (
    enrollment_id      UUID PRIMARY KEY,
    server_instance_id UUID NOT NULL,
    token_hash         BYTEA NOT NULL CHECK (octet_length(token_hash) = 32),
    scopes             TEXT[] NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at         TIMESTAMPTZ NOT NULL,
    consumed_at        TIMESTAMPTZ,
    created_by         TEXT NOT NULL
);
CREATE INDEX enrollment_artifacts_live_idx
    ON enrollment_artifacts (expires_at) WHERE consumed_at IS NULL;

CREATE TABLE mobile_credentials (
    client_id             UUID NOT NULL,
    key_id                UUID NOT NULL,
    label                 TEXT NOT NULL,
    public_key            BYTEA NOT NULL CHECK (octet_length(public_key) = 65),
    public_key_fingerprint TEXT NOT NULL,
    scopes                TEXT[] NOT NULL,
    issued_at             TIMESTAMPTZ NOT NULL,
    expires_at            TIMESTAMPTZ NOT NULL,
    last_used_at          TIMESTAMPTZ,
    revoked_at            TIMESTAMPTZ,
    overlap_until         TIMESTAMPTZ,
    PRIMARY KEY (client_id, key_id),
    CHECK (issued_at < expires_at),
    CHECK (revoked_at IS NULL OR revoked_at >= issued_at)
);
CREATE INDEX mobile_credentials_active_idx
    ON mobile_credentials (client_id, expires_at)
    WHERE revoked_at IS NULL;

CREATE TABLE credential_audit (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    client_id   UUID,
    key_id      UUID,
    action      TEXT NOT NULL CHECK (action IN ('enrollment_created', 'enrolled', 'rotated', 'revoked', 'used')),
    actor       TEXT NOT NULL,
    reason      TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE enrollment_artifacts IS
    'Single-use ten-minute artifacts retain only a SHA-256 token hash; plaintext is shown once to the operator.';
COMMENT ON TABLE mobile_credentials IS
    'Per-device public-key credentials; private keys remain in the device Secure Enclave.';
COMMENT ON TABLE credential_audit IS
    'Append-only lifecycle evidence for mobile enrollment and key use.';

COMMIT;
