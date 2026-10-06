# Bearable versus AnxietyWatch v1 and AnxietyWatch2

**Candidate:** Bearable (`bearable`) — Bearable Ltd, UK
**Comparison prompt:** `PROMPT_2.md` (base variant `../PROMPT2-USER-VALUE.md`)
**Research date:** 2026-10-06
**Implementation inspection:** not performed — see `bearable_IMPLEMENTATION_INSPECTION.md`

Evidence labels used below: **[claim]** public vendor claim · **[doc]** official Bearable
documentation · **[observed]** third-party or user report · **[v1]** AnxietyWatch v1
documented/implemented · **[v2c]** AnxietyWatch2 current · **[v2t]** AnxietyWatch2 specified target ·
**[inference]** labelled deduction · **[unknown]** not establishable · **[not supported]** documented
limitation or documented absence.

---

## 1. Plain-language executive summary

**Bearable helps a person notice patterns in their own health.** You tap in how you feel and what you
did — mood, symptoms, energy, sleep, medications, factors, notes — and it shows you which of those
factors coincided with better or worse days. Its explicit purpose is to make you better prepared for a
medical appointment and more able to see what improves or worsens your symptoms. It is a *subjective
data product with imported objective context*. It is not a monitor. It does not watch anything. It has
no sensors, no live data, no alarm, and no CPAP or oximetry support ([doc], §5–§8 of the candidate
file).

**AnxietyWatch v1 helps a person put what they feel next to what their body measured.** You log
anxiety severity in seconds during a panic episode, log a medication dose and are prompted again thirty
minutes later, and the app pairs that with your own HealthKit history, a Polar H10 chest strap
streaming beat-to-beat RR intervals, CPAP SD-card data, and an EMAY overnight oximeter — then produces a
personal baseline comparison and a clinical PDF for the psychiatrist ([v1]).

**AnxietyWatch2 is a different kind of thing: an always-on local monitoring platform.** A Raspberry Pi
collects BLE sensor data overnight, preserves the provenance of every sample, and is being built toward
detecting nocturnal CNS depression and sounding an alarm that does not depend on a phone or the
internet. The collectors, decoders, append-only store and AirSense 11 live path exist; the detector and
the alarm are **designed but not built** ([v2c], [v2t]).

**Verdict: Bearable is a partial competitor to AnxietyWatch v1 and not a competitor to AnxietyWatch2.**
It overlaps the v1 *journal-and-correlate* loop closely enough that a feature-by-feature comparison is
meaningful, but it has no sensor capture, no respiratory data, no PDF reporting, no local-first
storage and no alarm. Against AnxietyWatch2 it shares essentially nothing: no always-on collection, no
provenance model, no live CPAP, no local console, no three-state missing-data rule.

**Largest unresolved uncertainty.** Bearable's correlation engine is the whole product, and we could
not inspect the binary (no lawful artifact — see §3). We therefore cannot say whether the correlation
layer runs on-device or server-side, whether arbitrary imported metrics like HRV can become outcomes
(Q2), what happens on a day with a partially-covered imported metric (Q3), or what exactly leaves the
device. Those are the questions that would most change the comparison, and they remain open.

---

## 2. Audience and operating model

