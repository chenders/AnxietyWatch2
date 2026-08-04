# Tasks — Pi 5 Vitals Core (Stages 0–3)

> **For agentic workers:** each `### T-XX` block is a self-contained prompt. Paste one into a fresh
> agent. Read the `Reads:` list **plus `docs/specs/AMENDMENTS.md`, which is always in scope**.
> Acceptance is the `Verify` command's exit code — not your own assessment.
>
> **If a task brief contradicts an accepted amendment, the amendment wins.** Report the
> contradiction; do not implement either version silently.

**Goal:** A blood pressure reading taken on the A&D UA-651BLE lands in Postgres on the Pi with
correct provenance, recoverable from a verified backup.

**Architecture:** Collectors sign a batch and append it, already encoded as PostgreSQL binary `COPY`
rows, to a spool file. `vitals-spool-drain` streams the spool into Postgres through a private `TEMP`
staging table. Detection runs on in-memory ring buffers fed by `vitalsd`'s subscription fan-out, not
from the database. The life-safety path runs as static Rust binaries under systemd; Docker holds
nothing in this change.

**Tech stack:** Rust 2024 (pinned via `rust-toolchain.toml`) · cargo · PostgreSQL 16 + TimescaleDB ·
sqitch · pgBackRest · BlueZ via `bluer` · systemd · Python 3.12 (codegen only)

## Global constraints

- Host `$PI_HOST`, passwordless sudo. Hostname is `raspberrypi`; reach it as `the-pi`.
- Repo is authoritative for `vitals/registry.toml` and everything in `vitals/generated/`. Generated
  artifacts are committed, never regenerated ad hoc on the Pi.
- **UTC only.** `TIMESTAMPTZ` in Postgres with the cluster timezone pinned to UTC; epoch seconds in
  Rust. No local time at any layer below presentation.
- **Canonical unit per type**, converted once at the decoder.
- `samples` is **append-only in practice**: `INSERT` is the normal path, and any `UPDATE`/`DELETE`
  passes through the hash-chained audit trigger (amendment 02 §1).
- **Identity is `(source, type, session_id, seq)`.** `timestamp` is best-estimate event time and is
  **not** part of the key. Never key on packet arrival time.
- Rust, `cargo clippy --all-targets` clean at the workspace's `deny` levels. A warning is a failure.
- Decoder crates are `#![no_std]` and `#![forbid(unsafe_code)]`. Every byte-decoding function is
  pure: `(&[u8]) -> Option<T>`. No allocation, no I/O, no state.
- **Every alarm output is provably OFF from power-on until `klaxond` takes control**, in hardware.
- Commit after every task. Never commit to `main`; branch first.

---

## Task index

The conductor ticks a box **only after running that task's `Verify` command itself and seeing exit
0.** A worker's report is a claim; the exit code is the evidence. This is also what the herdr shell
pane renders (`grep -n '^- \[' tasks.md`).

- [ ] T-A1 — Migrate root filesystem to NVMe · human · pi
- [ ] T-A2 — Audio output path · human · pi
- [x] T-A3 — Bring up the Sena UD100-G03 · implement-standard · pi · **gate G1 owner**
- [x] T-A4 — Disable onboard Bluetooth · mechanical · pi · **blocked by G1**
- [ ] T-A5 — Watchdog, governor, update policy · mechanical · pi
- [ ] T-B1 — Eight-hour Polar H10 capture · implement-complex · pi · **review**
- [ ] T-C1 — PostgreSQL 16 bare metal on NVMe · implement-standard · pi · **blocked by G2**
- [ ] T-C2 — pgBackRest, three tiers · implement-standard · pi · **review**
- [ ] T-C3 — Scheduled restore verification · implement-complex · pi · **review · gate G3 owner**
- [x] T-D1 — Registry TOML and codegen · implement-standard · mac
- [x] T-D2 — IEEE-11073 SFLOAT decoder with fuzz target · implement-complex · either · **review**
- [x] T-D3 — COPY-binary encoder and spool · implement-complex · either · **review**
- [ ] T-D4 — vitalsd daemon and socket protocol · implement-complex · pi · **review**
- [ ] T-D5 — spool drainer · implement-complex · pi · **review · blocked by G3**
- [ ] T-D6 — bp-collector end to end · implement-complex · pi · **review · blocked by G3**

Parallelism: T-D1 and T-D2 have no hardware dependency and may run on separate workers concurrently
with all of Group A. T-B1 occupies a whole night — dispatch it the moment T-A4 is green and let the
other groups proceed around it.

---

## Group A — Hardware floor (Stage 0)

### T-A1 — Migrate root filesystem to NVMe

**Tier:** human · **Host:** pi · **Context budget:** ~2k · **Depends:** none · **Review required:** no
**Amends:** none

**Reads:** none.
**Modifies:** `/boot/firmware/cmdline.txt`, `/etc/fstab` on the-pi.

`BOOT_ORDER` is already `0xf461` (SD → NVMe → USB), so no EEPROM change is needed.

**Do:**

1. Power off. Install the M.2 HAT and NVMe SSD. Power on.
2. Confirm the drive enumerated:
   ```sh
   lsblk -o NAME,SIZE,TYPE | grep nvme
   sudo dmesg | grep -i nvme | tail -5
   ```
3. Partition and format:
   ```sh
   sudo parted /dev/nvme0n1 --script mklabel gpt mkpart primary ext4 0% 100%
   sudo mkfs.ext4 -L piroot /dev/nvme0n1p1
   ```
4. Copy the running root:
   ```sh
   sudo mkdir -p /mnt/nvme && sudo mount /dev/nvme0n1p1 /mnt/nvme
   sudo rsync -axHAWXS --numeric-ids --info=progress2 / /mnt/nvme
   ```
5. Point the boot at it. Get the new PARTUUID and edit both files:
   ```sh
   NEWUUID=$(sudo blkid -s PARTUUID -o value /dev/nvme0n1p1)
   sudo sed -i "s|root=PARTUUID=[^ ]*|root=PARTUUID=$NEWUUID|" /boot/firmware/cmdline.txt
   sudo sed -i "s|^PARTUUID=[^ ]*\s*/\s|PARTUUID=$NEWUUID  /  |" /mnt/nvme/etc/fstab
   ```
6. Add `noatime` to the root line in `/mnt/nvme/etc/fstab`.
7. `sudo reboot`. Wait 60 s, reconnect.

**Verify:** `ssh $PI_HOST 'findmnt -no SOURCE / | grep -q nvme && df -h / | tail -1'`
**Expect:** exit 0, root shown on `/dev/nvme0n1p1`.

**Commit:** nothing to commit — hardware state. Record the result in `## Implementation notes`.

---

### T-A2 — Audio output path

**Tier:** human · **Host:** pi · **Context budget:** ~2k · **Depends:** none · **Review required:** no
**Amends:** 03 (§4 alarm output path, §6 self-test by stable identity)

The Pi 5 has no analog audio. Today only `vc4hdmi0`/`vc4hdmi1` exist and no display is on HDMI, so
there is currently no way to make a sound. This task establishes the alarm's output path; `klaxond`
is Stage 5.

**Reads:** none.
**Creates:** `/etc/asound.conf` on the-pi.

**Do:**

1. Attach the USB DAC/amp and speaker. Confirm enumeration:
   ```sh
   aplay -l
   ```
   Note the card number of the USB device (call it `N`).
2. Make it the default sink — write `/etc/asound.conf`, substituting `N`:
   ```
   defaults.pcm.card N
   defaults.ctl.card N
   ```
3. Set volume to a known level and produce an audible tone:
   ```sh
   amixer -c N sset PCM 85% unmute 2>/dev/null || amixer -c N sset Speaker 85% unmute
   speaker-test -D default -t sine -f 1000 -l 1 -s 1
   ```
4. Confirm you heard it. This is the only step in this plan verified by ear.

**Verify:** `ssh $PI_HOST 'aplay -l | grep -viE "vc4hdmi" | grep -q "^card" && aplay -D default /usr/share/sounds/alsa/Front_Center.wav'`
**Expect:** exit 0, a non-HDMI card present, playback returns cleanly.

---

### T-A3 — Bring up the Sena UD100-G03

