# Tasks — Pi 5 Vitals Core (Stages 0–3)

> **For agentic workers:** each `### T-XX` block is a complete, self-contained prompt. Paste one
> block into a fresh agent; it needs no conversation history. Read the `Reads:` list and nothing
> else. Acceptance is the `Verify` command's exit code — not your own assessment.

**Goal:** A blood pressure reading taken on the A&D UA-651BLE lands in Postgres on the Pi with
correct provenance, recoverable from a verified backup.

**Architecture:** Collectors write through `libvitals` to `vitalsd`, the single writer of a narrow
append-only `samples` table in SQLite. `vitalsd` tails its own WAL into Postgres on NVMe. The
life-safety path runs as static C binaries under systemd; Docker holds nothing in this change.

**Tech stack:** C11 · meson/ninja · SQLite 3 (WAL) · PostgreSQL 16 · pgBackRest · BlueZ/D-Bus ·
systemd · Python 3.12 (codegen only)

## Global constraints

- Host `chris@minidude`, passwordless sudo. Hostname is `raspberrypi`; reach it as `minidude`.
- Pi-side working tree: `~/vitals`. This repo is authoritative for the registry TOML and generated
  headers; both are committed here and copied there.
- **UTC epoch seconds only** in the store. No local time, ever, at any layer below presentation.
- **Canonical unit per type**, converted once at the decoder. The store holds one unit per type.
- `samples` is **append-only**. No `UPDATE`, no `DELETE`. Removal is a tombstone row.
- C11, `-Wall -Wextra -Werror`, no warnings. `-fsanitize=address,undefined` in the test build.
- Every byte-decoding function is pure: `(const uint8_t*, size_t) -> result`. No allocation, no I/O.
- Commit after every task. Never commit to `main`; work on `design/pi5-vitals-core`.

---

## Task index

The conductor ticks a box **only after running that task's `Verify` command itself and seeing exit
0.** A worker's report is a claim; the exit code is the evidence. This is also what the herdr shell
pane renders (`grep -n '^- \[' tasks.md`).

- [ ] T-A1 — Migrate root filesystem to NVMe · human · pi
- [ ] T-A2 — Audio output path · human · pi
- [ ] T-A3 — Bring up the Sena UD100-G03 as hci1 · implement-standard · pi · **gate G1 owner**
- [ ] T-A4 — Disable onboard Bluetooth · mechanical · pi · **blocked by G1**
- [ ] T-A5 — Watchdog, governor, update policy · mechanical · pi
- [ ] T-B1 — Eight-hour Polar H10 capture · implement-complex · pi · **review**
- [ ] T-C1 — PostgreSQL 16 bare metal on NVMe · implement-standard · pi · **blocked by G2**
- [ ] T-C2 — pgBackRest, three tiers · implement-standard · pi · **review**
- [ ] T-C3 — Scheduled restore verification · implement-complex · pi · **review · gate G3 owner**
- [ ] T-D1 — Registry TOML and codegen · implement-standard · mac
- [ ] T-D2 — IEEE-11073 SFLOAT decoder with fuzz target · implement-complex · either · **review**
- [ ] T-D3 — libvitals append and query · implement-complex · either · **review**
- [ ] T-D4 — vitalsd daemon and socket protocol · implement-complex · pi · **review**
- [ ] T-D5 — SQLite WAL tail into Postgres · implement-complex · pi · **review**
- [ ] T-D6 — bp-collector end to end · implement-complex · pi · **review · blocked by G3**

Parallelism: T-D1 and T-D2 have no hardware dependency and may run on separate workers concurrently
with all of Group A. T-B1 occupies a whole night — dispatch it the moment T-A4 is green and let the
other groups proceed around it.

---

## Group A — Hardware floor (Stage 0)

### T-A1 — Migrate root filesystem to NVMe

**Tier:** human · **Host:** pi · **Context budget:** ~2k · **Depends:** none · **Review required:** no

**Reads:** none.
**Modifies:** `/boot/firmware/cmdline.txt`, `/etc/fstab` on minidude.

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

**Verify:** `ssh chris@minidude 'findmnt -no SOURCE / | grep -q nvme && df -h / | tail -1'`
**Expect:** exit 0, root shown on `/dev/nvme0n1p1`.

