#!/bin/sh
set -eu

export_dir=/var/lib/vitals/cold
registry=/usr/local/share/vitals/registry.toml
month=${1:-${VITALS_COLD_MONTH:-$(date -u -d "$(date -u +%Y-%m-01) -1 month" +%Y-%m)}}
case "$month" in
    ????-??) ;;
    *) echo "cold export blocked: invalid YYYY-MM month: $month" >&2; exit 2 ;;
esac
start="${month}-01T00:00:00Z"
end=$(date -u -d "$start +1 month" +%Y-%m-%dT%H:%M:%SZ)
final="$export_dir/$month"
if [ -d "$final" ]; then
    (cd "$final" && sha256sum -c SHA256SUMS >/dev/null) || {
        echo "cold export blocked: existing immutable bundle failed checksums: $final" >&2
        exit 4
    }
    printf 'immutable cold export already published and verified %s\n' "$final"
    exit 0
fi
test ! -e "$final" || { echo "cold export blocked: publication path is not a directory: $final" >&2; exit 4; }
stage=$(mktemp -d "$export_dir/.${month}.stage.XXXXXX")
cleanup() { find "$stage" -mindepth 1 -maxdepth 1 -type f -delete 2>/dev/null || true; rmdir "$stage" 2>/dev/null || true; }
trap cleanup EXIT HUP INT TERM
umask 077

fixture=${VITALS_COLD_TEST_FIXTURE_SQL:-}
if [ -n "$fixture" ]; then
    test -r "$fixture" || { echo "cold export blocked: fixture unreadable: $fixture" >&2; exit 3; }
    has_samples=t
else
    has_samples=$(psql -X -At -d vitals -c "SELECT to_regclass('public.samples') IS NOT NULL")
fi
if [ "$has_samples" = t ]; then
    {
        test -z "$fixture" || cat "$fixture"
        cat <<SQL
\\copy (SELECT row_to_json(s)::text FROM samples AS s WHERE timestamp >= '$start'::timestamptz AND timestamp < '$end'::timestamptz ORDER BY timestamp, source, type, session_id, seq) TO STDOUT
SQL
    } | psql -X -q -v ON_ERROR_STOP=1 -d vitals >"$stage/data.ndjson"
else
    : >"$stage/data.ndjson"
fi
zstd -q -T1 -9 --rm "$stage/data.ndjson" -o "$stage/data.ndjson.zst"
install -m 0600 "$registry" "$stage/registry.toml"
cat >"$stage/README.md" <<EOF
# Vitals cold export: $month

This directory is one immutable, self-contained export for UTC range [$start, $end). It can be read
without PostgreSQL, pgBackRest, or project code. Verify it first with \`sha256sum -c SHA256SUMS\`,
then stream records with \`zstd -dc data.ndjson.zst | jq -c .\`.

## Data format and schema

\`data.ndjson.zst\` is a Zstandard-compressed UTF-8 NDJSON file: one complete JSON object per line,
zero lines when \`public.samples\` did not exist or the month had no rows. Rows are ordered
reproducibly by \`timestamp, source, type, session_id, seq\`. The UTC interval includes its start
and excludes its end. PostgreSQL \`row_to_json\` supplies these current fields:

- \`source\`, \`type\`: non-null JSON integers; resolve identifiers in the bundled \`registry.toml\`.
- \`session_id\`, \`seq\`: non-null JSON integers forming ingest identity with source and type.
- \`timestamp\`: non-null ISO-8601 timestamp string denoting an absolute instant.
- \`value\`: non-null JSON number in the canonical unit declared by \`registry.toml\`.
- \`extra\`: JSON null or PostgreSQL bytea text string \`\\x<lowercase hex bytes>\`.
- \`node_id\`: non-null PostgreSQL bytea text string \`\\x<lowercase hex bytes>\`.
- \`batch_id\`: JSON null or integer linking signed-ingest provenance.

Readers must preserve unknown keys: future columns are emitted automatically by \`row_to_json\` and
make each row forward-readable. JSON integers can exceed IEEE-754's exact range, so consumers must
use an integer-capable parser rather than coercing identity fields to binary64. JSON null means SQL
NULL and is not zero or an empty byte string.

\`registry.toml\` is the exact source/type/unit registry snapshot copied into this bundle.
\`SHA256SUMS\` provides accidental-corruption checks for all three payload files; it is not a
cryptographic signature. The directory name is the exported UTC month and publication never
replaces an existing month.
EOF
chmod 0600 "$stage/README.md"
(cd "$stage" && sha256sum data.ndjson.zst registry.toml README.md >SHA256SUMS)
chmod 0600 "$stage/SHA256SUMS"
python3 - "$stage" "$export_dir" "$final" <<'PY'
import ctypes
import os
import sys
stage, parent, final = sys.argv[1:]
for name in ("data.ndjson.zst", "registry.toml", "README.md", "SHA256SUMS"):
    fd = os.open(os.path.join(stage, name), os.O_RDONLY)
    try: os.fsync(fd)
    finally: os.close(fd)
dfd = os.open(stage, os.O_RDONLY | os.O_DIRECTORY)
try: os.fsync(dfd)
finally: os.close(dfd)
libc = ctypes.CDLL(None, use_errno=True)
renameat2 = libc.renameat2
renameat2.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
renameat2.restype = ctypes.c_int
if renameat2(-100, os.fsencode(stage), -100, os.fsencode(final), 1) != 0:
    err = ctypes.get_errno()
    raise OSError(err, f"immutable cold export publication failed for {final}: {os.strerror(err)}")
pfd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
try: os.fsync(pfd)
finally: os.close(pfd)
PY
trap - EXIT HUP INT TERM
printf 'published immutable cold export %s\n' "$final"
