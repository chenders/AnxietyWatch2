# Tasks — admin-console-m0

The conductor ticks a box only after running that task's Verify itself and seeing exit 0.

- [x] T-M0a — Per-source cadence in the registry · mechanical · deepseek/deepseek-v4-flash
- [x] T-M0b — Attempt/observation/domain-record schema · implement-standard · openai/gpt-5.6-sol
- [x] T-M0c — Liveness evaluator, one pure function · implement-complex · gemini/gemini-3.1-pro-preview
- [x] T-M0d — vitals-console axum skeleton · implement-complex · gemini/gemini-3.1-pro-preview
- [x] T-M0e — Envelope write API + attempts · implement-complex · deepseek/deepseek-v4-pro (openai credits outage mid-first-attempt; round 2 fixed a silent-skip integration suite and a cross-test isolation failure)
- [x] T-WD1 — Web dashboard: three directions behind a switcher (amendment 08 §3) · implement-standard · openai/gpt-5.6-sol · PR #33

Two findings surfaced by T-M0b's scratch-replay Verify, recorded here for the operator:

1. **The sqitch plan tail is undeployable and undeployed.** Production sqitch status lists
   samples_hypertable, samples_compression, samples_1min and retention as Undeployed, and they
   cannot deploy: create_hypertable requires the partition column in every unique constraint,
   while amendments 04/05 forbid timestamp in the identity (source, type, session_id, seq).
   Amendment 01's "samples is a hypertable, compressed, with a continuous aggregate" does not
   describe the running system. This contradiction needs its own amendment-level decision;
   nobody resolved it silently tonight.
2. **The plan cannot replay onto a sibling scratch database** even up to the deployed point:
   pg_cron binds to cron.database_name = vitals. Physical restores (pgBackRest) are unaffected;
   dev-environment and from-scratch tooling are. T-M0b's four changes were therefore verified by
   direct script application (deploy, verify, revert, absence-check) on an isolated scratch DB.

T-M0a addendum pending: polar_h10_emulator cadence lands after PR #32 merges (worker correctly reported the cross-branch contradiction rather than inventing the source).

---

## Brief: m0b (dispatched verbatim)

