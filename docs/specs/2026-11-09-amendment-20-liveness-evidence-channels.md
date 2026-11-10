# Amendment 20 — Liveness has three evidence channels, and ids are permanent

**Date:** 2026-11-09
**Status:** proposed, awaiting operator acceptance
**Supersedes:** `SPEC-admin-console.md` §2.5 rows "Cadence measures what?" (clarified) and
"Event-driven sources" (changed); `vitals/registry.toml` source id 4 (restored)
**Follows:** amendment 19 (samples storage and identity), which is itself still proposed

---

## Why this exists

On 2026-10-30 the `AirSense` collector died. It was noticed on 2026-11-08, ten days later, by a
person looking at a chart — not by the silence detector, which ran every thirty seconds throughout
and emitted 15,628 events while the outage was in progress. None of them said anything had changed,
because nothing had: the source was already `UNKNOWN` before the collector died and stayed `UNKNOWN`
after. Total data loss and healthy operation rendered identically.

The detector was not broken. It was comparing two different physical quantities and the spec did not
distinguish them.

### The mechanism, in order

1. §2.5 says **"Cadence measures attempts."** That is correct and remains correct.
2. **Nothing has ever written an attempt row.** `attempts` has been empty since it was created.
3. So the detector substituted: when the newest sample was newer than the newest attempt, it wrote
   the *sample's* timestamp into `Evidence::last_attempt_epoch_s`.
4. It then compared that age against `expected_cadence_s` — which for the `AirSense` is the **sample
   rate** of 1 Hz, and for the H10 is a **notification rate**, also 1 Hz.
5. Store-arrival lag has p50 35 s and max 44 s (measured 2026-11-09, n=36 over 18 minutes, ~30 s
   commit cycle). Against a bound of 1 second, a perfectly healthy source is permanently stale.

So the substitution was not a shortcut that usually worked. It was the only evidence path in the
system, and it compared a quantity in seconds-since-commit against a quantity in samples-per-second.

### The second half

`UNKNOWN` had no upper bound. 921,331 seconds of silence scored exactly the same as a thirty-second
gap. The three-state rule says a missing signal "never escalates on its own", and that is right for
*absence of evidence*. Silence three orders of magnitude past any declared bound is **evidence of
absence**, which is measured-and-bad.

---

## What this amendment decides

### 1. Sample arrival is a declared quantity, with no fallback

Two new per-source registry fields, `max_sample_arrival_s` and `sample_silence_failure_s`, bound how
stale the newest **stored sample** may be. They are not `expected_cadence_s` and must never be
derived from it.

**A source with no declared arrival bound gets no sample-derived verdict at all.** The sample channel
stays silent and the attempt channel speaks alone.

This is the part most likely to be re-broken, so it is stated as a prohibition: an earlier draft of
this change fell back to `attempt_cadence` when no arrival bound was declared, to avoid regressing
the seven sources that declare none. Review executed both branches and found the fallback reproduced
the original defect exactly for `polar_h10` and `polar_h10_emulator` — same state, only the noun in
the reason string changed — while reading as though it had been fixed. **Not having a measurement is
a fact to expose, not a gap to fill with the nearest available number.**

### 2. Only an *observed* idle state suppresses a silence verdict

A third state is added for the question *"should data be arriving right now?"*: `Expected`, `Idle`,
`Unknown`. It exists for devices that are legitimately silent some of the time — an SD-card import, a
cuff, a therapy device that only streams while in use.

**`Unknown` does not suppress.** It falls back to the registry declaration, because declaring
`sample_silence_failure_s` *is* the assertion that this source produces continuously. A device that
is genuinely intermittent declares no silence bound, or reports its activity.

Getting that backwards is a defect this amendment already shipped once and is corrected here. The
first draft treated "not measured" as equivalent to "idle", which let a missing gate veto a declared
bound and measured silence — and since nothing reports activity today, every source sat at the
`Unknown` default and no silence bound could ever fire. That disabled the only escalation that
detects a dead collector. The three-state rule says a missing signal never escalates *on its own*; it
does not say a missing signal may silence evidence that does exist.

