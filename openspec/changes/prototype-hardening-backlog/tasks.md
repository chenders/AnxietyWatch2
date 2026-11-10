# Tasks: Prototype hardening backlog

None of these is urgent. They are independent and separately deployable, in no required
order. Pick one up when it is convenient; the measurement is recorded so you do not have to
re-derive it.

Five carry a judgement only the owner can make, marked **owner call**.

- [ ] T-PHB-1 — Escrow the repository decryption key off the device · ops · pi · **owner call**
- [ ] T-PHB-2 — Enable `data_checksums` · ops · pi · **owner call**
- [ ] T-PHB-3 — Put one copy on a second device, LAN-only · ops · pi · **owner call**
- [ ] T-PHB-4 — Order the backup timers after time sync · implement-simple · pi
- [ ] T-PHB-5 — Give the restore-verification alert a reader · implement · pi
- [ ] T-PHB-6 — Complete `air11_admin`'s immutability enforcement · implement · pi · **owner call**
- [ ] T-PHB-7 — Make `verify/air11_admin.sql` assert structure, not state · implement-simple · host
- [ ] T-PHB-8 — Recover or replace the emay collector · implement · host · **owner call**
- [ ] T-PHB-9 — Delete the orphaned drain-health file · implement-simple · pi

---

## T-PHB-1 — Escrow the repository decryption key off the device

**Depends:** none · **Review required:** no · **Owner call**

**Reads:** `docs/specs/AMENDMENTS.md` (amendment 02, encryption at rest).

**Do:** `repo1-cipher-pass` is plaintext in `/etc/pgbackrest/pgbackrest.conf`, on the
filesystem it protects, with copies in three `.rekey-*` siblings. Copy the value into a
password manager alongside the cipher type, stanza name and PostgreSQL major version.

Capture the recovery metadata pgBackRest does not back up — the pgbackrest config, `/etc/fstab`,
`/boot/firmware/cmdline.txt` and `config.txt`, and the backup units — into an archive stored
outside the repository. **That archive contains the passphrase: it must never be written
anywhere under the git tree.** `.githooks/pre-push` and `scripts/check-public-info.py` exist
to catch exactly that, and they should not be the thing that catches it.

**Owner call:** whether to rekey. The procedure is known — `pgbackrest.conf.rekey-20261003T030130Z`
shows it has been done before — and rekeying invalidates recoverability of existing backups
under the old key, so it is a sequencing decision, not a cleanup.

**Verify:** no automatable check exists; the evidence is a password-manager entry. The one
thing a command can confirm is that no copy leaked into the tree:
```sh
uv run python3 scripts/check-public-info.py && uv run python3 scripts/check-public-info.py --self-test
```

**Context budget:** 1 file.

---

## T-PHB-2 — Enable `data_checksums`

**Depends:** T-PHB-3 (have a second copy before rewriting every page) · **Review required:** yes · **Owner call**

**Reads:** nothing in the repo; this is a cluster setting.

**Do:** `data_checksums = off` with 334 unsafe shutdowns in 1,947 power-on hours. The drive is
electrically healthy, so the realistic loss mode is page corruption — the one mode pgBackRest
cannot detect without checksums, which means a corrupt page is backed up faithfully into every
generation until the last good full ages out at four weeks. `pg_amcheck` passing is real but
weaker evidence than it feels: it checks structural consistency, not heap bit-rot.

PostgreSQL 16 has no online enable. `pg_checksums --enable` rewrites every page with the
cluster stopped.

**Owner call:** the monitoring blackout. **~5–15 minutes at today's 46 GB, growing linearly to
1–2 hours a year from now.** This is the only item here whose cost rises, and whoever defers it
should know they are choosing the larger window later. Weigh it against what the silence
detector and the klaxon do while the daemon is down.

**Verify:**
```sh
psql -U vitals -d vitals -h /var/run/postgresql -At -v ON_ERROR_STOP=1 \
  -c "SELECT 1/CASE WHEN current_setting('data_checksums') = 'on' THEN 1 ELSE 0 END;" \
&& sudo -n -u postgres pgbackrest --stanza=vitals check
```

**Context budget:** 1 file.

---

## T-PHB-3 — Put one copy on a second device, LAN-only

**Depends:** none · **Review required:** yes · **Owner call**

**Reads:** `scripts/pi/verify-restore.sh`, `deploy/verify-restore.service`,
`docs/specs/AMENDMENTS.md` (amendment 05 §4, "off-box backup", specified and unimplemented).

**Do:** The live cluster (~47 GB), the pgBackRest repo (~18 GB) and the cold export (~19 GB)
all sit on `/dev/nvme0n1p1`. Three tiers on one filesystem is one tier.

**Constraints: LAN-only, free software.** pgBackRest supports multiple repositories natively,
so this is configuration rather than new machinery, and the stanza is already encrypted so a
second repo holds only ciphertext.

