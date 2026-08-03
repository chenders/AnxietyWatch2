# Amendment 02 — Data integrity, provenance, and encryption at rest

**Date:** 2026-08-03
**Status:** Proposed
**Amends:** design §2 (the core), amendment 01 §2 (spool), T-C1, T-D3, T-D4
**Adds tasks:** T-C4 (LUKS) · T-C5 (audit trail + roles) · T-C6 (reserve schema) · T-D7 (signed ingest) · T-H1 (HSM)

Three requested properties, which are three different threat models. Treating them as one produces
security theatre, so each is stated with what it actually defends against and — more importantly —
what it does not.

---

## 0. Threat model, stated first

| Threat | In scope? |
|---|---|
| A buggy or compromised collector writing wrong samples | **Yes** — §2 signed ingest |
| Accidental writes: a stray `psql`, a test script, demo data | **Yes** — §2 + §3 |
| Answering "is this reading real?" years later, in a clinical context | **Yes** — §2 |
| NVMe removed and read on another machine; drive RMA or disposal | **Yes** — §4 LUKS |
| Backup media read by someone else | Already covered — pgBackRest AES-256-CBC |
| Root **extracting** the signing key to forge elsewhere or later | **Yes, with an HSM** — §5 |
| Root **rewriting or deleting** existing history undetectably | **Yes, with an HSM monotonic counter** — §5.2 |
| Root obtained by **tampering with the boot chain** | **Yes, with PCR sealing** — §5.3 |
| Root on a **running, correctly-booted** box fabricating *new* signed rows | **No.** The key stays in the chip, but the signing oracle is reachable. |
| **The owner deciding to correct a row by hand** | **No, and deliberately so** — see §1. Changes are allowed; unrecorded changes are not. |

An earlier draft of this section said root was simply "not defensible." That was wrong, and lazily
so — it collapsed "cannot prevent everything" into "cannot prevent anything." With a hardware
security module the boundary moves in three specific, valuable places, detailed in §5. The honest
summary:

> An HSM converts **"root can silently rewrite history"** into **"root can add new signed rows, but
> cannot alter or delete existing ones undetectably, and cannot take the key anywhere else."**

For a clinical record that difference is most of the value. Prevention where possible, detection
where not, and never confuse the two.

---

## 1. Mutable, but auditable — hash-chained audit trail

The design has said `samples` is append-only since the first draft. **Nothing enforces it.** It is a
sentence in `AGENTS.md` and a Copilot review rule — exactly the "documented rather than structural"
pattern this project exists to eliminate.

The first draft of this amendment proposed a blanket `REVOKE UPDATE, DELETE`. That is rejected:
correcting a genuinely wrong row must remain possible, and a rule that forces you to a superuser
shell to fix a typo produces *less* auditability, not more, because the fix then happens outside any
recorded path.

**Changes are allowed; unrecorded changes are not.**

### 1.1 Method

Trigger-based audit into a single JSONB log — the well-established Postgres pattern — with a hash
chain over the entries to make it tamper-evident.

