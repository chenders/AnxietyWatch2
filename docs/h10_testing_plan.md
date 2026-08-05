# Polar H10 overnight testing plan (T-B1)

**Written 2026-08-05**, reconstructed from the 2026-08-04 session history and the task brief in
`openspec/changes/pi5-vitals-core/tasks.md` (§T-B1). That brief is the authoritative version; this
document is the "why and how" in one place for the night you actually wear the strap.

**Status when written:** `crates/hr-spike/` and `tools/gap_histogram.py` are built, committed, and
review-hardened. T-B1 is unchecked. The H10 strap was lost on Aug 4, so the EMAY oximeter was
queued as the interim overnight capture; the H10 run is still wanted in its own right — see "Why
the H10 specifically."

---

## What this is

**T-B1 — the eight-hour Polar H10 capture** is Stage 1 of the pi5-vitals-core plan: the *thesis
spike*. The entire project rests on the Pi holding a BLE connection all night, and this run
measures that **before** anything is built on top of it — which is why it sits ahead of the
storage and core work in the task order, and why `docs/HERDR-GUIDE.md` says "Do T-B1 before you
trust any of this."

The capture binary is throwaway; **the number it produces is not.** The gap histogram gets
recorded verbatim in `tasks.md` under `## Implementation notes` and becomes the project's
foundation figure.

## What the data is for

1. **Link-continuity proof.** One line per BLE notification lands in `night1.log`; the morning
   script diffs consecutive timestamps into gap buckets (`<2s, 2–10s, 10–60s, 60s–5m, >5m`) plus
   total duration, total RR count, and longest gap. **Abort criterion: if the longest gap exceeds
   5 minutes, stop and report before building anything further.**
2. **Clock-drift measurement** (Amendment 05 §1.4). Fit the cumulative RR sum against host arrival
   time; a rate outside ±0.5% is a sensor or host clock fault. Nothing else in the system measures
   this — the overnight run is the only chance to catch it.
3. **Dropped-notification estimate.** Not a `seq`-gap count — RR `seq` is defined as the count of
   intervals *received*, dense by construction, so `max(seq) − count(*)` is identically zero and
   detects nothing. Instead: the ratio of elapsed wall time to summed RR duration per connection,
   reported explicitly as an estimate.
4. **Live validation of the `0x2A37` decoder** in `vitals-decode` (flags byte, uint8/uint16 HR,
   energy-expended skip, RR at 1/1024 s) against a real device rather than fixtures.
5. **Connection-state taxonomy check** (Amendment 06 §3). The run must distinguish three states,
   not two: *device not found* (pairing fault), *out of range* (normal), and *bonded but
   `ServicesResolved` never fired within 10 s* — encryption could not resume, which looks exactly
   like a quiet night and is not one. `hr-spike` times that third state out explicitly.

## Why the H10 specifically

The H10 streams every heartbeat's RR interval at ~1 notification/second — the densest, most
gap-sensitive stream available, so it is the hardest honest test of link continuity, and its data
exercises the RR path end to end. The EMAY run answers "can the Pi hold *a* link all night"; the
H10 run additionally validates the RR pipeline that HRV work depends on.

**Role boundary (do not forget):** H10 heart-rate/RR data is a *corroborating* signal. Amendment
05 §2.1: only SpO₂ and respiratory rate can raise an alert tier past watch or clear a raised one —
HR/HRV must never launder a missing primary signal into "all clear." This capture is about the
radio and the pipeline, not about alarm inputs.

## Procedure (the night of)

Prerequisites: T-A4 green; `hr-spike` built for release on the Pi.

1. **Moisten the strap pads and put it on.** A dry strap does not advertise at all — this is also
   why scans on other nights never saw it.
2. **Pair once, out of band** (not `bluetoothctl trust` — LE Privacy rotates the address, so
   static-MAC trust fails silently; bonding stores the IRK, and the kernel accept-list does the
   connecting because a userspace poll can miss a short advertising window):
   ```sh
   sudo btmgmt find                              # note the H10's address
   sudo bluetoothctl pair <H10_MAC>              # bonding stores the IRK
   sudo btmgmt add-device -a 2 -t 1 <H10_MAC>    # action 2 = kernel auto-connect
   ```
3. **Start the capture under systemd-run so it survives the SSH session** — `--pipe`, an absolute
   ExecStart, and the redirect on `systemd-run` itself (without `--pipe`, stdout goes to the
   journal and the shell captures one banner line instead of eight hours):
   ```sh
   systemd-run --user --pipe --unit=hrspike --setenv=H10_MAC=<mac> \
     "$PWD/target/release/hr-spike" > ~/vitals/spike/night1.log
   ```
4. **Sleep.**
5. **In the morning**, the report and the gate are the same script, so the number read never
   drifts from the number that decides:
   ```sh
   ssh "$PI_HOST" 'cd ~/AnxietyWatch2 && python3 tools/gap_histogram.py \
     ~/vitals/spike/night1.log --require-duration-hours 8 --max-gap-seconds 300 \
     --require-drift-report'
   ```
   Exit 0 = T-B1 passes. Nonzero names which requirement failed (duration < 8 h, a gap > 300 s,
   or no drift figure computable).
6. **Record the histogram verbatim** in `openspec/changes/pi5-vitals-core/tasks.md` under
   `## Implementation notes`, and tick T-B1.

## What consumes the result

- **Stage 4 (the BLE fleet)** inherits its confidence from this number — every collector assumes
  the link behaviour this run measured.
- The observed drift figure calibrates whether device-side vs host-side timing can be trusted for
  beat-time reconstruction (Amendment 04 §1).
- The gap distribution informs the learned-staleness parameters the admin-console spec §8b adopts
  for BLE sources (declared cadence floor, learned interval refinement).