**Tier:** implement-standard · **Host:** pi · **Context budget:** ~3k · **Depends:** none
**Amends:** none
**Review required:** no

**GATE G1 owner.** T-A4 must not run until this task's verify exits 0.

**Reads:** none.

**Do:**

1. Plug in the Sena dongle. Confirm the kernel bound it:
   ```sh
   lsusb | grep -i "cambridge\|0a12\|0a5c"
   dmesg | grep -i "hci\|btusb" | tail -10
   hciconfig -a
   ```
   Expect a second controller. Onboard is UART (`hci0`); the Sena is USB.
2. Identify which index is the Sena — it is the one with `Bus: USB`:
   ```sh
   for h in /sys/class/bluetooth/hci*; do
     echo "$(basename $h): $(cat $h/device/../idVendor 2>/dev/null || echo uart)"
   done
   hciconfig -a | grep -E "^hci|Bus:"
   ```
3. Bring it up and scan on it explicitly:
   ```sh
   sudo hciconfig hci1 up
   sudo timeout 20 hcitool -i hci1 lescan --duplicates | head -20
   ```
4. Record the result — device count and the Sena's BD address — in `## Implementation notes`.

**Verify:** `ssh $PI_HOST 'hciconfig -a | grep -c "^hci" | grep -q 2 && hciconfig hci1 | grep -q "UP RUNNING"'`
**Expect:** exit 0, exactly two controllers, `hci1` up.

---

### T-A4 — Disable onboard Bluetooth

**Tier:** mechanical · **Host:** pi · **Context budget:** ~2k · **Depends:** T-A3 · **Review required:** no
**Amends:** none

**GATE G1.** Do not run this until T-A3's verify has exited 0. `dtoverlay=disable-bt` removes the
onboard controller; without a working Sena this leaves the box with no Bluetooth at all. **Dispatch
only on a green verify, never on a worker's report.**

Rationale: with one adapter, a collector binding the wrong radio stops being a possible bug.

**Modifies:** `/boot/firmware/config.txt` on the-pi.

**Do:**

1. Confirm the gate is actually green before touching anything:
   ```sh
   hciconfig hci1 | grep -q "UP RUNNING" || { echo "GATE G1 NOT MET - ABORT"; exit 1; }
   ```
2. Append to `/boot/firmware/config.txt` under `[all]`:
   ```
   dtoverlay=disable-bt
   ```
3. Disable the UART BT service so it does not hunt for a device that is gone:
   ```sh
   sudo systemctl disable hciuart
   ```
4. `sudo reboot`. Wait 60 s.

**Verify:** `ssh $PI_HOST 'hciconfig -a | grep -c "^hci" | grep -qx 1 && hciconfig -a | grep -q "Bus: USB"'`
**Expect:** exit 0, exactly one controller, and it is the USB one.

---

### T-A5 — Watchdog, governor, update policy

**Tier:** mechanical · **Host:** pi · **Context budget:** ~2k · **Depends:** T-A1 · **Review required:** no
**Amends:** 04 (§2 OOMScoreAdjust on safety units)

**Modifies:** `/boot/firmware/config.txt`, `/etc/systemd/system.conf`,
`/etc/apt/apt.conf.d/20auto-upgrades` on the-pi.
**Creates:** `/etc/systemd/system/cpu-performance.service` on the-pi.

**Do:**

1. Arm the BCM2712 hardware watchdog. Append to `/boot/firmware/config.txt`:
   ```
   dtparam=watchdog=on
   ```
2. Have systemd pet it. In `/etc/systemd/system.conf`:
   ```
   RuntimeWatchdogSec=15
   RebootWatchdogSec=2min
   ```
3. Pin the governor to `performance` — a collector that must not miss a connection interval should
   not wait on a frequency ramp, and there is 30+ °C of thermal headroom. Create
   `/etc/systemd/system/cpu-performance.service`:
   ```ini
   [Unit]
   Description=Pin CPU governor to performance
   After=multi-user.target

   [Service]
   Type=oneshot
   ExecStart=/bin/sh -c 'echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor'
   RemainAfterExit=yes

   [Install]
   WantedBy=multi-user.target
   ```
   Then `sudo systemctl enable --now cpu-performance`.
4. Stop unattended reboots. This box does not restart itself for a kernel patch mid-night. In
   `/etc/apt/apt.conf.d/20auto-upgrades`:
   ```
   APT::Periodic::Update-Package-Lists "0";
   APT::Periodic::Unattended-Upgrade "0";
   ```
5. `sudo reboot`. Wait 60 s.

**Verify:** `ssh $PI_HOST 'test -e /dev/watchdog && cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor | grep -qx performance && vcgencmd get_throttled | grep -qx "throttled=0x0"'`
**Expect:** exit 0.

---

## Group B — Thesis spike (Stage 1)

### T-B1 — Eight-hour Polar H10 capture

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~5k · **Depends:** T-A4
**Amends:** 06 (§1.2 seq scope, §3 bonded-but-unencrypted state) · 05 (§1.4 clock drift, §4.3 IRK pairing) · 04 (§1 RR beat-time reconstruction) · 03 (§1 tactile evidence)
**Review required:** yes

The entire project rests on the Pi holding a BLE connection all night. **Measure this before
building anything on top of it.** Deliberately placed before the storage and core work.

**Creates:** `crates/hr-spike/` (a throwaway binary crate), `tools/gap_histogram.py`

Both are committed. An earlier version put the histogram script at `~/vitals/spike/` — outside the
repo — while the Verify line depends on it and the commit block staged only `tasks.md`, so the
script the acceptance check runs was never version-controlled.

**Do:**

1. Install dependencies:
   ```sh
   sudo apt-get update && sudo apt-get install -y libdbus-1-dev bluez
   ```
2. Write `crates/hr-spike/src/main.rs`. Use the `bluer` crate for BlueZ over D-Bus, subscribe to
   Heart Rate Measurement `00002a37-0000-1000-8000-00805f9b34fb`, and append one line per
   notification to stdout: `<unix_epoch_seconds_float> <hr_bpm> <rr_ms,rr_ms,...>`.

   Decode `0x2A37` per the SIG spec — flags byte first:
   ```rust
   // byte 0 = flags: bit0 HR is uint16 (else uint8), bit3 energy present, bit4 RR present
   let flags = *buf.first()?;
   let mut off = 1usize;
   let hr: u16 = if flags & 0x01 != 0 {
       let v = u16::from_le_bytes(buf.get(off..off + 2)?.try_into().ok()?);
       off += 2;
       v
   } else {
       let v = u16::from(*buf.get(off)?);
       off += 1;
       v
   };
   if flags & 0x08 != 0 {
       buf.get(off..off + 2)?;   // energy expended, skipped -- but still bounds-checked
       off += 2;
   }
   // remaining bytes are RR intervals, uint16 LE, units of 1/1024 s
   let mut rr_ms = Vec::new();
   while let Some(chunk) = buf.get(off..off + 2) {
       let raw = u16::from_le_bytes(chunk.try_into().ok()?);
       rr_ms.push(f64::from(raw) * 1000.0 / 1024.0);
       off += 2;
   }
   ```
   `get()` rather than indexing — the crate denies `indexing_slicing`, and `buf` is
   attacker-controlled radio input. Returning `None` on a short buffer is the whole point: a
   truncated notification must be *absent*, never a partially-decoded reading.
3. Build with `cargo build --release -p hr-spike`.
4. Pair the H10 once, and **register auto-connect** (A05 §4.3, amendment 06 §3):
   ```sh
   # bluetoothctl's `scan on` emits nothing when its output is piped, and its
   # `connect` has been observed issuing no LE Create Connection at all. Use
   # btmgmt to discover and to register intent with the kernel.
   sudo btmgmt find                       # note the H10's address
   sudo bluetoothctl pair <H10_MAC>       # bonding stores the IRK
   sudo btmgmt add-device -a 2 -t 1 <H10_MAC>   # action 2 = auto-connect
   ```
   **`trust` is not sufficient and is not the mechanism.** A05 §4.3: a device implementing LE
   Privacy rotates its address and static-MAC trust then fails silently. Bonding stores the IRK;
   identify by IRK, not address. The kernel accept-list does the connecting, because a userspace
   poll cannot reliably catch a short advertising window.

   Distinguish three states, not two (amendment 06 §3): **device not found** (pairing fault),
   **out of range** (normal), and **bonded but `ServicesResolved` never fired within 10 s**
   (encryption could not resume — looks exactly like a quiet night, and is not one).
