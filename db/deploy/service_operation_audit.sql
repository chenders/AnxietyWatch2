-- Deploy vitals:service_operation_audit to pg
-- Immutable audit evidence for allowlisted service operations.
BEGIN;

CREATE TABLE service_operation_audit (
    id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    occurred_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    actor               TEXT NOT NULL,
    target              TEXT NOT NULL,
    action              TEXT NOT NULL CHECK (action IN ('restart', 'stop')),
    confirmation_class  TEXT NOT NULL CHECK (confirmation_class IN ('standard', 'strong')),
    attempt_id          BIGINT NOT NULL UNIQUE REFERENCES attempts(id),
    outcome             TEXT NOT NULL CHECK (outcome IN ('succeeded', 'failed', 'cancelled')),
    detail              TEXT,
    CHECK (length(trim(actor)) > 0),
    CHECK (length(trim(target)) > 0)
);

CREATE INDEX service_operation_audit_occurred_idx
    ON service_operation_audit (occurred_at DESC);

CREATE FUNCTION reject_service_operation_audit_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'service_operation_audit is append-only; operation=% is forbidden', TG_OP;
END;
$$;

CREATE TRIGGER service_operation_audit_immutable
    BEFORE UPDATE OR DELETE ON service_operation_audit
    FOR EACH ROW EXECUTE FUNCTION reject_service_operation_audit_mutation();

COMMENT ON TABLE service_operation_audit IS
    'Append-only evidence for authenticated, allowlisted service operations; one row per attempt.';

COMMIT;