**Commit:** nothing to commit — hardware state. Record the result in `## Implementation notes`.

---

### T-A2 — Audio output path

**Tier:** human · **Host:** pi · **Context budget:** ~2k · **Depends:** none · **Review required:** no

The Pi 5 has no analog audio. Today only `vc4hdmi0`/`vc4hdmi1` exist and no display is on HDMI, so
there is currently no way to make a sound. This task establishes the alarm's output path; `klaxond`
is Stage 5.

**Reads:** none.
**Creates:** `/etc/asound.conf` on minidude.

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

**Verify:** `ssh chris@minidude 'aplay -l | grep -viE "vc4hdmi" | grep -q "^card" && aplay -D default /usr/share/sounds/alsa/Front_Center.wav'`
**Expect:** exit 0, a non-HDMI card present, playback returns cleanly.

---

### T-A3 — Bring up the Sena UD100-G03 as `hci1`

**Tier:** implement-standard · **Host:** pi · **Context budget:** ~3k · **Depends:** none
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

**Verify:** `ssh chris@minidude 'hciconfig -a | grep -c "^hci" | grep -q 2 && hciconfig hci1 | grep -q "UP RUNNING"'`
**Expect:** exit 0, exactly two controllers, `hci1` up.

---

### T-A4 — Disable onboard Bluetooth

**Tier:** mechanical · **Host:** pi · **Context budget:** ~2k · **Depends:** T-A3 · **Review required:** no

**GATE G1.** Do not run this until T-A3's verify has exited 0. `dtoverlay=disable-bt` removes the
onboard controller; without a working Sena this leaves the box with no Bluetooth at all. **Dispatch
only on a green verify, never on a worker's report.**

Rationale: with one adapter, a collector binding the wrong radio stops being a possible bug.

**Modifies:** `/boot/firmware/config.txt` on minidude.

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

**Verify:** `ssh chris@minidude 'hciconfig -a | grep -c "^hci" | grep -qx 1 && hciconfig -a | grep -q "Bus: USB"'`
**Expect:** exit 0, exactly one controller, and it is the USB one.

---

### T-A5 — Watchdog, governor, update policy

**Tier:** mechanical · **Host:** pi · **Context budget:** ~2k · **Depends:** T-A1 · **Review required:** no

**Modifies:** `/boot/firmware/config.txt`, `/etc/systemd/system.conf`,
`/etc/apt/apt.conf.d/20auto-upgrades` on minidude.
**Creates:** `/etc/systemd/system/cpu-performance.service` on minidude.

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

**Verify:** `ssh chris@minidude 'test -e /dev/watchdog && cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor | grep -qx performance && vcgencmd get_throttled | grep -qx "throttled=0x0"'`
**Expect:** exit 0.

---

## Group B — Thesis spike (Stage 1)

### T-B1 — Eight-hour Polar H10 capture

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~5k · **Depends:** T-A4
**Review required:** yes

The entire project rests on the Pi holding a BLE connection all night. **Measure this before
building anything on top of it.** Deliberately placed before the storage and core work.

**Creates:** `~/vitals/spike/hr_capture.c`, `~/vitals/spike/meson.build`,
`~/vitals/spike/gap_histogram.py`

**Do:**

1. Install dependencies:
   ```sh
   sudo apt-get update && sudo apt-get install -y libglib2.0-dev libdbus-1-dev meson ninja-build bluez
   ```