5. Run the capture overnight under systemd-run so it survives your SSH session:
   ```sh
   # --pipe, an ABSOLUTE ExecStart, and the redirect on systemd-run itself.
   # Without --pipe the child's stdout goes to the journal and the shell
   # redirect captures only "Running as unit: hrspike.service" -- an 8-hour
   # capture into a one-line file, which the Verify then happily reads.
   # A relative path resolves against the unit's WorkingDirectory, not yours.
   systemd-run --user --pipe --unit=hrspike --setenv=H10_MAC=<mac> \
     "$PWD/target/release/hr-spike" > ~/vitals/spike/night1.log
   ```
6. In the morning, compute the gap histogram with `tools/gap_histogram.py`: read the log, diff
   consecutive timestamps, and print count of gaps in each bucket `<2s, 2-10s, 10-60s, 60s-5m, >5m`,
   plus total duration, total RR count, and longest gap.
   Also report, per A05 §1.4 and §5:
   - **observed clock drift** — fit cumulative RR sum against host arrival time; a rate outside
     ±0.5% is a sensor or host clock fault, not normal. Nothing else in this system will measure it.
   - **Dropped-notification count**, not a `seq` gap count. A05 §1.3 offers `max(seq) - count(*)`
     as the lost-beat metric, but A05 §1.1 defines RR `seq` as the count of intervals *received*,
     which is dense by construction — so that expression is identically zero and cannot detect
     anything. The H10 transmits no beat counter (the same payload anonymity amendment 06 §1
     documents for the cuff). Estimate drops instead from the ratio of elapsed wall time to summed
     RR duration per connection, and report it as an estimate.
7. Record the histogram verbatim in `## Implementation notes`. **This number is the project's
   foundation — if the longest gap exceeds 5 minutes, stop and report before proceeding.**

**Verify:** `ssh $PI_HOST 'cd ~/AnxietyWatch2 && python3 tools/gap_histogram.py ~/vitals/spike/night1.log --require-duration-hours 8 --max-gap-seconds 300 --require-drift-report'`
**Expect:** exit 0. The script must exit nonzero if duration < 8 h, if longest gap > 300 s, or if
`--require-drift-report` was passed and no drift figure could be computed. A flag a script may
accept and ignore is a check that cannot fail.

**Commit:**
```sh
git add openspec/changes/pi5-vitals-core/tasks.md crates/hr-spike/ tools/gap_histogram.py
git commit -m "spike(T-B1): record 8-hour Polar H10 capture gap histogram"
```

---

## Group C — Storage floor (Stage 2)

**GATE G2:** all of Group C requires T-A1 green. Postgres must never initialize on the microSD.

### T-C1 — PostgreSQL 16 bare metal on NVMe

**Tier:** implement-standard · **Host:** pi · **Context budget:** ~4k · **Depends:** T-A1
**Amends:** 01 (§3 TimescaleDB) · 02 (§1.4 roles) · 04 (§2 memory budget)
**Review required:** no

Not in Docker, not on the microSD.

**Creates:** `/etc/postgresql/16/main/conf.d/vitals.conf` on the-pi.

**Do:**

1. Confirm the gate:
   ```sh
   findmnt -no SOURCE / | grep -q nvme || { echo "GATE G2 NOT MET - ABORT"; exit 1; }
   ```
2. Install:
   ```sh
   sudo apt-get install -y postgresql-16 postgresql-client-16
   ```
3. Confirm `PGDATA` is on NVMe (default `/var/lib/postgresql/16/main`, which now lives on NVMe since
   `/` moved):
   ```sh
   sudo -u postgres psql -tAc "SHOW data_directory" | xargs findmnt -no SOURCE -T
   ```
4. Tune for 8 GB RAM. Write `/etc/postgresql/16/main/conf.d/vitals.conf`:
   ```
   shared_buffers = 2GB
   effective_cache_size = 4GB
   work_mem = 32MB
   maintenance_work_mem = 512MB
   max_connections = 40
   random_page_cost = 1.1
   effective_io_concurrency = 200
   wal_level = replica
   archive_mode = on
   archive_command = 'pgbackrest --stanza=vitals archive-push %p'
   max_wal_size = 2GB
   min_wal_size = 512MB
   checkpoint_completion_target = 0.9
   ```
   `random_page_cost` and `effective_io_concurrency` are set for NVMe, not spinning media.
5. Create the role and database:
   ```sh
   sudo -u postgres createuser --createdb vitals
   sudo -u postgres createdb -O vitals vitals
   ```
6. **Install extensions** (see Amendment 01 §3). arm64/bookworm/PG16 packages are published:
   ```sh
   curl -s https://packagecloud.io/install/repositories/timescale/timescaledb/script.deb.sh | sudo bash
   sudo apt-get update
   sudo apt-get install -y timescaledb-2-postgresql-16 timescaledb-toolkit-postgresql-16 postgresql-16-cron
   ```
7. Append to `vitals.conf`:
   ```
   shared_preload_libraries = 'timescaledb,pg_stat_statements,pg_cron'
   cron.database_name = 'vitals'
   timezone = 'UTC'
   timescaledb.max_background_workers = 4
   ```
   Optionally run `sudo timescaledb-tune --quiet --yes` first — but it assumes it owns the machine,
   and it does not: `vitalsd`, the collectors, and the klaxon share this box. **Re-assert
   `shared_buffers = 2GB` after running it** and re-check `work_mem`.
8. `sudo systemctl restart postgresql`, then enable the extensions in the `vitals` database:
   ```sh
   sudo -u postgres psql -d vitals -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"
   sudo -u postgres psql -d vitals -c "CREATE EXTENSION IF NOT EXISTS timescaledb_toolkit;"
   sudo -u postgres psql -d vitals -c "CREATE EXTENSION IF NOT EXISTS pg_cron;"
   sudo -u postgres psql -d vitals -c "CREATE EXTENSION IF NOT EXISTS pg_stat_statements;"
   ```

**Verify:** `ssh $PI_HOST 'sudo -u postgres psql -tAc "SHOW shared_buffers" | grep -q 2GB && sudo -u postgres psql -tAc "SHOW data_directory" | xargs findmnt -no SOURCE -T | grep -q nvme && sudo -u postgres psql -d vitals -tAc "SELECT count(*) FROM pg_extension WHERE extname IN ((chr(39)||chr(39)))" >/dev/null && test $(sudo -u postgres psql -d vitals -tAc "SELECT count(*) FROM pg_extension WHERE extname = ANY(ARRAY[chr(116)||chr(105)||chr(109)||chr(101)||chr(115)||chr(99)||chr(97)||chr(108)||chr(101)||chr(100)||chr(98)])") -eq 1'`
**Expect:** exit 0 — `shared_buffers=2GB`, `PGDATA` on NVMe, `timescaledb` installed.

**Simpler equivalent** if the quoting above is awkward to run by hand:
```sh
ssh $PI_HOST 'sudo -u postgres psql -d vitals -tAc "SELECT extname FROM pg_extension ORDER BY 1" | tr "\n" " "'
```
Expect: `pg_cron pg_stat_statements plpgsql timescaledb timescaledb_toolkit`

---

### T-C2 — pgBackRest, three tiers

**Tier:** implement-standard · **Host:** pi · **Context budget:** ~4k · **Depends:** T-C1
**Amends:** 02 (§4 LUKS interaction)
**Review required:** yes

**Creates:** `/etc/pgbackrest/pgbackrest.conf`, two systemd timers on the-pi.

**Do:**