### 3. Transport contact is a third evidence channel

`max_link_silence_s` bounds the age of the last transport-level contact — for the `AirSense`, the
host receipt time of the last BLE envelope. It is the **only** bound permitted to escalate while a
source is `Idle`.

It is the fallback for a device that stops producing legitimately, where sample silence cannot
distinguish idle from dead. `process_envelope` records receipt before any decode, so an envelope
counts even when it carries nothing storable. Measured rate: 4.77/s, steady across two separate
windows.

**No value is declared, and the `AirSense` does not need one.** That device streams continuously —
see the measurement correction below — so `sample_silence_failure_s` carries detection for it. This
field stays undeclared until a device needs it, rather than being given a number to look complete.

### 4. On-demand sources are bounded by event age, not stream arrival

§2.5's "Event-driven sources" row said on-demand state is "never time-degraded". That is amended to
**never time-degraded *to failing***, with two changes:

- An on-demand source's newest **event** is an attempt *or* a stored sample, whichever is newer.
  Counting only attempts made a blood-pressure reading taken one second ago render as "never
  completed an attempt" for four of nine sources — measured-and-fresh read as never-measured.
- Past a declared `max_event_age_s` the state is `UNKNOWN`, not `LIVE`. §2.5 requires only that an
  old timestamp not read as a *fault*; that is not a licence to read as healthy. Two sources were
  reporting `LIVE` on successes weeks old.

`max_event_age_s` is a **separate field** from `max_sample_arrival_s`. An earlier draft used the
latter for both, which compared a stream-arrival bound against an attempt timestamp — the same
conflation this amendment exists to remove, at a fresh site.

`sample_silence_failure_s` is **invalid** on an on-demand source and is rejected by name at build
time, as well as exempted in the evaluator.

### 5. A future-dated timestamp is rejected, never clamped

A sample or contact timestamp more than one second ahead of `now` is unusable and is named as such.
One second absorbs clock granularity: the detector takes one `now` per sweep and a 1 Hz source
writes rows after it.

Beyond that it is a clock fault, and clamping it to a plausible age pinned a source `LIVE` for the
whole clock excursion while disabling the only escalation path. The box has a known RTC-battery
fault. Per the no-clamping rule, this is rejected with a reason, not absorbed.

### 6. Source ids are permanent, and the guard is in the database