Candidates, with what each actually buys:
- **`mmcblk0p2`** — the Pi's SD card, ~109 GB free, a genuinely separate device and controller.
  Bounds device failure and root-filesystem corruption; does **not** bound fire, theft or a PSU
  event. Note it is *not* idle: its sibling `mmcblk0p1` holds the live `/boot/firmware`, which
  is itself backed up nowhere. Use a managed repo with `retention-full=1`, never a recurring
  bulk copy — re-copying the repo nightly would be ~5.5 TB/year of writes against a card
  showing 46 GB lifetime.
- **A pull-based repo on a LAN host** — safer against a push failing silently when the target
  is asleep, since a sleeping target yields a stale copy rather than a failed backup. Needs a
  freshness monitor either way (T-PHB-5).

**`archive-async=y` must land before any second repo.** Without it, pgBackRest's own guidance
is that the other repositories can only get one WAL segment ahead of an unreachable one, and
WAL that cannot be pushed is not removed from `pg_wal`. Set `spool-path` and
`archive-push-queue-max` with it.

**Owner call:** which device, and whether buying an always-on LAN target is acceptable —
"free software" constrains software, not hardware.

**Verify:**
```sh
sudo -n -u postgres pgbackrest --stanza=vitals check \
&& sudo -n -u postgres pgbackrest --stanza=vitals --repo=2 verify \
&& test "$(findmnt -no SOURCE /mnt/sdbridge 2>/dev/null)" != ""
```

**Context budget:** 3 files.

---

## T-PHB-4 — Order the backup timers after time sync

**Depends:** none · **Review required:** yes

**Reads:** `deploy/verify-restore.service`, `deploy/verify-restore-staleness.service`,
`deploy/vitals-cold-export.service`.

**Do:** The RTC battery is flat and the kernel logs
`rpi_rtc: setting system clock to 1970-01-01T00:00:09 UTC` on every boot. All three backup
timers are `Persistent=true` and ordered only `After=postgresql@16-main.service`, so a cold
boot can fire a backup against a 1970 clock — minting a label that sorts before every real
backup, interacting badly with `retention-full=4`, and breaking the 45-day staleness
arithmetic.

Add `After=time-sync.target` and `Wants=time-sync.target` to each. Capture the change in
`deploy/` rather than editing only on the box, since those units are now repo-managed.

**Verify:**
```sh
for u in verify-restore verify-restore-staleness vitals-cold-export; do
  grep -q 'After=.*time-sync.target' "deploy/$u.service" || exit 1
  grep -q 'Wants=.*time-sync.target' "deploy/$u.service" || exit 1
done
```

**Context budget:** 3 files.

---

## T-PHB-5 — Give the restore-verification alert a reader

**Depends:** none · **Review required:** yes

**Reads:** `deploy/verify-restore-alert.service`, `scripts/pi/verify-restore.sh`,
`crates/vitals-console/src/bin/silence-detector.rs` (how `notification_events` is written).

**Do:** `verify-restore-alert.service` touches `/var/lib/vitals/restore-verify.alert` and
writes one journal line. The only other reference to that path is the `rm -f` clearing it on
success — so nothing notifies anyone. It guards the only evidence that the backups restore,
and `verify-restore.timer` is monthly against a 45-day threshold, so a failure can sit up to
45 days before firing into a void.

Add a check that asserts the alert file's absence and every repo's newest-backup age, and
route it to `notification_events` plus a failed unit. **Not the klaxon** — AGENTS.md forbids
an infrastructure fault firing the alarm tone. Run it in waking hours; an alert at 03:05 is
an alert read at noon.

Read `notification_events`' actual DDL before writing to it.

**Verify:**
```sh
psql -U vitals -d vitals -h /var/run/postgresql -c '\d notification_events' >/dev/null \
&& bash -n /usr/local/bin/backup-offbox-check.sh \
&& sudo -n -u postgres /usr/local/bin/backup-offbox-check.sh
```

**Context budget:** 4 files.

---

## T-PHB-6 — Complete `air11_admin`'s immutability enforcement

**Depends:** T-PHB-7 · **Review required:** yes · **Owner call**

**Reads:** `db/deploy/air11_admin.sql`, `db/verify/air11_admin.sql`.

**Do:** `air11_integration_config` and `air11_admin_audit` exist with their index and two check
constraints. **Zero of three functions and zero of two triggers exist.** The audit table is
documented as append-only "enforced by the database", and the triggers rejecting
`UPDATE`/`DELETE`/`TRUNCATE` have been absent since 2026-09-13.

Deploying the rest as written does not work. The script ends with
`REVOKE INSERT, UPDATE, DELETE ... FROM vitals`, but `vitals` **owns** both tables, so a
`SECURITY DEFINER` function created by `vitals` would also lack `INSERT` — leaving no path to
append an audit row at all. The design wants a privileged owner distinct from the app role; it
currently has one role as both.