| Dimension | Bearable | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Intended user | Anyone with a chronic or episodic condition; anxiety is one of many listed conditions | Someone with anxiety or panic disorder who wants their physiology alongside their experience, and a better psychiatrist appointment | A technical owner who wants overnight physiological monitoring they control, with an alarm path eventually |
| Required devices | A phone. Nothing else. No wearable, no sensor, no hub | iPhone + Apple Watch, plus optional Polar H10, EMAY oximeter, CPAP SD card | Raspberry Pi 5 (Linux), BLE sensors, plus a thin Apple client |
| Technical skill | None | None | High — Linux, Docker/deployment, PostgreSQL backups |
| Setup burden | Install, create an account, start tapping. Minutes | Install, grant HealthKit permissions, optionally pair sensors; optionally stand up a sync server | Days to weeks. Sensors, dongles, broker, database, backups, deployment |
| Ongoing attention | Daily check-in(s). The product's value is proportional to logging discipline | Daily logging plus sensor use on chosen nights | Low once running, but it must *keep* running. This is a server, not an app |
| Account | **Required** — email/password or Google/Apple/Facebook [doc] | None | None for the Pi; local network access |
| Subscription | Free tier is genuinely usable; correlation reports, the correlations grid, 60/90/365-day ranges, unlimited goals/experiments/notes/gratitudes and custom health measurements are Premium [doc] | Free, open source (MIT), no subscription | Free, open source; hardware, electricity and optional AI API cost |
| Where it works | Anywhere with a phone, online; degraded without connectivity | On-device; sync only to a server you run | On your LAN/home; explicitly designed so the alarm path does not need the internet |
| Claim vs observed | **Inspection not performed.** All Bearable behaviour below is public claim or documentation, not verified runtime behaviour | — | — |

**One practical asymmetry worth stating plainly.** Bearable produces value on day three. AnxietyWatch2
produces value after you have built it, deployed it, worn sensors overnight, and accumulated a personal
baseline. They are not competing for the same effort budget.

---

## 3. Claim-versus-implementation reconciliation

**Implementation inspection was not performed for Bearable.** Reason, recorded in
`bearable_IMPLEMENTATION_INSPECTION.md`: Bearable is closed-source and store-only, no third-party APK
mirror was used (redistribution is not authorization), and obtaining an artifact from a device would
require the owner's store credentials plus — for anything past the sign-up screen — creating a Bearable
account, which the research brief forbids.

What this does and does not mean:

- It does **not** mean Bearable's claims are false. Most of them are documented in detail by the vendor
  and are corroborated by scale (500K+ Play Store downloads, 10.3K reviews, an active support base).
- It **does** mean that for every capability below, the evidence is *what the vendor says*, not what the
  code does. Where the comparison hinges on internals — where correlation is computed, what is sent to
  the server, what the CSV actually contains — the answer is **[unknown]**.

| Feature | Public claim | Static evidence | Runtime observation | Conclusion |
|---|---|---|---|---|
| Mood/symptom/factor logging | Claimed and documented | *not obtained* | *not observed* | Unresolved by inspection; publicly demonstrated at scale |
| Medication logging + reminders | Claimed and documented | *not obtained* | *not observed* | Unresolved by inspection |
| Correlation / Factor Effect reports | Claimed, documented, Premium-gated | *not obtained* | *not observed* | Unresolved by inspection |
| Free CSV export of all data | Documented with the in-app path | *not obtained* | *not observed* | Unresolved by inspection |
| PDF export | **Not claimed** — listed as an outstanding community request | *not obtained* | *not observed* | Absent from public evidence |
| Apple Health / Health Connect import | Claimed and documented, read-only | *not obtained* | *not observed* | Unresolved by inspection |
| Weather sync | Explicitly **not supported**; manual factors only | *not obtained* | *not observed* | Vendor-documented limitation |
| CPAP / oximetry / BLE sensors | Not claimed anywhere examined | *not obtained* | *not observed* | Absent from public evidence; **not** confirmed absent in the binary |
| AI / LLM analysis | Not claimed | *not obtained* | *not observed* | Absent from public evidence |
| Server-side encryption of health data | Claimed in the privacy policy | *not obtained* | *not observed* | Unresolved; not externally verifiable |
| No sale of personal data | Claimed repeatedly | *not obtained* | *not observed* | Unresolved; legal-document claim |
| Third-party data sharing | Play Store data-safety declaration names Location + Personal info + 2 others | *not obtained* | *not observed* | Unresolved; **contradiction worth reconciling** (Q7) |

**Toolchain note.** The free inspection stack was validated end-to-end against a disposable emulator
running only Android system components: mitmproxy 12.2.3 on `127.0.0.1:8080`, emulator proxy
`10.0.2.2:8080`, flows captured, HTTPS opaque because no CA was installed (by design), proxy removed and
verified `null` afterwards. The blocker for Bearable is artifact authorization, not tooling.