1. `sudo apt-get install -y pgbackrest`
2. Write `/etc/pgbackrest/pgbackrest.conf`. Until the 1 TB drive arrives, the repo lives on NVMe;
   moving it later is a `repo1-path` change plus one `pgbackrest stanza-upgrade`:
   ```ini
   [global]
   repo1-path=/var/lib/pgbackrest
   repo1-retention-full=4
   repo1-retention-diff=7
   repo1-cipher-type=aes-256-cbc
   repo1-cipher-pass=<generate with: openssl rand -base64 48>
   compress-type=zst
   compress-level=6
   process-max=4
   log-level-console=info
   log-level-file=detail
   start-fast=y

   [vitals]
   pg1-path=/var/lib/postgresql/16/main
   pg1-port=5432
   ```
   Store the cipher passphrase in your password manager. A backup you cannot decrypt is not a backup.
3. Create and check the stanza:
   ```sh
   sudo -u postgres pgbackrest --stanza=vitals stanza-create
   sudo -u postgres pgbackrest --stanza=vitals check
   ```
4. Take a first full backup:
   ```sh
   sudo -u postgres pgbackrest --stanza=vitals --type=full backup
   ```
5. Create `pgbackrest-full.timer` (weekly, Sun 03:00) and `pgbackrest-incr.timer` (daily, 03:00),
   each with a matching `.service` running
   `pgbackrest --stanza=vitals --type=full backup` / `--type=incr backup` as user `postgres`.
   Enable both.
6. Write the monthly cold export — tier 3 of the design's three tiers. A timer running
   `/usr/local/bin/vitals-cold-export.sh`, which writes
   `/var/lib/vitals/cold/YYYY-MM.ndjson.zst` containing every `samples` row as newline-delimited
   JSON plus a copy of `registry.toml`, and a one-page `README.md` describing the format. This tier
   exists to be readable with no Postgres, no pgBackRest, and no code from this repo.

**Scope note — tier 2 (off-box replication) is deferred**, not dropped: the 1 TB drive has not
arrived and no second location is chosen. Tiers 1 and 3 are both independently sufficient for
recovery in the meantime. Tier 2 becomes a `repo2-*` block in this same config file plus one
`pgbackrest stanza-upgrade`; it is tracked in `## Implementation notes` and must land before Stage 5
puts the klaxon into service.

**Verify:** `ssh $PI_HOST 'sudo -u postgres pgbackrest --stanza=vitals check && sudo -u postgres pgbackrest --stanza=vitals info | grep -q "status: ok"'`
**Expect:** exit 0, `status: ok`.

---

### T-C3 — Scheduled restore verification

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~4k · **Depends:** T-C2
**Amends:** 02 (§3 integrity verification)
**Review required:** yes

**GATE G3 owner.** No sample worth keeping is written until a restore has been demonstrated.

A backup that has never been restored is a hypothesis. This makes it a measurement.

**Creates:** `/usr/local/bin/verify-restore.sh`, `verify-restore.service`, `verify-restore.timer`
on the-pi.

**Do:**

1. Write `/usr/local/bin/verify-restore.sh`. It must:
   - Restore the latest backup into a scratch cluster at `/var/lib/postgresql/verify` on a
     throwaway port (5433), using `pgbackrest --stanza=vitals --delta restore`.
   - Start it, wait for readiness, then compare against production: for every table in
     `information_schema.tables` where `table_schema='public'`, compare `count(*)`.
   - Write a JSON result to `/var/lib/vitals/restore-verify.json`:
     `{"ts": <epoch>, "ok": true|false, "tables_checked": N, "mismatches": [...], "restore_seconds": N}`
   - Stop and remove the scratch cluster.
   - **Exit nonzero on any mismatch.** A verification that cannot fail verifies nothing.
2. Make it executable, run it once by hand, and confirm the JSON.
3. Create `verify-restore.timer` (monthly, first Sunday 04:00) and its `.service`. Enable both.
4. Wire the result into the health surface: `/var/lib/vitals/restore-verify.json` is read by the
   Stage 8 panel and by `vitalsd`'s health endpoint. Staleness beyond 45 days is itself a fault.

**Verify:** `ssh $PI_HOST 'sudo /usr/local/bin/verify-restore.sh && jq -e ".ok == true" /var/lib/vitals/restore-verify.json'`
**Expect:** exit 0, `ok: true`.

---

## Group D — vitals core (Stage 3)

### T-D1 — Registry TOML and codegen

**Tier:** implement-standard · **Host:** mac · **Context budget:** ~5k · **Depends:** none
**Amends:** 01 (§3.1 hypertable) · 02 (§2.2 reserved columns) · 04 (§1 identity, §8 clocks removed)
**Review required:** no

The keystone. One file declares every source, type, canonical unit, and valid range; three artifacts
are generated from it. `"polar_h10"` exists in exactly one place and is an integer everywhere else.

**Creates:** `vitals/registry.toml`, `vitals/tools/gen_registry.py`,
`crates/vitals-registry/src/generated.rs`, `vitals/generated/registry.sql`,
`vitals/generated/registry.json`

The Swift artifact is **dropped** — the iPhone is a thin client that reads the API, not a consumer
of the registry. Sqitch owns table *structure* (`db/deploy/`), so `registry.sql` emits reference-data
`INSERT`s only; two systems declaring the same table is how a migration silently diverges from the
schema it is migrating.

**Do:**

1. Write `vitals/registry.toml`. Ids are permanent — never reuse or renumber:
   ```toml
   [meta]
   version = 1

   [type.hr]
   id = 1; unit = "bpm";      range = [20.0, 250.0];  precision = 1.0
   [type.spo2]
   id = 2; unit = "percent";  range = [50.0, 100.0];  precision = 0.1
   [type.accel]
   id = 3; unit = "g";        range = [-16.0, 16.0];  precision = 0.001
   [type.hrv_rmssd]
   id = 4; unit = "ms";       range = [1.0, 500.0];   precision = 0.1
   [type.rr_interval]
   id = 5; unit = "seconds";  range = [0.2, 3.0];     precision = 0.001
   [type.bp_systolic]
   id = 6; unit = "mmHg";     range = [50.0, 260.0];  precision = 1.0
   [type.bp_diastolic]
   id = 7; unit = "mmHg";     range = [30.0, 160.0];  precision = 1.0
   [type.bp_map]
   id = 8; unit = "mmHg";     range = [40.0, 200.0];  precision = 1.0

   [source.emay]
   id = 0; transport = "ble"; provides = ["spo2", "hr"]
   [source.polar_h10]
   id = 1; transport = "ble"; service = "0x180D"; provides = ["hr", "rr_interval"]
   [source.healthkit]
   id = 2; transport = "push"; provides = ["hr", "hrv_rmssd"]
   [source.derived]
   id = 3; transport = "internal"; provides = ["hrv_rmssd"]
   [source.ad_ua651ble]
   id = 6; transport = "ble"; service = "0x1810"; indication = true
   provides = ["bp_systolic", "bp_diastolic", "bp_map", "hr"]
   ```
   `indication = true` records that `0x2A35` requires CCCD `0x0002`. Writing `0x0001` yields a
   successful subscription that delivers nothing.
2. Write `vitals/tools/gen_registry.py` — stdlib only, `tomllib`. It emits all three artifacts. Each
   generated file starts with a `GENERATED FROM registry.toml — DO NOT EDIT` banner in that file's
   comment syntax.
   - `crates/vitals-registry/src/generated.rs`: `#[repr(i32)]` enums `VitalsType` and `VitalsSource`
     with explicit discriminants (`Hr = 1`, `BpSystolic = 6`, `Emay = 0`, `PolarH10 = 1`,
     `AdUa651ble = 6`, …), plus `pub const fn range(self) -> (f64, f64)` and
     `pub const fn unit(self) -> &'static str`. `#![no_std]`, no allocation — this crate is a
     dependency of the decoders.
     A real enum rather than integer constants is the point: an `i32` that could hold any value is
     what lets a source id and a type id be transposed at a call site without the compiler noticing.
   - `registry.sql`: `INSERT`s into the `vitals_type` and `vitals_source` lookup tables. **Reference
     data only** — sqitch owns `CREATE TABLE`.
   - `registry.json`: the parsed structure verbatim, for the API layer.
3. Add a regeneration check so a hand-edited generated file cannot survive CI:
   ```sh
   python3 vitals/tools/gen_registry.py --check
   ```
   which regenerates to a temp dir and diffs, exiting nonzero on difference.

   **The check must fail when the generator emits nothing.** A `--check` that compares an empty
   generator run against its own empty output passes forever and proves nothing; assert each
   artifact is non-empty and contains at least one known discriminant before diffing.

