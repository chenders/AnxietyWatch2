#!/bin/bash
set -euo pipefail

result=/var/lib/vitals/restore-verify.json
history=/var/lib/vitals/restore-verify-history
g3_evidence=/var/lib/vitals/restore-verify-g3.json
work_parent=/var/lib/postgresql/verify
run_parent=/run/vitals-restore
image=$work_parent/restore.ext4
mount_dir=$work_parent/mnt
scratch=$mount_dir/pgdata
tablespace_dir=$mount_dir/tablespaces
run_mount=$run_parent/mnt
run_dir=$run_mount/socket
lock_file=/run/vitals-restore/verify-restore.lock
port=5433
started=0
start_epoch=$(date +%s)
backup_label=unknown
evidence_lsn=unknown
tables_checked=0
mismatches='[]'
checks='[]'
error='verification did not complete'
image_dev= image_ino= mount_dev= mount_ino= run_mount_dev= run_mount_ino=
data_mounted=0
run_mounted=0

if [[ ${1:-} == --staleness-check ]]; then
    [[ -r "$result" ]] || { echo "restore verification fault: result missing: $result" >&2; exit 2; }
    now=$(date +%s)
    ts=$(jq -er '.ts | select(type == "number" and floor == .)' "$result") || { echo 'restore verification fault: result timestamp invalid' >&2; exit 3; }
    ok=$(jq -er '.ok | booleans' "$result") || { echo 'restore verification fault: result status invalid' >&2; exit 4; }
    tables=$(jq -er '.tables_checked | select(type == "number" and floor == . and . >= 0)' "$result") || { echo 'restore verification fault: table count invalid' >&2; exit 5; }
    age=$((now-ts))
    [[ $ok == true ]] || { echo 'restore verification fault: latest result is not ok' >&2; exit 6; }
    if ((tables == 0)); then
        jq -e '.ok == true and .tables_checked > 0 and (.checks | index("recovery_lsn") != null) and (.checks | index("pg_amcheck") != null)' "$g3_evidence" >/dev/null 2>&1 || { echo 'restore verification fault: durable non-vacuous G3 evidence invalid' >&2; exit 7; }
    fi
    ((age >= -300)) || { echo 'restore verification fault: result timestamp is in the future' >&2; exit 8; }
    ((age <= 45*86400)) || { echo "restore verification fault: result age ${age}s exceeds 45 days" >&2; exit 9; }
    echo "restore verification current: age_seconds=$age"
    exit 0