---

## 4. Feature-level comparison

`[v1]` = documented/implemented · `[v2c]` = current · `[v2t]` = specified target · Bearable per the
evidence labels.

| Capability | AnxietyWatch v1 | AnxietyWatch2 current | AnxietyWatch2 specified target | Bearable | Evidence/status notes |
|---|---|---|---|---|---|
| Anxiety severity logging | Yes, 1–10 with notes/tags, plus watchOS quick-log `[v1]` | Via Apple client (thin) `[v2c]` | — | Yes, as a symptom/emotion; no panic-specific flow `[claim][doc]` | Bearable has no validated anxiety instrument and no panic workflow |
| Panic-time interaction | Designed for it: colour-graded grid, haptic confirm, "under five seconds" `[v1]` | — | — | Standard check-in screens `[claim]` | Compare directly in §5.1 |
| Random/unprompted check-ins | Configurable random push notifications `[v1]` | — | — | User-scheduled reminders `[doc]` | Not randomised sampling |
| Symptom / factor tracking | Correlations over health metrics `[v1]` | — | — | **Much stronger**: unlimited custom symptoms, groups, custom feelings, custom scales `[doc]` | Bearable wins this row clearly |
| Journal / notes / free text | Notes per entry `[v1]` | — | — | Notes, gratitude, significant events; "Extra Notes" unlimited only on Premium `[doc]` | Roughly comparable |
| Songs / lyrics (earworm) capture | Yes, with server-side Genius/Musixmatch lyric lookup `[v1]` | — | — | **Not found** | Genuinely unique to v1 |
| Medication logging | Yes `[v1]` | — | — | Yes, with dose strength, per-dose time/count, reminders, adherence calendar `[doc]` | Bearable's adherence UI is stronger |
| Dose-triggered before/after efficacy | **Yes** — rate at dose, prompt again at 30 min; paired N-of-1 style data `[v1]` | — | — | **No.** Correlational review of meds against outcomes `[doc]` | The single most important v1 differentiator in this row. Bearable's vendor explicitly warns against medication decisions from its correlational reports `[doc]` |
| HealthKit ingestion | 20+ types, actor-isolated, anchored background queries, per-sample source attribution `[v1]` | Apple client `[v2c]` | — | Apple Health / Google Health Connect, **read-only, one-way** `[doc]` | Bearable has no per-sample provenance; Health Connect source-priority implies a single de-duplicated value per metric `[inference]` |
| Direct BLE sensor capture | Polar H10 raw RR at ~1 Hz, background mode, state restoration `[v1]` | BLE collectors + pure decoders `[v2c]` | — | **None documented** | Bearable cannot capture anything itself |
| Raw sample storage + provenance | Yes `[v1]`; `(source,type,session_id,seq)` identity `[v2c]` | Yes `[v2c]` | — | **No** — imported summaries only `[claim]` | |
| HRV | Apple Watch + Polar-derived RMSSD/SDNN/pNN50, LF/HF; personal 30-day baseline with below-baseline alert `[v1]` | Registry types `[v2c]` | — | Visible only if the health platform exposes it; not named by the vendor `[observed]` | Bearable has **no personal physiological baseline** — its comparison is factor-present vs factor-absent against a subjective outcome `[doc]` |
| SpO₂ | HealthKit average; EMAY import + live BLE streaming `[v1]` | EMAY decoder `[v2c]` | — | Not named by the vendor; only via health platform `[unknown]` | |
| CPAP / AirSense / AHI / leak | SD-card import, AHI, leak rate, usage hours, pressure stats, event breakdowns; OSCAR CSV; server-side EDF leak parsing `[v1]` | **Real-time AirSense 11 live stream**, bounded read-only path `[v2c]` | — | **None.** A site search for "cpap" returns no product article; no store or support page mentions CPAP `[not supported]` | Absence of evidence, not proof of absence in the binary |
| Respiratory rate | Via HealthKit sleep metrics `[v1]` | XE125 radar decoder `[v2c]` | — | Not documented `[unknown]` | |
| Sleep | Stages from HealthKit; CPAP-linked next-day anxiety analysis `[v1]` | Overnight collection `[v2c]` | — | Native quality/quantity logging + platform import `[doc]` | Different resolution: subjective rating vs staged/streamed data |
| Always-on / overnight capture | Polar H10 background recording `[v1]` | **Yes** — collectors + `vitalsd` on the Pi `[v2c]` | — | **None** | |
| Barometric pressure / weather context | Core Motion barometric pressure; environmental sound levels `[v1]` | — | — | Manual weather factors only; **explicitly no weather sync** `[doc]` | |
| Aircraft / environmental-noise attribution | — | — | — | — | **[Unresolved] in all three.** No product in this comparison addresses it; see §5.5 |
| Correlation engine | Physiological correlation engine identifying which health metrics most influence anxiety `[v1]` | — | Quality gate → personal baseline → max-not-average fusion → tier machine `[v2t]` | Factor-vs-outcome comparison: ≥3 days with, ≥3 without, timing-aware; 1–7 day effect windows `[doc]` | Bearable's engine is broader in *inputs you can define* and narrower in *what it can conclude*. No confounder control — the vendor's own "acts of kindness" example shows a false attribution surfaced to the user `[doc]` |
| Personal physiological baselines | Yes, rolling, compared to self not population `[v1]` | Baselines per registry types `[v2c]` | Personal baseline in the tier machine `[v2t]` | **No physiological baseline** | Central architectural difference |
| Missing-data semantics | Three-state honesty: measured-good, measured-bad, **not measured**; never interpolated `[v1]`, enforced as a project invariant `[v2c]` | Yes `[v2c]` | Extends to fusion and tiers `[v2t]` | **Not documented**; blank-entry articles exist but their content was not retrieved `[unknown]` | Q3/Q4. This is the clearest philosophical gap |
| AI / LLM analysis | Opt-in server-side Claude analysis over a chosen date range, from an admin page on your own server `[v1]` | Server-side analysis `[v2c]` | — | **None claimed.** Free tier explicitly tells you to export CSV and use ChatGPT yourself `[doc]` | Bearable has no AI feature; it hands you a file |
| Clinical PDF report | Yes, first-class feature `[v1]` | — | — | **No.** PDF export is an outstanding community request; recommended workflow is screenshots + CSV `[doc]` | Largest functional gap vs v1 |
| Export | JSON/CSV date-range export; clinical PDF `[v1]` | — | — | Full CSV export on the free tier; 3/6/all-time ranges; category filtering `[doc]` | Bearable's raw export is good and ungated |
| Provider sharing | PDF to hand to a clinician `[v1]` | — | — | Share sheet (file/screenshot); no clinician view or link `[doc]` | |
| Manual data entry to the store | — | — | — | — | AnxietyWatch has no equivalent of Bearable's manual UI; this is a real Bearable advantage |
| Local alarm | Local alerts + `AlertsDeduper`/`AlertChannelUploader` `[v1]` | — | Supervised channels + hardware dead-man switch `[v2t]` | **None** | The v2t alarm **does not exist yet** |
| Survives phone/internet loss | Partially `[v1]` | Designed for it `[v2c]` | Yes, explicitly `[v2t]` | **No** — cloud account, phone-only | See §5.8 |
| Privacy / local-first | Local-first, self-hosted sync, no telemetry, no account `[v1]` | Local-first `[v2c]` | — | Cloud account required; server-side encryption claimed; telemetry conceded in the privacy policy; no self-hosting `[doc]` | |
| Cost | Free software; hardware optional | Free software; Pi + sensors + electricity | — | Free tier usable; Premium subscription; **no hardware cost at all** | Play Store IAP range US$4.99–$199.99 `[claim]` |
| Desktop / web client | iPhone + watchOS | Pi console + HTTP API `[v2c]` | — | **None** — explicitly not prioritised `[doc]` | |