**Verify:** `python3 vitals/tools/gen_registry.py --check && grep -q "BpSystolic = 6" crates/vitals-registry/src/generated.rs && cargo build -p vitals-registry`
**Expect:** exit 0.

**Commit:**
```sh
git add vitals/registry.toml vitals/tools/gen_registry.py vitals/generated/ crates/vitals-registry/
git commit -m "feat(T-D1): vitals registry TOML and codegen for Rust, SQL, JSON"
```

---

### T-D2 — IEEE-11073 SFLOAT decoder with fuzz target

**Tier:** implement-complex · **Host:** either · **Context budget:** ~4k · **Depends:** T-D1
**Amends:** 04 (§11 kani in addition to cargo-fuzz)
**Review required:** yes

Split out from the collector because it is the subtlest code in this change and the easiest to prove
correct: pure, no state, no I/O, one buffer in.

SFLOAT is **not** an IEEE-754 half float. It is 16-bit with a 4-bit signed exponent in the high
nibble and a 12-bit signed mantissa below, and it has reserved sentinels that must map to *absent* —
never to a number. A naive `_Float16` cast produces plausible-looking garbage, which is the worst
possible failure in a medical value: 118/74 and 141/92 are both believable.

**Creates:** `crates/vitals-decode/src/sfloat.rs`, `fuzz/fuzz_targets/fuzz_sfloat.rs`

**Do:**

1. Write `crates/vitals-decode/src/sfloat.rs`:
   ```rust
   /// Reserved mantissa values. These mean **absent**, never a number.
   const NAN_: u16 = 0x07FF;
   const NRES: u16 = 0x0800;
   const POS_INF: u16 = 0x07FE;
   const NEG_INF: u16 = 0x0802;
   const RESERVED: u16 = 0x0801;

   /// Decodes an IEEE-11073 SFLOAT.
   ///
   /// Returns `None` for NaN, NRes, ±INFINITY and Reserved — all of which mean
   /// the measurement is absent.
   pub const fn decode_sfloat(raw: u16) -> Option<f64> {
       let mantissa = raw & 0x0FFF;
       if matches!(mantissa, NAN_ | NRES | POS_INF | NEG_INF | RESERVED) {
           return None;
       }
       let exponent = ((raw >> 12) as i8) << 4 >> 4;          // sign-extend 4-bit
       let mantissa = ((mantissa as i16) << 4 >> 4) as i32;   // sign-extend 12-bit
       Some(mantissa as f64 * powi10(exponent))
   }
   ```
   `Option<f64>` rather than a `bool` + out-parameter is the concrete case the C-vs-Rust evaluation
   argued from: the sentinel cannot be read as a number because there is no number to read. In the C
   version a caller that ignored the return value silently got an uninitialised `double`.

   Use a lookup table for `powi10` rather than `pow()`, so the crate stays `no_std` with no libm
   dependency. `#![forbid(unsafe_code)]`; clippy `indexing_slicing`, `unwrap_used`, `panic` and the
   cast lints at `deny`.

2. Tests in the same file (`#[cfg(test)]`), covering at minimum:
   - `0x0076` → 118.0 (systolic, exponent 0)
   - `0x004A` → 74.0
   - `0xF04A` → 7.4 (exponent −1, mantissa 74). **NOTE:** an earlier draft of this brief said
     `0xF14A`, which is 33.0 — mantissa `0x14A` is 330. That error survived because the check was
     "did the test command exit 0", not "is the expected value right". Verify every vector by hand.
   - `0x07FF`, `0x0800`, `0x07FE`, `0x0802`, `0x0801` → all return `None`
   - `0x0FFF` is *not* a sentinel (mantissa `0xFFF` ≠ `0x7FF`) → returns `Some`
   - An exhaustive sweep of all 2^16 inputs asserting no panic and that every `Some` is finite.
   - Compare floats with `(a - b).abs() < 1e-9`, never `==`.

3. Write `fuzz/fuzz_targets/fuzz_sfloat.rs`:
   ```rust
   #![no_main]
   use libfuzzer_sys::fuzz_target;
   use vitals_decode::sfloat::decode_sfloat;

   fuzz_target!(|data: &[u8]| {
       let Some(chunk) = data.get(..2) else { return };
       let raw = u16::from_le_bytes([chunk[0], chunk[1]]);
       if let Some(v) = decode_sfloat(raw) {
           // A decode that succeeds must produce a finite number.
           assert!(v.is_finite(), "decode_sfloat({raw:#06x}) -> {v}");
       }
   });
   ```
5. Add a `cargo-fuzz` target, and a `kani` proof harness — kani proves absence of panics and
   overflow across all 2^16 inputs symbolically, which fuzzing only approximates (amendment 04 §11).

**Verify:** `cargo test -p vitals-decode && cargo kani -p vitals-decode --harness sfloat_never_panics`
**Expect:** exit 0 for both; fuzzer reports `Done` with no crashes.

**Commit:**
```sh
git add crates/vitals-decode/
git commit -m "feat(T-D2): IEEE-11073 SFLOAT decoder with sentinel handling and fuzz target"
```

---

### T-D3 — COPY-binary encoder and spool

**Tier:** implement-complex · **Host:** either · **Context budget:** ~7k · **Depends:** T-D1
**Amends:** 06 (§2.1 sync flag, §1.2 seq scope) · 01 (§2, spool replaces the embedded store) · 02 (§2.2, reserve columns) · 04 (§1, identity) · 05 (§1 session_id/seq derivation)
**Review required:** yes

**Replaces the embedded store entirely.** There is no on-Pi embedded database and no WAL tail. Read
`docs/specs/AMENDMENTS.md` before starting.

**Creates:** `crates/vitals-core/src/copybin.rs`, `crates/vitals-core/src/spool.rs`,
`crates/vitals-core/src/session.rs`, and unit tests for each.

**Interfaces produced** — later tasks depend on these:

```rust
/// One sample. `seq` is the stable identity within a session; `timestamp` is a
/// best-estimate event time and is NOT part of the key.
pub struct Sample {
    pub source: i32,
    pub r#type: i32,
    pub session_id: i64,
    pub seq: i64,
    pub timestamp: SystemTime,
    pub value: f64,
    pub extra: Option<Vec<u8>>,
    pub batch_id: Option<i64>,
}

/// Encodes one row in PostgreSQL binary COPY format. Pure: no allocation beyond
/// `out`, no I/O. Portable — this is why it lives in vitals-core and not the
/// Linux-only crate, and why it unit-tests on macOS.
pub fn encode_row(out: &mut Vec<u8>, s: &Sample) -> Result<usize, EncodeError>;
pub fn copy_header() -> [u8; 19];
pub fn copy_trailer() -> [u8; 2];

/// Append-only spool. fs4 advisory locks; a second writer fails rather than races.
pub struct Spool { /* … */ }
impl Spool {
    pub fn open(dir: &Path) -> io::Result<Self>;
    /// `sync: true` fsyncs the segment before returning (amendment 06 §2.1).
    /// Without this parameter T-D4 was told to "fsync before replying" through
    /// an API that exposed no fsync, forcing it to either reach into a crate it
    /// does not own or invent an extension -- the exact ownership break
    /// amendment 06 diagnosed one layer up.
    pub fn append(&mut self, rows: &[Sample], sync: bool) -> io::Result<()>;
    pub fn rotate_if_due(&mut self) -> io::Result<()>;
}
```

**Do:**

1. `copybin.rs` — the binary `COPY` encoder. **All integers are network byte order.** Both hosts are
   little-endian arm64, so a `memcpy` of a native integer produces a silently wrong value that
   Postgres will happily accept. Use `to_be_bytes()` everywhere.
   - 19-byte header: the 11-byte signature `PGCOPY` + `\n\377\r\n\0` (literal bytes `50 47 43 4F 50 59 0A FF 0D 0A 00`), then a 4-byte flags field and a
     4-byte header-extension length, both zero.
   - Each row: `i16` field count, then per field `i32` length + raw bytes. Length `-1` is SQL NULL.
   - Field order is fixed by `vitals/generated/registry.json` and must match the table exactly.
   - **Tests assert against byte arrays captured from a real server** (`COPY … TO STDOUT (FORMAT
     binary)`), not against the encoder's own output. Self-consistency is not correctness.
