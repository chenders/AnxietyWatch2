#!/usr/bin/env bash
# Apply only the checked-in silence-detector schema under the existing recovery gate.
set -euo pipefail

if [[ "${1:-}" == "--self-test" ]]; then
    grep -Fq 'VITALS_SCHEMA_APPLY_CONFIRM' "$0"
    grep -Fq 'github-deploy-gate.json' "$0"
    grep -Fq 'samples' "$0"
    grep -Fq 'ON_ERROR_STOP' "$0"
    printf '%s\n' 'apply-silence-detector-schema: self-test passed'
    exit 0
fi

if (( $# != 0 )); then
    printf 'schema apply blocked: unexpected argument: %s\n' "$1" >&2
    exit 1
fi
if [[ "${VITALS_SCHEMA_APPLY_CONFIRM:-}" != "1" ]]; then
    printf '%s\n' 'schema apply blocked: VITALS_SCHEMA_APPLY_CONFIRM=1 is required' >&2
    exit 1
fi
if (( EUID != 0 )); then
    printf 'schema apply blocked: uid=%s, expected uid=0\n' "$EUID" >&2
    exit 1
fi

repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
sql_file="$repo_root/db/deploy/silence_detector.sql"
database=${VITALS_DATABASE_NAME:-vitals}
gate=${VITALS_BACKUP_GATE:-/var/lib/vitals/recovery/github-deploy-gate.json}
receipt=${VITALS_SCHEMA_RECEIPT:-/var/lib/vitals/recovery/silence-detector-schema-latest.json}
expected_sql_sha256=b49f57e35b9ced4b1b18189b4c6ff81a8cee69c04e057fde138ac8edf90e8a63

[[ -f "$sql_file" && ! -L "$sql_file" ]] || {
    printf 'schema apply blocked: checked-in SQL file is missing or symlinked: %s\n' "$sql_file" >&2
    exit 1
}
actual_sql_sha256=$(sha256sum "$sql_file" | awk '{print $1}')
[[ "$actual_sql_sha256" == "$expected_sql_sha256" ]] || {
    printf 'schema apply blocked: checked-in SQL hash actual=%s expected=%s\n' "$actual_sql_sha256" "$expected_sql_sha256" >&2
    exit 1
}
[[ -f "$gate" && ! -L "$gate" ]] || {
    printf 'schema apply blocked: backup gate is missing or symlinked: %s\n' "$gate" >&2
    exit 1
}
gate_uid=$(stat -c '%u' "$gate")
gate_mode=$(stat -c '%a' "$gate")
gate_mode_value=$((8#$gate_mode))
[[ "$gate_uid" == 0 && $((gate_mode_value & 022)) == 0 ]] || {
    printf 'schema apply blocked: backup gate owner uid=%s mode=%s is not root-only\n' "$gate_uid" "$gate_mode" >&2
    exit 1
}
jq -e '
    .backup_verified == true and
    .restore_verified == true and
    .off_pi == true and
    (.spool_fingerprint | type == "string" and length > 0) and
    (.rollback_release | type == "string" and length > 0) and
    (.encryption_key_location | type == "string" and length > 0)
' "$gate" >/dev/null || {
    printf 'schema apply blocked: backup gate lacks verified off-Pi recovery evidence: %s\n' "$gate" >&2
    exit 1
}
if grep -Eiq '(^|[^[:alnum:]_])(UPDATE|DELETE|samples)([^[:alnum:]_]|$)' "$sql_file"; then
    printf 'schema apply blocked: migration references samples or a removal/update statement: %s\n' "$sql_file" >&2
    exit 1
fi

schema_parts=$(runuser -u postgres -- psql --dbname "$database" --tuples-only --no-align \
    --command "SELECT to_regclass('public.status_observations') IS NOT NULL, (SELECT count(*) FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'status_observations' AND column_name IN ('next_repeat_at', 'transition_id', 'declared_absent', 'muted_until')), to_regclass('public.notification_events') IS NOT NULL")
IFS='|' read -r base_present detector_column_count notification_present <<< "$schema_parts"
[[ "$base_present" == "t" ]] || {
    printf 'schema apply blocked: public.status_observations is absent\n' >&2
    exit 1
}
base_schema_ok=$(runuser -u postgres -- psql --dbname "$database" --tuples-only --no-align <<'SQL'
WITH expected(column_name, data_type, is_nullable) AS (
    VALUES
        ('subject', 'text', 'NO'),
        ('state', 'text', 'NO'),
        ('reason', 'text', 'YES'),
        ('reported_at', 'timestamp with time zone', 'NO'),
        ('changed_at', 'timestamp with time zone', 'NO')
), columns_ok AS (
    SELECT count(*) = 5 FROM expected e JOIN information_schema.columns c
      ON c.table_schema = 'public' AND c.table_name = 'status_observations'
     AND c.column_name = e.column_name AND c.data_type = e.data_type
     AND c.is_nullable = e.is_nullable
), primary_key_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_constraint
      WHERE conrelid = to_regclass('public.status_observations') AND contype = 'p'
        AND pg_get_constraintdef(oid) = 'PRIMARY KEY (subject)')
)
SELECT (SELECT * FROM columns_ok) AND (SELECT * FROM primary_key_ok);
SQL
)
[[ "$base_schema_ok" == "t" ]] || {
    printf 'schema apply blocked: pre-existing status_observations shape is not verified\n' >&2
    exit 1
}

schema_complete=$(runuser -u postgres -- psql --dbname "$database" --tuples-only --no-align <<'SQL'
WITH expected(table_name, column_name, data_type, is_nullable) AS (
    VALUES
        ('status_observations', 'subject', 'text', 'NO'),
        ('status_observations', 'state', 'text', 'NO'),
        ('status_observations', 'reason', 'text', 'YES'),
        ('status_observations', 'reported_at', 'timestamp with time zone', 'NO'),
        ('status_observations', 'changed_at', 'timestamp with time zone', 'NO'),
        ('status_observations', 'next_repeat_at', 'timestamp with time zone', 'YES'),
        ('status_observations', 'transition_id', 'text', 'NO'),
        ('status_observations', 'declared_absent', 'boolean', 'NO'),
        ('status_observations', 'muted_until', 'timestamp with time zone', 'YES'),
        ('notification_events', 'id', 'bigint', 'NO'),
        ('notification_events', 'subject', 'text', 'NO'),
        ('notification_events', 'transition_id', 'text', 'NO'),
        ('notification_events', 'state', 'text', 'NO'),
        ('notification_events', 'reason', 'text', 'NO'),
        ('notification_events', 'emitted_at', 'timestamp with time zone', 'NO'),
        ('notification_events', 'suppressed_by_mute', 'boolean', 'NO')
), columns_ok AS (
    SELECT count(*) = 16
      FROM expected e
      JOIN information_schema.columns c
        ON c.table_schema = 'public' AND c.table_name = e.table_name
       AND c.column_name = e.column_name AND c.data_type = e.data_type
       AND c.is_nullable = e.is_nullable
), defaults_ok AS (
    SELECT COALESCE((SELECT pg_get_expr(d.adbin, d.adrelid) = $$''::text$$ FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum WHERE d.adrelid = to_regclass('public.status_observations') AND a.attname = 'transition_id'), false)
       AND COALESCE((SELECT pg_get_expr(d.adbin, d.adrelid) = 'false' FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum WHERE d.adrelid = to_regclass('public.status_observations') AND a.attname = 'declared_absent'), false)
       AND COALESCE((SELECT pg_get_expr(d.adbin, d.adrelid) = 'false' FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum WHERE d.adrelid = to_regclass('public.notification_events') AND a.attname = 'suppressed_by_mute'), false)
), primary_key_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_constraint
     WHERE conrelid = to_regclass('public.notification_events') AND contype = 'p'
       AND pg_get_constraintdef(oid) = 'PRIMARY KEY (id)')
), identity_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_attribute
     WHERE attrelid = to_regclass('public.notification_events') AND attname = 'id'
       AND attidentity = 'a')
), index_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid
     WHERE c.oid = to_regclass('public.notification_events_subject_transition_idx')
       AND i.indrelid = to_regclass('public.notification_events')
       AND NOT i.indisunique AND NOT i.indisprimary
       AND i.indnkeyatts = 3 AND i.indnatts = 3
       AND pg_get_indexdef(i.indexrelid) = 'CREATE INDEX notification_events_subject_transition_idx ON public.notification_events USING btree (subject, transition_id, emitted_at DESC)')
)
SELECT (SELECT * FROM columns_ok) AND (SELECT * FROM defaults_ok)
   AND (SELECT * FROM primary_key_ok) AND (SELECT * FROM identity_ok)
   AND (SELECT * FROM index_ok);