**Capability presence is not maturity.** Reading the table: Bearable's rows are *mature and shipped*.
AnxietyWatch v1's rows are *built and demonstrated*. AnxietyWatch2's `[v2c]` rows are *built and
partially deployed* — the README states the TimescaleDB hypertable chain is not claimed as
verified-deployed. Every `[v2t]` row is *designed, not built*. The comparison must not be read as
three peers at the same stage.

---

## 5. User-journey narrative

### 5.1 Logging anxiety during an acute panic episode

**AnxietyWatch v1** treats this as the design centre. The watchOS quick-log is a colour-graded 1–10
grid with haptic confirmation, explicitly built so that "when your hands are shaking and your thinking
is clouded, you can still log how you feel in under five seconds" `[v1]`. The entry lands on the wrist
with no phone unlock.

**Bearable** requires the phone (or a widget) and a navigation into the check-in, then selecting mood,
then emotions, then optionally symptoms — a sheet of facets designed for a daily review, not a
five-second panic tap. There is no panic flow, no breathing tool, no crisis pathway, and no
speed-optimised interaction documented. This is a real and predictable usability gap for the panic
use case, and it is exactly the case AnxietyWatch was built around.

**AnxietyWatch2** has a thin Apple client `[v2c]`; the panic-interaction design lives in v1.

