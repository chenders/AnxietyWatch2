# Tasks: Resolve `samples.type` at read

Order is load-bearing. T-STR-1 removes the irreversible path; T-STR-7 and T-STR-8 are the
only steps that change what the database declares, and neither may run before T-STR-6.

- [x] T-STR-0 — Preserve the old scheme as evidence · implement-simple · host
- [x] T-STR-1 — Guard `vitals_type` against relabelling an id that has rows · implement-simple · host
- [ ] T-STR-2 — Add the `sample_type_resolution` table · implement-simple · pi
- [ ] T-STR-3 — Populate the map from the rows' own attestation · implement · pi
- [ ] T-STR-4 — Add `samples_resolved` and move every reader onto it · implement · host
- [ ] T-STR-5 — Quarantine source 4's sub-minimum respiratory rates · implement-simple · pi
- [ ] T-STR-6 — Extend the guard to require full resolution coverage · implement · host
- [ ] T-STR-7 — Reconcile `vitals_type` to the declared scheme · deploy · pi
- [ ] T-STR-8 — Rebuild the collector and record the era boundary · deploy · pi
- [ ] T-STR-9 — Correct the amendments this change contradicts · docs · host

---

## T-STR-0 — Preserve the old scheme as evidence

**Depends:** none · **Review required:** no

**Amends:** amendment 20 §6 (extends id permanence to the type axis).

**Reads:** `docs/specs/2026-11-09-amendment-20-liveness-evidence-channels.md` §6.

**Modifies:** `docs/specs/attachments/`.

**Do:** Copy `$HOME/aw2-air11-build/vitals/registry.toml` from the Pi into
`docs/specs/attachments/2026-09-10-air11-build-registry.toml`, **verbatim** — its bytes are
the record and its hash is the attestation, so do not add a header. Record provenance, date,
size and `sha256` in a sibling `README.md`.

That directory is not a git repository. This was a single uncommitted file on one disk, in no
history anywhere, and it is the authoritative record of what 110,666,216 rows mean.

**Verify:**
```sh
test "$(shasum -a 256 docs/specs/attachments/2026-09-10-air11-build-registry.toml | awk '{print $1}')" \
  = "eda48c2b0423f82d0803a275193c72a8aee06f917f694d0657ba7df78cf197bc" \
  && grep -q 'eda48c2b0423f82d0803a275193c72a8aee06f917f694d0657ba7df78cf197bc' docs/specs/attachments/README.md
```

**Context budget:** 2 files.

---

## T-STR-1 — Guard `vitals_type` against relabelling an id that has rows

**Depends:** none · **Review required:** yes

**Amends:** amendment 20 §6 — its guard covers `vitals_source` only; this is the type half it
left open, and the amendment does not mention that `samples.type` holds two schemes at all.

**Reads:** `vitals/tools/gen_registry.py`, `vitals/generated/registry.sql`.

**Modifies:** `vitals/tools/gen_registry.py`, `vitals/tools/tests/test_gen_registry.py`.

**Do:** Emit a `DO $$` guard before any `vitals_type` upsert that raises when a stored id's
name, unit, `range_lo` or `range_hi` differs from the declaration **and** `samples` holds rows
at that id. Name the id, both names, both units and the row count. Check unit and range as well
as name: a stable name with a changed unit is the dimensional defect this project exists to
prevent. Require in `validate()` that both guards precede their upserts.

**Verify:** landed as #100 — the guard refuses the real collision:
```sh
uv run python3 vitals/tools/gen_registry.py --check \
  && uv run --with pytest python3 -m pytest vitals/tools/tests/test_gen_registry.py -q \
  && grep -q 'FROM vitals_type AS stored' vitals/generated/registry.sql \
  && uv run python3 -c "
import sys; sql=open('vitals/generated/registry.sql').read()
sys.exit(0 if sql.index('FROM vitals_type AS stored') < sql.index('INSERT INTO vitals_type ') else 1)"
```

**Context budget:** 3 files.

---

## T-STR-2 — Add the `sample_type_resolution` table

**Depends:** T-STR-1 · **Review required:** yes

**Amends:** none.

**Reads:** `db/deploy/registry_tables.sql`, `db/sqitch.plan`, `AGENTS.md` (append-only rule).

**Modifies:** `db/deploy/sample_type_resolution.sql`, `db/revert/…`, `db/verify/…`,
`db/sqitch.plan`.