SQL
)
if [[ "$schema_complete" == "t" ]]; then
    already_applied=true
elif [[ "$detector_column_count" == 0 && "$notification_present" == "f" ]]; then
    already_applied=false
else
    printf 'schema apply blocked: partial detector schema columns=%s table=%s\n' "$detector_column_count" "$notification_present" >&2
    exit 1
fi

if [[ "$already_applied" == false ]]; then
    runuser -u postgres -- psql --dbname "$database" --set ON_ERROR_STOP=1 --file "$sql_file"
fi
# Reassert the runtime role grants when the migration was applied before the grant clauses existed.
runuser -u postgres -- psql --dbname "$database" --set ON_ERROR_STOP=1 \
    --command 'GRANT SELECT, INSERT ON notification_events TO vitals; GRANT USAGE, SELECT ON SEQUENCE notification_events_id_seq TO vitals;'
privileges_ok=$(runuser -u postgres -- psql --dbname "$database" --tuples-only --no-align --command "SELECT has_table_privilege('vitals', 'public.notification_events', 'SELECT') AND has_table_privilege('vitals', 'public.notification_events', 'INSERT') AND has_sequence_privilege('vitals', 'public.notification_events_id_seq', 'USAGE') AND has_sequence_privilege('vitals', 'public.notification_events_id_seq', 'SELECT')")
[[ "$privileges_ok" == "t" ]] || {
    printf 'schema apply failed: vitals runtime role privileges did not verify\n' >&2
    exit 1
}