### 5.2 Understanding why last night's sleep was poor

**Bearable:** you rate your sleep quality and quantity in the check-in. If you also connect Apple
Health or Health Connect, sleep duration may arrive automatically. Then you use the comparison graph
(rotate the phone) to plot sleep quality against factors, or the premium Factor Effect report to rank
which factors correlated with better or worse sleep `[doc]`. The answer is: *"on days when I did X, my
sleep rating averaged Y."* Timing is considered, so an evening factor is not wrongly matched to a
morning outcome `[doc]`. What Bearable cannot tell you is anything about the *night itself* — no
staging, no respiratory events, no SpO₂ trace, no awakenings.

**AnxietyWatch v1:** HealthKit supplies sleep duration, stages, skin-temperature deviation and
respiratory rate; CPAP import supplies AHI, leak rate and event breakdowns; the EMAY oximeter supplies
overnight SpO₂; a Polar H10 night supplies minute-by-minute RMSSD, HF power and LF/HF `[v1]`. The
"Last Night Headline" view surfaces the night directly. The answer is a physiological account of the
night, not a correlation with a rating.

**AnxietyWatch2 current:** the Pi collected the night at sample level with provenance, and can stream
AirSense 11 data live `[v2c]`.

**Practical trade-off:** Bearable answers a *statistical* question about your ratings with very little
effort. AnxietyWatch answers a *physiological* question about the night with much more setup. If the
user's real question is "was I breathing badly?", only AnxietyWatch can answer it.

### 5.3 Whether CPAP or respiratory events mattered

Bearable: nothing. No CPAP import, no AHI, no leak, no respiratory rate, no SpO₂ claim `[not supported]`.
A user with sleep apnea who wants to connect therapy to next-day anxiety gets no support from Bearable
at all — they would need a second app (OSCAR or SleepHQ) and to hold both in their head.

AnxietyWatch v1: this is a headline feature — CPAP SD-card import with AHI, leak, usage, pressure and
event breakdowns, and explicit "connect sleep apnea treatment to next-day anxiety" framing `[v1]`.
AnxietyWatch2 current: real-time AirSense 11 live monitoring over BLE `[v2c]`.

**This is the sharpest single asymmetry in the comparison.** For an apnoeic anxiety patient,
Bearable is not a lesser alternative — it is missing the capability entirely.

### 5.4 Testing whether a medication helps

**Bearable:** you log doses (with dose strength, time and count) and set reminders. Its advanced
medication report shows adherence on a calendar and lets you correlate medications against symptoms,
mood and side effects `[doc]`. Crucially, **the vendor's own guidance tells you not to trust this for
medication decisions**: "because they're correlative and not causal we recommend not making any
decisions about your medication using Effect On reports in Bearable", directing users to their
clinician instead `[doc]`. The method is group comparison over days, not paired measurement.