```
You are executing one task in the AnxietyWatch2 repo. Your working tree is /Users/chris/Source/aw2-m0b on branch task/m0b-console-schema — work ONLY there. Read AGENTS.md and docs/specs/AMENDMENTS.md first; they are binding. Acceptance is the Verify command's exit code, never your own assessment. Commit when green with trailer "Co-Authored-By: pi worker (openai/gpt-5.6-sol) <noreply@example.com>". Do NOT push.

### T-M0b — Attempt / observation / domain-record schema (admin console M0)

**Reads:** db/sqitch.plan · db/deploy/*.sql db/verify/*.sql db/revert/*.sql (style + plan conventions) · docs/design/SPEC-admin-console.md §2.3, §2.3b, §2.5, §8b "reported_at distinct from changed_at", §9 M0 criterion 3.

**Why:** the console cannot show what was never written. Attempt records exist BEFORE work begins; failure and no-op replay must be recordable; freshness is a stored field.

**Do:** four new sqitch changes appended to db/sqitch.plan, each with deploy/verify/revert scripts matching the existing style (comments carrying the invariant, verify scripts that actually SELECT against the structure):

1. attempts — id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY; kind TEXT NOT NULL; subject TEXT NOT NULL; started_at TIMESTAMPTZ NOT NULL DEFAULT now(); expected_duration_s INTEGER NOT NULL CHECK (expected_duration_s > 0); finished_at TIMESTAMPTZ; outcome TEXT CHECK (outcome IN ('succeeded','failed','cancelled')); effect JSONB; high_water_mark TIMESTAMPTZ; detail TEXT; CHECK ((finished_at IS NULL) = (outcome IS NULL)). Index on (kind, subject, started_at DESC). Comment: a row exists before its work begins; a killed process leaves outcome NULL, which the liveness evaluator degrades to UNKNOWN past expected_duration_s; effect distinguishes created/updated/replayed/rejected counts because a no-op replay success is not evidence of progress.

2. status_observations — subject TEXT PRIMARY KEY; state TEXT NOT NULL; reason TEXT; reported_at TIMESTAMPTZ NOT NULL; changed_at TIMESTAMPTZ NOT NULL. Comment: reported_at is the last affirmative assertion, changed_at the last value change; freshness is stored, never derived (Home Assistant precedent, spec §8b).

3. domain_records — id UUID PRIMARY KEY; kind TEXT NOT NULL; content JSONB NOT NULL; content_sha256 BYTEA NOT NULL CHECK (octet_length(content_sha256) = 32); created_at TIMESTAMPTZ NOT NULL DEFAULT now(); updated_at TIMESTAMPTZ NOT NULL DEFAULT now(). Index on (kind, created_at DESC). Comment: identity is the client-minted UUID (spec §2.3b); same id + same hash replays as a no-op; same id + different hash is a conflict recorded as rejected, never an overwrite.

4. envelope_clients — client_id UUID PRIMARY KEY; secret BYTEA NOT NULL; created_at TIMESTAMPTZ NOT NULL DEFAULT now(); last_seen_at TIMESTAMPTZ. Comment: register-once/update-many envelope credentials; the secret is the libsodium secretbox key, never logged.

None of these touches the samples table — state that in each deploy header comment so the spool-empty guard question cannot arise.

**Verify (runs the whole plan into a scratch DB on the Pi and reverts it):**

cd /Users/chris/Source/aw2-m0b && PI_HOST=chris@minidude PI_PATH=/home/chris/aw2-m0b scripts/sync-to-pi.sh && ssh chris@minidude 'set -e; sudo -u postgres createdb m0b_scratch; cd /home/chris/aw2-m0b/db; rc=0; sudo -u postgres sqitch deploy db:pg:m0b_scratch && sudo -u postgres sqitch verify db:pg:m0b_scratch && sudo -u postgres sqitch revert -y db:pg:m0b_scratch || rc=1; sudo -u postgres dropdb m0b_scratch; exit $rc'

**Expect:** exit 0. The scratch DB is created and dropped inside the command — never touch the vitals database.

Report when done: what changed, Verify output tail, contradictions if any (report contradictions, do not silently resolve them).
```

## Brief: m0c (dispatched verbatim)