schema_complete_after=$(runuser -u postgres -- psql --dbname "$database" --tuples-only --no-align <<'SQL'
WITH expected(table_name, column_name, data_type, is_nullable) AS (
    VALUES
        ('status_observations', 'subject', 'text', 'NO'),
        ('status_observations', 'state', 'text', 'NO'),
        ('status_observations', 'reason', 'text', 'YES'),
        ('status_observations', 'reported_at', 'timestamp with time zone', 'NO'),
        ('status_observations', 'changed_at', 'timestamp with time zone', 'NO'),
        ('status_observations', 'next_repeat_at', 'timestamp with time zone', 'YES'),
        ('status_observations', 'transition_id', 'text', 'NO'),
        ('status_observations', 'declared_absent', 'boolean', 'NO'),
        ('status_observations', 'muted_until', 'timestamp with time zone', 'YES'),
        ('notification_events', 'id', 'bigint', 'NO'),
        ('notification_events', 'subject', 'text', 'NO'),
        ('notification_events', 'transition_id', 'text', 'NO'),
        ('notification_events', 'state', 'text', 'NO'),
        ('notification_events', 'reason', 'text', 'NO'),
        ('notification_events', 'emitted_at', 'timestamp with time zone', 'NO'),
        ('notification_events', 'suppressed_by_mute', 'boolean', 'NO')
), columns_ok AS (
    SELECT count(*) = 16 FROM expected e JOIN information_schema.columns c
     ON c.table_schema = 'public' AND c.table_name = e.table_name
       AND c.column_name = e.column_name AND c.data_type = e.data_type
       AND c.is_nullable = e.is_nullable
), defaults_ok AS (
    SELECT COALESCE((SELECT pg_get_expr(d.adbin, d.adrelid) = $$''::text$$ FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum WHERE d.adrelid = to_regclass('public.status_observations') AND a.attname = 'transition_id'), false)
       AND COALESCE((SELECT pg_get_expr(d.adbin, d.adrelid) = 'false' FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum WHERE d.adrelid = to_regclass('public.status_observations') AND a.attname = 'declared_absent'), false)
       AND COALESCE((SELECT pg_get_expr(d.adbin, d.adrelid) = 'false' FROM pg_attrdef d JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum WHERE d.adrelid = to_regclass('public.notification_events') AND a.attname = 'suppressed_by_mute'), false)
), primary_key_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_constraint
     WHERE conrelid = to_regclass('public.notification_events') AND contype = 'p'
       AND pg_get_constraintdef(oid) = 'PRIMARY KEY (id)')
), identity_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_attribute
     WHERE attrelid = to_regclass('public.notification_events') AND attname = 'id'
       AND attidentity = 'a')
), index_ok AS (
    SELECT EXISTS (SELECT 1 FROM pg_index i JOIN pg_class c ON c.oid = i.indexrelid
     WHERE c.oid = to_regclass('public.notification_events_subject_transition_idx')
       AND i.indrelid = to_regclass('public.notification_events')
       AND NOT i.indisunique AND NOT i.indisprimary
       AND i.indnkeyatts = 3 AND i.indnatts = 3
       AND pg_get_indexdef(i.indexrelid) = 'CREATE INDEX notification_events_subject_transition_idx ON public.notification_events USING btree (subject, transition_id, emitted_at DESC)')
)
SELECT (SELECT * FROM columns_ok) AND (SELECT * FROM defaults_ok)
   AND (SELECT * FROM primary_key_ok) AND (SELECT * FROM identity_ok)
   AND (SELECT * FROM index_ok);
SQL
)
[[ "$schema_complete_after" == "t" ]] || {
    printf 'schema apply failed: complete migration shape did not verify\n' >&2
    exit 1
}

install -d -o root -g root -m 0750 "$(dirname -- "$receipt")"
tmp_receipt=$(mktemp "${receipt}.tmp.XXXXXX")
trap 'rm -f "$tmp_receipt"' EXIT
jq -n \
    --arg schema silence_detector \
    --arg database "$database" \
    --arg sql_sha256 "$actual_sql_sha256" \
    --arg applied_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --argjson already_applied "$already_applied" \
    '{schema: $schema, database: $database, sql_sha256: $sql_sha256, already_applied: $already_applied, applied_at: $applied_at, verified: true}' \
    > "$tmp_receipt"
chown root:root "$tmp_receipt"
chmod 600 "$tmp_receipt"
install -o root -g root -m 0600 "$tmp_receipt" "$receipt"
printf 'schema apply: verified %s\n' "$receipt"
