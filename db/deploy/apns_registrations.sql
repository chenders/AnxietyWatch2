-- Deploy vitals:apns_registrations to pg
-- APNs tokens are encrypted at rest; plaintext tokens and provider credentials never enter audit rows.
BEGIN;

CREATE TABLE apns_device_registrations (
    server_instance_id       UUID NOT NULL,
    device_id                UUID NOT NULL,
    client_id                UUID NOT NULL,
    token_ciphertext         BYTEA,
    token_fingerprint        TEXT,
    environment              TEXT NOT NULL CHECK (environment IN ('sandbox', 'production')),
    app_identifier           TEXT NOT NULL,
    app_version              TEXT NOT NULL,
    build                    TEXT NOT NULL,
    authorization_status     TEXT NOT NULL,
    alerts_enabled           BOOLEAN NOT NULL,
    sounds_enabled           BOOLEAN NOT NULL,
    badges_enabled           BOOLEAN NOT NULL,
    first_seen_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at               TIMESTAMPTZ,
    last_provider_reason     TEXT,
    PRIMARY KEY (server_instance_id, device_id),
    UNIQUE (server_instance_id, client_id),
    CHECK ((token_ciphertext IS NULL) = (token_fingerprint IS NULL)),
    CHECK (revoked_at IS NULL OR revoked_at >= first_seen_at)
);

CREATE INDEX apns_device_registrations_active_idx
    ON apns_device_registrations (server_instance_id, last_seen_at DESC)
    WHERE revoked_at IS NULL;

CREATE TABLE apns_registration_audit (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    server_instance_id    UUID NOT NULL,
    device_id             UUID NOT NULL,
    client_id             UUID NOT NULL,
    action                TEXT NOT NULL CHECK (action IN ('registered', 'refreshed', 'revoked', 'provider_rejected')),
    reason                TEXT,
    occurred_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE apns_device_registrations IS
    'Current APNs registration facts; token ciphertext requires the protected server token key.';
COMMENT ON TABLE apns_registration_audit IS
    'Append-only APNs registration lifecycle evidence without token material.';

GRANT SELECT, INSERT, UPDATE ON apns_device_registrations TO vitals;
GRANT SELECT, INSERT ON apns_registration_audit TO vitals;

COMMIT;
