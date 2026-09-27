# Tasks — 99-last-specs consolidation

This change holds recovered tasks until their final OpenSpec destination is selected. It does not
lower any task's acceptance bar. Read `docs/specs/AMENDMENTS.md` before working on any task.

## Migration ledger

| Task | Former change | Current status | Destination |
|---|---|---|---|
| T-B1 | `openspec/changes/pi5-vitals-core/tasks.md` | not started; physical overnight evidence required | pending |

- [ ] T-B1 — Eight-hour Polar H10 capture · implement-complex · pi · **review**

## T-B1 — Eight-hour Polar H10 capture

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~5k · **Depends:** T-A4
**Amends:** 06 (§1.2 seq scope, §3 bonded-but-unencrypted state) · 05 (§1.4 clock drift, §4.3 IRK pairing) · 04 (§1 RR beat-time reconstruction) · 03 (§1 tactile evidence)
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`

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
   truncated notification must be absent, never a partially-decoded reading.
3. Build with `cargo build --release -p hr-spike`.
4. Pair the H10 once, and **register auto-connect** (A05 §4.3, amendment 06 §3):
   ```sh
   # bluetoothctl's `scan on` emits nothing when its output is piped, and its
   # `connect` has been observed issuing no LE Create Connection at all. Use
   # `btmgmt` to discover and to register intent with the kernel.
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
   consecutive timestamps, and print count of gaps in each bucket `<2s, 2-10s, 10-60s, 60s-5m,
   >5m`, plus total duration, total RR count, and longest gap.
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

**Verify:** `ssh "$PI_HOST" 'cd ~/AnxietyWatch2 && python3 tools/gap_histogram.py ~/vitals/spike/night1.log --require-duration-hours 8 --max-gap-seconds 300 --require-drift-report'`
**Expect:** exit 0. The script must exit nonzero if duration < 8 h, if longest gap > 300 s, or if
`--require-drift-report` was passed and no drift figure could be computed. A flag a script may
accept and ignore is a check that cannot fail.

**Commit:**
```sh
git add openspec/changes/99-last-specs/tasks.md crates/hr-spike/ tools/gap_histogram.py
git commit -m "spike(T-B1): record 8-hour Polar H10 capture gap histogram"
```