2. Write `~/vitals/spike/hr_capture.c`. Use BlueZ over D-Bus (`org.bluez.GattCharacteristic1`),
   subscribe to Heart Rate Measurement `00002a37-0000-1000-8000-00805f9b34fb`, and append one line
   per notification to stdout: `<unix_epoch_seconds_float> <hr_bpm> <rr_ms,rr_ms,...>`.

   Decode `0x2A37` per the SIG spec — flags byte first:
   ```c
   // byte 0 = flags: bit0 HR is uint16 (else uint8), bit3 energy present, bit4 RR present
   size_t off = 1;
   uint8_t flags = buf[0];
   uint16_t hr;
   if (flags & 0x01) { if (len < off+2) return -1; hr = buf[off] | (buf[off+1] << 8); off += 2; }
   else              { if (len < off+1) return -1; hr = buf[off]; off += 1; }
   if (flags & 0x08) { if (len < off+2) return -1; off += 2; }   // energy expended, skipped
   // remaining bytes are RR intervals, uint16 LE, units of 1/1024 s
   while (off + 2 <= len) {
       uint16_t rr_raw = buf[off] | (buf[off+1] << 8); off += 2;
       double rr_ms = rr_raw * 1000.0 / 1024.0;
       emit_rr(rr_ms);
   }
   ```
   Every bounds check above is required. `len` is attacker-controlled radio input.
3. Build with `meson setup build && ninja -C build`.
4. Pair and trust the H10 once:
   ```sh
   bluetoothctl --timeout 30 scan on
   bluetoothctl pair <H10_MAC>; bluetoothctl trust <H10_MAC>
   ```
5. Run the capture overnight under systemd-run so it survives your SSH session:
   ```sh
   systemd-run --user --unit=hrspike --setenv=H10_MAC=<mac> \
     ~/vitals/spike/build/hr_capture > ~/vitals/spike/night1.log
   ```
6. In the morning, compute the gap histogram with `gap_histogram.py`: read the log, diff consecutive
   timestamps, and print count of gaps in each bucket `<2s, 2-10s, 10-60s, 60s-5m, >5m`, plus total
   duration, total RR count, and longest gap.
7. Record the histogram verbatim in `## Implementation notes`. **This number is the project's
   foundation — if the longest gap exceeds 5 minutes, stop and report before proceeding.**

**Verify:** `ssh chris@minidude 'python3 ~/vitals/spike/gap_histogram.py ~/vitals/spike/night1.log --require-duration-hours 8 --max-gap-seconds 300'`
**Expect:** exit 0. The script must exit nonzero if duration < 8 h or longest gap > 300 s.

**Commit:**
```sh
git add openspec/changes/pi5-vitals-core/tasks.md
git commit -m "spike(T-B1): record 8-hour Polar H10 capture gap histogram"
```

---

## Group C — Storage floor (Stage 2)

**GATE G2:** all of Group C requires T-A1 green. Postgres must never initialize on the microSD.

### T-C1 — PostgreSQL 16 bare metal on NVMe

**Tier:** implement-standard · **Host:** pi · **Context budget:** ~4k · **Depends:** T-A1
**Review required:** no

Not in Docker, not on the microSD.

**Creates:** `/etc/postgresql/16/main/conf.d/vitals.conf` on minidude.

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

**Verify:** `ssh chris@minidude 'sudo -u postgres psql -tAc "SHOW shared_buffers" | grep -q 2GB && sudo -u postgres psql -tAc "SHOW data_directory" | xargs findmnt -no SOURCE -T | grep -q nvme && sudo -u postgres psql -d vitals -tAc "SELECT count(*) FROM pg_extension WHERE extname IN ((chr(39)||chr(39)))" >/dev/null && test $(sudo -u postgres psql -d vitals -tAc "SELECT count(*) FROM pg_extension WHERE extname = ANY(ARRAY[chr(116)||chr(105)||chr(109)||chr(101)||chr(115)||chr(99)||chr(97)||chr(108)||chr(101)||chr(100)||chr(98)])") -eq 1'`
**Expect:** exit 0 — `shared_buffers=2GB`, `PGDATA` on NVMe, `timescaledb` installed.

**Simpler equivalent** if the quoting above is awkward to run by hand:
```sh
ssh chris@minidude 'sudo -u postgres psql -d vitals -tAc "SELECT extname FROM pg_extension ORDER BY 1" | tr "\n" " "'
```
Expect: `pg_cron pg_stat_statements plpgsql timescaledb timescaledb_toolkit`

---

### T-C2 — pgBackRest, three tiers

**Tier:** implement-standard · **Host:** pi · **Context budget:** ~4k · **Depends:** T-C1
**Review required:** yes

**Creates:** `/etc/pgbackrest/pgbackrest.conf`, two systemd timers on minidude.

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