2. `session.rs` — session and sequence management.

   **`session_id` must be unique across process restarts, and this is load-bearing.** A per-process
   counter starting at 0 is the obvious implementation and it silently destroys data: `bp-collector`
   restarts, re-mints `session_id = 0`, and its next reading lands on `(6, 6, 0, 0)` — the same key
   as the first reading of a previous run — where `ON CONFLICT DO NOTHING` drops it without a trace.
   For RR the collision surface is narrow because `seq` is a beat index into the thousands; for BP
   `seq` is 0–3 every single time, so it is near-total, on the one source amendment 06 §2 says is
   reconstructible from nothing.

   Mint it from a monotonically increasing persisted counter (`/var/lib/vitals/session_seq`,
   incremented and fsynced before use) or from a UUID-derived 64-bit value. **Not** from a
   process-local counter, and **not** from wall-clock time — the RTC can be wrong at boot, which is
   the whole reason T-A5 exists.

   A `session_id` is minted per **upstream connection** — for `bp-collector` that is the BLE
   connection to the cuff, not the collector's socket connection to `vitalsd` (amendment 06 §1.1).
   `seq` is a monotonic per-session counter; for RR intervals it derives from the cumulative RR sum
   so it is unaffected by arrival jitter or retransmission (amendment 05 §1).

   `node_id` is 16 bytes from `/var/lib/vitals/node_id`, generated once via `getrandom`, and
   **never regenerated** — it is provenance and survives a store rebuild.
3. `spool.rs` — Linux side. `O_APPEND` writes, `fs4::FileExt::lock_exclusive` on the segment and the
   offset file, `fsync` per batch or every 250 ms, rotate at 64 MB or hourly.
   - The spool header carries a **schema fingerprint**: a hash of column names, order and types.
   - A collector must never block on disk **unless it asked to**. Batching bounds loss at 250 ms
     for streaming sources; `append(rows, sync: true)` opts a caller into durability-before-return
     (amendment 06 §2.1). Only `bp-collector` sets it — a BP frame is deleted by the device on the
     kernel's ATT confirmation and is reconstructible from nothing, while a night of RR is ~250 MB
     and ~100k appends that per-append fsync would destroy.

**Verify:** `cargo test -p vitals-core && cargo clippy -p vitals-core --all-targets`
**Expect:** exit 0. Includes a round-trip test: encode 1000 samples, drain into a scratch database,
assert every value matches within 1e-9 and every `seq` is preserved.

**Commit:**
```sh
git add crates/vitals-core/
git commit -m "feat(T-D3): COPY-binary encoder and append-only spool

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

### T-D4 — `vitalsd` daemon and socket protocol

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~6k · **Depends:** T-D3
**Amends:** 01 (§2 spool) · 02 (§2 signed ingest) · 04 (§2 OOM, §8 liveness) · 05 (§2.1 heartbeat condition, §4.11 backpressure)
**Review required:** yes

Single writer. Every collector connects; `vitalsd` alone touches the database.

**Creates:** `crates/vitalsd/src/main.rs`, `crates/vitals-proto/src/lib.rs`,
`/etc/systemd/system/vitalsd.service` on the-pi

**Interfaces produced:**
- Unix socket at `/run/vitals/vitalsd.sock`, mode `0660`, group `vitals`.
- Wire format: 4-byte LE length prefix + CBOR map. Verbs `append`, `query`, `subscribe`, `health`.
- **`append` takes `sync: bool`, default `false`** (amendment 06 §2.1). When true, `vitalsd` fsyncs
  the spool segment **before** replying, so the caller's append does not return until the bytes are
  durable. Only `bp-collector` sets it: a BP frame is deleted by the device on the kernel's ATT
  confirmation and is reconstructible from nothing, while a night of RR is ~250 MB and ~100k appends
  that per-append fsync would destroy. The default path stays batched (T-D3: fsync per batch or
  every 250 ms), and **a collector must never block on disk unless it asked to.**

**Do:**

1. `crates/vitalsd/src/main.rs`: a `tokio` current-thread runtime accepting on the Unix socket, one
   task per client. Single-threaded — a single writer needs no locking, and this is the whole reason
   for the design. `tokio::select!` over accept, per-client reads and the liveness tick replaces the
   hand-rolled readiness loop; use `LocalSet` so no state needs `Send`.
1b. **Heartbeat condition (A05 §2.1).** `klaxond`'s pin pulses only if a monitoring window is
   active AND a primary-source sample arrived within 30 s AND `cns-engine` produced a tier
   evaluation within 30 s AND the drain offset advanced or the spool is empty. Outside a monitoring
   window the heartbeat is unconditional. `vitalsd` owns these four facts and exposes them.
1c. **Subscriber backpressure (A05 §4.11).** Each subscriber gets a bounded queue. On overflow,
   **drop and count** — never block the collector — and raise a degraded state **naming the
   subscriber**. A detector that missed samples must know it missed them; the count feeds the same
   "can't assess" path as a sensor gap.
2. `health` returns, as CBOR: uptime, per-source last-sample age, total rows, WAL size, the p99
   radio-to-evaluation latency (A05 §4.9), time since the last tier evaluation (A05 §4.7), and the
   contents of `/var/lib/vitals/restore-verify.json`. **Any blocked or degraded state names its
   blocker and the value that blocked it** — never a bare boolean. A guard that will not say why
   costs an hour of guessing.
3. `subscribe` registers a selector and fans out matching appends to that client. This is the
   HealthKit observer equivalent, without the latency or the background-delivery budget.
4. Per-source liveness: track the last append time for each `source`. Absence past a configured
   cadence sets that source to `stale` in `health` — distinct from "readings are normal." Silence is
   never reassurance.
5. Write `vitalsd.service`:
   ```ini
   [Unit]
   Description=vitals core daemon
   After=network.target

   [Service]
   Type=notify
   ExecStart=/usr/local/bin/vitalsd --spool /var/lib/vitals/spool --socket /run/vitals/vitalsd.sock
   OOMScoreAdjust=-1000
   MemoryMin=64M
   MemoryHigh=192M
   Restart=always
   RestartSec=1
   User=vitals
   Group=vitals
   RuntimeDirectory=vitals
   StateDirectory=vitals
   WatchdogSec=30
   NoNewPrivileges=yes
   ProtectSystem=strict
   ProtectHome=yes
   PrivateTmp=yes

   [Install]
   WantedBy=multi-user.target
   ```
   `WantedBy=multi-user.target`, not `graphical.target` — this must survive the compositor dying.
6. `sudo useradd -r -s /usr/sbin/nologin vitals`, install, `systemctl enable --now vitalsd`.

**Verify:** `ssh $PI_HOST 'systemctl is-active vitalsd && vitals-cli health | jq -e ".uptime_seconds > 0"'`
**Expect:** exit 0.

---

### T-D5 — spool drainer

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~6k · **Depends:** T-D4, T-C3
**Amends:** 01 (§2.2 drain protocol) · 04 (§6 staging table, §7 migration guard, §8 liveness) · 05 (§4.1 liveness, §4.2 watermarks)
**Review required:** yes

**GATE G3:** requires T-C3 green. No sample worth keeping is written before a restore works.

**Replaces the WAL-tail design.** Read `docs/specs/AMENDMENTS.md` before starting.

**Creates:** `crates/vitals-drain/`, `/etc/systemd/system/vitals-spool-drain.service`

**Do:**

1. **Single-drainer enforcement.** On start, take `pg_advisory_lock(0x5641_4C53)` and exit cleanly if
   it cannot be acquired. `systemd` with `Restart=always` can respawn before the old process exits;
   two drainers must be impossible, not merely unlikely.
2. **Drain protocol** — one transaction, private staging:
   ```sql
   BEGIN;
   CREATE TEMP TABLE staging (LIKE samples INCLUDING DEFAULTS) ON COMMIT DROP;
   COPY staging FROM STDIN (FORMAT binary);
   INSERT INTO samples SELECT * FROM staging
     ON CONFLICT (source, type, session_id, seq) DO NOTHING;
   UPDATE spool_status SET offset_bytes = $1 WHERE segment = $2;
   COMMIT;
   ```
   **`TEMP`, not a shared staging table.** A shared unlogged table lets two drainers
   interleave `COPY` and commit a partial batch while the other's rows are lost — amendment 04 §6.
   The offset advances **in the same transaction** as the rows it accounts for, or not at all.
3. **`--verify` mode.** Compare the spool header's schema fingerprint against the live table shape;
   refuse to drain a mismatch and name the differing field. Migrating with a non-empty spool orphans
   buffered rows with no downgrade path (amendment 04 §7).
4. **Liveness, not just aliveness.** Expose on the health socket: `lag_bytes`, `lag_age_seconds`,
   `last_advanced_at`. A monotonic offset that stops advancing is a **named degraded state** —
   `systemctl is-active` is true while Postgres is in recovery, a lock blocks, or `--verify` fails.
5. **Disk watermarks.** Warn at 80%, refuse new spool *segments* at 95% — segments, not samples, so
   ingest degrades to in-memory rather than stopping. Postgres and pgBackRest must not be the things
   that discover the disk is full.
6. On Postgres unreachable: log, sleep 30 s, retry forever. **Never advance the offset. Never drop
   from the spool.**

**Verify:** `ssh $PI_HOST 'cargo test -p vitals-drain && ~/AnxietyWatch2/scripts/drain-crash-test.sh'`
**Expect:** exit 0. The crash test encodes 10 000 samples, drains, `SIGKILL`s mid-batch, restarts,
and asserts exactly 10 000 rows with no duplicates and the offset at EOF.

### T-D6 — `bp-collector` end to end

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~7k · **Depends:** T-D2, T-D4, T-D5
**Amends:** 06 (§1 identity from the transport, §2 durability, §3 device state) · 04 (§1 identity model) · 02 (§2 batch signing) · 05 (§1 identity, §4.3 BLE pairing)
**Review required:** yes

**GATE G3:** requires T-C3 green. No sample worth keeping is written before a restore works.

**Reads:** `docs/research/2026-08-03-ua651ble-ble-capture.md` (a live capture from this exact
hardware — it disproved three things an earlier version of this brief asserted; read it first) ·
`crates/vitals-decode/fixtures/ua651ble-2a35.md` (step 2's test vectors) ·
`vitals/registry.toml` (step 4's enum variants) · T-D3's `Sample` and T-D4's socket protocol below.

The first real producer. The cuff speaks standard `0x1810`, so this is the cleanest end-to-end
proof of the path — but the device is harder to talk to than "standard profile" suggests.

**Creates:** `crates/vitals-decode/src/bp.rs` (currently a doc-comment stub),
`crates/vitals-collectors/src/bin/bp_collector.rs`, `fuzz/fuzz_targets/fuzz_bp_decode.rs`,
`/etc/systemd/system/vitals-bp.service` on the-pi

**Do:**

1. Implement `bp_decode_2a35` in `crates/vitals-decode/src/bp.rs`:
   ```rust
   pub struct BpMeasurement {
       pub systolic: Option<f64>,      // mmHg — always mmHg, converted at this boundary
       pub diastolic: Option<f64>,     // mmHg
       pub map: Option<f64>,           // mmHg
       pub pulse_bpm: Option<f64>,
       pub timestamp: Option<i64>,     // UTC epoch seconds
       pub user_id: Option<u8>,
       pub irregular_pulse: bool,
       pub body_movement: bool,
       pub cuff_too_loose: bool,
   }
   /// Decodes a `0x2A35` Blood Pressure Measurement. `None` on malformed input.
   pub fn bp_decode_2a35(buf: &[u8]) -> Option<BpMeasurement>;
   ```
   `Option` rather than a `has_*` flag per field — this is the concrete case the C-vs-Rust
   evaluation argued from. Field order after the flags byte: three SFLOATs (systolic, diastolic,
   MAP), then timestamp (7 bytes), pulse (SFLOAT), user id (1 byte), status (2 bytes), each present
   only if its flag bit is set. Bounds-check before every read; the crate is `#![forbid(unsafe_code)]`
   with `indexing_slicing` denied, so use `get()`.

   If `flags & 0x01` (kPa), convert with `× 7.50062` **here**, so the store only ever holds mmHg.

