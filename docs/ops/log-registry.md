# Operational log registry

**Updated:** 2026-09-05 18:16 PDT
**Audit scope:** T-S5 local verification and operational evidence for PostgreSQL, the spool/drainer,
BlueZ/kernel/systemd, the panel/admin readers, CI, deployment, and retention.

This is the inventory of operational evidence, not a claim that a running process is healthy.
Every row must identify the producer, the event, the durable location, the retention/reader, and
what remains unknown. A missing or stale record is `UNKNOWN`, never `OK`.

## Stage vocabulary

The stages are intentionally separate:

| Stage | Meaning | Minimum fields |
|---|---|---|
| `received` | `vitalsd` accepted a request from a collector | source, type, session, seq range, count, request id, received time |
| `spooled` | bytes were appended to a named segment | segment, offset/byte range, count, byte count, result/error |
| `fsynced` | the relevant spool bytes reached stable storage | segment, durable offset, count, sync reason (`explicit`/`periodic`), result/error |
| `drained` | the drainer read a segment suffix and loaded its private staging table | segment, start/end offset, rows read, framing/fingerprint, result/error |
| `committed` | the PostgreSQL transaction committed and `spool_status` advanced | segment, committed offset, inserted, duplicate, rejected, commit time |
| `published` | `vitalsd` enqueued a sample for a subscriber | subscriber id, source/type/session/seq, queue depth, result |
| `displayed` | a UI renderer consumed and rendered a value | feed generation, durable identity, display time, pending/durable state |

`published` is not `displayed`; a socket write or queue admission cannot prove that a panel or
browser rendered the value.

## Registry

| Producer / owner | Evidence required | Current evidence at this audit | Gap / required action | Location / reader |
|---|---|---|---|---|
| `vitals-core::Spool` / vitalsd | append result, segment name, byte growth, explicit and periodic fsync result, rotation result | `append` returns `Result`; `sync=true` is represented in the reply; the deployed `fe529c0` artifact emits stage/error events and exposes whether unsynced bytes existed | Post-install health reset to zero observed samples and named `source 0 has never produced a sample`; fresh physiological ingest remains unproven | journald via vitalsd; `vitalsd` health; drainer snapshot |
| `vitalsd` ingest boundary | received → spooled → fsynced ledger; malformed-frame and client disconnect reason; bounded subscriber drops | Deployed `fe529c0` emits `event=vitalsd.*` records, reads `.drain-health.json`, and reported a named unavailable-then-healthy drain transition after restart | Current health reports zero new samples and `published=0`; subscriber `published` is still not UI `displayed` | journald; Unix-socket `Health` response |
| `vitals-proto` | machine-readable error class and enough context for caller logging | `ErrorKind` and error detail exist; malformed CBOR/frame errors are returned | Pure wire crate must not log; every caller must log the returned class, peer id, and frame/request context | caller journal; wire `Health`/`Error` response |
| `vitals-spool-drain` | startup/lock/schema validation, pending bytes/age, per-segment drained/committed result, inserted/duplicate/rejected counters, last successful commit, last named error | Deployed `fe529c0` publishes an atomically replaced `.drain-health.json`, reports the locked active segment, and emits `event=drainer.*`; the snapshot currently says `available=true`, `pending_segments=0`, `pending_bytes=0`, all row counters zero, and `last_success_unix_secs=null` | No fresh physiological append has reached this post-install daemon; the null last-success field and zero ingest health remain evidence to reconcile, not success | `/var/lib/vitals/spool/.drain-health.json`; journald; vitalsd health reader |
| `spool_status` / PostgreSQL owner | transactional segment offset and fingerprint; commit evidence must agree with local offset | `spool_status` has `segment`, `offset_bytes`, `drained`, `fingerprint`, `last_advanced`; unique primary key is present | No explicit database-side stage/audit event for a failed transaction; drainer must expose commit counters and named DB errors without advancing status | PostgreSQL `spool_status`; drainer snapshot |
| active segment | pending rows must be visible even while the writer holds its lock | the drainer skips a locked active segment during normal processing | Pending calculation must include the active segment without opening it; an old/locked segment must not be reported as empty | drainer snapshot; vitalsd health |
| schema/fingerprint guard | migration mismatch must name expected/actual fingerprint and first changed field | `SchemaFingerprint`, live layout query, and named mismatch errors exist | Ensure every mismatch is emitted to journal and health; no silent retry that looks healthy | drainer journal and snapshot; sqitch operator output |
| subscriber / panel boundary | queue admission and drops must be distinct from UI display | bounded `try_send` and per-subscriber drop counters exist | `vitalsd` can prove only `published`; panel/browser `displayed` evidence is outside T-S2 and must be registered in T-S4 | vitalsd health; panel/admin feed ledger |

## PostgreSQL and backup evidence

PostgreSQL is authoritative for durable history. A successful connection or a row visible in a
client is not, by itself, evidence that the spool checkpoint and the row commit agree.