```
You are executing one task in the AnxietyWatch2 repo. Your working tree is /Users/chris/Source/aw2-m0c on branch task/m0c-liveness-evaluator — work ONLY there. Read AGENTS.md and docs/specs/AMENDMENTS.md first; they are binding. Acceptance is the Verify command's exit code, never your own assessment. Commit when green with trailer "Co-Authored-By: pi worker (gemini/gemini-3.1-pro-preview) <noreply@example.com>". Do NOT push.

### T-M0c — The liveness evaluator: one pure function (admin console M0, spec §9 criterion 1)

**Reads:** docs/design/SPEC-admin-console.md §2.2, §2.5, §2.5b, §2.6, §8b (learned BLE staleness), §9 M0 criteria 1 and 3 · crates/vitals-core/src/lib.rs (module wiring style) · crates/vitals-core/src/spool.rs test-module style.

**Why:** this is the ONLY implementation of source-health policy in the system. The console, the silence detector (M2), and every API response call it; nothing may reimplement precedence or cadence policy anywhere else.

**Do:** new module crates/vitals-core/src/liveness.rs, exported as pub mod liveness from lib.rs. Pure — no I/O, no clock reads, no allocation beyond the reason String. Exact public API:

pub enum Cadence { EverySeconds(u32), OnDemand }
pub struct Inflight { pub started_epoch_s: i64, pub expected_duration_s: u32 }
pub struct Evidence {
    pub last_attempt_epoch_s: Option<i64>,
    pub last_attempt_failure: Option<String>,   // Some(reason) when the most recent finished attempt failed
    pub last_success_epoch_s: Option<i64>,
    pub inflight: Option<Inflight>,             // an attempt with no outcome yet
    pub learned_interval_s: Option<u32>,        // learned advertising interval; may TIGHTEN only
}
pub enum HealthState { Live, Failing, Unknown }
pub struct Assertion { pub state: HealthState, pub reason: String, pub degrade_deadline_epoch_s: Option<i64> }
pub fn evaluate(now_epoch_s: i64, declared: Cadence, evidence: &Evidence) -> Assertion

Rules, all mandatory, each carried by a doc comment naming its spec section:
1. Never observed (no attempt, no success, no inflight) -> Unknown, reason "never observed".
2. Precedence is FAILING > UNKNOWN > LIVE (spec §2.5). A source whose most recent finished attempt failed is Failing with that reason, even when it is also stale past its cadence — staleness is mentioned in the reason but does not demote to Unknown.
3. Interval sources (Cadence::EverySeconds): cadence measures ATTEMPTS. age = now - last_attempt. The effective cadence is the declared value, tightened by the learned interval when present: effective = min(declared, learned). A learned value larger than the declaration is ignored (the declaration is the floor of trust, spec §8b). age > effective -> Unknown with reason naming the age and the expectation ("no attempt for Xs, expect <= Ys"). age <= effective -> Live, reason carrying age and expectation. THE BOUNDARY: age == effective is still Live; stale means strictly past cadence. degrade_deadline_epoch_s = last_attempt + effective for Live; None otherwise unless rule 5 sets one.
4. Cadence::OnDemand: never time-degraded (spec §2.5). State is the outcome and age of the last event: failed -> Failing, succeeded -> Live, never -> Unknown. deadline None.
5. An inflight attempt past its expected duration (now - started > expected_duration_s) -> Unknown, reason naming elapsed vs expected ("attempt running Xs, expected <= Ys") — this is how a killed process stops reading as running forever (spec §9 criterion 3). It overrides Live but never overrides Failing. An inflight attempt within its expected duration does not change the state; for a Live interval source the deadline is then min(rotation deadline, started + expected).
6. Every reason names the value and the threshold that produced it. "restore blocked: HealthSnapshot=91", never "restore blocked" — same rule here.

Tests, in-module #[cfg(test)], covering every rule above plus explicitly:
- the boundary test the spec demands: age == effective cadence is Live, age == effective + 1 is Unknown;
- learned interval tightens (declared 300, learned 60, age 120 -> Unknown) and never loosens (declared 60, learned 300, age 120 -> Unknown);
- Failing beats stale; OnDemand old success stays Live; inflight overdue beats Live but not Failing;
- fixed integer timestamps only, no wall clock anywhere; no float assertions.

Workspace lints are deny (no unwrap/expect/panic outside tests, no indexing). Match the crate's existing doc-comment density.

**Verify:** cd /Users/chris/Source/aw2-m0c && cargo test -p vitals-core liveness && cargo clippy -p vitals-core --all-targets && cargo fmt --check
**Expect:** exit 0.

Report when done: what changed, Verify output tail, contradictions if any (report contradictions, do not silently resolve them).
```

## Brief: m0d (dispatched verbatim)