**Do:** New sqitch change. Key `(source, stored_type, valid_from)`; reference
`vitals_type(name)`, **not** its id, so the map survives T-STR-7 — `name` is `NOT NULL UNIQUE`,
so this is a legal and id-stable foreign key. Carry `stored_unit`, `stored_range_lo`,
`stored_range_hi` so the dimension a value was written in is never in doubt, plus `attested_by`,
`attested_value`, `attested_rows`, `total_rows` and a free-text `note`.

The verify script must **assert**, not merely `SELECT` — a catalog query with a predicate
returns zero rows when the object is absent, and zero rows is not an error. Use
`SELECT 1/count(*)`, and `scripts/tests/test-sqitch-verify-asserts.sh` will reject it otherwise.

**Verify:**
```sh
bash scripts/tests/test-sqitch-verify-asserts.sh \
  && ( cd db && sqitch deploy --target "db:pg://vitals@/vitals?host=/var/run/postgresql" \
         --top-dir . --to sample_type_resolution \
     && sqitch verify --target "db:pg://vitals@/vitals?host=/var/run/postgresql" --top-dir . )
```

**Context budget:** 5 files.

---

## T-STR-3 — Populate the map from the rows' own attestation

**Depends:** T-STR-2 · **Review required:** yes

**Amends:** none.

**Reads:** `docs/specs/attachments/2026-09-10-air11-build-registry.toml`,
`crates/vitals-collectors/src/as11_build.rs` (the `extra` envelope shape).

**Modifies:** `scripts/pi/populate-sample-type-resolution.sql` (generated output committed
alongside the generator).

**Do:** Generate the rows by querying `samples` — **never hand-write them**. For source 5 use
`extra.signal`; for source 2 use `extra.healthkit_type`; set `attested_rows = total_rows` where
the cross-tab is 1:1. Include the non-colliding pairs too, so coverage is total and a missing
row means *unknown* rather than *assume identity*.

Source 4 is the weak case and must be labelled as such: its `extra` carries no signal field
(only `protocol=pc60f-provisional;safety=unvalidated`), so its type 9 is attested only at
declaration level — `attested_by = 'value_range+vitals_type+amendment_20'`, `attested_rows = 0`.
Its evidence is that the observed max of 19.8 exceeds `sleep_stage`'s declared max of 4.0, and
that amendment 20 §6's recorded 52,059 rows match exactly.

**Verify:** every `(source, type)` pair present in `samples` must resolve:
```sh
psql -U vitals -d vitals -h /var/run/postgresql -v ON_ERROR_STOP=1 -c "
SELECT 1/CASE WHEN count(*) = 0 THEN 1 ELSE 0 END FROM (
  SELECT DISTINCT s.source, s.type FROM samples s
  EXCEPT SELECT r.source, r.stored_type FROM sample_type_resolution r) AS unresolved;"
```

**Context budget:** 4 files.

---

## T-STR-4 — Add `samples_resolved` and move every reader onto it

**Depends:** T-STR-3 · **Review required:** yes

**Amends:** none.

**Reads:** `crates/vitals-console/src/api/air11.rs`, `crates/vitals-console/src/api/postgres.rs`.

**Modifies:** `db/deploy/samples_resolved.sql` (+ revert/verify), `db/sqitch.plan`,
`crates/vitals-console/src/api/air11.rs`, `.github/workflows/ci.yml`.

**Do:** `LEFT JOIN`, not inner — an unresolved row must surface as `resolved = false`, not
vanish. Three states, and the view is where a reader meets them. Move the live consumer at
`crates/vitals-console/src/api/air11.rs` onto the view. Add a CI gate that fails any
`JOIN vitals_type … ON … = s.type`, because a consumer joining on the raw id gets a
confidently wrong answer and that must be impossible rather than discouraged.

**Readers move before the declaration flips.** This task must land before T-STR-7.

**Verify:**
```sh
cargo test --workspace --locked \
  && ! git grep -nE 'JOIN[[:space:]]+vitals_type[^;]*=[[:space:]]*s\.type' -- 'crates/**/*.rs' \
  && psql -U vitals -d vitals -h /var/run/postgresql -At -v ON_ERROR_STOP=1 -c \
       "SELECT 1/CASE WHEN bool_and(resolved) THEN 1 ELSE 0 END FROM samples_resolved LIMIT 1000;"
```