| Producer / event | Required evidence | Location | Readers and failure rule |
|---|---|---|---|
| `postgresql@16-main` startup/readiness | active unit, `pg_isready`, UTC timezone, data directory on NVMe | systemd journal; `SHOW data_directory`, `SHOW timezone` | operator and deployment checker; missing or stale readiness is `UNKNOWN` |
| drainer advisory lock | one session owns `pg_advisory_lock(0x5641_4C53)`; another instance says the lock is held | PostgreSQL session/query result; drainer journal | drainer and operator; lock contention is a named blocker, not a retry-success |
| COPY transaction | private `TEMP` staging table, rows read/inserted/duplicate/rejected, transaction result | drainer journal; `.drain-health.json`; PostgreSQL statement log when enabled | drainer, vitalsd health, panel/admin; no offset may advance after COPY or commit failure |
| `samples` durable identity | `(source,type,session_id,seq)` key, UTC event timestamp, row count, duplicate replay count | PostgreSQL `samples`; SQL query output | panel history and admin; equal values with different identities remain separate |
| `spool_status` checkpoint | segment, offset, fingerprint, `drained`, `last_advanced`; DB checkpoint equals the committed segment boundary | PostgreSQL `spool_status` | drainer and restore verifier; disagreement is `UNKNOWN` and blocks cleanup |
| WAL/checkpoint/connection fault | named PostgreSQL error, connection age, WAL/checkpoint diagnostic | PostgreSQL journal/log and drainer journal | operator/admin; infrastructure faults never become physiological alarms |
| pgBackRest backup | stanza check, backup type/time, repository and encryption configuration, result | pgBackRest repository and journal | restore verifier/operator; a backup not restored is only a hypothesis |
| restore verification | JSON with `ok`, timestamp, tables checked, mismatches, restore duration | `/var/lib/vitals/restore-verify.json` | vitalsd health, panel, admin, operator; missing, stale, malformed, or false is `UNKNOWN` |

The Pi is not claimed healthy from this checkout: the `fe529c0` core artifact is now installed and
hash-matched, but the post-install daemon has zero fresh samples, H10 continuity is timing out,
EMAY is in an exit-70 restart loop, and G1 has no `hci1`. The database query is evidence of retained
history, not proof of fresh ingest or display.

## BlueZ, kernel, and systemd evidence

The hardware boundary must report which state was actually observed. In particular, *device not
found*, *out of range*, and *bonded but `ServicesResolved` never fired* are different states. A quiet
log is not a successful overnight capture.

| Producer / event | Required evidence | Location | Readers and failure rule |
|---|---|---|---|
| controller/kernel | controller count, stable adapter identity (`hci1` Sena when provisioned), `UP RUNNING`, USB binding | `btmgmt info` and `/sys/class/bluetooth` on 2026-09-05 report only powered `hci0` (`00:01:95XX:XX:XX`); the required `hci1` was absent | G1 is not green; Pi acceptance and operator must not apply `dtoverlay=disable-bt` |
| BlueZ discovery/pairing | target address/name, bond/IRK state, discovery result and bounded timeout | H10 `24:AC:ACXX:XX:XX` is paired/bonded/trusted but disconnected; configured EMAY `D9:80:A3XX:XX:XX` was not available; H10 journal recorded a bounded connect timeout and EMAY exited 70 with the missing-object error | H10/EMAY continuity is `UNKNOWN`/failed; collector and operator must use bond/IRK evidence, not a static address claim |
| H10 GATT | connection, `ServicesResolved`, `0x180D`/`0x2A37` subscription, decoded/undecodable counts | `hr-spike` stderr/stdout and systemd journal | T-B1 histogram and operator; unresolved services are an explicit exit state |
| EMAY live/stored path | live `EB` status-only identity, stored `EC`/`ED` cursor identity, transfer result, return-to-live state | collector status JSON and journal | panel/admin; anonymous live values never become durable numeric rows |
| systemd lifecycle | active state, restart count, exit status, OOM score/resource limits, dependency result | After atomic install, `vitalsd` and `vitals-spool-drain` are active; `vitals-emay` is `activating (auto-restart)` with exit 70; `vitals-h10` is active but timing out; panel services remain active | operator/admin; `active` without fresh stage evidence is not healthy, and the EMAY restart loop is a named blocker |
| continuity checker | fixed window, H10/EMAY sample ages, gaps, link state, and exact blocker | checker JSON/journal | panel/admin and operator; absent input holds `UNKNOWN` and does not reassure |

These are Pi-only facts. A macOS unit test may cover argument, state, and capture decision logic,
but it must not be described as Bluetooth hardware success. The real radio, kernel, BlueZ, and
systemd path remains a required acceptance check on `pi-host`.

## Panel, HTTP, and admin readers

The panel and admin console are read-only consumers. They must preserve the distinction between a
durable database point, a pending spool/live overlay, an explicit gap, and `UNKNOWN`.