**Owner call:** which role owns these two tables. Options: transfer them to `postgres` and
deploy as `postgres` so `SECURITY DEFINER` means something; or drop the `REVOKE`s and accept
that the app role can write its own audit log directly. The second is simpler and weaker, and
for a single-user prototype it may be the right trade — say which was chosen and why.

**Verify:**
```sh
psql -U vitals -d vitals -h /var/run/postgresql -At -v ON_ERROR_STOP=1 -c "
SELECT 1/CASE WHEN (SELECT count(*) FROM pg_proc
                     WHERE proname IN ('set_air11_opt_in','append_air11_admin_audit',
                                       'reject_air11_admin_audit_mutation')) = 3
              AND (SELECT count(*) FROM pg_trigger
                     WHERE tgname LIKE 'air11_admin_audit%') = 2
         THEN 1 ELSE 0 END;"
```

**Context budget:** 3 files.

---

## T-PHB-7 — Make `verify/air11_admin.sql` assert structure, not state

**Depends:** none · **Review required:** yes

**Reads:** `db/verify/air11_admin.sql`, `scripts/tests/test-sqitch-verify-asserts.sh`.

**Do:** It asserts `SELECT 1/count(*) FROM air11_integration_config WHERE singleton = TRUE AND
opted_in = FALSE` — a seed value that stopped being true when the owner opted in on 2026-09-13,
so the verify now fails by design on a correctly-deployed change. A verify script must assert
the schema, not data that legitimately changes.

Replace that clause with one asserting the singleton constraint exists, keeping the structural
assertions. The script is already in the asserting style that
`test-sqitch-verify-asserts.sh` requires, so keep it that way.

**Verify:**
```sh
bash scripts/tests/test-sqitch-verify-asserts.sh \
&& psql -U vitals -d vitals -h /var/run/postgresql -q -v ON_ERROR_STOP=1 \
     -f db/verify/air11_admin.sql
```

**Context budget:** 2 files.

---

## T-PHB-8 — Recover or replace the emay collector

**Depends:** none · **Review required:** yes · **Owner call**

**Reads:** `deploy/vitals-emay.service`, `deploy/README.md`, `crates/emay-spike/src/main.rs`,
`crates/vitals-decode/src/emay.rs`.

**Do:** `/usr/local/bin/emay-collector` is a stripped ARM64 binary dated 2026-09-25, built by
neither this repo nor gen-1, and `vitals-deploy` does not install it. `emay-spike` is a
different program and does not produce its log messages. **A collector feeding the clinical
record cannot be rebuilt or patched from source.**

It also crash-loops. It names its blocker correctly —
`EMAY device <addr> was not discovered before the 15s deadline` — then exits 70, and
`Restart=always` with `RestartSec=20` has reached NRestarts > 12,000. Between crashes
`systemctl is-active` reports `active`, so the unit looks healthy while collecting nothing:
§2.1's "running and useless is indistinguishable from working", live. A device that only
advertises while worn should report absence as a steady named state and back off.

`vitals-decode/src/emay.rs` already holds the decoder, so a Rust replacement would reuse the
hard part.

**Owner call:** write a replacement collector in this workspace, or retire the EMAY path.
The decision depends on whether the EMAY path is still needed.

**Verify:** once a replacement exists in-tree:
```sh
cargo build --workspace --locked \
&& test -f crates/vitals-collectors/src/bin/emay_collector.rs \
&& grep -q 'emay' scripts/install-air11-pi-artifact.sh
```

**Context budget:** 4 files.

---

## T-PHB-9 — Delete the orphaned drain-health file

**Depends:** none · **Review required:** no

**Reads:** `crates/vitalsd/src/main.rs` (`update_drain_health`, which reads the spool directory
and `.drain-status` — **not** this file).

**Do:** `/var/lib/vitals/spool/.drain-health.json` is dated 2026-09-25 and claims
`available: false`, 154,526 pending segments, 2.8 GB pending, `oldest_pending_age_secs: 719746`,
and `last_error: "PostgreSQL blocked drain"`. Reality: the drain is active, **zero segments on
disk**, spool 15 MB.

Nothing reads or writes it — its keys (`pending_segments`, `oldest_pending_age_secs`) appear in
no source file in either repository — so it feeds nothing. It is debris from a deleted code
path. The only cost is that a human who finds it will believe it, which is enough reason to
remove it.

Confirm no reader before deleting; if one is found, this becomes a staleness-check task
instead.

**Verify:**
```sh
! git grep -lE 'drain-health|pending_segments|oldest_pending_age_secs' -- 'crates/**' 'scripts/**' \
&& ! ssh "$PI_HOST" 'test -e /var/lib/vitals/spool/.drain-health.json'
```

**Context budget:** 2 files.
