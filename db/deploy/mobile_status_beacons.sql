-- Deploy vitals:mobile_status_beacons to pg
-- Operational phone evidence is separate from the append-only physiological samples.
BEGIN;

CREATE TABLE mobile_status_beacons (
    client_id             UUID NOT NULL,
    beacon_id             UUID NOT NULL,
    body_sha256           BYTEA NOT NULL CHECK (octet_length(body_sha256) = 32),
    asserted_at           TIMESTAMPTZ NOT NULL,
    changed_at            TIMESTAMPTZ NOT NULL,
    app_version           TEXT NOT NULL,
    registry_sha256       TEXT NOT NULL,
    outbox_count          BIGINT NOT NULL CHECK (outbox_count >= 0),
    oldest_outbox_age_s   BIGINT CHECK (oldest_outbox_age_s IS NULL OR oldest_outbox_age_s >= 0),
    last_server_ack_at    TIMESTAMPTZ,
    type_facts            JSONB NOT NULL,
    received_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (client_id, beacon_id),
    CONSTRAINT mobile_status_beacons_changed_at_check CHECK (changed_at <= asserted_at)
);

CREATE INDEX mobile_status_beacons_latest_idx
    ON mobile_status_beacons (client_id, asserted_at DESC);

COMMENT ON TABLE mobile_status_beacons IS
    'Append-only operational phone assertions; no HealthKit measurement values are stored here.';
COMMENT ON COLUMN mobile_status_beacons.type_facts IS
    'Per-type configured and observed outcomes; read authorization is never represented as granted.';

COMMIT;