**Triggers rather than [pgaudit](https://www.pgaudit.org/) alone**, for two specific reasons.
pgaudit logs *statements*, not row values, so it cannot tell you what a value was before it changed.
And its writes go through the standard logging facility, which is **best-effort and not
transactional** — an entry is not flushed synchronously with the transaction that produced it, so a
crash can lose the record of a change that committed. For a clinical record that is the wrong
guarantee. pgaudit is still worth enabling *alongside* for connection and DDL logging, where
statement-level detail is what you want.

```sql
CREATE TABLE audit_log (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    table_name  TEXT        NOT NULL,
    operation   TEXT        NOT NULL CHECK (operation IN ('UPDATE','DELETE')),
    row_key     JSONB       NOT NULL,     -- the primary key of the affected row
    old_value   JSONB       NOT NULL,
    new_value   JSONB,                    -- NULL for DELETE
    changed_by  TEXT        NOT NULL DEFAULT session_user,
    client_addr INET                DEFAULT inet_client_addr(),
    txid        BIGINT      NOT NULL DEFAULT txid_current(),
    changed_at  TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    prev_hash   BYTEA,                    -- NULL only for the genesis row
    entry_hash  BYTEA       NOT NULL      -- sha256(prev_hash || canonical(this entry))
);
```

Each entry hashes the previous entry's hash together with its own canonical content. Altering,
deleting or reordering any entry breaks the chain from that point forward, and the break is
detectable by a single sequential scan.

`INSERT`s are deliberately **not** audited: they are the normal path, they are already covered by the
signed-ingest provenance in §2, and logging every sample twice would double the write volume of the
busiest table in the system.

### 1.2 The audit log is the thing that stays append-only

```sql
REVOKE UPDATE, DELETE, TRUNCATE ON audit_log FROM PUBLIC, vitals_writer, vitals_reader;
```

The protection moves one level up: `samples` rows may be corrected, but the *record* of the
correction cannot be quietly removed. A superuser can still bypass this — the hash chain is what
makes doing so visible afterwards, which is the achievable property.

### 1.3 Verification

`vitals-verify-integrity` (§3) walks the chain and reports the first broken link. This is the same
discipline as the restore check: a guarantee nobody verifies is decoration.

**Alternative considered:** [pgMemento](https://github.com/pgMemento/pgMemento) — transaction-based
logging with schema versioning and selective rollback. More capable, and the right answer for a
multi-user system with complex correction workflows. Rejected here as disproportionate: it logs
deltas per transaction with DDL versioning, and this is a single-user appliance where corrections are
rare and the requirement is "prove nothing changed silently," not "roll back transaction 41,203."

### 1.4 Role separation

| Role | Grants on `samples` | Used by |
|---|---|---|
| `vitals_writer` | `INSERT`, `UPDATE`, `DELETE` (all audited by trigger) | `vitals-spool-drain`, corrections |
| `vitals_reader` | `SELECT` only | API, admin console, analysis, panel |
| `vitals_owner` | DDL; no routine login | sqitch deploys |

Today a single `vitals` role does everything. A read-only API cannot leak write access it does not
have.

Retention is unaffected: TimescaleDB's `drop_chunks` operates at the chunk level as the table owner
and does not fire row triggers — which is correct. Dropping a whole expired chunk by policy is not a
correction and should not fill the audit log with two years of rows.


---

## 2. Signed ingest — provenance that survives

### 2.1 Where the signature goes

**Not in the spool.** The spool is raw PostgreSQL binary `COPY` bytes, and its value is that it needs
no framing (amendment 01 §2). Adding a signature envelope would reintroduce exactly the
translation layer that amendment removed.

Signatures live at the **`vitalsd` ingest boundary** instead:

```
collector ──sign(batch)──▶ vitalsd ──verify──▶ spool (plain COPY bytes)
                              │
                              └──▶ ingest_batch  (producer, signature, range, count)
```

1. Each producer holds an **Ed25519** keypair.
2. A producer signs a batch — the canonical serialisation of its samples, not each sample.
3. `vitalsd` verifies before anything is written. **A batch that fails verification is rejected with
   a named reason and quarantined; it never reaches the spool.**
4. The signature, producer id, sample count and time range are recorded in `ingest_batch`.

Per-batch, not per-sample: Ed25519 verification is ~50 µs, batches flush every 250 ms, so this is
roughly 4 verifications per second per producer — unmeasurable. Per-sample would add 64 bytes to
every row for no additional guarantee.

### 2.2 Schema

```sql
CREATE TABLE ingest_batch (
    id           BIGGENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    producer     TEXT        NOT NULL REFERENCES vitals_producer(name),
    signature    BYTEA       NOT NULL,          -- Ed25519 over the canonical batch
    digest       BYTEA       NOT NULL,          -- BLAKE3 of the canonical batch
    sample_count INTEGER     NOT NULL CHECK (sample_count > 0),
    ts_first     TIMESTAMPTZ NOT NULL,
    ts_last      TIMESTAMPTZ NOT NULL,
    received_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (ts_first <= ts_last)
);

CREATE TABLE vitals_producer (
    name       TEXT PRIMARY KEY,
    public_key BYTEA NOT NULL,       -- Ed25519, 32 bytes
    added_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);
```

`samples` gains `batch_id BIGINT REFERENCES ingest_batch(id)`. **Nullable, deliberately** — a NULL
`batch_id` means "this row did not arrive through the signed path," which is precisely the signal
worth having. It is not an error state to be hidden; it is the detection mechanism. The admin
console surfaces the unsigned-row count as a first-class number, and a nonzero value that you did not
expect is a finding.

This column is being added at the same time as the RR primary-key fix from the multi-model review, so
it costs one schema change rather than two.

### 2.3 Key storage

Private keys via **systemd credentials** (`LoadCredentialEncrypted=`), which on a TPM-less host
encrypts against `/var/lib/systemd/credential.secret`, readable only by root and only on this
machine. The key is decrypted into the unit's private `/run` credential directory and is not visible
to other services or in the process environment.

Public keys live in `vitals_producer`. Key rotation is an `INSERT` of a new producer row plus
`revoked_at` on the old — historical batches remain verifiable against the key that signed them,
which is the whole point.

### 2.4 What this actually buys

**It buys:** a permanent, checkable answer to "did this reading come from the BP cuff collector, or
from something else?" A verification pass over any date range re-checks every batch signature against
the recorded public key and reports mismatches, unsigned rows, and batches whose digest no longer
matches their samples.

**It does not buy:** protection from local root, who can read the signing keys and produce valid
signatures. Stated plainly so nobody later mistakes this for tamper-proofing.

The realistic failure it catches is the mundane one: a test harness, a demo seeder, or a
half-finished import writing into the production store and later being indistinguishable from real
measurements. v1 had a `DemoSeeder`. This makes that class of confusion impossible to sustain.

---

## 3. Verification is scheduled, not assumed

A signature nobody checks is decoration. `vitals-verify-integrity`, monthly via `pg_cron`, over the
trailing window:

1. Re-derive each batch digest from its samples; compare to `ingest_batch.digest`.
2. Verify each signature against the producer's public key as of that batch.
3. Count `samples` rows with `batch_id IS NULL`.
4. Write a JSON result next to `restore-verify.json`, and **exit nonzero on any failure**.

Same shape as the restore verification: it is only real if it can fail, and it must be able to say
which batch and which producer.

---

## 4. Encryption at rest — and the failure mode it introduces

### 4.1 Read this before enabling it

**Encryption introduces a way for the alarm not to run.** If the LUKS volume cannot unlock — wrong
keyfile, corrupted header, failed SD card holding the key — the Pi boots without `PGDATA`, without
the spool, and without the collectors' state directory. On a system whose purpose is to make a noise
at 3 a.m., a silent boot into a non-monitoring state is a worse outcome than an unencrypted disk.

Therefore, non-negotiable if §4.2 is adopted:

- The unlock is a **systemd dependency of the monitoring stack**, so a failure is a failed unit and
  not a quiet degradation.
- A failed unlock triggers the **audible alarm** and an APNs push. The one thing that must work when
  storage does not is the speaker.
- `klaxond` must **not** depend on the encrypted volume — its binary, config and self-test state live
  on the unencrypted boot volume, so it can still sound when nothing else works.

That last point is a design change: `klaxond` currently shares `StateDirectory=vitals` with
everything else.

### 4.2 LUKS2 on the data volume

One encrypted volume covering `PGDATA`, the WAL, the spool, and the pgBackRest repository — rather
than column-level `pgcrypto`, which would break indexing and aggregation on exactly the columns that
need both.

- **LUKS2**, `aes-xts-plain64`, 512-bit key. The Pi 5's Cortex-A76 has ARMv8 crypto extensions, so
  AES is hardware-accelerated; measure the actual overhead in T-C4 rather than assuming it is free.
- Boot `/` and `/boot/firmware` stay unencrypted — they hold no health data, and keeping them
  readable means a failed unlock still boots far enough to alarm and be diagnosed over SSH.

### 4.3 Unlock strategy, honestly compared

| Option | Protects against | Does **not** protect against | Cost |
|---|---|---|---|
| **A. Keyfile on the boot microSD** | NVMe pulled and read elsewhere; drive RMA; disposal | Theft of the whole Pi | ~1 hour |
| **B. External TPM (LetsTrust / Infineon on GPIO) + PCR sealing** | The above, **plus** whole-unit theft, plus boot tampering | A powered-on running machine | ~$25 + secure-boot config |
| **C. Clevis/Tang network-bound** | The above, if the tang server is unreachable to a thief | Anyone who steals both machines | Needs a second always-on host |

**Recommendation: A now, B as an upgrade.** Option A is honest about what it does, takes an hour, and
covers the realistic risks — a disk being returned, discarded, or removed. Option C is a poor fit:
megadude is being decommissioned, so there is no second always-on host to run tang, and adding one to
enable disk unlock inverts the project's own "the Pi must not depend on another machine" principle.

Option B is the genuinely strong answer and worth doing once the system is otherwise stable. It is
listed as a follow-up rather than a prerequisite because adding a boot-time hardware dependency to a
life-safety device before the device works is the wrong order.

---

## 5. Hardware security module — what it actually moves

The Pi has a GPIO header and USB. Both take a security element, and doing so changes the threat model
rather than decorating it.

### 5.1 Key non-extraction

The Ed25519 private key is generated **inside** the chip and never leaves it. Signing is a request:
the host sends a digest, the chip returns a signature.

Root can therefore *use* the key while the box runs. Root cannot *take* it. That removes:

- forging signatures on another machine, at leisure, with better tooling;
- forging anything after the module is removed, revoked, or the box is powered off;
- a stolen NVMe yielding any signing capability at all — the disk holds ciphertext and no key.

### 5.2 Monotonic counters — the mechanism that catches history rewriting

This is the part the earlier draft missed. A TPM 2.0 or secure element exposes counters held in
internal NV storage that **can only be incremented, never decremented or set**, and that cannot be
manipulated from outside the chip.

Bind one to ingest: every `ingest_batch` row carries the counter value at signing time, and the
counter is included in the signed digest.

| Attack | Result |
|---|---|
| Delete a batch | Gap in the counter sequence. Root cannot renumber the survivors — signatures cover the counter value. |
| Reorder batches | Counter order contradicts `received_at` order. |
| Backdate fabricated history | Would need a counter value below the current one. The chip will not issue it. |
| Replay an old batch | Duplicate counter value against a monotonic sequence. |

Root can still append genuinely-new signed rows going forward. Root cannot make the past look
different. That is precisely the property a clinical record needs, and it is achievable.

This is not novel — it is the construction in *Continuous Tamper-proof Logging Using TPM 2.0* (CMU,
TRUST 2014), whose stated result is that TPM-backed logging detects tampering by adversaries **with
privileged access on the machine generating the logs**.

### 5.3 PCR sealing — closing the offline-tamper path

Sealing the LUKS key and the signing key's authorisation to TPM PCRs means they are released only
when the measured boot state — firmware, bootloader, kernel, initramfs, command line — matches what
was sealed. A modified kernel or an initramfs with an added shell does not get the key.

This closes the realistic route to root on a device you physically hold: boot it into something you
control. It also replaces §4.3 option A entirely, so the disk-encryption and data-provenance problems
are solved by the same component.

TPM 2.0 additionally locks after 32 authorisation failures, forgetting one per 10 minutes, which
makes brute-forcing the authorisation value impractical.

### 5.4 Options

| Module | Interface | Gives | Cost |
|---|---|---|---|
| **[Zymbit ZYMKEY 5](https://www.zymbit.com/zymkey5/)** | GPIO / I2C, Pi-specific | 640 key slots, secure key generation and storage, filesystem encryption, **and two independent perimeter circuits with configurable policy including key destruction on breach** | ~$100 |
| **TPM 2.0 module** (Infineon SLB967x, LetsTrust) | GPIO / SPI | PCR sealing, monotonic counters, NV storage, attestation, command auditing | ~$25 |
| **ATECC608B breakout** | I2C | Key storage, ECDSA, monotonic counters | ~$5 |

**Recommendation: ZYMKEY 5.** It is the only option that does all three jobs — key custody, tamper
response, and the LUKS unattended-unlock problem from §4.3 — with one part designed for this board.
The physical tamper response matters more here than it would elsewhere: this is a device sitting
beside a bed, and a perimeter breach zeroising keys is a meaningful control rather than a datasheet
bullet.

A TPM 2.0 module at a quarter of the price gets sealing and counters but no tamper detection and no
Pi-specific integration. The ATECC608B is the minimum that delivers §5.1 and §5.2 and nothing else.

### 5.5 Honest limits

- A running, correctly-booted box with root can request signatures on fabricated data. The key is
  safe; the oracle is not. Rate-limiting and logging every signing request narrows this but does not
  close it.
- Plaintext on a mounted volume is readable by root. Encryption at rest protects the disk, not the
  running system.
- Tamper response protects against opening the case, not against someone using the machine.

### 5.6 Effect on sequencing

An HSM is **hardware that must be present before keys are generated**, because the entire point is
that keys are born inside it and never exist outside. Generating software keys now and "moving them
to the HSM later" is not possible — it would mean the key had once existed in software, which
forfeits §5.1 permanently.

So: if an HSM is wanted, order it before T-D7, and generate no production signing keys until it
arrives. The audit trail (§1) and role separation are unaffected and can proceed either way.

## 6. What this does not change

- The spool stays raw COPY bytes. Signatures are at the ingest boundary, not in the format.
- `samples` stays append-only — now enforced by grants rather than by convention.
- The registry remains the single source of truth for sources, types and units.
- No change to the detection or alarm logic.


## 7. Can this be deferred to the end?

Asked directly, and the answer differs per item. The dividing line is whether deferring costs more
than doing it now.

| Item | Defer to the end? | Why |
|---|---|---|
| Role separation (§1.4) | **Yes** | Grants are independent of data. Same effort whenever. |
| Audit triggers + hash chain (§1) | **Yes** | Triggers attach to an existing table. Only caveat: rows changed before the trigger exists have no audit entry, so the chain starts from installation, not from the first sample. |
| Signed-ingest *logic* (§2.1, §2.3) | **Yes** | Verification code and key handling can land any time. |
| Monthly integrity verification (§3) | **Yes** | It verifies whatever exists when it runs. |
| **LUKS (§4)** | **No — do it before data lands** | Encrypting in place afterwards means either `cryptsetup reencrypt` on a live database (slow, risky, and the failure mode is an unbootable box) or a full dump/restore with downtime. On an empty disk it is a partition, a format and an hour. This is the single largest defer-cost in the list. |
| **The `batch_id` and audit columns (§2.2)** | **No — reserve them now** | Amendment 01 §2.3: binary `COPY` freezes column count and order in every buffered row. A later schema change with a non-empty spool orphans those rows with no downgrade path — a finding from the multi-model review. Adding columns now, while the schema is empty and no spool exists, costs nothing. |

**So: reserve the schema now, defer the code.** Add `batch_id` and the audit tables in the same
sqitch change as the RR primary-key fix, and get LUKS in place before Stage 3 writes anything worth
keeping. Everything else — triggers, signing, verification — can be built at the end without penalty.

The one honest caveat on deferring the audit trigger: it cannot retroactively record changes made
before it existed. If Stages 3–5 involve hand-correcting data during development, those corrections
are invisible. That argues for installing the trigger early even though the *rest* of the work waits,
since it is a few dozen lines and attaches to a table that already exists.

## 8. Task ordering

| Task | Tier | Host | Depends | Timing |
|---|---|---|---|---|
| **T-C4** — LUKS2, keyfile unlock, failed-unlock alarm path, measured overhead | implement-complex | pi | T-A1 | **Before Stage 3** |
| **T-C6** — reserve `batch_id`, `ingest_batch`, `vitals_producer`, `audit_log` in the schema | mechanical | pi | T-C1 | **With the RR primary-key fix** |
| **T-C5** — role separation + audit triggers + hash chain | implement-standard | pi | T-C6 | Early, cheap |
| **T-D7** — Ed25519 signing, `vitalsd` verification, integrity verification job | implement-complex | either | T-D4, T-C6, T-H1 | Deferrable to the end |
| **T-H1** — HSM: fit the module, generate keys **inside** it, bind the monotonic counter to ingest, seal the LUKS key to PCRs | implement-complex | pi | hardware | **Before T-D7 — see §5.6** |