**Verify:** `ssh chris@minidude 'sudo -u postgres pgbackrest --stanza=vitals check && sudo -u postgres pgbackrest --stanza=vitals info | grep -q "status: ok"'`
**Expect:** exit 0, `status: ok`.

---

### T-C3 — Scheduled restore verification

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~4k · **Depends:** T-C2
**Review required:** yes

**GATE G3 owner.** No sample worth keeping is written until a restore has been demonstrated.

A backup that has never been restored is a hypothesis. This makes it a measurement.

**Creates:** `/usr/local/bin/verify-restore.sh`, `verify-restore.service`, `verify-restore.timer`
on minidude.

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

**Verify:** `ssh chris@minidude 'sudo /usr/local/bin/verify-restore.sh && jq -e ".ok == true" /var/lib/vitals/restore-verify.json'`
**Expect:** exit 0, `ok: true`.

---

## Group D — vitals core (Stage 3)

### T-D1 — Registry TOML and codegen

**Tier:** implement-standard · **Host:** mac · **Context budget:** ~5k · **Depends:** none
**Review required:** no

The keystone. One file declares every source, type, canonical unit, and valid range; four artifacts
are generated from it. `"polar_h10"` exists in exactly one place and is an integer everywhere else.

**Creates:** `vitals/registry.toml`, `vitals/tools/gen_registry.py`,
`vitals/generated/vitals_types.h`, `vitals/generated/registry.sql`,
`vitals/generated/VitalsRegistry.swift`, `vitals/generated/registry.json`

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
2. Write `vitals/tools/gen_registry.py` — stdlib only, `tomllib`. It emits all four artifacts. Each
   generated file starts with `/* GENERATED FROM registry.toml — DO NOT EDIT */`.
   - `vitals_types.h`: `VITALS_TYPE_HR = 1`, `VITALS_TYPE_BP_SYSTOLIC = 6`, … **and**
     `VITALS_SOURCE_EMAY = 0`, `VITALS_SOURCE_POLAR_H10 = 1`, `VITALS_SOURCE_AD_UA651BLE = 6`, …
     (both enums — later tasks reference `VITALS_SOURCE_*` by name). Plus `vitals_type_range(int)`
     returning `{double lo, hi}` and `vitals_type_unit(int)` returning a `const char *`.
   - `registry.sql`: `vitals_type` and `vitals_source` lookup tables with `INSERT`s, plus a
     `CHECK` on `samples.type` referencing them.
   - `VitalsRegistry.swift`: `public enum VitalsType: Int32` and `VitalsSource: Int32`.
   - `registry.json`: the parsed structure verbatim, for Python and the API layer.
3. Add a regeneration check so a hand-edited generated file cannot survive CI:
   ```sh
   python3 vitals/tools/gen_registry.py --check
   ```
   which regenerates to a temp dir and diffs, exiting nonzero on difference.

**Verify:** `python3 vitals/tools/gen_registry.py --check && grep -q "VITALS_TYPE_BP_SYSTOLIC = 6" vitals/generated/vitals_types.h`
**Expect:** exit 0.

**Commit:**
```sh
git add vitals/registry.toml vitals/tools/gen_registry.py vitals/generated/
git commit -m "feat(T-D1): vitals registry TOML and codegen for C, SQL, Swift, JSON"
```

---

### T-D2 — IEEE-11073 SFLOAT decoder with fuzz target

**Tier:** implement-complex · **Host:** either · **Context budget:** ~4k · **Depends:** T-D1
**Review required:** yes

Split out from the collector because it is the subtlest code in this change and the easiest to prove
correct: pure, no state, no I/O, one buffer in.

SFLOAT is **not** an IEEE-754 half float. It is 16-bit with a 4-bit signed exponent in the high
nibble and a 12-bit signed mantissa below, and it has reserved sentinels that must map to *absent* —
never to a number. A naive `_Float16` cast produces plausible-looking garbage, which is the worst
possible failure in a medical value: 118/74 and 141/92 are both believable.

**Creates:** `vitals/src/sfloat.c`, `vitals/src/sfloat.h`, `vitals/tests/test_sfloat.c`,
`vitals/fuzz/fuzz_sfloat.c`