```
You are executing one task in the AnxietyWatch2 repo. Your working tree is /Users/chris/Source/aw2-m0d on branch task/m0d-console-skeleton — work ONLY there. Read AGENTS.md and docs/specs/AMENDMENTS.md first (amendment 07 governs this task); they are binding. Acceptance is the Verify command's exit code, never your own assessment. Commit when green with trailer "Co-Authored-By: pi worker (gemini/gemini-3.1-pro-preview) <noreply@example.com>". Do NOT push.

### T-M0d — vitals-console: the axum service skeleton (admin console M0)

**Reads:** docs/specs/2026-08-05-amendment-07-api-in-rust.md (whole file — one wire-protocol implementation is the entire argument) · docs/design/SPEC-admin-console.md §7b (the three contract mismatches — your mapping table), §9 M0 criterion 1, §9c (bind/auth is deliberately open; default private) · crates/vitals-proto/src/lib.rs (Request/Response/Health/SourceHealth/Liveness — you consume these types, never restate them) · crates/vitals-core/src/liveness.rs (the evaluator — already on this branch; call it, do not reimplement any of its policy) · crates/vitals-registry/src/generated.rs (ALL_SOURCES + expected_cadence — already on this branch) · crates/vitals-collectors/src/bin/bp_collector.rs append_durable() for the socket framing idiom.

**Why:** the console backend consumes vitalsd's CBOR-over-Unix-socket protocol directly in Rust — one wire implementation, not two. This task is the read-only substrate: service, socket client, health mapping, registry left-join.

**Do:**
1. New crate crates/vitals-console (add to workspace members): axum + tokio; lib + a thin bin. Dependencies: axum, tokio, vitals-proto, vitals-core, vitals-registry, serde, serde_json. No sqlx yet — the DB endpoints are T-M0e.
2. Socket client module: connect to VITALSD_SOCKET (default /run/vitals/vitalsd.sock), send Request::Health, read the length-prefixed CBOR reply (use check_frame_len exactly like the collectors), 10s timeout, every failure a named error.
3. GET /api/health returns JSON built by this mapping (spec §7b, verbatim requirement):
   - Start from the REGISTRY: iterate ALL_SOURCES so a source vitalsd has never seen still appears (the left-join; criterion 7 depends on it).
   - For each source, translate vitalsd SourceHealth through vitals_core::liveness::evaluate with the registry cadence: vitalsd Live/Stale/NeverSeen all become evaluator Evidence (last_attempt from last_sample_age_secs; NeverSeen or absent-from-map = never observed). There is NO FAILING on this wire today — FAILING only ever comes from the attempt ledger, which is T-M0e; do not fake one.
   - Response per source: source id + name? No — ids only plus the generated enum name (Debug format of VitalsSource is acceptable), the evaluator Assertion (state, reason, degrade_deadline_epoch_s), the observation age, and the raw vitalsd liveness for transparency.
   - Include the daemon-level facts: uptime, degraded[] (verbatim — those strings name their blockers), heartbeat_pulsing, and a top-level "vitalsd": "reachable" | {"unreachable": reason} — an unreachable daemon must produce a page-level UNKNOWN, never an empty 200.
4. GET /api/version: crate version + git describe if available at build time (option_env), plus the schema note "attempts schema: not yet migrated here" — honest placeholder until T-M0e.
5. Bind: VITALS_CONSOLE_BIND env, default 127.0.0.1:8480 (spec §9c defers the auth posture; the default must be private). Log the bind and the socket path at startup.
6. systemd unit deploy/vitals-console.service modeled on deploy/vitals-bp.service hardening (User=vitals, ProtectSystem=strict, private tmp, OOMScoreAdjust positive +200 — this is NOT the safety path; Postgres/console die before collectors).
7. Tests (in-crate, no network): spawn a stub Unix-socket server in the test (std/tokio, reply canned Health CBOR frames using vitals-proto types) and assert: (a) a source present as Live maps to LIVE with age and deadline; (b) Stale and NeverSeen map to UNKNOWN, never FAILING; (c) a registry source absent from the reply still appears as UNKNOWN "never observed"; (d) unreachable socket produces the unreachable marker, not a panic and not an empty list; (e) the reason strings name value + threshold.

Workspace lints are deny: no unwrap/expect/panic outside tests, no indexing. Keep handlers producing plain serde structs — one view-model per screen is the M1 rule and starts here.

**Verify:** cd /Users/chris/Source/aw2-m0d && cargo test -p vitals-console && cargo clippy -p vitals-console --all-targets && cargo fmt --check
**Expect:** exit 0.

Report when done: what changed, Verify output tail, contradictions if any (report contradictions, do not silently resolve them).
```

## Brief: m0e (dispatched verbatim)

