-- Deploy vitals:apns_delivery to pg
-- Immutable operator message, provider-attempt, and client-observation evidence.
BEGIN;

CREATE TABLE apns_messages (
    message_id             UUID PRIMARY KEY,
    server_instance_id     UUID NOT NULL,
    audit_actor            TEXT NOT NULL,
    recipient_scope        TEXT NOT NULL CHECK (recipient_scope IN ('device', 'all_active', 'user')),
    target_device_id       UUID,
    target_user_id         UUID,
    target_snapshot        JSONB NOT NULL,
    payload_class          TEXT NOT NULL,
    category               TEXT,
    deep_link              TEXT,
    is_test                BOOLEAN NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((recipient_scope = 'device') = (target_device_id IS NOT NULL)),
    CHECK ((recipient_scope = 'user') = (target_user_id IS NOT NULL)),
    CHECK (recipient_scope <> 'all_active' OR (target_device_id IS NULL AND target_user_id IS NULL))
);

CREATE TABLE apns_delivery_attempts (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    message_id            UUID NOT NULL REFERENCES apns_messages(message_id),
    server_instance_id    UUID NOT NULL,
    device_id             UUID NOT NULL,
    client_id             UUID NOT NULL,
    token_fingerprint     TEXT NOT NULL,
    attempt_number        INTEGER NOT NULL CHECK (attempt_number > 0),
    state                 TEXT NOT NULL CHECK (state IN ('attempted', 'accepted', 'rejected', 'retryable', 'invalidated')),
    provider_status       INTEGER,
    provider_reason       TEXT,
    retryable             BOOLEAN NOT NULL,
    attempted_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, device_id, attempt_number)
);

CREATE INDEX apns_delivery_attempts_client_idx
    ON apns_delivery_attempts (message_id, client_id);

CREATE TABLE apns_client_outcomes (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    message_id            UUID NOT NULL REFERENCES apns_messages(message_id),
    client_id             UUID NOT NULL,
    outcome               TEXT NOT NULL CHECK (outcome IN ('displayed', 'acknowledged')),
    observed_at           TIMESTAMPTZ NOT NULL,
    received_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (message_id, client_id, outcome)
);

COMMENT ON TABLE apns_messages IS
    'Immutable operator-directed APNs message requests and recipient snapshots.';
COMMENT ON TABLE apns_delivery_attempts IS
    'Append-only APNs provider acceptance/rejection evidence; acceptance is not delivery.';
COMMENT ON TABLE apns_client_outcomes IS
    'Append-only device-observed display and acknowledgement facts.';

GRANT SELECT, INSERT, UPDATE ON apns_messages, apns_delivery_attempts, apns_client_outcomes TO vitals;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO vitals;

COMMIT;