**AnxietyWatch v1:** logging a dose triggers a prompt to rate anxiety at that moment, and a
notification asks again thirty minutes later. Over repeated doses this produces *paired before/after
observations around each dose* — the documentation frames it as "something closer to a personal N-of-1
trial than anything a consumer app typically produces" `[v1]`. The measurement is anchored to the
pharmacological event, not to the calendar day.

**Why the distinction matters.** A paired design controls for the day-level confounders that dominate
Bearable's comparison — a dose taken on a bad day looks harmful in a group comparison even if it
helped. Bearable is the more usable adherence tool; AnxietyWatch is the more informative efficacy tool.
Both are correlational in the end, and neither should be used to change a prescription without a
clinician.

### 5.5 Investigating aircraft or environmental noise as a sleep interrupter

**Nothing in this comparison addresses this**, and that is the honest finding. AnxietyWatch2's
project material raises aircraft and environmental-noise attribution as an idea `[Unresolved]`, with no
specified detector, no data source and no acceptance criteria. Bearable offers manual banded factors —
you can hand-create "aircraft noise: none/some/lots" and correlate it with sleep, which is a crude but
real workaround, and the vendor even suggests a similar pattern for air-quality bands `[doc]`.
AnxietyWatch v1 records environmental sound levels via HealthKit `[v1]`, which is a signal but not an
attribution method.

**Practical assessment.** Bearable's manual-factor approach can answer *"do noisy nights feel worse?"*
today, with manual effort. AnxietyWatch has better raw sensing but no specified attribution pipeline.
This is a case where the manual product is momentarily ahead of the open one — worth stating plainly
rather than assuming the sensor platform wins by default.

### 5.6 Combining notes, symptoms, medication, songs/lyrics, weather and physiology

**AnxietyWatch v1** is the only one of the three that can hold all of these together: notes and tags,
medication doses with paired follow-up, earworm/song capture with server-side lyric lookup, barometric
pressure and environmental sound from Core Motion, plus HealthKit, Polar, CPAP and oximetry `[v1]`.
The LLM analysis path can then be run over a chosen date range on your own server `[v1]`.

**Bearable** holds notes, symptoms, medications and manual factors very well — arguably better than v1
as a *logging* experience, with unlimited custom trackers and scales — but it lacks songs, barometric
pressure, environmental sound and any captured physiology `[doc][not supported]`. Weather is manual
only `[doc]`.

**AnxietyWatch2 current** holds the physiological stream with provenance `[v2c]`; its `[v2t]`
server-side analysis is designed to fuse the layers.

**Assessment:** Bearable wins on *breadth of what you can type in*; v1 wins on *breadth of what can be
measured*. A user who cares about songs, weather pressure or physiology has no Bearable path.

### 5.7 Producing something useful for a psychiatric or sleep-medicine appointment

**AnxietyWatch v1** generates a clinical PDF over a chosen date range `[v1]`. You hand over a document.

**Bearable** gives you CSV export on the free tier and tells you the best outcomes come from
*preparing* rather than exporting: review trend charts, use the symptom-prevalence and factor-breakdown
sections, and **screengrab** the trend and impacts reports to share with your doctor `[doc]`. PDF
export is listed by the vendor as an outstanding community request `[doc]`. The data is genuinely
available — the presentation layer for a clinician is not.

**Practical assessment.** Bearable's advice is not unreasonable, and its 30+ report types arguably give
a clinician more *angles* than a v1 PDF. But screenshots are a weaker artefact than a generated report:
no consistent date range, no cover page, no provenance of where each number came from, and it puts the
assembly work on a patient who is already fatigued. For the "fifteen-minute psychiatrist visit"
problem both projects care about, v1 produces the better handover artefact and Bearable produces the
better *exploration* tool.

### 5.8 When the phone, internet, cloud or account is unavailable

**Bearable: nothing still captures.** There is no sensing to stop, because there is no sensing `[doc]`.
What stops is the product's reasoning: with no account you are at the sign-up gate; offline, sync and
any server-side analysis lapse, and device migration depends on the cloud restore the account exists
for `[doc]`. Data entry may function locally, but the value layer is cloud-anchored `[inference]`.