```
You are executing one task in the AnxietyWatch2 repo. Your working tree is /Users/chris/Source/aw2-m0e on branch task/m0e-envelope-api — work ONLY there. Read AGENTS.md and docs/specs/AMENDMENTS.md first; they are binding. Acceptance is the Verify command's exit code, never your own assessment. Commit when green with trailer "Co-Authored-By: pi worker (openai/gpt-5.6-sol) <noreply@example.com>". Do NOT push.

### T-M0e — The client write API skeleton: envelope + idempotent domain records + attempts (admin console M0)

**Reads:** docs/design/SPEC-admin-console.md §2.3 (attempt records), §2.3b (the idempotency table — your contract, implement every row), §8b "The webhook envelope" row, §9 M0 criterion 3 · crates/vitals-console (on this branch — you extend it) · db/deploy/attempts.sql, domain_records.sql, envelope_clients.sql (on this branch — the schema you write to).

**Why:** every future phone write rides this envelope, and the whole retry story ("send again until acknowledged") only works if replays are perfect no-ops and conflicts are loud.

**Do, extending crates/vitals-console:**
1. DB access via sqlx (postgres, runtime queries — sqlx::query, NOT the compile-time macros; the build must not need a database). Pool from VITALS_CONSOLE_DATABASE_URL. Endpoints that need the DB return a named 503 when it is absent — the service must still serve /api/health without a database.
2. POST /api/envelope/register: body {client_id: uuidv4/v7, secret: base64 32 bytes} -> stores into envelope_clients (upsert on client_id only when the secret matches, else 409 conflict — a re-register with a different secret is a takeover attempt and gets refused by name). Registration is plaintext-over-localhost for now; the bind default is 127.0.0.1 and the auth posture is an open decision (spec §9c) — say this in a route comment.
3. POST /api/envelope: body {client_id, nonce: base64 24 bytes, payload: base64 secretbox ciphertext}. Decrypt with crypto_secretbox (XSalsa20-Poly1305; use the crypto_secretbox crate) using the registered secret. Inside: {"class": "domain_record", "record": {"id": uuidv7, "kind": "...", "content": {...}}}. Unknown class -> 400 naming the class (the sample class lands at the phone-ingest stage, not now).
4. Idempotency, exactly spec §2.3b's table, applied to domain_records with content_sha256 = SHA-256 of the canonical JSON (serde_json to_vec of the content value — document that canonicalization is byte-equality of serialization and why that is sufficient for a single writer today):
   - same id, same hash -> no-op replay; return the stored record with the SAME status as the original create (200 with replayed marker), effect counted as replayed;
   - same id, different hash -> 409 conflict, recorded as a rejected attempt, NEVER an overwrite;
   - no id -> 400 rejected (content-hash identity is explicitly wrong, the spec says why);
   - new id -> insert, 201, effect created.
5. Every envelope POST is wrapped in an attempt record written BEFORE the work (kind='envelope', subject=client_id, expected_duration_s=30), finished with outcome + effect breakdown {created,updated,replayed,rejected} afterwards. A handler crash mid-way must leave the outcome NULL row (that is the point — criterion 3).
6. GET /api/attempts?limit=N: most recent attempt rows, the breakdown visible.
7. A crude test page at /test (static HTML, one form that registers + sends an envelope via JS with a bundled tweetnacl-style secretbox implementation OR a server-side /api/envelope/plaintext-test route guarded behind VITALS_CONSOLE_TEST_ROUTES=1 — pick the simpler, say which; the spec calls this page deliberately not part of the console).
8. Integration tests: gated on VITALS_CONSOLE_TEST_DATABASE_URL being set (skip cleanly when absent, printing that they were skipped). They must cover all four idempotency rows and the attempt-before-work property (kill path can be simulated by writing the attempt then erroring before finish; assert the NULL-outcome row).

**Verify (unit tests always; integration against a scratch DB in the aw2-refpg docker container on this Mac):**
cd /Users/chris/Source/aw2-m0e && cargo test -p vitals-console && cargo clippy -p vitals-console --all-targets && cargo fmt --check
Then report the integration-test command you validated with VITALS_CONSOLE_TEST_DATABASE_URL pointing at a scratch database you created and dropped in the aw2-refpg container (docker exec aw2-refpg createdb/dropdb; discover its port with docker port aw2-refpg; deploy the sqitch schema from db/ before the run — sqitch runs on the Pi if absent locally, or apply the three deploy scripts directly with psql, in plan order, and say which you did).
**Expect:** exit 0 on the Verify line; integration suite green with the DB URL set.

Report when done: what changed, Verify output tail, the integration evidence, contradictions if any.
```

