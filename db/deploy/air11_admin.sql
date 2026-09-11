-- Deploy vitals:air11_admin to pg
-- AirSense admin state contains no BLE address, client identifier, key, or
-- other credential material. The collector continues to load its credential
-- from the operator-managed external file.
BEGIN;

CREATE TABLE air11_integration_config (
    singleton  BOOLEAN     PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    opted_in   BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO air11_integration_config (singleton, opted_in)
VALUES (TRUE, FALSE);

CREATE TABLE air11_admin_audit (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    action     TEXT        NOT NULL CHECK (action IN
                 ('opt_in', 'opt_out', 'start', 'stop', 'restart', 'credential_check')),
    outcome    TEXT        NOT NULL CHECK (outcome IN ('requested', 'succeeded', 'failed', 'blocked')),
    detail     TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX air11_admin_audit_created_idx
    ON air11_admin_audit (created_at DESC, id DESC);

COMMENT ON TABLE air11_integration_config IS
    'Non-secret AirSense opt-in state. BLE credentials remain in the external operator-managed file.';
COMMENT ON TABLE air11_admin_audit IS
    'Append-only audit of fixed AirSense admin actions; never store credential values or raw device identifiers.';

COMMIT;