**AnxietyWatch v1** keeps data on-device, continues Polar H10 background recording across
screen-off and memory-pressure termination using `bluetooth-central` background mode and state
restoration `[v1]`, and only needs the self-hosted server when you choose to sync `[v1]`.

**AnxietyWatch2** is designed for exactly this case, and it is the reason the project exists: the alarm
path must behave the same with every screen unplugged, and the v2 architecture deliberately treats a
viewer as a non-dependency `[v2c]`. The supervised alarm channels and hardware dead-man switch are
`[v2t]` — **designed, not built** — so this remains a goal, not a shipped guarantee.

**Honest summary:** for offline resilience, v1 and v2c are strictly better than Bearable today. For
the *alarm*, nobody in this comparison has shipped one for an anxiety or CNS indication.

---

## 6. Privacy, operating model and cost

**Where data lives.** Bearable stores health data **on the device and on Bearable's servers**, under
server-side encryption, tied to an email address, with cloud restore as the stated reason for the
account `[doc]`. AnxietyWatch v1 and AnxietyWatch2 store locally; sync goes to a server the user runs
`[v1][v2c]`.

**What the vendor concedes.** The privacy policy is unusually candid and specific: device data
(model, identifiers, settings, crash information) and event/usage data (which pages and tabs are
opened) are collected; IP address is collected via cookies for approximate location, with consent
`[doc]`. The website additionally loads Google Tag Manager and Google Ads/DoubleClick endpoints —
that is a *website* fact, not app telemetry, and must not be conflated with it.

**What the vendor promises, and the caveat.** No sale of personal data, ever, stated repeatedly and
extended to HealthKit/Google Fit data specifically for advertising and data-broker purposes
`[doc]`. No name, age, sex or gender collected, deliberately `[doc]`. The February 2025 policy update
clarifies that as a UK company it is not required to comply with US criminal subpoenas — that is a
statement about *legal obligation*, not a technical guarantee that data cannot be produced, and it
should be read as such `[inference]`.

**The unresolved contradiction (Q7).** The vendor's policy says data is not shared with third parties
"unless you specifically choose to enable that option", while the Play Store data-safety declaration
states the app "may share these data types with third parties: Location, Personal info and 2 others"
and "may collect ... Location, Personal info and 5 others" `[doc]`. These can be reconciled — platform
declarations are often conservative — but they have not been reconciled here, and neither statement is
verified.

**Telemetry and third-party SDKs.** Bearable concedes telemetry `[doc]`. AnxietyWatch projects claim
none: v1 states "no third-party SDKs, no analytics, no tracking pixels, no data collection of any kind"
`[v1]`. That is a claims comparison, not a verified one, but the architectural difference is real —
v1/v2 have no cloud account to report to.

**Export and lock-in.** Bearable: free full CSV export with 3/6/all-time ranges, account deletion
in-app, and an explicit acknowledgement that some users prefer burner accounts `[doc]`. That is a
genuinely portable posture. AnxietyWatch: JSON/CSV export plus a clinical PDF, and a self-hosted
database you already possess `[v1]`.

**If the service ends.** Bearable users retain their CSV and lose the app, the correlation layer, the
server-side restore and multi-device sync `[inference]`. AnxietyWatch users lose nothing, because there
is no service to end — which is the point of the design.

**Cost — separating free software from free operation.**

| | Bearable | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Software | Free tier, genuinely usable; Premium for correlation reports, correlations grid, longer ranges, unlimited goals/experiments/notes `[doc]` | Free, MIT `[v1]` | Free, open source `[v2c]` |
| Price | Not published by the vendor; Play Store US IAP range **$4.99–$199.99** `[claim]`. No lifetime option `[doc]` | $0 | $0 |
| Hardware | **$0** — phone only | iPhone + Apple Watch; Polar H10 (~$90) and EMAY oximeter optional | Raspberry Pi 5, BLE dongle, sensors, storage; non-trivial |
| Running cost | $0 | $0 (or a small VPS if you self-host sync) | Electricity, always-on; optional AI API charges `[v1]` |
| Hidden cost | The analysis you most want is behind the paywall; your data is on someone else's server | Your time and the sensor purchases | **Your time, and the risk that the alarm you are building does not exist yet** |