**Do:**

1. Write `vitals/src/sfloat.h`:
   ```c
   #ifndef VITALS_SFLOAT_H
   #define VITALS_SFLOAT_H
   #include <stdbool.h>
   #include <stdint.h>
   /* Decodes an IEEE-11073 16-bit SFLOAT.
    * Returns false for NaN/NRes/±INFINITY/Reserved — the value is ABSENT.
    * *out is untouched when false is returned. */
   bool vitals_sfloat_decode(uint16_t raw, double *out);
   #endif
   ```
2. Write `vitals/src/sfloat.c`:
   ```c
   #include "sfloat.h"
   #include <math.h>

   bool vitals_sfloat_decode(uint16_t raw, double *out) {
       int16_t mantissa = (int16_t)(raw & 0x0FFF);
       int8_t  exponent = (int8_t)(raw >> 12);

       /* Reserved mantissa values are ABSENT, never numbers. */
       switch (mantissa) {
           case 0x07FF:  /* NaN        */
           case 0x0800:  /* NRes       */
           case 0x07FE:  /* +INFINITY  */
           case 0x0802:  /* -INFINITY  */
           case 0x0801:  /* Reserved   */
               return false;
           default: break;
       }
       if (exponent >= 0x08) exponent -= 0x10;      /* sign-extend 4-bit  */
       if (mantissa >= 0x0800) mantissa -= 0x1000;  /* sign-extend 12-bit */
       *out = (double)mantissa * pow(10.0, (double)exponent);
       return true;
   }
   ```
3. Write `vitals/tests/test_sfloat.c` covering, at minimum:
   - `0x0076` → 118.0 (systolic, exponent 0)
   - `0x004A` → 74.0
   - `0xF04A` → 7.4 (exponent −1, mantissa 74). NOTE: an earlier draft of this brief said `0xF14A`, which is 33.0 — mantissa 0x14A is 330. Verify test vectors by hand.
   - `0x07FF`, `0x0800`, `0x07FE`, `0x0802`, `0x0801` → all return `false`
   - `0x0FFF` is *not* a sentinel (mantissa 0xFFF ≠ 0x7FF) → returns true
   - Compare doubles with `fabs(a-b) < 1e-9`, never `==`.
4. Write `vitals/fuzz/fuzz_sfloat.c`:
   ```c
   #include "../src/sfloat.h"
   #include <stddef.h>
   #include <stdint.h>
   int LLVMFuzzerTestOneInput(const uint8_t *d, size_t n) {
       if (n < 2) return 0;
       double v;
       uint16_t raw = (uint16_t)(d[0] | (d[1] << 8));
       if (vitals_sfloat_decode(raw, &v)) {
           /* A decode that succeeds must produce a finite number. */
           if (!(v == v) || v > 1e300 || v < -1e300) __builtin_trap();
       }
       return 0;
   }
   ```
5. Wire both into `meson.build` with `-fsanitize=address,undefined` on the test build.

**Verify:** `meson test -C vitals/build sfloat --print-errorlogs && vitals/build/fuzz_sfloat -max_total_time=60 -runs=1000000`
**Expect:** exit 0 for both; fuzzer reports `Done` with no crashes.

**Commit:**
```sh
git add vitals/src/sfloat.c vitals/src/sfloat.h vitals/tests/test_sfloat.c vitals/fuzz/fuzz_sfloat.c vitals/meson.build
git commit -m "feat(T-D2): IEEE-11073 SFLOAT decoder with sentinel handling and fuzz target"
```

---

### T-D3 — `libvitals` append and query

**Tier:** implement-complex · **Host:** either · **Context budget:** ~6k · **Depends:** T-D1
**Review required:** yes

**Creates:** `vitals/src/libvitals.c`, `vitals/src/libvitals.h`, `vitals/src/hlc.c`,
`vitals/src/hlc.h`, `vitals/tests/test_libvitals.c`

**Interfaces produced** — later tasks depend on these exact signatures:

```c
typedef struct { int32_t source, type; double timestamp, value;
                 const uint8_t *extra; size_t extra_len; } vitals_sample;

typedef struct vitals_conn vitals_conn;

vitals_conn *vitals_connect(const char *socket_path);
void         vitals_close(vitals_conn *);
/* Returns rows accepted, or negative errno. Idempotent on (source,type,timestamp). */
int          vitals_append(vitals_conn *, const vitals_sample *, size_t n);
int          vitals_query(vitals_conn *, int32_t source, int32_t type,
                          double from, double to, vitals_sample *out, size_t cap);
```

**Do:**

1. Write `hlc.h`/`hlc.c` implementing a Hybrid Logical Clock:
   ```c
   typedef struct { int64_t physical; int32_t logical; uint8_t node_id[16]; } hlc_t;
   /* Advances the clock for a local event. Never moves physical backwards. */
   void hlc_now(hlc_t *clock, hlc_t *out);
   /* Merges a remote stamp. Bounds forward drift to 60s. */
   void hlc_merge(hlc_t *clock, const hlc_t *remote, hlc_t *out);
   int  hlc_compare(const hlc_t *a, const hlc_t *b);
   ```
   `node_id` is 16 bytes read once from `/var/lib/vitals/node_id`, generated on first run from
   `getrandom(2)`. It is **never** regenerated — persisted separately from the database so a store
   rebuild does not mint a new identity.
2. Write the SQLite schema in `libvitals.c` matching the design spec exactly:
   ```sql
   PRAGMA journal_mode = WAL;
   PRAGMA synchronous = NORMAL;
   PRAGMA foreign_keys = ON;

   CREATE TABLE IF NOT EXISTS samples (
     source INTEGER NOT NULL, type INTEGER NOT NULL,
     timestamp REAL NOT NULL, value REAL NOT NULL, extra BLOB,
     hlc_physical INTEGER NOT NULL, hlc_logical INTEGER NOT NULL,
     node_id BLOB NOT NULL,
     PRIMARY KEY (source, type, timestamp)
   ) WITHOUT ROWID, STRICT;
   CREATE INDEX IF NOT EXISTS idx_samples_hlc
     ON samples(node_id, hlc_physical, hlc_logical);
   ```
3. `vitals_append` validates every sample against `vitals_type_range()` from the generated header
   **before** insert. Out-of-range values are rejected with `-EDOM` and written to a
   `quarantine` table with the raw value and reason. They are never clamped — a clamped value is
   indistinguishable from a real one downstream.
4. Insert with `INSERT ... ON CONFLICT(source,type,timestamp) DO NOTHING` so append is idempotent.
   No `UPDATE`, no `DELETE` anywhere in this file.
5. Tests in `test_libvitals.c`, against a `:memory:` database:
   - append 1000 samples, query back, assert count and values within 1e-9
   - append the same sample twice, assert the second returns 0 accepted
   - append `spo2 = 150.0`, assert `-EDOM` and one quarantine row
   - assert `hlc_compare` is a total order over 10k generated stamps
   - assert no `UPDATE` or `DELETE` statement text appears in the source:
     `grep -icE '\b(UPDATE|DELETE)\b' src/libvitals.c` returns 0

**Verify:** `meson test -C vitals/build libvitals --print-errorlogs`
**Expect:** exit 0, all cases pass.

**Commit:**
```sh
git add vitals/src/libvitals.c vitals/src/libvitals.h vitals/src/hlc.c vitals/src/hlc.h vitals/tests/test_libvitals.c
git commit -m "feat(T-D3): libvitals append/query with HLC stamping and range quarantine"
```

---

### T-D4 — `vitalsd` daemon and socket protocol

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~6k · **Depends:** T-D3
**Review required:** yes

Single writer. Every collector connects; `vitalsd` alone touches the database.

**Creates:** `vitals/src/vitalsd.c`, `vitals/src/proto.c`, `vitals/src/proto.h`,
`/etc/systemd/system/vitalsd.service` on minidude

**Interfaces produced:**
- Unix socket at `/run/vitals/vitalsd.sock`, mode `0660`, group `vitals`.
- Wire format: 4-byte LE length prefix + CBOR map. Verbs `append`, `query`, `subscribe`, `health`.

**Do:**