2. Tests from `crates/vitals-decode/fixtures/ua651ble-2a35.md`: the four real frames with their
   expected values, plus every synthetic case listed there. The real capture only exercises
   exponent 0 — the synthetic cases are not optional.

3. `fuzz_bp_decode.rs` passing the raw buffer straight in. Run under ASan.

4. `bp_collector.rs` — and note what the capture proved about this device:

   - **Do not call `Device1.Connect()`.** Confirmed against `btmon`: 269 advertising reports with a
     completely empty ATT trace and no `LE Create Connection` ever issued. The cuff's radio is dark
     except for ~5–10 s after a measurement. Register intent with the kernel instead:
     `btmgmt add-device -a 2 -t 1 <MAC>`, then react to `ServicesResolved`. A collector built on
     `connect()` works when you trigger the cuff by hand and captures nothing unattended.
   - **`StartNotify()`, never a hand-written CCCD.** `0x2A35` is `indicate`; writing `0x0001`
     subscribes successfully and delivers nothing forever.
   - **Append with `sync: true`, and wait for the reply** (amendment 06 §2.1). BlueZ sends the ATT
     confirmation from the kernel before this process sees the frame, and the cuff deletes its record
     on that confirmation. `vitalsd` owns the spool, so the collector cannot fsync it directly —
     `sync: true` makes `vitalsd` fsync before replying. Do **not** set it on any streaming source.
   - **Journal the raw bytes before decoding**, fsynced, with arrival time. A decoder bug is then
     replayable; a lost frame never is. Four frames a day makes this free.
   - **`seq` is the drain index within one connection**, `session_id` minted per connection
     (amendment 06 §1.1). The cuff sends no timestamp, user id or counter, so identity comes from the
     transport: `0x2A35` is an indication and the cuff deletes only on confirmation, which makes
     redelivery impossible. This is ordering bookkeeping, not dedup.
   - **Say both true things about the guarantee.** Exactly-once holds for delivery to the host stack;
     it says nothing about durability in this process (amendment 06 §1.3). That gap is why
     `sync: true` and the raw journal exist.
   - **Set `extra` to mark `timestamp` as an arrival estimate** (amendment 06 §1.4). The cuff may
     have held the reading for hours; a later consumer must not read it as a device timestamp.
   - **Handle "bonded but cannot resume encryption" as a named degraded state** (amendment 06 §3):
     bonded device reached `Connected` but `ServicesResolved` never fired within 10 s. On kernel
     6.12.96 the peripheral's SMP Security Request (`0x0b`) is rejected, so encryption never starts
     and GATT never opens — indistinguishable from a quiet night. Name the device and the elapsed
     time; recovery is remove-bond-and-re-pair.
   - Emit one sample per present field with `source = VitalsSource::AdUa651ble` (from
     `vitals-registry`'s generated enum — never the integer literal, never the string), and types
     `systolic → VitalsType::BpSystolic`, `diastolic → BpDiastolic`, `map → BpMap`,
     **`pulse_bpm → VitalsType::Hr`** (the cuff's `provides` list carries `hr`; there is no
     `VitalsType::Pulse`).

5. **Investigate whether setting the cuff's clock enables timestamps.** `flags` bit1 was 0 across
   the whole capture. If writing Date Time (`0x2A08`) or the vendor service
   `233bf000-5a34-1b6d-975c-000d5690abe4` makes the cuff emit real event times, that replaces an
   arrival-time estimate with a true one and resolves the identity gap above. Record the result in
   the research doc either way — a negative result is worth as much here.

6. Install `vitals-bp.service`, `WantedBy=multi-user.target`, `Restart=always`.

7. **Take a real blood pressure reading.** Confirm it lands.

**Verify:** `ssh $PI_HOST 'cd ~/AnxietyWatch2 && cargo test -p vitals-decode bp && cargo fuzz run fuzz_bp_decode -- -max_total_time=60 && psql -U vitals -tAc "SELECT count(*) FROM samples WHERE source=6 AND type=6" | awk "\$1 > 0"'`
**Expect:** exit 0; tests pass; fuzzer clean at 60 s; at least one systolic row present with
`source = 6`.

**Commit:**
```sh
git add crates/vitals-decode/src/bp.rs crates/vitals-collectors/src/bin/bp_collector.rs fuzz/fuzz_targets/fuzz_bp_decode.rs
git commit -m "feat(T-D6): A&D UA-651BLE collector via standard 0x1810 blood pressure service"
```

---

## Acceptance — this change is done when

```sh
ssh $PI_HOST '
  findmnt -no SOURCE / | grep -q nvme &&
  hciconfig -a | grep -c "^hci" | grep -qx 1 &&
  test -e /dev/watchdog &&
  systemctl is-active vitalsd vitals-bp &&
  sudo -u postgres pgbackrest --stanza=vitals check &&
  jq -e ".ok == true" /var/lib/vitals/restore-verify.json &&
  cd ~/AnxietyWatch2 && cargo test --workspace &&
  psql -U vitals -tAc "SELECT count(*) FROM samples WHERE source=6" | awk "\$1 > 0"
'
```

Exit 0 means: root on NVMe, one Bluetooth adapter and it is the Sena, watchdog armed, both daemons
running, backups checked, a restore actually verified, all tests green, and a real blood pressure
reading in the store.

---

## Implementation notes

<!-- Conductor: record scope deltas, measured results, and decisions here as they happen.
     T-B1's gap histogram goes here verbatim — it is the project's foundational measurement. -->

### T-A3 — Sena UD100-G03 · PASSED 2026-08-03

Verify exited 0: two controllers present, `hci1` UP RUNNING. **Gate G1 is open.**

```
lsusb    0a12:0001 Cambridge Silicon Radio, Ltd Bluetooth Dongle (HCI mode)
hci1     BD Address 00:01:95:XX:XX:XX   Bus: USB   UP RUNNING
         HCI Version 4.0 (0x6)  Revision 0x2031
         LMP Version 4.0 (0x6)  Subversion 0x2031
         Manufacturer: Cambridge Silicon Radio (10)
hci0     BD Address 2C:CF:67:XX:XX:XX   Bus: UART  (onboard, still up)
```

**Authenticity.** CSR dongles reporting `0a12:0001` are among the most counterfeited USB parts, so
this was checked rather than assumed: the BD address OUI `00:01:95` belongs to **Sena Technologies**,
and `HCI Version 4.0` matches the advertised Bluetooth 4.0 Class 1 part. Genuine.

**LE scan (25 s on hci1)** discovered 23 devices, including two project sensors:

| Device | Address |
|---|---|
| `ResMed device` — the AirSense 11 | `50:32:5F:XX:XX:XX` |
| `SleepO2-2_XXXX` — the EMAY oximeter | `D9:80:A3:XX:XX:XX` |

No Polar H10 — expected, it only advertises when worn with a moist strap.

**Two findings for later tasks:**

1. **`hci1` came up RF-killed on first attempt** (`Operation not possible due to RF-kill (132)`) and
   succeeded on retry a few seconds later — the dongle was still initialising after `btusb` bound.
   Collector startup must **retry adapter bring-up with backoff**, not fail on first error. `rfkill`
   was not installed; now is.
2. **`hci1` is not brought up at boot.** `bluetooth.service` is enabled but nothing raises the second
   adapter, so after a reboot it will be DOWN. T-A4 must add a unit that brings up the surviving
   adapter and **verifies it is RUNNING**, otherwise the box reboots into a state with no working
   Bluetooth and no complaint.

Finding 2 matters more than it looks: it is the same shape as every other silent failure in this
project — the service is `active`, the adapter is dead, and nothing says so.

### T-A4 — Disable onboard Bluetooth · PASSED 2026-08-03

Verify exited 0: exactly one controller, and it is the USB one. A 20 s LE scan on the surviving
adapter still finds the AirSense 11 and the EMAY oximeter.

```
hci0    BD Address 00:01:95:XX:XX:XX   Bus: USB   UP RUNNING     (the Sena)
        1 controller total; the onboard UART radio is gone
config.txt:50  dtoverlay=disable-bt
backup         /boot/firmware/config.txt.pre-disable-bt
hciuart        disabled
```

**The Sena renumbered from `hci1` to `hci0`** once the onboard adapter was removed — adapter indices
are assigned in registration order, not fixed to a device. `vitals-bt-up` handled it because it
selects the first adapter rather than hardcoding an index, which is why it is written that way.

**Every remaining reference to `hci1` in this repo is now wrong.** Collectors must select the adapter
by **BD address or by "the only adapter"**, never by index — an index that changed once can change
again when a second radio is added.

### Order of operations, and why it mattered

T-A3 passed but only proved `hci1` worked *at that moment*. It did not prove the adapter survives a
reboot — and nothing brought it up at boot, so `bluetooth.service` would have reported `active` with
a dead radio. Disabling the onboard adapter first would have left the box with no working Bluetooth
and no complaint.

So `vitals-bt-up.service` went in first and was **proven across a real reboot while `hci0` was still
present as a fallback**, and only then was the onboard radio disabled. There was never a moment where
a reboot could have produced a Bluetooth-less box.

The unit retries with backoff because the dongle returned `RF-kill (132)` when addressed before
`btusb` finished binding, and it **fails loudly** — `Restart=on-failure` plus a nonzero exit — rather
than absorbing the error. A bring-up that cannot fail is the same as no bring-up.

### Rollback

```sh
sudo cp /boot/firmware/config.txt.pre-disable-bt /boot/firmware/config.txt
sudo systemctl enable hciuart && sudo reboot
```

### T-D3 — COPY-binary encoder and spool · PASSED 2026-08-03

`cargo test --workspace` 40 passed · `cargo clippy --workspace --all-targets -- -D warnings` clean ·
round-trip verified against PostgreSQL 16.

**Reference bytes earned their keep immediately.** The brief insists tests assert against output
captured from a real server rather than the encoder's own. That caught the format's sharpest edge:

```
'2000-01-01 00:00:00+00'  →  00 00 00 00 00 00 00 00
```

`TIMESTAMPTZ` in binary `COPY` is **microseconds since 2000-01-01**, not the Unix epoch. An encoder
written from the specification alone would very plausibly emit Unix-epoch microseconds — every
timestamp thirty years off, and `Postgres` accepts it without a word. Self-consistent tests would
have agreed with the bug.

Confirmed by mutation: setting the offset to zero fails 4 unit tests and flags all 1001 round-trip
rows.

**Scope deltas.**

1. **`encode_row(out, &sample)` as published could not produce a valid row.** `Sample` carries no
   `node_id` and the column is `NOT NULL`. The signature is now
   `encode_row(out, &sample, &node_id)` — `node_id` is constant for the life of the process, so
   carrying it per sample would waste 16 bytes each for a value that never varies. **T-D4 consumes
   this signature.**

2. **`Spool::append` gained the `node_id` parameter** for the same reason:
   `append(&mut self, rows, node_id, sync)`.

3. **The advisory lock did not lock.** `fs4::try_lock_exclusive` returns `io::Result<bool>`, where
   `Ok(false)` means the lock was *not* acquired. The first version only handled `Err`, so `Ok(false)`
   passed and the spool ran unlocked — a guard against undetectable interleaved writes that silently
   did not guard. Caught by `a_second_writer_is_refused`, which is the only reason it is known.

4. **`session_id` restart-uniqueness is implemented and tested**, per amendment 06. The counter is
   fsynced *before* the id is returned; returning first would reuse an id across a crash. A corrupt
   counter is an error, never a reset to zero — resetting would re-issue ids already in the store.

5. **The round-trip skipped in CI.** `build-test` now runs a `postgres:16` service container, and a
   `SKIPPED:` line in the test output **fails the job**. A database test that quietly skips is worth
   less than no test, and this repo has produced that shape five separate times.

**Also implemented beyond the brief:** a per-segment schema fingerprint (FNV-1a over column names,
order and type OIDs) with tests proving reordering, retyping, renaming and name-boundary ambiguity
all change it. Binary `COPY` is positional, so transposing `source` and `type` — both `INTEGER` —
would otherwise be undetectable.