**Context budget:** 6 files.

---

## T-STR-5 — Quarantine source 4's sub-minimum respiratory rates

**Depends:** T-STR-2 · **Review required:** yes

**Amends:** none.

**Reads:** `AGENTS.md` (never clamp; never coerce a sentinel), `vitals/registry.toml`
(`respiratory_rate` range).

**Modifies:** `db/deploy/sample_quarantine.sql` (+ revert/verify), `db/sqitch.plan`.

**Do:** 277 of source 4's 52,059 `respiratory_rate` rows are below the declared minimum of
1.0, distributed 0.2(8) 0.4(38) 0.5(43) 0.6(20) 0.7(32) 0.8(60) 0.9(76).

**That distribution is the finding: nothing was clamped.** Clamping would have piled them at
exactly 1.0; instead sub-minimum values passed straight through, so the range check was absent
on that path. 0.2–0.9 breaths/min is not a breathing human — it is the PC-60F's no-detection
state leaking out as a number.

Declare a predicate-based quarantine with a stated reason. Exclude at read time; **delete
nothing** and reinterpret nothing. The remaining 51,782 rows stay fully readable.

**Verify:**
```sh
psql -U vitals -d vitals -h /var/run/postgresql -v ON_ERROR_STOP=1 -c "
SELECT 1/count(*) FROM sample_quarantine
 WHERE source = 4 AND stored_type = 9 AND row_count = 277
   AND reason LIKE '%not clamped%';" \
&& psql -U vitals -d vitals -h /var/run/postgresql -At -c \
     "SELECT count(*) FROM samples WHERE source=4 AND type=9;" | grep -qx 52059
```

**Context budget:** 4 files.

---

## T-STR-6 — Extend the guard to require full resolution coverage

**Depends:** T-STR-3, T-STR-4 · **Review required:** yes

**Amends:** none.

**Reads:** `vitals/tools/gen_registry.py` (the T-STR-1 guard).

**Modifies:** `vitals/tools/gen_registry.py`, `vitals/tools/tests/test_gen_registry.py`.

**Do:** Permit a relabel only when every affected `(source, stored_type)` has a resolution row
whose `stored_unit`, `stored_range_lo` and `stored_range_hi` equal what `vitals_type` currently
holds. Partial coverage must raise, naming the uncovered pair and its row count.

This is what turns T-STR-7 from destructive into mechanical: after it, the reconciliation can
only succeed if the old meaning is already recorded.

**Verify:**
```sh
uv run --with pytest python3 -m pytest vitals/tools/tests/test_gen_registry.py -q \
  && uv run python3 vitals/tools/gen_registry.py --check
```

**Context budget:** 3 files.

---

## T-STR-7 — Reconcile `vitals_type` to the declared scheme

**Depends:** T-STR-6 · **Review required:** yes · **Destructive — stop and confirm**

**Amends:** none.

**Reads:** `vitals/generated/registry.sql`, amendment 01 §7.

**Modifies:** the Pi's `vitals_type` and `vitals_source` rows only.

**Do:** Apply `vitals/generated/registry.sql`. The guard from T-STR-6 permits it only once
coverage is total. This also deploys amendment 20 §6's source half, which has never reached the
Pi — `vitals_source` currently holds ids 0–7 with **no `retired` column**, so `wellue_ks60fwb`
retired at 4, `resmed_air11_sd` at 8 and `acconeer_xe125` at 9 all arrive here.

`registry.toml` is the single source of truth, so main's scheme is the declaration — and
`vitals_type.name` being `UNIQUE` means staying on the old scheme would permanently block ids
24–43, and with them HealthKit ingest, the SD importer and any collector rebuild.

**Take a fresh `pgbackrest --type=diff` first** and confirm the restore point.

**Verify:**
```sh
psql -U vitals -d vitals -h /var/run/postgresql -v ON_ERROR_STOP=1 -c "
SELECT 1/CASE WHEN count(*) = 43 THEN 1 ELSE 0 END FROM vitals_type;" \
&& psql -U vitals -d vitals -h /var/run/postgresql -At -c \
     "SELECT string_agg(id::text, ',' ORDER BY id) FROM vitals_source;" | grep -qx '0,1,2,3,4,5,6,7,8,9' \
&& psql -U vitals -d vitals -h /var/run/postgresql -At -v ON_ERROR_STOP=1 -c \
     "SELECT 1/CASE WHEN bool_and(resolved) THEN 1 ELSE 0 END FROM samples_resolved LIMIT 1000;"
```