The honest framing: Bearable is cheaper and faster to start, and its free tier is more generous than
most competitors. AnxietyWatch is free but not free to operate, and AnxietyWatch2 has the largest
effort and cost of the three — in exchange for capabilities the others do not have at all.

---

## 7. Balanced conclusion

### Best-fit user

- **Bearable** — someone with a chronic or episodic condition who wants to understand patterns in their
  own symptoms with near-zero setup, is willing to log daily, does not own or want sensors, and is
  comfortable with a cloud account. For an anxiety patient whose question is *"what makes my bad days
  bad?"*, Bearable is the more immediately useful product, and it is honest about the limits of
  correlational inference.
- **AnxietyWatch v1** — someone with anxiety or panic disorder who wants their physiology next to their
  experience, owns (or will buy) a chest strap, oximeter or CPAP machine, values a clinical PDF, and
  wants their data to stay on their own devices. Especially: an apnoeic patient who wants therapy data
  connected to next-day anxiety, which Bearable cannot do at all.
- **AnxietyWatch2** — a technical owner who wants overnight monitoring they control, with provenance,
  and who accepts that the detector and the alarm are still ahead of them rather than behind them.

### Strongest overlaps

Daily subjective logging; medication tracking with reminders; a factor-versus-outcome correlation
engine; custom trackers so the app fits the person; and a shared framing that the product's payoff is a
better clinician conversation. In these areas the products are genuinely comparable and Bearable is
often the more polished implementation.

### Important gaps on both sides

**Bearable lacks:** any sensor capture or BLE support; CPAP, AHI, leak, oximetry and respiratory data;
per-sample provenance; personal physiological baselines; the three-state missing-data rule; a PDF
clinical report; any AI analysis; barometric/weather automation; aircraft/noise context; a panic-time
interaction; offline resilience; and self-hosting.

**AnxietyWatch lacks:** Bearable's breadth of custom trackers, custom scales and condition-specific
report types; a manual entry experience that does not depend on sensors; a mature multi-device cloud
restore; a shipped status at scale; and — for v2 — an actual detector and alarm. AnxietyWatch2 in
particular must not be described to a prospective user as an alarm system, because the alarm does not
exist.

### Competitor, complement, or neither

- Against **v1**: a **partial competitor**. Same user intent, same daily loop, different evidence
  base. It competes on the journal-and-correlate half and does not compete on the measure-and-report
  half.
- Against **v2**: **not a competitor**. No always-on collection, no provenance store, no live CPAP,
  no local console, no alarm architecture, no three-state honesty model. Different category.
- Against the **combined vision**: **complementary**. Bearable is a plausible source of the subjective
  layer that AnxietyWatch2 currently lacks, and AnxietyWatch is the objective, continuous,
  provenance-backed layer that Bearable will never be. A user could reasonably run both — and given
  that Bearable exports everything as CSV for free, that integration is technically reachable today.

### Confidence in these findings

**Well-supported:** the capability differences themselves. Every row rests on Bearable's own
documentation, its privacy policy, its store listings or its peer-reviewed citation, all dated and
linked in the candidate evidence file.

**Uncertain:** Bearable's internals — where correlation is computed, what enters the correlation engine
as an outcome, what actually leaves the device, what the CSV contains, and what happens on a day with
a partially-covered imported metric (Q2, Q3, Q5, Q6, Q7). These need an authorized inspection, and the
report says so rather than guessing.

**Explicitly not established:** whether Bearable's correlation engine produces statistically sound
inferences, whether the cited oncology systematic review rated Bearable favourably or merely included
it (Q9), and whether the "2M users" figure is accurate.

**One thing to say clearly.** This comparison does not show that AnxietyWatch is better than Bearable.
It shows they solve different problems with different trade-offs, and that Bearable is the more mature
product in the area where they overlap. The value of the exercise is the map, not a winner.
