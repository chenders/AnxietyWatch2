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

CREATE FUNCTION reject_air11_admin_audit_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'air11_admin_audit is append-only; operation=% is forbidden', TG_OP;
END;
$$;

CREATE TRIGGER air11_admin_audit_immutable
    BEFORE UPDATE OR DELETE ON air11_admin_audit
    FOR EACH ROW EXECUTE FUNCTION reject_air11_admin_audit_mutation();

CREATE TRIGGER air11_admin_audit_truncate_immutable
    BEFORE TRUNCATE ON air11_admin_audit
    FOR EACH STATEMENT EXECUTE FUNCTION reject_air11_admin_audit_mutation();

GRANT SELECT, INSERT, UPDATE ON air11_integration_config TO vitals;
GRANT SELECT, INSERT ON air11_admin_audit TO vitals;
REVOKE UPDATE, DELETE, TRUNCATE ON air11_admin_audit FROM vitals;

COMMENT ON TABLE air11_integration_config IS
    'Non-secret AirSense opt-in state. BLE credentials remain in the external operator-managed file.';
COMMENT ON TABLE air11_admin_audit IS
    'Append-only audit of fixed AirSense admin actions; never store credential values or raw device identifiers.';

COMMIT;
