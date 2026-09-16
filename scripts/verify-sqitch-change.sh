#!/bin/sh
# Verify the checked-in Sqitch change set without requiring PostgreSQL.
# The task contract names the changes explicitly; fail closed when a change
# is incomplete or is not ordered after its predecessor.
set -eu

repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
plan="$repo/db/sqitch.plan"
[ "$#" -gt 0 ] || {
    printf '%s\n' 'FAIL: at least one Sqitch change name is required' >&2
    exit 1
}

previous=''
for change in "$@"; do
    case "$change" in
        ''|*[!A-Za-z0-9_-]*)
            printf 'FAIL: invalid Sqitch change name: %s\n' "$change" >&2
            exit 1
            ;;
    esac
    for phase in deploy revert verify; do
        file="$repo/db/$phase/$change.sql"
        [ -s "$file" ] || {
            printf 'FAIL: missing or empty %s\n' "$file" >&2
            exit 1
        }
    done

    line=$(grep -E "^${change}([[:space:]]|$)" "$plan" || true)
    [ -n "$line" ] || {
        printf 'FAIL: %s is absent from db/sqitch.plan\n' "$change" >&2
        exit 1
    }
    case "$line" in
        *'['*']'*) ;;
        *)
            printf 'FAIL: %s has no Sqitch dependency\n' "$change" >&2
            exit 1
            ;;
    esac
    if [ -n "$previous" ] && ! printf '%s\n' "$line" | grep -Fq "[$previous]"; then
        printf 'FAIL: %s is not ordered after [%s]\n' "$change" "$previous" >&2
        exit 1
    fi
    grep -Fq 'BEGIN;' "$repo/db/deploy/$change.sql" || {
        printf 'FAIL: deploy/%s.sql is not transactional\n' "$change" >&2
        exit 1
    }
    grep -Fq 'COMMIT;' "$repo/db/deploy/$change.sql" || {
        printf 'FAIL: deploy/%s.sql has no commit\n' "$change" >&2
        exit 1
    }
    previous=$change
done

# These invariants are the reason this change set exists, not merely file
# presence: records are isolated, versions are append-only, and conflicts
# retain incoming evidence plus audit history.
grep -Fq 'CREATE SCHEMA IF NOT EXISTS domain' "$repo/db/deploy/domain_schema.sql"
grep -Fq 'CREATE TABLE domain.record_links' "$repo/db/deploy/domain_versions.sql"
grep -Fq 'CREATE TABLE domain.record_conflicts' "$repo/db/deploy/domain_conflicts.sql"
grep -Fq 'CREATE TABLE domain.record_audit' "$repo/db/deploy/domain_conflicts.sql"
grep -Fq 'incoming_content_sha256' "$repo/db/deploy/domain_conflicts.sql"

for fixture in domain-record.json song-record.json record-link.json conflict.json; do
    [ -s "$repo/api/fixtures/domain-v1/$fixture" ] || {
        printf 'FAIL: missing domain fixture: %s\n' "$fixture" >&2
        exit 1
    }
done
grep -Fq 'lyrics_source' "$repo/api/fixtures/domain-v1/song-record.json"
grep -Fq '"status": "open"' "$repo/api/fixtures/domain-v1/conflict.json"

printf 'sqitch-change: PASS: %s\n' "$*"