## Brief: wd1 (dispatched verbatim)

```
You are executing one task in the AnxietyWatch2 repo. Your working tree is /Users/chris/Source/aw2-wd1 on branch task/wd1-dashboard-directions — work ONLY there. Read AGENTS.md and docs/specs/AMENDMENTS.md first (amendment 08 governs this task); they are binding. Acceptance is the Verify command's exit code plus the invariants below. Commit when green with trailer "Co-Authored-By: pi worker (openai/gpt-5.6-sol) <noreply@example.com>". Do NOT push.

### T-WD1 — Web dashboard: all three directions behind a runtime switcher (amendment 08 §3)

**Reads:** docs/specs/2026-08-06-amendment-08-layout-evaluation.md (§3 is your contract) · docs/design/web-dashboard/README.md in full (the "Wiring it up" table and both invariants) · docs/design/web-dashboard/index.html (direction A, already ported onto the model() seam — model() is the single source of view data and render() consumes it) · docs/design/web-dashboard/claude-design/Direction B - Night Report.dc.html · docs/design/web-dashboard/claude-design/Direction C - Terminal.dc.html · docs/design/pi-panel/direction-a-clinical/index.html ONLY for its PANELS-array + ?view= + localStorage switcher pattern (the precedent named by the amendment).

**Why:** no layout is chosen from mockups. All three directions ship as live renderers over the SAME view model and are evaluated in operation; the losers are later deleted as a file deletion, which is only cheap if you keep every scrap of data logic out of the renderers.

**Do, all inside docs/design/web-dashboard/index.html (it stays one self-contained file, inline CSS/JS, no external dependencies):**
1. Extract direction A's current rendering into renderA(m). It must produce pixel-identical output to today for the default case.
2. Port direction B (Night Report) and direction C (Terminal) from their claude-design mockups as renderB(m) and renderC(m), each consuming ONLY the object returned by model(). Their CSS comes along namespaced (e.g. body[data-dir="b"] scoping) so the three directions cannot bleed into each other.
3. Switcher, exactly the amendment's contract: ?dir=a|b|c overrides, localStorage persists the last choice, default is the persisted value, falling back to a. Add a small fixed "change layout" control consistent with direction A's visual language. Set data-dir on body so CSS scoping follows.
4. NO direction-specific data logic anywhere: renderB/renderC must not fetch, must not read status.json, must not compute derived health values — if a value they need is missing from model(), extend model() ONCE so all three renderers see it, and note that in your report.
5. Both README invariants bind all three renderers: a gap arrives as null and renders as a gap (never coerce null to 0 or interpolate — grep yourself for "?? 0" and "|| 0" style coercions before finishing); trends stay one-point-per-night server-side aggregates (do not invent client-side resampling).
6. Keep the mockup files in claude-design/ untouched — they are the record the pruning task deletes later.

**Verify:** cd /Users/chris/Source/aw2-wd1 && node docs/design/web-dashboard/palette-check.mjs docs/design/web-dashboard/index.html && node -e "const s=require('fs').readFileSync('docs/design/web-dashboard/index.html','utf8'); for (const t of ['function renderA','function renderB','function renderC','localStorage','?dir=']) if(!s.includes(t)) { console.error('missing: '+t); process.exit(1);} console.log('structure ok')"
**Expect:** exit 0 on both. If palette-check legitimately conflicts with direction B or C styling, STOP and report the conflict — do not weaken the checker and do not restyle the directions to dodge it silently.

Report when done: what changed, Verify output tail, any model() extensions you made, contradictions if any.
```

