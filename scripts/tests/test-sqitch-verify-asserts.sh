#!/usr/bin/env bash
# Every sqitch verify script must be able to FAIL.
#
# `[deploy] verify = true` runs each verify script as its change deploys, and a
# script that cannot fail turns that into false assurance. One of the 26 was in
# that state: db/verify/domain_schema.sql queried information_schema with a
# filter, so an absent schema yielded zero rows -- not an error -- and psql
# exited 0. A change was recorded as deployed while its schema did not exist,
# and the next change failed with "schema does not exist".
#
# The rule, derived by running all 26 against an empty database: a script is
# safe if it either
#   (a) names a non-catalog relation, which errors at parse time when the
#       relation or any named column is missing -- this is why the common
#       `SELECT col, col FROM tbl WHERE FALSE` idiom is fine, and why a
#       grep for assertion keywords alone produces false positives; or
#   (b) contains an assertion that raises, such as `1 / count(*)`.
# A script that only reads catalogs with a predicate and never asserts is
# fail-open.
set -Eeuo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
dir="$root/db/verify"
failures=0

[[ -d $dir ]] || { printf 'verify-asserts: no %s\n' "$dir" >&2; exit 1; }

shopt -s nullglob
scripts=("$dir"/*.sql)
(( ${#scripts[@]} > 0 )) || { printf 'verify-asserts: no verify scripts found\n' >&2; exit 1; }

for f in "${scripts[@]}"; do
    name=$(basename "$f")
    body=$(grep -v '^[[:space:]]*--' "$f")

    # (b) an assertion that raises
    if printf '%s' "$body" | grep -qiE '1[[:space:]]*/[[:space:]]*(count|CASE)|ASSERT|RAISE[[:space:]]+EXCEPTION'; then
        continue
    fi
    # (a) a FROM naming something that is not a catalog view
    if printf '%s' "$body" \
        | grep -ioE 'FROM[[:space:]]+[a-z_][a-z0-9_.]*' \
        | awk '{print $2}' \
        | grep -qvE '^(information_schema\.|pg_)'; then
        continue
    fi

    printf 'verify-asserts FAIL: %s reads only catalogs and never asserts, so it cannot fail\n' "$name" >&2
    failures=$((failures + 1))
done

if (( failures )); then
    printf 'verify-asserts: %d fail-open script(s)\n' "$failures" >&2
    exit 1
fi
printf 'verify-asserts: all %d verify scripts can fail\n' "${#scripts[@]}"
