#!/usr/bin/env bash
# Destructive only to a uniquely named scratch database and scratch directory.
set -euo pipefail

rows=10000
tag="t_d5_${USER}_$$_$(date +%s)"
db="${tag//[^a-zA-Z0-9_]/_}"
spool="$(mktemp -d "/tmp/${tag}.XXXXXX")"
drainer="/tmp/${tag}.vitals-spool-drain"
fixture="$(pwd)/target/debug/vitals-spool-fixture"
pid=""

cleanup() {
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    sudo kill -KILL "$pid" 2>/dev/null || true
  fi
  sudo -u postgres dropdb --if-exists "$db" >/dev/null 2>&1 || true
  sudo rm -f -- "$drainer"
  sudo rm -rf -- "$spool"
}
trap cleanup EXIT INT TERM

throttle="$(vcgencmd get_throttled)"
active=$(( ${throttle#*=} & 0xF ))
if (( active != 0 )); then
  echo "drain-crash-test: active Pi throttle bits in $throttle" >&2
  exit 1
fi

cargo build -p vitals-drain --bins
sudo install -o postgres -g postgres -m 0755 target/debug/vitals-spool-drain "$drainer"
sudo -u postgres createdb "$db"
sudo -u postgres psql -v ON_ERROR_STOP=1 -d "$db" <<'SQL'
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE TABLE vitals_source (id INTEGER PRIMARY KEY);
CREATE TABLE vitals_type (id INTEGER PRIMARY KEY);
INSERT INTO vitals_source VALUES (901005);
INSERT INTO vitals_type VALUES (902005);
CREATE TABLE samples (
 source INTEGER NOT NULL REFERENCES vitals_source(id),
 type INTEGER NOT NULL REFERENCES vitals_type(id), session_id BIGINT NOT NULL,
 seq BIGINT NOT NULL, timestamp TIMESTAMPTZ NOT NULL,
 value DOUBLE PRECISION NOT NULL, extra BYTEA, node_id BYTEA NOT NULL,
 batch_id BIGINT, PRIMARY KEY(source,type,session_id,seq)
);
-- Production's current identity key omits the time partition column, so
-- TimescaleDB correctly refuses create_hypertable here. This test reproduces
-- production FKs/indexes and PostgreSQL conflict semantics; compressed-chunk
-- conflict behavior remains a deployment migration concern until that schema
-- incompatibility is resolved.
CREATE INDEX samples_time_idx ON samples (source,type,timestamp DESC);
CREATE TABLE spool_status (
 segment TEXT PRIMARY KEY, offset_bytes BIGINT NOT NULL DEFAULT 0,
 drained BOOLEAN NOT NULL DEFAULT FALSE, fingerprint BYTEA NOT NULL,
 last_advanced TIMESTAMPTZ NOT NULL DEFAULT now()
);
SQL

segment="$($fixture "$spool" "$rows")"
sudo chown -R postgres:postgres "$spool"
sudo chmod 0770 "$spool"
sudo chmod 0660 "$segment"
eof="$(sudo stat -c %s "$segment")"

# Pause after COPY has consumed all rows but before INSERT/status commit, then
# kill the process. PostgreSQL must roll the transaction back and replay all
# rows from the unchanged segment on restart.
sudo -u postgres env VITALS_DRAIN_TEST_PAUSE_BEFORE_COMMIT=60 \
  "$drainer" --once --database "host=/var/run/postgresql dbname=$db user=postgres" --spool "$spool" &
pid=$!
for _ in $(seq 1 100); do
  state="$(sudo -u postgres psql -Atqc "SELECT state FROM pg_stat_activity WHERE datname='$db' LIMIT 1" postgres 2>/dev/null || true)"
  [[ "$state" == "idle in transaction" ]] && break
  sleep 0.1
done
[[ "$state" == "idle in transaction" ]] || {
  echo "drain-crash-test: drainer never reached the uncommitted crash window (state=${state:-absent})" >&2
  exit 1
}
child="$(pgrep -P "$pid" | head -1 || true)"
[[ -n "$child" ]] || {
  echo "drain-crash-test: could not identify drainer child of sudo pid $pid" >&2
  exit 1
}
sudo kill -KILL "$child"
wait "$pid" 2>/dev/null || true
pid=""

before="$(sudo -u postgres psql -Atqc 'SELECT count(*) FROM samples' "$db")"
[[ "$before" == "0" ]] || {
  echo "drain-crash-test: killed transaction leaked $before rows" >&2
  exit 1
}

sudo -u postgres "$drainer" --once --database "host=/var/run/postgresql dbname=$db user=postgres" --spool "$spool"
read -r count distinct_count offset drained status_count <<<"$(sudo -u postgres psql -AtF' ' -d "$db" -c "SELECT count(*),count(DISTINCT (source,type,session_id,seq)),coalesce(max(offset_bytes),-1),bool_and(drained),count(DISTINCT segment) FROM samples CROSS JOIN spool_status")"
[[ "$count" == "$rows" && "$distinct_count" == "$rows" ]] || {
  echo "drain-crash-test: expected $rows unique rows, got count=$count distinct=$distinct_count" >&2
  exit 1
}
[[ "$offset" == "$eof" && "$drained" == "t" && "$status_count" == "1" ]] || {
  echo "drain-crash-test: expected one drained status at EOF $eof, got offset=$offset drained=$drained statuses=$status_count" >&2
  exit 1
}
[[ ! -e "$segment" ]] || {
  echo "drain-crash-test: fully drained segment still exists: $segment" >&2
  exit 1
}

throttle_after="$(vcgencmd get_throttled)"
active_after=$(( ${throttle_after#*=} & 0xF ))
if (( active_after != 0 )); then
  echo "drain-crash-test: active Pi throttle bits after test in $throttle_after" >&2
  exit 1
fi
printf 'drain-crash-test: PASS rows=%s unique=%s offset=%s eof=%s throttle=%s\n' \
  "$count" "$distinct_count" "$offset" "$eof" "$throttle_after"