Source id 4 was reassigned from `wellue_ks60fwb` to `acconeer_xe125` on 2026-09-28 (#56) while
holding **156,201 sample rows** — 52,071 `hr`, 52,071
`spo2`, 52,059 `respiratory_rate`. `acconeer_xe125` is a 60 GHz radar sensor that declares only
`respiratory_rate` and cannot measure blood oxygen at all, because that requires optical pulse
oximetry.

The reuse was detectable only because reconciliation had not run since. `registry.sql` upserts with
`ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name`, which cannot distinguish a rename from a
reuse; the next run would have relabelled those rows and destroyed the evidence.

Decided:

- **Ids are permanent.** A retired device keeps its id forever; the replacement takes the next free
  one. `wellue_ks60fwb` is restored at id 4 with `retired = true`; `acconeer_xe125` moves to id 9.
- **A `retired` source stays declared** so historical rows keep resolving to the device that wrote
  them, stays visible in the health view so nothing silently vanishes from monitoring, and is
  labelled `RETIRED` rather than evaluated. There is no device to assert health about, and a
  permanently-`UNKNOWN` card teaches an operator to stop reading cards.
- **The guard lives in generated SQL, not in the generator.** Only the database knows whether an id
  has rows. It raises with the id, the stored name, the declared name and the row count. A `WHERE`
  clause on the `DO UPDATE` would fail closed *silently*, which §"Guards must say why" forbids.

### 7. Liveness bounds must reach every artifact

The bounds are emitted into `registry.json` as well as `generated.rs`. While they existed only in
the Rust artifact, every non-Rust consumer still saw `expected_cadence_s` as the sole
staleness-relevant number — a sample rate for the 1 Hz sources. A second implementation of this
defect in the phone client is still this defect.

`validate()` checks each emitted bound arm against `registry.toml` by value. `--check` cannot catch
a broken generator: it regenerates and diffs against the committed files, so a generator that
silently emitted `None` for every bound would pass while the `AirSense` returned to its pre-fix
behaviour.

---

## Measurement method, corrected twice

**First error — the wrong distribution.** The first draft justified its bounds with "18,650,231
inter-sample gaps since 2026-10-20: p50 0 s, p99 0 s". `samples.timestamp` is host receipt at
collection, so batching does not create gaps in it; the quantity the detector compares is
`now() - max(samples.timestamp)` — **store-arrival lag**. Percentiles of the wrong distribution.

**Second error — a duty cycle inferred two different wrong ways.** The same 18.65M figure was then
read as 25.4 rows/s against a declared 81 rows/s, i.e. a low duty cycle. But that window *contains
the ten-day outage*, so the average is low for the obvious reason. A three-day hourly histogram
appeared to confirm it — samples in two hours only — but that was read in UTC while
being reasoned about in PST, and the only populated hours were the ~2.5 hours since the collector had
been restarted.

**What is actually true.** Hourly counts for type 10 across two full pre-outage days
(2026-10-21/10-22, local time) are populated in **48 of 48 hours at ~84,000 rows/hour**, minimum
79,636. The device streams continuously at 23.3 rows/s while powered and paired: a **100% duty
cycle**. The "62 false `FAILING` notifications a day" concern was therefore unfounded, and
`sample_silence_failure_s = 1800` is well justified — a repeat of the 2026-10-30 outage is `FAILING`
within half an hour.

**Any liveness bound must be derived from the quantity the evaluator actually compares, measured at
the cadence it actually runs.** The declared values come from polling `now() - max(timestamp)` every
30 s for 18 minutes (n=36): min 5 s, p50 35 s, p90 42 s, max 44 s, mean 24.2 s. So
`max_sample_arrival_s = 300` carries about 6.8× headroom over the observed maximum.

---

## Known open, not closed by this amendment

| Open item | Consequence today |
|---|---|
| `max_link_silence_s` undeclared | None for the `AirSense`, which streams continuously; needed only for a device that stops producing legitimately |
| `activity` and `last_link_epoch_s` not read from the collector status file | No effect on escalation — `Unknown` falls back to the declaration. An observed `Idle` would be needed only for an intermittent device |
| Nothing writes attempt rows | Seven of nine sources report `UNKNOWN / "never completed an attempt"`. Honest, and not useful |
| Arrival bounds measured for `resmed_air11` only | Every other source's sample channel is silent, correctly |
| `expected_cadence_s` still carries two meanings (sample rate for `polar_h10`, sync interval for `healthkit`) | Not split here; no longer load-bearing for staleness |
| ~~Six `AirSense` clinical channels reporting null~~ — **withdrawn 2026-11-09, misdiagnosed** | Not a defect and nothing is lost. The device sends those six as explicit JSON `null` when they have no value and `build_signal` correctly stores nothing. Absence contributing nothing is the rule working |
| **`samples.type` holds two incompatible numbering schemes** | The deployed `AirSense` collector was built from an unmerged Pi-local worktree declaring its types at ids 9–23; `#52` independently gave ids 9–29 to `HealthKit` types. Both have rows: ~103M `AirSense` and ~14k `HealthKit` share ids 9–21, so id 15 means both minutes and L/min. No renumbering repairs it — `samples` is append-only and both sides are committed. **Owner decision required**; the generated `registry.sql` now refuses to relabel a type id that has rows |

With the activity fallback corrected, **the 2026-10-30 outage is now detectable within 1800 seconds**
and there is a regression test that replays it. What remains open is breadth, not detection: six of
nine sources have no measured arrival bound, and nothing writes attempt rows, so those sources report
`UNKNOWN / "never completed an attempt"` — honest, and not yet useful.

Beyond that, the evaluator no longer reports a healthy source as stale, no longer reports a measured
source as never-measured, no longer calls an observed-idle device broken, and has no path that
silently invents a threshold.