**Context budget:** 3 files.

---

## T-STR-8 — Rebuild the collector and record the era boundary

**Depends:** T-STR-7 · **Review required:** yes · **Destructive — stop and confirm**

**Amends:** none.

**Reads:** `crates/vitals-collectors/src/bin/resmed_air11_collector.rs`,
`crates/vitals-collectors/src/as11_session.rs`, `scripts/install-air11-pi-artifact.sh`.

**Modifies:** the deployed collector binary; one `sample_type_resolution` row closed and one
opened.

**Do:** The rebuilt collector writes `patient_flow` as type 30 while 51.7M historical
`patient_flow` rows sit at type 10. That is only safe once `valid_to` stops being decorative:
close `(5, 10, valid_to = <cutover>)` and open `(5, 30, valid_from = <cutover>)` **in the same
transaction as the binary swap**, and cut over at a session boundary so no session spans two
schemes.

Install only via `scripts/install-air11-pi-artifact.sh` — never an ad-hoc `scp`.

Keeping the Pi on a 9–23 fork is not an alternative: it violates "the generated enums
everywhere, never a bare integer" and leaves the Pi permanently un-rebuildable.

**Verify:**
```sh
psql -U vitals -d vitals -h /var/run/postgresql -v ON_ERROR_STOP=1 -c "
SELECT 1/CASE WHEN count(*) = 2 THEN 1 ELSE 0 END FROM sample_type_resolution
 WHERE source = 5 AND canonical_name = 'patient_flow'
   AND ((stored_type = 10 AND valid_to IS NOT NULL) OR (stored_type = 30 AND valid_to IS NULL));" \
&& psql -U vitals -d vitals -h /var/run/postgresql -At -v ON_ERROR_STOP=1 -c \
     "SELECT 1/CASE WHEN bool_and(resolved) THEN 1 ELSE 0 END FROM samples_resolved
       WHERE timestamp > now() - interval '10 minutes';" \
&& test -c /dev/null
```

**Context budget:** 5 files.

---

## T-STR-9 — Correct the amendments this change contradicts

**Depends:** T-STR-3 · **Review required:** yes

**Amends:** amendments 01, 19 and 20.

**Reads:** `docs/specs/2026-08-03-amendment-01-timescale-copy-spool.md`,
`docs/specs/2026-11-08-amendment-19-samples-storage-and-identity.md`,
`docs/specs/2026-11-09-amendment-20-liveness-evidence-channels.md`.

**Modifies:** those three, and `docs/specs/AMENDMENTS.md` if a new amendment is required for
the per-source type resolution decision.

**Do:** A plan doc that does not reflect what shipped is a defect, and these four mislead a
worker who obeys the scoping rule:

- **A19 §2** measured 99,020,648 rows / 44 GB on 2026-11-08. It is **112,030,357 / 46 GB**.
  Its §2.1 "`resmed_air11` is 99.2%" is 98.8%.
- **A20 §6** decides ids are permanent and says the guard "lives in generated SQL". True for
  sources, **false for types** until T-STR-1 — and §6 does not mention that `samples.type`
  holds two schemes at all. Say so, or the next worker reads §6 and believes the class is
  closed.
- **A20 §6** records `acconeer_xe125` at id 9 and `wellue_ks60fwb` retired at 4 as decided.
  Neither has been deployed and `vitals_source` has no `retired` column. Record as undeployed.
- **A01 §7** says reference data is "reconciled on service start". Nothing implements it.
  Strike it or mark it unimplemented — a reader who believes it will conclude, wrongly, that a
  restart can fire the destructive upsert.

**Verify:**
```sh
bash scripts/verify-amendment-index.sh \
  && bash scripts/tests/test-amendment-index.sh \
  && uv run python3 scripts/check-amendment-refs.py \
  && ! grep -q '99,020,648' docs/specs/2026-11-08-amendment-19-samples-storage-and-identity.md \
  && grep -q 'reconciled on service start' -r docs/specs/ \
       && grep -qE 'unimplemented|never implemented' docs/specs/2026-08-03-amendment-01-timescale-copy-spool.md
```

**Context budget:** 5 files.