1. `vitalsd.c`: `epoll` loop over the listening socket, one connection state per client. Single
   thread — a single writer needs no locking, and this is the whole reason for the design.
2. `health` returns, as CBOR: uptime, per-source last-sample age, total rows, WAL size, and the
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
   ExecStart=/usr/local/bin/vitalsd --db /var/lib/vitals/tsdb.sqlite --socket /run/vitals/vitalsd.sock
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

**Verify:** `ssh chris@minidude 'systemctl is-active vitalsd && vitals-cli health | jq -e ".uptime_seconds > 0"'`
**Expect:** exit 0.

---

### T-D5 — SQLite WAL tail into Postgres

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~5k · **Depends:** T-D4, T-C3
**Review required:** yes

SQLite is the durable ingest buffer; Postgres is the queryable system of record. If Postgres is down
for an upgrade, ingest does not stop and nothing is lost.

**Creates:** `vitals/src/wal_tail.c`, `/etc/systemd/system/vitals-waltail.service` on minidude

**Do:**

1. Track a replication cursor in Postgres, not in a file:
   ```sql
   CREATE TABLE IF NOT EXISTS vitals_repl_cursor (
     node_id      BYTEA PRIMARY KEY,
     hlc_physical BIGINT NOT NULL,
     hlc_logical  INTEGER NOT NULL
   );
   ```
   **The cursor is an HLC position derived from rows already written — never a wall-clock
   timestamp.** This is the structural fix for the `lastSyncDate = .now` race that opened a data hole
   twice in the previous system. There is no clock in this path to be wrong about.
2. The tail loop, per iteration:
   - read the cursor for each `node_id`
   - `SELECT ... FROM samples WHERE node_id = ? AND (hlc_physical, hlc_logical) > (?, ?)
      ORDER BY hlc_physical, hlc_logical LIMIT 5000`
   - `COPY` the batch into Postgres inside one transaction
   - **advance the cursor to the max HLC of the rows actually copied, in the same transaction**
   Cursor advance and the copy that carried the rows are committed together or not at all. A cursor
   that can advance past rows that were never copied is the same bug in a new costume.
3. On Postgres unreachable: log, sleep 30 s, retry forever. Never advance the cursor. Never drop
   from SQLite.
4. Test: insert 10k rows, run one tail pass, assert Postgres count is 10k and the cursor equals the
   max HLC. Then kill the tailer mid-batch with `SIGKILL`, restart, and assert the final count is
   still exactly 10k with no duplicates.

**Verify:** `ssh chris@minidude 'meson test -C ~/vitals/build waltail --print-errorlogs && psql -U vitals -tAc "SELECT count(*) FROM samples" | grep -qx 10000'`
**Expect:** exit 0, exactly 10000 rows, no duplicates after the kill-restart cycle.

---

### T-D6 — `bp-collector` end to end

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~6k · **Depends:** T-D2, T-D4, T-D5
**Review required:** yes

**GATE G3:** requires T-C3 green. No sample worth keeping is written before a restore works.

The first real producer. The A&D cuff speaks standard `0x1810`, so this is the cleanest possible
end-to-end proof of the whole path.

**Creates:** `vitals/src/collectors/bp_collector.c`, `vitals/src/bp_decode.c`,
`vitals/src/bp_decode.h`, `vitals/tests/test_bp_decode.c`, `vitals/fuzz/fuzz_bp_decode.c`,
`/etc/systemd/system/vitals-bp.service` on minidude

**Do:**

1. Write the pure decoder `vitals/src/bp_decode.h`:
   ```c
   typedef struct {
       bool   has_systolic, has_diastolic, has_map, has_pulse, has_timestamp;
       double systolic, diastolic, map, pulse;   /* mmHg, mmHg, mmHg, bpm */
       int64_t timestamp;                        /* UTC epoch seconds, 0 if absent */
       bool   irregular_pulse;
       bool   units_kpa;                         /* caller converts; store is mmHg */
   } bp_measurement;
   /* Decodes a 0x2A35 Blood Pressure Measurement. Returns false on malformed input. */
   bool bp_decode_2a35(const uint8_t *buf, size_t len, bp_measurement *out);
   ```
