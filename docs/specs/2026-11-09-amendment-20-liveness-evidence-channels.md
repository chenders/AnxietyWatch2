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

### 2. Silence only counts while the source should be producing

A third state is added for the question *"should data be arriving right now?"*: `Expected`, `Idle`,
`Unknown`. Only `Expected` licenses a silence-derived verdict. An unobserved gate behaves as `Idle`.

This is not a refinement; without it the bounds in §1 are actively harmful. The `AirSense` is a
therapy device, not a continuous monitor. Hourly row counts across three days show samples in two
hours, both overnight; the duty cycle is near 31%, roughly 7.4 h/day. A 1800 s silence bound applied
without the gate produces **62 `FAILING` notifications on an ordinary non-use day** — the same
"outage and healthy are indistinguishable" defect, inverted, and a card nobody believes.

### 3. Transport contact is a third evidence channel

`max_link_silence_s` bounds the age of the last transport-level contact — for the `AirSense`, the
host receipt time of the last BLE envelope. It is the **only** bound permitted to escalate while a
source is `Idle`.

It exists because no sample-derived rule can distinguish "idle and healthy" from "collector dead"
during the sixteen hours a day a CPAP stores nothing. Transport contact can: `process_envelope`
records receipt before any decode, so an envelope counts even when it carries nothing storable.
Measured in-therapy envelope rate: 4.74/s, steady over 18 minutes.

**No value is declared yet.** The number requires the standby envelope interval, which has not been
measured — the device was in therapy throughout the measurement window. Until it is declared, the
`AirSense` cannot escalate on transport silence, and a repeat of the original outage would still go
undetected. That gap is recorded here rather than closed with a guess.

### 4. On-demand sources are bounded by event age, not stream arrival

§2.5's "Event-driven sources" row said on-demand state is "never time-degraded". That is amended to
**never time-degraded *to failing***, with two changes:

- An on-demand source's newest **event** is an attempt *or* a stored sample, whichever is newer.
  Counting only attempts made a blood-pressure reading taken one second ago render as "never
  completed an attempt" for four of nine sources — measured-and-fresh read as never-measured.
- Past a declared `max_event_age_s` the state is `UNKNOWN`, not `LIVE`. §2.5 requires only that an
  old timestamp not read as a *fault*; that is not a licence to read as healthy. Two sources were
  reporting `LIVE` on successes 52.8 days old.

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
holding **156,201 sample rows** written between 2026-08-30 and 2026-09-17 — 52,071 `hr`, 52,071
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

## Measurement method, corrected

The first draft of this amendment justified its bounds with "18,650,231 inter-sample gaps since
2026-10-20: p50 0 s, p99 0 s". That measured the wrong distribution. `samples.timestamp` is host
receipt at collection, so batching does not create gaps in it, and the quantity the detector
compares is `now() - max(samples.timestamp)` — **store-arrival lag**.

The same figure also refutes the claim it was offered to support. 18.65M rows over a ~743,000 s
window is 25.4 rows/s against a declared stream of 81 rows/s: a 31% duty cycle. Under genuine 24/7
streaming that window would hold roughly 60M rows.

**Any liveness bound must be derived from the quantity the evaluator actually compares, measured at
the cadence it actually runs.** The declared values come from polling `now() - max(timestamp)` every
30 s for 18 minutes (n=36): min 5 s, p50 35 s, p90 42 s, max 44 s, mean 24.2 s. So
`max_sample_arrival_s = 300` carries about 6.8× headroom over the observed maximum.

---

## Known open, not closed by this amendment

| Open item | Consequence today |
|---|---|
| `max_link_silence_s` undeclared (needs the standby envelope interval) | A repeat of the original outage is still not detected |
| `activity` and `last_link_epoch_s` not read from the collector status file | The activity gate is always `Unknown`, so no sample-derived escalation fires for any source |
| Nothing writes attempt rows | Seven of nine sources report `UNKNOWN / "never completed an attempt"`. Honest, and not useful |
| Arrival bounds measured for `resmed_air11` only | Every other source's sample channel is silent, correctly |
| `expected_cadence_s` still carries two meanings (sample rate for `polar_h10`, sync interval for `healthkit`) | Not split here; no longer load-bearing for staleness |
| Six `AirSense` clinical channels silent since 2026-10-29 with no tombstone and no log line | `respiratory_rate`, `leak`, `tidal_volume`, `minute_ventilation`, `snore_index`, `flow_limitation` are not being stored, invisibly. Separate defect |

The first three are the reason this amendment does not claim the outage is now detectable. It claims
the evaluator no longer reports a healthy source as stale, no longer reports a measured source as
never-measured, no longer calls an idle device broken, and no longer has a path that silently invents
a threshold.