| Reader | Inputs | Evidence it may claim | Fail-closed behavior |
|---|---|---|---|
| panel status publisher | cursor-based PostgreSQL query, live H10/EMAY status, pipeline health | status revision, `written_at`, durable identity, pending identity/state, displayed feed generation | malformed, stale, future, or contradictory input is `UNKNOWN`; never merge pending into durable history |
| bedside panel | status/history JSON over local HTTP | rendered revision/time and durable-versus-pending label | stale/missing JSON shows the diagnostic state; no synthetic point or gap filling |
| admin monitoring page | status, history, health, drainer snapshot, service evidence | source state, stage ledger, pending bytes/age, bounded UTC history, gaps, timeline, raw evidence id | missing, stale, malformed, or inconsistent feeds produce a fail-closed banner; GET only |
| browser/HTTP service | local `127.0.0.1` page and static assets | HTTP status/body hash and page test result | network/service failure is display `UNKNOWN`, never a sensor alarm |
| operator | all above plus journal and SQL outputs | command, target, timestamp, exit code, and artifact/hash identity | no production claim without independent evidence at each stage |

`published` at a socket or queue is not `displayed` on a screen. The display stage is only recorded
when the reader has consumed a valid revision; a fresh process or green colour cannot substitute for
that evidence.

## CI and artifact evidence

Every change runs the same deterministic contract. The per-file floor prevents a high-coverage
library from hiding an untested executable or adapter seam:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features --no-fail-fast
cargo llvm-cov --workspace --all-features \
  --fail-under-file-lines 90 \
  --fail-under-lines 95 --summary-only
```

The coverage command must report at least 90% lines for every major Rust source target, including
binaries, and at least 95% for the workspace aggregate. No binary or source may be excluded and no
coverage exclusion may hide code merely to pass. The exact output, toolchain, commit, and exit code
belong in the Pacific-time status update. On 2026-09-05 the portable/macOS report was green, while
the required Linux-target command was red because the EMAY executable wiring is not yet covered;
that is an open gate, not an inferred pass. Additional gates are `cargo audit --deny warnings`,
`cargo deny check advisories bans licenses sources`, decoder fuzz smoke runs, mutation tests, and
panel/admin tests. A skipped test or missing target is a failed gate, not a green result.

## Deployment evidence

Deployment uses a clean artifact/worktree, never the dirty checkout. `scripts/sync-to-pi.sh` uses
`rsync --delete`; this run used a detached clean worktree at `fe529c0`, and the dirty root was not
transferred. The safe record contains:

1. source commit and exact build commands;
2. SHA-256 for every binary, service unit, script, generated registry artifact, and page bundle;
3. staging directory and atomic rename/installation result;
4. only the intended units reloaded/restarted, with before/after status and restart counters;
5. rollback artifact/hash and a tested rollback command;
6. post-install spool health, PostgreSQL row/checkpoint queries, status JSON, HTTP output, panel/admin
   rendering, and H10/EMAY continuity evidence.

A hash comparison proves bytes, not behavior. Do not claim deployment or production success when an
artifact is merely built locally, when the Pi still runs an older hash, or when any verification
command lacks an exit code.

## Retention and readers

Retention is part of the evidence contract, not an afterthought:

| Data | Retention/publication rule | Primary reader |
|---|---|---|
| active/published spool segments | append-only until a committed EOF checkpoint and locked cleanup; pending bytes/age remain visible | drainer, vitalsd, operator |
| `.drain-health.json` and status snapshots | latest atomic snapshot; missing/stale snapshots are `UNKNOWN`, not history | vitalsd, panel, admin |
| PostgreSQL raw `samples` | TimescaleDB raw retention is 2 years; compress chunks after 7 days; no interpolation/gap filling | panel history, admin, analysis/backup |
| `samples_1min` aggregate | continuous min/max aggregate with explicit NULL gaps | bounded charts/admin; never used to invent raw points |
| pgBackRest repository | configured full/differential/incremental retention and encrypted repository; verify restores on schedule | restore verifier/operator |
| monthly cold export | previous completed UTC month, immutable self-contained `data.ndjson.zst`, registry snapshot, schema README, `SHA256SUMS`; never replace an existing month | offline reader/restore operator |
| systemd/journal evidence | host log policy must retain enough history to cover continuity and restore windows; exact policy is a deployment check | operator/admin |
| Pacific status updates | permanent repository record of commands, output summaries, exit codes, and unknown blockers | reviewers, future operators |

The following reader order is normative when investigating a row: **source event → vitalsd append
receipt → fsync evidence → spool segment/offset → drainer commit counters → `samples` identity →
status/history revision → rendered/displayed evidence**. If one link is missing, report the first
missing stage and its blocker; do not infer later stages.

## Registry maintenance rule

When a producer, filename, JSON schema, event name, retention policy, or reader changes, update this
file in the same commit and add a Pacific-time status update recording the verification command and
exit code. If a source cannot produce an event, record `UNKNOWN` and the named blocker rather than
inventing a substitute timestamp or sequence.