fi
[[ $# -eq 0 ]] || { echo "restore verification blocked: unknown argument: $1" >&2; exit 64; }

install -d -o root -g postgres -m 0710 "$work_parent" "$run_parent"
unset PGHOST PGHOSTADDR PGPORT PGDATABASE PGUSER PGSERVICE PGSERVICEFILE PGPASSFILE PGOPTIONS
exec 9>"$lock_file"
flock -n 9 || { echo 'restore verification blocked: another run holds the lock' >&2; exit 75; }

# Recover only root-owned remnants from a prior forced kill. Their root-owned parents make the
# top-level names non-replaceable by postgres; loop mounts lived only in PrivateMounts.
for stale in "$mount_dir" "$run_mount"; do
    if [[ -e $stale ]]; then
        [[ -d $stale && ! -L $stale && $(stat -c %U "$stale") == root ]] || { echo "unsafe stale path: $stale" >&2; exit 76; }
        rmdir "$stale" || { echo "nonempty stale mountpoint: $stale" >&2; exit 77; }
    fi
done
if [[ -e $image ]]; then
    [[ -f $image && ! -L $image && $(stat -c %U "$image") == root ]] || { echo "unsafe stale image: $image" >&2; exit 78; }
    rm -f "$image"
fi

publish_result() {
    local ok=$1 end_epoch tmp
    end_epoch=$(date +%s)
    mkdir -p "$history"
    tmp=$(mktemp /var/lib/vitals/.restore-verify.XXXXXX)
    jq -n --argjson ts "$end_epoch" --argjson ok "$ok" \
      --argjson tables_checked "$tables_checked" --argjson mismatches "$mismatches" \
      --argjson restore_seconds "$((end_epoch-start_epoch))" --arg backup_label "$backup_label" \
      --arg evidence_lsn "$evidence_lsn" --argjson checks "$checks" --arg error "$error" \
      '{ts:$ts,ok:$ok,tables_checked:$tables_checked,mismatches:$mismatches,restore_seconds:$restore_seconds,backup_label:$backup_label,evidence_lsn:$evidence_lsn,checks:$checks} + (if $ok then {} else {error:$error} end)' >"$tmp"
    chmod 0644 "$tmp"
    python3 - "$tmp" <<'PY'
import os, sys
fd=os.open(sys.argv[1],os.O_RDONLY)
try: os.fsync(fd)
finally: os.close(fd)
PY
    mv -f "$tmp" "$result"
    [[ $ok == true ]] && rm -f /var/lib/vitals/restore-verify.alert
    if [[ $ok == true && $tables_checked -gt 0 ]]; then
        g3_tmp=$(mktemp /var/lib/vitals/.restore-verify-g3.XXXXXX)
        cp --preserve=mode "$result" "$g3_tmp"
        python3 - "$g3_tmp" <<'PY'
import os, sys
fd=os.open(sys.argv[1],os.O_RDONLY)
try: os.fsync(fd)
finally: os.close(fd)
PY
        mv -f "$g3_tmp" "$g3_evidence"
    fi
    cp --preserve=mode "$result" "$history/$(date -u -d "@$end_epoch" +%Y%m%dT%H%M%SZ).json"
    find "$history" -maxdepth 1 -type f -name '*.json' -printf '%T@ %p\n' | sort -nr | awk 'NR>13{sub(/^[^ ]+ /,"");print}' | xargs -r rm -f --
    python3 - /var/lib/vitals "$history" <<'PY'
import os, sys
for path in sys.argv[1:]:
    fd=os.open(path,os.O_RDONLY|os.O_DIRECTORY)
    try: os.fsync(fd)
    finally: os.close(fd)
PY
}

# Remove only a root-parented object whose device/inode still matches. Scratch descendants are
# discarded by unmounting their private filesystems, never by recursive pathname deletion.
remove_owned_leaf() {
    local parent=$1 name=$2 dev=$3 ino=$4 kind=$5
    [[ -n $dev && -n $ino ]] || return 0
    python3 - "$parent" "$name" "$dev" "$ino" "$kind" <<'PY'
import os, stat, sys
parent,name,dev,ino,kind=sys.argv[1],sys.argv[2],int(sys.argv[3]),int(sys.argv[4]),sys.argv[5]
pfd=os.open(parent,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
try:
    st=os.stat(name,dir_fd=pfd,follow_symlinks=False)
    if (st.st_dev,st.st_ino)!=(dev,ino): raise RuntimeError('owned object identity changed')
    if kind=='dir': os.rmdir(name,dir_fd=pfd)
    elif kind=='file': os.unlink(name,dir_fd=pfd)
    else: raise RuntimeError('invalid object kind')
finally: os.close(pfd)
PY
}

cleanup() {
    local rc=${1:-$?} pid
    trap - EXIT HUP INT
    trap 'error="restore verification cleanup timed out"; publish_result false || true; exit 124' TERM
    if ((started)) || [[ -f $scratch/postmaster.pid ]]; then
        if runuser -u postgres -- /usr/lib/postgresql/16/bin/pg_ctl -D "$scratch" status >/dev/null 2>&1; then
            if ! runuser -u postgres -- /usr/lib/postgresql/16/bin/pg_ctl -D "$scratch" -m immediate -w stop >/dev/null 2>&1; then
                error='cleanup blocked: isolated PostgreSQL did not stop'; publish_result false || true; exit 89
            fi
        elif [[ -f $scratch/postmaster.pid ]]; then
            pid=$(head -1 "$scratch/postmaster.pid" 2>/dev/null || true)
            if [[ $pid =~ ^[0-9]+$ ]] && kill -0 "$pid" 2>/dev/null; then
                error='cleanup blocked: scratch postmaster status is indeterminate'; publish_result false || true; exit 89
            fi
        fi
        started=0
    fi
    if ((data_mounted)); then
        mountpoint -q "$mount_dir" || { error='scratch mount identity lost'; publish_result false || true; exit 90; }
        umount "$mount_dir" || { error='scratch filesystem unmount failed'; publish_result false || true; exit 91; }
        data_mounted=0
    fi
    if ((run_mounted)); then
        mountpoint -q "$run_mount" || { error='run mount identity lost'; publish_result false || true; exit 92; }
        umount "$run_mount" || { error='run filesystem unmount failed'; publish_result false || true; exit 93; }
        run_mounted=0
    fi
    remove_owned_leaf "$work_parent" mnt "$mount_dev" "$mount_ino" dir || { error='scratch mountpoint identity cleanup failed'; publish_result false || true; exit 94; }
    remove_owned_leaf "$run_parent" mnt "$run_mount_dev" "$run_mount_ino" dir || { error='run mountpoint identity cleanup failed'; publish_result false || true; exit 95; }
    remove_owned_leaf "$work_parent" restore.ext4 "$image_dev" "$image_ino" file || { error='scratch image identity cleanup failed'; publish_result false || true; exit 96; }
    ((rc == 0)) || publish_result false || true
    exit "$rc"
}
trap cleanup EXIT
interrupted(){ error="restore verification interrupted by signal $1"; cleanup 128; }
trap 'interrupted HUP' HUP
trap 'interrupted INT' INT
trap 'interrupted TERM' TERM
fail(){ error=$1; echo "restore verification failed: $error" >&2; exit 1; }

state=$(vcgencmd get_throttled) || fail 'vcgencmd unavailable'
value=${state#*=}; (((value&0xf)==0)) || fail "active throttle bits before restore: $state"
[[ $(stat -c '%U:%G:%a' "$work_parent") == root:postgres:710 ]] || fail "unsafe work parent metadata: $work_parent"
[[ $(stat -c '%U:%G:%a' "$run_parent") == root:postgres:710 ]] || fail "unsafe run parent metadata: $run_parent"
[[ ! -e $image && ! -e $mount_dir && ! -e $run_mount ]] || fail 'scratch mount path already exists'
umask 077

# A fresh full verification backup makes the tested set explicit. The evidence boundary is taken
# afterward while every public relation is ACCESS EXCLUSIVE locked, then WAL is archived through it.
runuser -u postgres -- pgbackrest --config=/etc/pgbackrest/pgbackrest.conf --stanza=vitals --type=full backup
backup_info=$(runuser -u postgres -- pgbackrest --config=/etc/pgbackrest/pgbackrest.conf --stanza=vitals info --output=json | jq -ec '.[0].backup[-1] | select(.type=="full")') || fail 'fresh full backup metadata unavailable'
backup_label=$(jq -er .label <<<"$backup_info")
restore_bytes=$(jq -er '.info.size|select(type=="number" and .>0)' <<<"$backup_info")
available_bytes=$(df -PB1 "$work_parent"|awk 'NR==2{print $4}')
image_bytes=$((restore_bytes*2+1024*1024*1024))
required_bytes=$((image_bytes+5*1024*1024*1024))
((available_bytes>=required_bytes)) || fail "insufficient scratch capacity: available=$available_bytes required=$required_bytes"
truncate -s "$image_bytes" "$image"; read -r image_dev image_ino < <(stat -c '%d %i' "$image")
mkfs.ext4 -q -F "$image"
mkdir "$mount_dir"; read -r mount_dev mount_ino < <(stat -c '%d %i' "$mount_dir")
mount -o loop,nodev,nosuid,noexec "$image" "$mount_dir"; data_mounted=1
mkdir "$scratch" "$tablespace_dir"; chown postgres:postgres "$scratch" "$tablespace_dir"; chmod 0710 "$scratch" "$tablespace_dir"
mkdir "$run_mount"; read -r run_mount_dev run_mount_ino < <(stat -c '%d %i' "$run_mount")
mount -t tmpfs -o nodev,nosuid,noexec,mode=0710,size=64M tmpfs "$run_mount"; run_mounted=1
chown root:postgres "$run_mount"
mkdir "$run_dir"; chown postgres:postgres "$run_dir"

psql_prod=(runuser -u postgres -- psql -X -qAt -v ON_ERROR_STOP=1 -h /var/run/postgresql -p 5432 -U postgres -d vitals)
boundary=$("${psql_prod[@]}" <<'SQL'
BEGIN ISOLATION LEVEL READ COMMITTED;
LOCK TABLE pg_catalog.pg_class, pg_catalog.pg_namespace IN SHARE MODE;
SELECT format('LOCK TABLE %I.%I IN ACCESS EXCLUSIVE MODE;',n.nspname,c.relname)
FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
WHERE n.nspname='public' AND c.relkind IN ('r','p') ORDER BY c.oid \gexec
-- The statement snapshot below is acquired only after every relation lock has been granted.
WITH evidence AS MATERIALIZED (
 SELECT
  COALESCE((SELECT jsonb_object_agg(table_name,row_count ORDER BY table_name) FROM (
    SELECT c.relname table_name,(xpath('/row/c/text()',query_to_xml(format('SELECT count(*) AS c FROM %I.%I',n.nspname,c.relname),false,true,'')))[1]::text::bigint row_count
    FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind IN ('r','p')) q),'{}'::jsonb) counts,
  COALESCE((SELECT jsonb_object_agg(sequence_name,jsonb_build_object('last_value',last_value,'is_called',is_called) ORDER BY sequence_name) FROM (
    SELECT c.relname sequence_name,
      (xpath('/row/last_value/text()',x))[1]::text::bigint last_value,
      (xpath('/row/is_called/text()',x))[1]::text::boolean is_called
    FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
    CROSS JOIN LATERAL query_to_xml(format('SELECT last_value, is_called FROM %I.%I',n.nspname,c.relname),false,true,'') x
    WHERE n.nspname='public' AND c.relkind='S') q),'{}'::jsonb) sequences,
  (SELECT jsonb_object_agg(extname,extversion ORDER BY extname) FROM pg_extension) extensions
)
SELECT jsonb_build_object(
 'lsn',pg_logical_emit_message(false,'t_c3_restore_boundary',clock_timestamp()::text)::text,
 'counts',counts,
 'sequences',sequences,
 'extensions',extensions)
FROM evidence;
COMMIT;
SQL
) || fail 'transactional evidence boundary failed'
boundary=$(grep '^{' <<<"$boundary" | tail -1)
evidence_lsn=$(jq -er .lsn <<<"$boundary")
evidence_counts=$(jq -ec .counts <<<"$boundary")
evidence_sequences=$(jq -ec .sequences <<<"$boundary")
evidence_ext=$(jq -ec .extensions <<<"$boundary")
tables_checked=$(jq 'length' <<<"$evidence_counts")
if ((tables_checked == 0)); then
    jq -e '.ok == true and .tables_checked > 0 and (.checks | index("recovery_lsn") != null) and (.checks | index("pg_amcheck") != null)' "$g3_evidence" >/dev/null 2>&1 || fail 'empty-schema override requires durable non-vacuous G3 evidence'
fi
# pg_walfile_name() maps an exact segment-boundary LSN to the new segment, although the boundary
# record itself is in the preceding segment. Ask PostgreSQL for the segment containing LSN-1.
target_segment=$("${psql_prod[@]}" -c "SELECT pg_walfile_name(('$evidence_lsn'::pg_lsn - 1))")
"${psql_prod[@]}" -c 'SELECT pg_switch_wal()' >/dev/null
for _ in $(seq 1 60); do
    archive_max=$(runuser -u postgres -- pgbackrest --config=/etc/pgbackrest/pgbackrest.conf --stanza=vitals info --output=json | jq -er '.[0].archive[-1].max | select(type=="string" and test("^[0-9A-F]{24}$"))') || archive_max=
    if [[ $target_segment =~ ^[0-9A-F]{24}$ && $archive_max =~ ^[0-9A-F]{24}$ ]] && { [[ $archive_max == "$target_segment" ]] || [[ $archive_max > "$target_segment" ]]; }; then break; fi
    sleep 1
done
[[ $target_segment =~ ^[0-9A-F]{24}$ && $archive_max =~ ^[0-9A-F]{24}$ ]] && { [[ $archive_max == "$target_segment" ]] || [[ $archive_max > "$target_segment" ]]; } || fail "WAL boundary was not archived: target=$target_segment max=$archive_max"
wal_probe="$run_dir/$target_segment"
runuser -u postgres -- pgbackrest --config=/etc/pgbackrest/pgbackrest.conf --stanza=vitals --pg1-path=/var/lib/postgresql/16/main archive-get "$target_segment" "$wal_probe" || fail "exact target WAL segment unavailable: $target_segment"
[[ -s $wal_probe ]] || fail "exact target WAL segment empty: $target_segment"
rm -f "$wal_probe"

runuser -u postgres -- pgbackrest --config=/etc/pgbackrest/pgbackrest.conf --stanza=vitals --pg1-path="$scratch" --set="$backup_label" --delta --type=lsn --target="$evidence_lsn" --target-action=promote --archive-mode=off --tablespace-map-all="$tablespace_dir" restore || fail "pgBackRest restore failed for $backup_label at $evidence_lsn"
link_manifest="$run_dir/restored-links"
find "$scratch" -type l -print0 >"$link_manifest" || fail 'restored PGDATA symlink scan failed'
while IFS= read -r -d '' link; do
    relative=${link#"$scratch/pg_tblspc/"}
    [[ $link == "$scratch/pg_tblspc/"* && $relative =~ ^[0-9]+$ ]] || fail "restored unsafe symlink: $link"
    resolved=$(readlink -f -- "$link") || fail "unresolvable tablespace symlink: $link"
    [[ $resolved == "$tablespace_dir"/* ]] || fail "tablespace escapes scratch root: $link -> $resolved"
done <"$link_manifest"
find "$tablespace_dir" -type l -print0 >"$link_manifest" || fail 'restored tablespace symlink scan failed'
[[ ! -s $link_manifest ]] || fail 'restored tablespace content contains symlinks'
rm -f "$link_manifest"
rm -f "$scratch/postgresql.conf" "$scratch/pg_hba.conf" "$scratch/pg_ident.conf"
cat >"$scratch/postgresql.conf" <<EOF
port=$port
listen_addresses=''
unix_socket_directories='$run_dir'
hba_file='$scratch/pg_hba.conf'
ident_file='$scratch/pg_ident.conf'
ssl=off
archive_mode=off
shared_preload_libraries='timescaledb,pg_stat_statements'
timezone='UTC'
max_connections=40
EOF
printf 'local all all peer\n' >"$scratch/pg_hba.conf"; : >"$scratch/pg_ident.conf"
cat >"$scratch/postgresql.auto.conf" <<EOF
restore_command='pgbackrest --config=/etc/pgbackrest/pgbackrest.conf --stanza=vitals --pg1-path=$scratch archive-get %f "%p"'
recovery_target_lsn='$evidence_lsn'
recovery_target_action='promote'
EOF
chown postgres:postgres "$scratch/postgresql.conf" "$scratch/pg_hba.conf" "$scratch/pg_ident.conf" "$scratch/postgresql.auto.conf"
chmod 0600 "$scratch/postgresql.conf" "$scratch/pg_hba.conf" "$scratch/pg_ident.conf" "$scratch/postgresql.auto.conf"

started=1
opts="-c port=$port -c listen_addresses= -c unix_socket_directories=$run_dir -c hba_file=$scratch/pg_hba.conf -c ident_file=$scratch/pg_ident.conf -c ssl=off -c archive_mode=off"
runuser -u postgres -- /usr/lib/postgresql/16/bin/pg_ctl -D "$scratch" -l "$run_dir/verify.log" -o "$opts" -w start || { tail -40 "$run_dir/verify.log" >&2||true; fail 'isolated PostgreSQL failed readiness startup'; }
psql_rest=(runuser -u postgres -- psql -X -qAt -v ON_ERROR_STOP=1 -h "$run_dir" -p "$port" -U postgres -d vitals)
recovery_state=t
for _ in $(seq 1 60); do
    recovery_state=$("${psql_rest[@]}" -d postgres -c 'SELECT pg_is_in_recovery()') || fail 'recovery-state query failed'
    [[ $recovery_state == f ]] && break
    sleep 1
done
[[ $recovery_state == f ]] || fail 'restore did not promote at evidence boundary within 60 seconds'
recovery_lsn=$("${psql_rest[@]}" -d postgres -c 'SELECT pg_current_wal_lsn()') || fail 'promoted recovery LSN unavailable'
recovery_reached=$("${psql_rest[@]}" -d postgres -c "SELECT '$recovery_lsn'::pg_lsn >= '$evidence_lsn'::pg_lsn") || fail 'promoted recovery LSN comparison failed'
[[ $recovery_reached == t ]] || fail "promoted cluster did not reach evidence boundary: current=$recovery_lsn evidence=$evidence_lsn"
query="SELECT COALESCE(jsonb_object_agg(table_name,row_count ORDER BY table_name),'{}'::jsonb) FROM (SELECT c.relname table_name,(xpath('/row/c/text()',query_to_xml(format('SELECT count(*) AS c FROM %I.%I',n.nspname,c.relname),false,true,'')))[1]::text::bigint row_count FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind IN ('r','p'))q"
rest_counts=$("${psql_rest[@]}" -c "$query") || fail 'restored table count failed'
sequence_query="SELECT COALESCE(jsonb_object_agg(sequence_name,jsonb_build_object('last_value',last_value,'is_called',is_called) ORDER BY sequence_name),'{}'::jsonb) FROM (SELECT c.relname sequence_name,(xpath('/row/last_value/text()',x))[1]::text::bigint last_value,(xpath('/row/is_called/text()',x))[1]::text::boolean is_called FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace CROSS JOIN LATERAL query_to_xml(format('SELECT last_value, is_called FROM %I.%I',n.nspname,c.relname),false,true,'') x WHERE n.nspname='public' AND c.relkind='S')q"
rest_sequences=$("${psql_rest[@]}" -c "$sequence_query") || fail 'restored sequence-state query failed'
# Crash recovery may replay a WAL-preallocated sequence value beyond the SQL-visible evidence value.
# It must never be behind; at an equal value, is_called must match, and an advanced value must be called.
sequence_mismatches=$(jq -cn --argjson p "$evidence_sequences" --argjson r "$rest_sequences" '[((($p|keys_unsorted)+($r|keys_unsorted))|unique[]) as $s | select(($p[$s]==null) or ($r[$s]==null) or ($r[$s].last_value < $p[$s].last_value) or (($r[$s].last_value == $p[$s].last_value) and ($r[$s].is_called != $p[$s].is_called)) or (($r[$s].last_value > $p[$s].last_value) and ($r[$s].is_called != true))) | {sequence:$s,evidence:($p[$s]//null),restored:($r[$s]//null)}]')
[[ $(jq length <<<"$sequence_mismatches") -eq 0 ]] || fail "restored sequence state is missing or behind evidence: $sequence_mismatches"
mismatches=$(jq -cn --argjson p "$evidence_counts" --argjson r "$rest_counts" '[((($p|keys_unsorted)+($r|keys_unsorted))|unique[]) as $t|select($p[$t]!=$r[$t])|{table:$t,evidence:($p[$t]//null),restored:($r[$t]//null)}]')
[[ $(jq length <<<"$mismatches") -eq 0 ]] || fail "boundary count mismatches: $mismatches"
rest_ext=$("${psql_rest[@]}" -c "SELECT jsonb_object_agg(extname,extversion ORDER BY extname) FROM pg_extension")
jq -en --argjson expected "$evidence_ext" --argjson restored "$rest_ext" '$expected == $restored' >/dev/null || fail "restored extension inventory differs from boundary: evidence=$evidence_ext restored=$rest_ext"
runuser -u postgres -- /usr/lib/postgresql/16/bin/pg_amcheck --all --install-missing -h "$run_dir" -p "$port" || fail 'pg_amcheck found structural corruption'
checks='["fresh_full_backup","locked_lsn_boundary","archived_boundary","pgbackrest_restore","postgres_readiness","public_table_counts","public_sequence_floor","extension_inventory","recovery_lsn","pg_amcheck"]'
runuser -u postgres -- /usr/lib/postgresql/16/bin/pg_ctl -D "$scratch" -m fast -w stop || fail 'isolated PostgreSQL failed clean shutdown'
started=0
state=$(vcgencmd get_throttled); value=${state#*=}; (((value&0xf)==0)) || fail "active throttle bits after restore: $state"
error=''; publish_result true
