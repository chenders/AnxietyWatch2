# Proposal: Record the prototype's hardening debt instead of carrying it in someone's head

This change records nine operational gaps found while debugging the 2026-10-30 collector
outage. None of them is being fixed here, and none should be treated as urgent.

## Why this is a backlog and not a fire

AnxietyWatch2 is a prototype with one user, running inside a private LAN. The threat model
that makes several of these serious — a stolen box, a hostile network segment, a
multi-tenant host, an auditor — is not the one this system is in. Surfacing them as
blockers spends the owner's attention on risk he has already priced, which is its own cost.

So they are written down, with enough measurement that whoever picks one up does not have to
re-derive it, and left closed.

**What does not belong here.** Data loss and data misattribution are still raised
immediately: a dead collector, a type id whose meaning changed under stored rows, a bound
that reads healthy on silence. Those corrupt the record itself rather than exposing it, and
the record is the product. The dividing line is whether the defect makes the data *wrong*
or merely makes it *reachable*.

## What was found

**1. The repository decryption key sits on the device it protects.** `repo1-cipher-pass` is
a plaintext line in `/etc/pgbackrest/pgbackrest.conf`, on the same filesystem as the repo,
with copies in three `.rekey-*` siblings. Backups are `aes-256-cbc` client-side encrypted,
so a copy without the passphrase is not a copy — which means the count of *recoverable*
copies after that filesystem is lost is zero rather than three. Mitigating: the key never
leaves the LAN, and the box is physically in the owner's home.

**2. `data_checksums = off`,** with **334 unsafe shutdowns across 1,947 power-on hours**. The
drive itself is healthy — 0% wear, 100% spare, zero media errors — so the realistic loss mode
is page corruption rather than device death, and that is precisely the mode pgBackRest cannot
detect without checksums: a corrupt page is backed up faithfully into every generation until
the last good full ages out. Enabling it needs `pg_checksums --enable` offline, which is
~5–15 minutes of monitoring blackout at today's 46 GB and grows linearly — about 1–2 hours a
year from now. That trade is the owner's to make, and it gets worse by waiting.

**3. All storage tiers share one device.** Live cluster (~47 GB), pgBackRest repo (~18 GB)
and the cold export directory (~19 GB) are all on `/dev/nvme0n1p1`. One filesystem, one
journal, one corruption event. Amendment 05 §4 already lists off-box backup as
specified-and-unimplemented, so this is a known-open item rather than a discovery.
Constraint for whoever takes it: **LAN-only, free software**. The obvious candidates are the
Pi's own SD card (`mmcblk0p2`, ~109 GB free — same chassis, so it bounds device failure only)
and a pull-based repo on a LAN host.

**4. Backup timers can fire against a 1970 clock.** The RTC has a flat battery and the kernel
logs `setting system clock to 1970-01-01T00:00:09 UTC` on every boot. All three backup timers
are `Persistent=true` and ordered only `After=postgresql@16-main.service` — **no
`After=time-sync.target`** — so a cold boot can mint a backup label that sorts before every
real one, which interacts badly with `retention-full=4` and breaks the 45-day staleness
arithmetic.

**5. The restore-verification alert has no reader.** `verify-restore-alert.service` touches
`/var/lib/vitals/restore-verify.alert` and writes one journal line; the only other reference
to that path is the `rm -f` that clears it on success. Nothing notifies anyone. It is the
monitor guarding the only evidence that the backups restore — and
`verify-restore.timer` is monthly against a 45-day threshold, so a failure can sit up to 45
days before firing into a void.

**6. `air11_admin` is half-applied, and the missing half is the enforcement.** Both tables,
the index and two check constraints exist; **zero of three functions and zero of two triggers**
do. The audit table is documented as append-only "enforced by the database", and the triggers
that reject `UPDATE`/`DELETE`/`TRUNCATE` are simply absent — since 2026-09-13. Completing it
is blocked on a design question, not on typing: the script ends with
`REVOKE INSERT, UPDATE, DELETE ... FROM vitals`, but `vitals` **owns** those tables, so a
`SECURITY DEFINER` function created by `vitals` would also lack `INSERT`, leaving no path to
append an audit row at all. The privilege separation needs a privileged owner distinct from
the app role.

**7. Its verify script asserts a mutable seed value.** `verify/air11_admin.sql` requires
`opted_in = FALSE`, which stopped being true when the owner opted in on 2026-09-13. A verify
script should assert structure, not state that legitimately changes.

**8. The emay collector has no source.** `/usr/local/bin/emay-collector` is a stripped ARM64
binary dated 2026-09-25, built by neither this repo nor gen-1, and `vitals-deploy` does not
install it. `emay-spike` is a different program. It also crash-loops: it names its blocker
correctly, exits 70, and `Restart=always`/`RestartSec=20` has reached **NRestarts > 12,000**,
during which `systemctl is-active` reports `active`. A collector feeding the clinical record
that cannot be rebuilt from source is the gap; the crash-loop is a symptom.

**9. An orphaned state file misreports a 2.8 GB backlog.** `/var/lib/vitals/spool/.drain-health.json`
is dated 2026-09-25 and claims `available: false`, 154,526 pending segments, 2.8 GB, and
`last_error: "PostgreSQL blocked drain"`. Reality: the drain is active, **zero segments on
disk**, spool 15 MB. Nothing in either repository reads or writes it — its keys appear in no
source file — so it feeds nothing, but a human who finds it will believe it.

## Shape of the work

Each item is independent and separately deployable. Nothing here blocks anything else, and
nothing here is a prerequisite for `sample-type-resolution`. Items 4, 5, 7 and 9 are small
and self-contained; 1, 2, 3, 6 and 8 each carry a judgement the owner has to make, and the
tasks say which.