2. Implement per the SIG spec. Flags byte first, then three SFLOATs, then optional fields **in this
   order**: timestamp (7 bytes), pulse (SFLOAT), user id (1 byte), measurement status (2 bytes).
   ```c
   /* flags: bit0 units kPa(1)/mmHg(0), bit1 timestamp, bit2 pulse,
             bit3 user id, bit4 measurement status */
   if (len < 7) return false;              /* flags + 3 SFLOATs minimum */
   uint8_t flags = buf[0]; size_t off = 1;
   out->units_kpa = flags & 0x01;
   uint16_t raw;
   raw = buf[off] | (buf[off+1] << 8); off += 2;
   out->has_systolic  = vitals_sfloat_decode(raw, &out->systolic);
   raw = buf[off] | (buf[off+1] << 8); off += 2;
   out->has_diastolic = vitals_sfloat_decode(raw, &out->diastolic);
   raw = buf[off] | (buf[off+1] << 8); off += 2;
   out->has_map       = vitals_sfloat_decode(raw, &out->map);
   if (flags & 0x02) { if (len < off+7) return false; /* parse timestamp */ off += 7; }
   if (flags & 0x04) { if (len < off+2) return false;
                       raw = buf[off] | (buf[off+1] << 8); off += 2;
                       out->has_pulse = vitals_sfloat_decode(raw, &out->pulse); }
   if (flags & 0x08) { if (len < off+1) return false; off += 1; }
   if (flags & 0x10) { if (len < off+2) return false;
                       uint16_t st = buf[off] | (buf[off+1] << 8); off += 2;
                       out->irregular_pulse = st & 0x0004; }
   ```
   Every `len` check is required — this is attacker-controlled radio input.
   If `units_kpa`, convert to mmHg (`× 7.50062`) **at this boundary**, so the store only ever holds
   mmHg for these types.
3. Tests in `test_bp_decode.c`: a real capture from the cuff; a frame with all optional fields
   absent; a frame with SFLOAT `0x07FF` in systolic, asserting `has_systolic == false` and that
   `systolic` is untouched; and a truncated frame at every length from 0 to 20, asserting `false`
   and no read past `len`.
4. `fuzz_bp_decode.c` passing the raw buffer straight to `bp_decode_2a35`. Run under ASan.
5. `bp_collector.c`: connect to the cuff over BlueZ, **write `0x0002` to the CCCD** (indication, not
   notification — `0x0001` gives a successful subscription that silently delivers nothing forever),
   decode each indication, and `vitals_append` one sample per present field with
   `source = VITALS_SOURCE_AD_UA651BLE`.
   The cuff buffers ~30 readings internally, so the collector drains its memory on connect rather
   than requiring presence at measurement time.
6. Install `vitals-bp.service`, `WantedBy=multi-user.target`, `Restart=always`.
7. **Take a real blood pressure reading.** Confirm it lands.

**Verify:** `ssh chris@minidude 'meson test -C ~/vitals/build bp_decode --print-errorlogs && ~/vitals/build/fuzz_bp_decode -max_total_time=60 && psql -U vitals -tAc "SELECT count(*) FROM samples WHERE source=6 AND type=6" | awk "\$1 > 0"'`
**Expect:** exit 0; tests pass; fuzzer clean at 60 s; at least one systolic row present with
`source = 6`.

**Commit:**
```sh
git add vitals/src/bp_decode.c vitals/src/bp_decode.h vitals/src/collectors/bp_collector.c vitals/tests/test_bp_decode.c vitals/fuzz/fuzz_bp_decode.c
git commit -m "feat(T-D6): A&D UA-651BLE collector via standard 0x1810 blood pressure service"
```

---

## Acceptance — this change is done when

```sh
ssh chris@minidude '
  findmnt -no SOURCE / | grep -q nvme &&
  hciconfig -a | grep -c "^hci" | grep -qx 1 &&
  test -e /dev/watchdog &&
  systemctl is-active vitalsd vitals-bp &&
  sudo -u postgres pgbackrest --stanza=vitals check &&
  jq -e ".ok == true" /var/lib/vitals/restore-verify.json &&
  meson test -C ~/vitals/build &&
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
