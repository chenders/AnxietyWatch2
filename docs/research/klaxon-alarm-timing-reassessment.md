# CNS Klaxon — alarm timing & gating re-assessment

Evidence-based review of when the CNS-depression "klaxon" should fire. This research dictates the fusion engine parameters implemented in `vitals-core` for AnxietyWatch2.

> A home klaxon is a *backstop*, not a treatment or a substitute for a discussion with the prescriber about the underlying medication and sleep-breathing treatment.

## The core reframe

The engine is tuned to **suppress false alarms**: MAX-not-average fusion, a hard binary quality gate, lone-source caps, corroboration caps, 30 s contiguous-good requirements. For a single-user life-safety alarm whose owner has chosen to err toward alarming, the balance is inverted: **nearly every "uncertainty" path resolves to "hold," and hold-from-clear is silence.** The fix philosophy is to make primary-signal uncertainty resolve *toward* the alarm.

## Two problems, and the silence bugs are the bigger one

### A. It can fail to fire at all (silence-in-error)

1. **Lone-oximeter dead zone.** With one primary source (a single continuous oximeter via `vitalsd`), a sustained *moderate* desat (SpO₂ 86–87%, severity ~0.67) is capped at risk 0.5. *Fix:* exempt a high-fidelity continuous **primary** source from lone-source damping.
2. **The crash starves its own detector.** A real severe desat is accompanied by peripheral vasoconstriction (low perfusion index). *Fix:* wire it — a below-onset reading at PI in [hardFloor, softFloor) is escalation-eligible.
3. **No dead-man's-switch.** Losing the only primary signal (finger off, perfusion collapse, Bluetooth link drop) makes every "can't assess" path **hold** the tier. *Fix:* losing the only primary while in a risk window must **escalate to a rousing alarm**, not hold; shorten detection well below 60 s.

### B. It fires too slowly

Reaching klaxon climbs clear→watch→confirm→klaxon one tier at a time. A reading at 70% waits exactly as long as one grazing each threshold. *Fix:* severity-scaled latency — see the config below.

## Recommended configuration (vitals-core parameters)

Latencies are *time from a signal-quality-valid trigger to klaxon*.

| Band | SpO₂ trigger | RR / apnea | HR | Time-to-klaxon |
|---|---|---|---|---|
| **CRITICAL** | severity ≈ 1.0 via personalized ramp **OR** absolute backstop **≤ 80%** | RR ≤ 5/min sustained **or** apnea > 30 s → **escalates independently** | HR < 40 corroborating | **~10–15 s**; **no confirm timer** |
| **MODERATE** | 86–89% sustained below baseline | 6–9/min sustained | 40–49 corroborating | **~20–45 s** |
| **MILD / WATCH** | 90–92% below baseline | ~10/min | mild | **~60 s watch; no klaxon** |
| **CLEAR** | — | — | — | slow **120 s** clear |

Key design rules:
- **Trigger = personalized floor OR absolute backstop (≤ 80%), take the union.**
- **Independent RR/apnea escalation.** Under PAP therapy or with respiratory-depressant medication, SpO₂ is a *lagging* sign.
- **Fast path shortens *every* rise hop**.
- **Fast to alert, slow to clear.** Never shorten the clear window; a raised alarm stays up.

## Calibration from continuous oximetry

Calibrate the personalized floor from the user's own high-fidelity continuous oximetry, recorded over multiple nights. Opportunistic smartwatch readings produce spurious deep nadirs and are noise. **Do not let low-fidelity opportunistic Watch data set alarm thresholds.** In AnxietyWatch2, the `vitals-core` Spool uses strict provenance weighting to discount these samples.

## Guardrails

- Compute fast-path severity only from **quality-gated good samples**.
- Require **~8–15 s of consecutive good critical-range samples** to reject glitches.
- Keep **MAX-not-average** primary fusion.
- Add outlier rejection to the nadir baseline.
