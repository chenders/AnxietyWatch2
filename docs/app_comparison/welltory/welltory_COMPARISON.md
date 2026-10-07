# Welltory versus AnxietyWatch v1 and AnxietyWatch2

**Candidate:** Welltory (shortlist rank 2 of 10)
**Slug:** `welltory`
**Research date:** 2026-10-06
**Base prompt:** `PROMPT_2.md` (variant: `../PROMPT2-EVIDENCE-FIRST.md`)
**Inputs:** `welltory_CLAIMED_FEATURES_AND_PURPOSE.md`, `welltory_IMPLEMENTATION_INSPECTION.md`

> **Reading note.** AnxietyWatch v2 has three states in this report and they are never merged:
> **V2 current** exists in the repository today; **V2 specified target** is fully specified in
> `openspec/` but is not shipped; **Unresolved** is unknown. Welltory has one state — shipped — plus a
> set of documented claims, a partially inspected public research surface, and no runtime observation.

---

## 1. Executive finding

**Strongest supported conclusion.** Welltory is the only candidate on this shortlist that shares
AnxietyWatch's central methodological commitment — measure against *this person's* baseline rather than
a population threshold — and it is the only one that both publishes the research behind its accuracy
claim and states the limitation of that commitment in its own documentation. Its PPG work was validated
against **Polar H10**, the same reference sensor AnxietyWatch uses, with the paired dataset released
under CC0 and the method published in a peer-reviewed journal. On the question *did anyone else build
the personal-baseline idea*, the answer is yes, and Welltory built it well enough to ship to millions.

**Most important unknown.** *Which rule governs the verdict a user actually sees.* Welltory markets a
self-learning personal baseline and simultaneously documents that a **fixed medical threshold** overrides
it: users whose HRV is chronically below that threshold "see constant alerts" regardless of their
baseline, and where cardiac regulation is impaired, HRV "can't provide meaningful insights". The
vendor's own article titled *"Why Welltory isn't really for people with chronic conditions yet"* states
that such users should probably not use the app at all. But no analysis code is published, so the
mechanism cannot be inspected — only the vendor's description of it. Whether the baseline is the primary
decision rule with a safety floor, or the fixed threshold is primary with personalisation layered on
top for presentation, is **U — unknown**, and it is the single most consequential open question in this
report.

**Why it matters for AnxietyWatch.** If the fixed threshold dominates, then Welltory is a documented
case study of exactly the failure AnxietyWatch's design is meant to avoid, and the vendor's honesty
about it is more valuable than a competitor's silence would be. If the baseline dominates and the
threshold is a narrow safety floor, then Welltory has solved a problem AnxietyWatch v1 has also solved
and done so at a scale that has surfaced its edge cases in public. Either way the design lesson is the
same and it is the vendor's own: **a personal baseline with an undisclosed population-norm override is
not a personal baseline in the eyes of the user who gets the alert.**

---

## 2. Product and availability

| Field | Finding | Evidence |
|---|---|---|
| Identity | Welltory — HRV-first "health intelligence" app with 21 HRV metrics, stress/recovery episode detection and lifestyle correlations | B |
| Vendor | Welltory Inc. / "Welltory Technologies, Inc.", Redwood City, CA. DPO Ametros Group Ltd (UK); EU representative appointed | A (privacy policy) |
| Platforms | iOS, Android, web (`app.welltory.com`), Huawei AppGallery, Galaxy Store | A |
| Platform parity | `My patterns`, `Today Screen`, `Insights` and `Self-Discovery Experiment` are documented as **iOS-only**. Parity is not established | N/A — documented asymmetry |
| Scale | Play: 5M+ downloads, 4.6★, ~61K reviews, updated 2026-09-14. Site claims 17M users, 44 store features, 30+ university partnerships | B (site), A (platform) |
| Free tier | 1 measurement/day, 5 metrics, 3-day trial; simplified sleep and BP reports; experiments disabled; **data sources unconnectable**; **history limited to 30 days and older data is deleted** | A |
| Premium | 21 metrics, full history, sleep/activity/workout reports, BP analysis, lifelogging, geotracking, "clinically validated tests" for anxiety/burnout/depression | A |
| **Pricing** | **Annual $99/yr** (displayed as $8.25/mo, billed once) · **Lifetime $599** one-time · Play IAP range **$4.99–$79.00 per item** · **no monthly plan shown**; 35% promo active at research date; 3-day trial | A (plans page), A (Play) |
| Hidden cost | **AI Coach requires ChatGPT Plus *in addition to* Welltory Premium** — two subscriptions, two vendors | A |
| Required hardware | **None.** Phone-camera PPG is sufficient. Chest strap (~$50–90) or Apple/Samsung Watch optional. Netatmo/Withings indoor sensors optional | A |
| Refunds | 14 days from first payment; 48 hours for renewals; annual only; App Store handled by Apple | A |
| Target user | **Explicitly not** the chronic-condition patient. "80% of people don't have a chronic condition, nor are they athletes... That's the gap we're filling." Managing a serious medical condition → "we're probably not the app for you (yet)" | A (vendor article) |
| Regulatory | Not a medical device; not for diagnosis; not a substitute for professional advice. BP interpretations "cannot be considered a medical assessment" | N — vendor-documented limitation |

**Retrieval failures, recorded not papered over:** `welltory.com/pricing/` → HTTP 404 (pricing taken
from `welltory.com/plans/`); `welltory.com/terms-of-use/` → HTTP 404; the iOS App Store listing was
**not retrieved**, so no Apple-side ratings or IAP data appear anywhere in this report.

---

## 3. Claim-versus-implementation reconciliation

Carried forward from `welltory_IMPLEMENTATION_INSPECTION.md` §6, with dispositions expanded.

| # | Public claim | Static finding | Runtime observation | Disposition |
|---|---|---|---|---|
| 1 | Personal-baseline algorithm instead of one-size-fits-all thresholds | No analysis code published anywhere in the vendor's 13 public repos | None | **U — unresolved by inspection**, consistent with an unpublished closed engine |
| 2 | A fixed medical threshold overrides the baseline for chronic conditions | No code; the mechanism is described only in the vendor's article | None | **N — vendor-documented limitation.** Treated as the vendor's own statement, not as an inspected fact |
| 3 | *Phone camera PPG equals Polar chest straps*; store copy says "as accurate as ECGs" | **Public CC0 dataset with Polar H10 ground truth** (21 recordings / 13 healthy adults 25–35, resting, finger-on-camera, 68–112 s) + peer-reviewed method paper (Sensors 2021, 21:6798) | None | **B — public claim, narrowed by its own evidence.** The dataset supports *short resting finger-on-camera* PPG; the store wording generalises past it |
| 4 | Research validated Welltory for HRV in elite endurance athletes (Apr 2022) | **Not located** | None | **U — unverified** |
| 5 | Presented at the 41st IEEE EMBC, Berlin | Not verified | None | **U — unverified** |
| 6 | "Backed by over 25,000 studies on PubMed" | Refers to HRV literature generally, not to Welltory | None | **Misleading framing**, not a validity claim |
| 7 | Indoor environment integration | **Corroborated at source**: `python-foobot` (indoor air quality) and `python-cardioqvark` published by the vendor under MIT | None | **A — supported** |
| 8 | Weather auto-integrated by geolocation | Processing side unknown | None | **U** |
| 9 | AI Coach external on OpenAI ChatGPT, needs Premium + ChatGPT Plus, beta with hallucination warning | No AI code published | None | **U** by inspection; the vendor's own disclosure is unusually explicit |
| 10 | "Never sell your personal data"; health data not shared for advertising | Vendor's own policy names **Meta Pixel / Custom & Lookalike Audiences / Remarketing, Google Ads & Ad Manager, TikTok** plus **Amplitude, AppsFlyer, Firebase, Crashlytics** | None | **A — vendor-disclosed.** The disclaimer and the disclosed ad/analytics footprint coexist; both are recorded |
| 11 | Core functionality depends on server-side health-data analysis | **Consistent with** the complete absence of published client or engine code — but an unpublished closed client explains it equally well | None | **D — inference.** Not proven |
| 12 | Free tier deletes data older than 30 days | Documentation only | None | **A — documented**, stated in two independent support articles |
| 13 | CPAP / AHI / leak / therapy adherence | No code, schema or data artifact references PAP therapy; apnea appears only in editorial content | None | **No evidence of the capability.** Absence of evidence, not proof of absence in the binary |
| 14 | Medication / dose tracking | Nothing in any published schema or repository | None | **No evidence of the capability** |

**The one row that moves.** Row 3 is the only claim this inspection materially advanced. Before the
GitHub pass, the vendor's PPG/Polar equivalence claim was an unverifiable marketing assertion. It is now
partly corroborated: real paired data, real ground truth, a real peer-reviewed venue. It is also
narrower than the store copy implies — and the ground truth device is the Polar **H10**, which is worth
noting because it means Welltory's validation target and AnxietyWatch's reference sensor are the same
instrument.

**Everything else stayed unresolved, and that is the honest result.** Ten of fourteen rows could not be
decided by inspection, because no product binary and no analysis code were available. The report states
which rows are open rather than resolving them by inference.

---

## 4. Evidence ledger

| # | Claim | Product | State | Source | Date/version | Caveat |
|---|---|---|---|---|---|---|
| 1 | Personal-baseline algorithm is the core differentiator | Welltory | **B** | support article | updated ~2026-09 | No code; contradicted in practice by row 2 |
| 2 | Fixed medical threshold overrides baseline; chronic-condition users get constant alerts; HRV unusable if cardiac regulation impaired | Welltory | **N** | vendor article "Why Welltory isn't really for people with chronic conditions yet" | 2026-01-23 | Vendor's own limitation; threshold value not published |
| 3 | PPG ≈ Polar chest strap | Welltory | **B**, partly corroborated | CC0 dataset + Sensors 2021 21:6798 | 2021-10-13 | n=13, resting, 25–35, 68–112 s; vendor-authored |
| 4 | "As accurate as ECGs" | Welltory | **B** | Play listing | 2026-09-14 | Exceeds what row 3's evidence supports |
| 5 | HRV from Fitbit/Garmin/Oura/Mi Band | Welltory | **contradiction** | Play listing vs support article | 2026-09-14 / updated ~2026-09 | Store implies HRV; docs say impossible (W1) |
| 6 | 21 HRV metrics; Health/Battery/Stress scores | Welltory | **A** | support articles, plans page | updated ~2026-09 | Computation location unknown |
| 7 | Stress/recovery **episode detection** from physiological signals → auto-tagged free text | Welltory | **A** | My patterns article | 2026-09-03 | Detection algorithm unpublished |
| 8 | Tags auto-extracted from typed or spoken text, correctable | Welltory | **A** | My patterns article | 2026-09-03 | Contradicted in practice by a 6-month user report |
| 9 | Paired before/after **Experiments**, 3-hour window, one activity at a time | Welltory | **A** | Experiments article | 2023-05-15 | User-initiated; **not** dose-attached |
| 10 | No correlations found after 6 months of twice-daily measurement; tags/times not editable | Welltory | **C** (single) | Play review text | undated | One reviewer; unverified; relevant to W8 |
| 11 | Weather (temperature, humidity, pressure) automatic by location | Welltory | **A** | support article | 2022-07-05 | Requires geolocation permission |
| 12 | Indoor air quality / CO₂ / indoor climate via Foobot, Netatmo, Withings Body+ | Welltory | **A** + source-corroborated | support articles + `python-foobot` | 2025-01-27 / 2016 repo | Third-party hardware required |
| 13 | Blood pressure: not measured; manual or synced; ESC/ESH 2024; alerts on alarming readings | Welltory | **A** | support article | updated ~2026-09 | Explicitly not a medical assessment |
| 14 | Printable BP report to send to a doctor | Welltory | **B** | plans page | live | General multi-domain PDF not documented |
| 15 | Sleep stages, snoring, respiration rate, SpO₂, ECG, EDA, perfusion index ingestible | Welltory | **B** | privacy policy category list | 2026-09-01 | Per-platform reachability **unverified** (W7) |
| 16 | Sync latency "a few hours or one day"; sleep not on the next morning's graph | Welltory | **A** | support article | 2025-10-02 | Rules out same-night analysis from synced sources |
| 17 | **No CPAP / AHI / leak / PAP support** | Welltory | **absence** | all vendor surfaces examined | — | Absence of evidence ≠ absence in binary |
| 18 | **No medication feature found** | Welltory | **absence** | all vendor surfaces examined | — | As above |
| 19 | **No environmental-noise measurement** | Welltory | **absence** | audio is playback only (Echo/Sleep Flow) | — | As above |
| 20 | **No aircraft-overflight context** | Welltory | **absence** | — | — | As above |
| 21 | "Clinically validated tests" for anxiety, burnout, depression | Welltory | **B** | plans page | live | Instruments**not named** (W6) |
| 22 | AI Coach = external GPT on ChatGPT; needs Premium **and** ChatGPT Plus; beta; may hallucinate; user must verify | Welltory | **A** | support article | updated ~2026-09 | Two paid subscriptions |
| 23 | "Robust security", passwords encrypted | Welltory | **B** | privacy policy | 2026-09-01 | At-rest encryption of health data not stated |
| 24 | Discloses hashed email, online identifiers, device info and internet activity to **Meta, Google, TikTok**; analytics via Amplitude, AppsFlyer, Firebase, Crashlytics | Welltory | **A** | privacy policy | 2026-09-01 | Health data stated as excluded from advertising |
| 25 | "Never sell personal data"; no sale for monetary consideration | Welltory | **B** | privacy policy | 2026-09-01 | Coexists with row 24 |
| 26 | Not HIPAA-covered; addresses CCPA/CPRA, VCDPA, **Washington MHMDA** | Welltory | **A** | privacy policy | 2026-09-01 | Unusually specific for a consumer app |
| 27 | **Free tier deletes data older than 30 days** | Welltory | **A** | two support articles | 2026-01-23 | Vendor advises exporting first |
| 28 | Account deletion unavailable while a Premium subscription is active | Welltory | **A** | support article | 2026-01-23 | Contact support, or wait for expiry |
| 29 | Annual $99 / Lifetime $599 / no monthly / IAP $4.99–$79.00 | Welltory | **A** | plans page + Play | 2026-10-06 | Regional variation not established |
| 30 | Not a medical device; not for diagnosis; not for medical decisions | Welltory | **N** | multiple vendor pages | — | Consistent across surfaces |
| 31 | Vendor's own COVID study: n=14 eligible subjects, **preprint** | Welltory | **B** | medRxiv 2021.07.02.21259891 | 2021-07-05 | Not peer-reviewed |
| 32 | Dataset reused by ≥3 independent research groups | Welltory | **A** | Crossref-verified P3–P5 | 2020-12 → 2022-04 | Studies evaluate their own models, **not the product** |
| 33 | No app source code published in the vendor's GitHub organisation | Welltory | **A** (static) | GitHub API, 13 repos, 2026-10-06 | 2026-10-06 | Forks and data only |
| 34 | Free tier cannot connect data sources | Welltory | **A** | support article | 2025-06-30 | Integrations are effectively Premium |
| 35 | Panic-specific UX | Welltory | **absence** | "tips to... support a panic attack" is content, not a flow | — | No panic-triggered logging documented |

---

## 5. Feature matrix

Legend: **✅** present · **◐** partial / limited · **—** not found · **⊘** explicitly not supported ·
**◻** specified target, not shipped · **?** unresolved · `[B]` public claim, limited operational detail

| Capability | AW v1 (documented) | AW v2 (current) | AW v2 (specified target) | Welltory | Evidence status |
|---|---|---|---|---|---|
| **Measurement modality** | Polar H10 raw RR ~1 Hz + HealthKit + CPAP SD + EMAY oximeter | Same + AirSense 11 live BLE stream | + XE125 respiratory rate | Phone-camera **PPG** (~2 min), Apple/Samsung Watch, chest straps incl. H10 | A (both) |
| Personal physiological baseline | ✅ rolling personal baseline | ✅ | ✅ | ✅ claimed, **fixed threshold overrides** | B / N |
| Population-threshold override disclosed | Not applicable | Not applicable | Not applicable | ✅ **disclosed** — "constant alerts" for chronic conditions | N (vendor) |
| Continuous overnight capture | ✅ H10 background recording | ✅ always-on collectors on Pi | ✅ | **—** on-demand only | A |
| Phone-independent operation | ◐ (phone is the collector) | ✅ **Pi collects with no phone, no network** | ✅ | **—** documented server dependency | A / D |
| CPAP / AHI / leak / adherence | ✅ SD import | ✅ | ✅ live stream | **—** | absence |
| Oximetry / SpO₂ | ✅ EMAY import + BLE stream | ✅ | ✅ | ◐ SpO₂ as an ingestible health-platform metric only | B (W7) |
| Respiratory rate | ◐ via CPAP/oximeter | ✅ | ✅ XE125 | ◐ enumerated in the privacy policy; reachability unverified | B |
| Real physiological **episode detection** | ◐ below-baseline alerts | ✅ | ✅ | ✅ stress/recovery episodes, prompts for context | A |
| Panic-time logging UX | ✅ watchOS quick log | ✅ | ✅ | **—** (panic content only) | absence |
| Medication tracking | ✅ dose logging | ✅ | ✅ | **—** | absence (W3) |
| **Dose-triggered paired efficacy** | ✅ before/after around a dose | ✅ | ✅ | ◐ **Experiments** are paired before/after but around *arbitrary activities*, not doses | A |
| Free-text → structured tags | ✅ notes + tags | ✅ | ✅ | ✅ auto-extracted from typed/voice text | A |
| Subjective outcome is primary | ✅ rating is the outcome | ✅ | ✅ | Inverse — tags explain detected episodes | A |
| Weather / barometric context | ✅ Core Motion barometer (automatic) | ✅ | ✅ | ✅ weather service by geolocation (automatic) | A (both) |
| Indoor environment hardware | — | — | — | ✅ Netatmo, Foobot, Withings Body+ CO₂ | A + source-corroborated |
| Environmental **noise** | — | ◻ aircraft/environmental-noise direction | ◻ | **—** | absence |
| Aircraft overflight context | — | ◻ | ◻ | **—** | absence |
| Statistical correlation engine | ✅ correlation engine (v1 spec) | ✅ | ✅ | ✅ "algorithms search for correlations" + Insights | A / B |
| Correlation confounder handling | Not claimed | — | ◻ | Not documented; **Experiments warn that multiple tags cannot be disentangled** | A |
| AI analysis | ✅ Anthropic-based (v1) | ✅ | ◻ local analysis with provenance | ◐ external GPT; **beta**; needs Premium **+ ChatGPT Plus**; hallucination warning | A |
| Provenance per sample | ✅ | ✅ | ✅ | **—** no per-sample provenance documented | absence |
| Missing-data semantics | Three-state model (measured-good / measured-bad / **not measured**) | ✅ | ✅ | Not documented; stale sync produces gaps silently | absence |
| Append-only store | ✅ | ✅ | ✅ | **— free tier DELETES data >30 days** | A |
| Local alarm | ✅ local alerts | ◻ supervised alarm + **dead-man switch** | ◻ | **—** | absence |
| Clinical PDF report | ✅ PDF over date range | ✅ | ✅ | ◐ printable **BP** report only | B |
| Clinician portal | — | ◻ | ◻ | **—** | absence |
| Export | ✅ | ✅ | ✅ | ◐ data-portability right documented; format/granularity unknown | B (W4) |
| Self-hosting / local-first | ◐ local-first sync | ✅ self-hosted | ✅ | **—** US cloud, account required | A / B |
| Account required | — | — | — | ✅ email/password or Apple/Google/Facebook | A |
| Telemetry / ad SDKs | None claimed | None claimed | None claimed | ✅ Meta, Google, TikTok pixels; Amplitude, AppsFlyer, Firebase, Crashlytics | A (vendor-disclosed) |
| Open source | ◐ | ✅ Rust, open source | ✅ | **—** no app source; CC0 datasets + MIT wrappers only | A (static) |
| Required hardware cost | Polar H10 (~$90) | Pi + H10 | Pi + H10 + XE125 | **$0** (phone camera) | A |
| Subscription cost | $0 | $0 | $0 | **$99/yr · $599 lifetime**; AI needs ChatGPT Plus | A |
| Platform breadth | iOS/watchOS | Pi + iOS client | + admin console | iOS, Android, **web**, AppGallery, Galaxy | A |
| Medical-device claim | None | None | None | None | N (consistent) |

---

## 6. Evidence-backed workflows

Each journey is walked for all four systems. Evidence state is marked inline.

### 6.1 Panic logging at the moment it happens

- **v1** — watchOS quick log plus phone entry; the user rates anxiety and it lands in the local store.
  **[V1 documented]**
- **v2 current** — the Pi is collecting continuously; a panic event is a point in an already-running
  record rather than something the user must remember to capture. **[V2 current]**
- **v2 target** — the same, with the supervised alarm and escalation path. **[V2 specified target]**
- **Welltory** — **no panic-triggered flow was found.** Marketing offers "tips to... support a panic
  attack", which is content, not an interaction design. Episode detection may *ask* what happened after
  a detected stress episode, but that is retrospective and device-mediated, and the app requires the
  user to have taken a measurement or be wearing a supported watch. **[absence of evidence]**

**Why it matters.** This is the sharpest unresolved-versus-documented distinction in the report.
Welltory's episode detection is genuinely closer to real-time than anything else on this shortlist, and
it is a real capability. But it is an *interpretation* delivered to a functioning user, not a path that
works when the user is in the middle of a panic attack and cannot operate a phone. AnxietyWatch v1's
quick log at least assumes one deliberate tap; v2 assumes nothing from the user at all.

### 6.2 Sleep review the morning after

- **v1** — H10 overnight recording plus CPAP SD import and oximetry, available locally at wake.
- **v2 current** — Pi-collected overnight data is already stored locally; the review is a query, not a sync.
- **v2 target** — as above, with respiratory-rate context.
- **Welltory** — sleep arrives from Apple Health / Health Connect / Samsung Health, and the vendor
  documents that "sometimes it can take a few hours or one day. For example, if you analyze sleep, the
  data won't be uploaded on the graph right after the night but the next day." **[A — documented]** The
  morning-after review is therefore *not* reliable by design, and on the free tier any sleep older than
  30 days no longer exists. **[A — documented]**

**Why it matters.** A day's latency is fatal for the specific use case both products gesture at —
connecting last night to this morning's anxiety. AnxietyWatch v2 avoids the problem structurally by
collecting locally and never round-tripping through a health platform.

### 6.3 CPAP / respiratory investigation

- **v1** — SD-card import with AHI, leak and usage. **[V1 documented]**
- **v2 current** — plus a live AirSense 11 BLE stream. **[V2 current]**
- **v2 target** — plus XE125 respiratory rate. **[V2 specified target]**
- **Welltory** — **nothing.** No CPAP, AHI, leak, pressure or therapy-adherence capability appears on any
  vendor surface examined. Obstructive sleep apnea is mentioned only in editorial blog content, as an
  *explanation* for BP and SpO₂ relationships. **[absence of evidence, not proof of absence]**

**Why it matters.** For the apnoeic anxiety patient — the population AnxietyWatch is built around — this
is not a gap in Welltory's feature set, it is the absence of the entire domain. Welltory cannot observe
the nightly respiratory events that plausibly drive next-day anxiety, and its sleep analysis would show
fragmented sleep without naming the cause.

### 6.4 Medication follow-up

- **v1** — dose logging with a paired before/after rating around the dose. **[V1 documented]**
- **v2** — same, with the Pi providing the physiological half automatically. **[V2 current]**
- **v2 target** — as above. **[V2 specified target]**
- **Welltory** — **no medication feature was found at all** (W3). The closest analogue is **Experiments**,
  a genuine paired before/after HRV design: measure, do the activity, tag it, measure again within three
  hours, get a report on what changed. **[A — documented]** The vendor is careful with it, warning that
  multiple tags cannot be disentangled and that one activity at a time is required — a confounder caveat
  Bearable's vendor makes too, and one AnxietyWatch's own N-of-1 design shares.

**Why it matters.** Experiments show the *shape* of a paired design can be shipped to consumers, which
validates the interaction pattern AnxietyWatch relies on. But it is user-initiated around arbitrary
activities. It does not know a dose happened, and it cannot prompt one. AnxietyWatch's contribution is
not the before/after measurement — it is **attaching** it to a medication event automatically and
repeatedly.

### 6.5 Environmental interruption analysis

- **v1** — automatic barometric pressure context. **[V1 documented]**
- **v2 current** — as above. **[V2 current]**
- **v2 target** — plus aircraft-overflight and environmental-noise correlation as possible sleep
  interrupters. **[V2 specified target]**
- **Welltory** — **weather is automatic and real**: temperature, humidity and atmospheric pressure are
  transferred from a weather service based on device location, "automatically connected for all users".
  **[A — documented]** This is closer to v1's barometric context than Bearable's manual factors, and in
  one respect better: it needs no sensor. Indoor environment (air quality, CO₂, indoor climate) is
  supported through Netatmo, Foobot and Withings hardware — a coverage AnxietyWatch does not claim.
  **[A + source-corroborated]** Environmental **noise** is not a capability: audio is playback only.
  Aircraft overflight context: nothing found. **[absence]**

**Why it matters.** For weather and pressure, Welltory and AnxietyWatch v1 are at rough parity, with
Welltory winning on zero hardware and AnxietyWatch winning on not sending location to a cloud service.
For noise and aircraft — the two environmental interrupters AnxietyWatch2 specifies — there is no
competitor at all, which means the feature is genuinely differentiating *and* has no external benchmark
to design against.

### 6.6 Clinical sharing

- **v1** — a PDF report over a chosen date range. **[V1 documented]**
- **v2 current** — as above. **[V2 current]**
- **v2 target** — as above. **[V2 specified target]**
- **Welltory** — a **printable blood-pressure report** for the doctor is documented as a Premium feature,
  along with detailed BP interpretations and 10 extra metrics. **[B]** A general multi-domain clinical
  PDF is **not** documented. **[B]**

**Why it matters.** Welltory's report is real but narrow: it presents blood pressure, not the
stress-baseline-sleep-context picture a clinician treating anxiety would need. Notably, the
highest-value clinical artifact on the AnxietyWatch side — CPAP data with AHI and leak — has no Welltory
counterpart to compare against.

### 6.7 Offline / phone-independent operation

- **v1** — phone-centric; the phone is the collector. **[V1 documented]**
- **v2 current** — the Pi collects continuously with **no phone and no network**. **[V2 current]**
- **v2 target** — as above. **[V2 specified target]**
- **Welltory** — core functionality "relies on the analysis of health data" in the vendor's service, and
  data sources are unconnectable on the free tier. Analysis is documented as server-side. **[D —
  inference from the policy, consistent with the absence of published client code; not proven]**
  Observationally, none of Welltory's intelligence is local-first: the baseline model, episode detection,
  correlations, insights and the AI Coach all sit on the vendor side or on a third party's platform.

**Why it matters.** This is the widest structural divergence between the two products, and it is not a
feature-count question. AnxietyWatch2's design premise is that the alarm must work when the network does
not. Welltory's design premise is the opposite: the value is in the cloud analysis, so connectivity is
load-bearing. Neither is wrong; they are answering different questions. But a user who needs monitoring
that survives an outage is not choosing between two implementations of the same thing.

### 6.8 Longitudinal self-study over a year

- **v1 / v2** — append-only store; samples are never deleted, and removal is a tombstone row. A year of
  data is a year of data. **[V1/V2 documented]**
- **Welltory** — Premium retains full history. **The free tier deletes data older than 30 days**, with
  the vendor advising users to "export it in advance to avoid losing old data". **[A — documented,
  two independent articles]**

**Why it matters.** A 30-day retention cliff is incompatible with the question both products claim to
help answer — what has changed about me over months. It is a monetisation choice, and it is honestly
disclosed, which is better than the alternative. But it means Welltory's free tier cannot do
longitudinal self-study at all, and the paid tier starts at $99/year or $599 once. AnxietyWatch2's
append-only, never-delete store is a direct architectural repudiation of that trade.

---

## 7. Data and analysis audit

| Dimension | Welltory | AnxietyWatch v1 / v2 | Note |
|---|---|---|---|
| Raw vs summary | PPG-derived RR from camera; watch HRV summaries from Apple/Samsung; Fitbit summaries explicitly insufficient for analysis | Raw RR intervals at ~1 Hz from H10; CPAP SD records; oximeter streams | Welltory's own docs concede summary data is inadequate — the same conclusion AnxietyWatch reached |
| Live vs retrospective | On-demand measurement; synced sleep is hours-to-a-day old | Continuous overnight capture; v2 live AirSense stream | Opposite collection models |
| Provenance | Not documented per sample | Per-sample provenance is a v2 design pillar | No Welltory equivalent found |
| Freshness | Documented latency up to a day | Local, immediate | Structural difference |
| Missingness | Not documented; gaps appear as absent graph points | Explicit three-state model: measured-good / measured-bad / **not measured**; missing holds state, never reassures or escalates | Welltory has no published missing-data semantics at all |
| Baselines | Self-learning personal baseline **plus** an overriding fixed medical threshold | Rolling personal baseline, alerts on deviation from *your* baseline; no fixed override documented | The central comparison; unresolved mechanism (W2) |
| Correlations | Automated correlation search + Insights (~7 occurrences threshold) + Experiments | v1 correlation engine; v2 adds confounder-aware design | Welltory's Experiments explicitly warn that multiple simultaneous tags cannot be disentangled — a limit AnxietyWatch shares |
| AI context | External GPT on ChatGPT, consent-gated, Premium + ChatGPT Plus, explicit hallucination warning, Zero Data Retention claim; vendor states it cannot receive data back | v1 Anthropic-based analysis; v2 specified local analysis with provenance | Welltory's AI is more clearly walled off and more clearly unreliable, and it is honest about both |
| Destructive operations | Free tier deletes at 30 days; deletion blocked while subscribed | No UPDATE, no DELETE against samples; removal is a tombstone | Architecture-level divergence |
| Computation location | Server-side per policy [D] | Local/Pi-first; AI is opt-in | Determines outage behaviour |

**Reading.** Welltory's *analysis* layer is deeper than anything else on the shortlist — 21 metrics, an
adaptive baseline, episode classification, correlation mining — and it is built on a sensor model
(PPG or watch summaries) that is weaker at the raw-signal level than AnxietyWatch's H10-at-1 Hz
foundation. Welltory does considerably more with less raw data. That is a genuine engineering
achievement, and it is also why the fixed-threshold finding is not a trivial bug: the algorithm is
sophisticated enough that users would reasonably trust its verdicts.

---

## 8. Privacy and cost audit

### Privacy

| Aspect | Welltory | AnxietyWatch |
|---|---|---|
| Hosting | Vendor cloud; US controller, UK DPO, EU representative | Local-first; self-hosted |
| Account | **Required** (email/password or Apple/Google/Facebook) | None required |
| Data sale | Denied: "never sell your personal data"; "do not share health or wellness data for advertising purposes" | No third parties |
| Advertising disclosure | **Named**: Meta Pixel/Custom & Lookalike Audiences/Remarketing, Google Ads & Ad Manager, TikTok — hashed email, online identifiers, device info, internet activity | None claimed |
| Analytics/attribution | **Amplitude, AppsFlyer, Firebase, Crashlytics** | None claimed |
| Health data in ads | Stated as excluded | N/A |
| Research sharing | Aggregated/anonymised on legitimate interest; individual-level only with additional consent; a 2020 COVID dataset was released openly under CC0 | Not applicable |
| HIPAA | Explicitly not a covered entity | Not applicable |
| Regional rights | CCPA/CPRA, VCDPA, **Washington MHMDA** addressed with consumer-health-data rights | Not applicable |
| Local-only mode | **None** | The default |
| Deletion | In-app and web, **but not while a Premium subscription is active** | Tombstone-based removal |
| Worst privacy finding | Advertising pixels and analytics SDKs with hashed identifiers leave the device — disclosed by the vendor in its own policy | — |
| Best privacy finding | The policy is unusually specific: named partners, named DPO, named regulations, explicit separation of health data from advertising, and an explicit statement that the vendor cannot see or receive AI Coach conversations | — |

**Reading.** Welltory's disclosure quality is genuinely good and should be credited: it names
advertising partners rather than burying them, addresses MHMDA, and separates health data from ad data.
But disclosure is not the same as minimisation. A user comparing Welltory to AnxietyWatch is comparing
*a detailed honest account of data leaving the device* with *the data that does not leave the device*. Those
are not equivalent, and the AnxietyWatch positioning is only credible because it is architectural rather
than a promise.

The unresolved item is **W9**: the Play data-safety declaration lists "Location, Personal info and 3
others" as shared and "Personal info, Health and fitness and 3 others" as collected. Those "3 others"
are not resolved in this pass and are recorded as open.

### Cost

| Item | Welltory | AnxietyWatch |
|---|---|---|
| Software | $0 free tier (crippled) · **$99/yr** · **$599 lifetime** · no monthly | Free/open source |
| AI | Requires **ChatGPT Plus on top of Premium** — two vendors | v1 Anthropic API cost (user-supplied key); v2 local analysis |
| Required hardware | **$0** — phone camera | Polar H10 ~$90 + Raspberry Pi + (v2) AirSense 11 ownership |
| Optional hardware | Chest strap ~$50–90; Apple/Samsung Watch; Netatmo/Withings sensors | XE125 radar (v2 target) |
| Electricity / hosting | $0 (vendor-hosted) | Pi power draw; storage |
| Promotion/refund | 35% promo at research date; 14-day refund; gift certificates; App Store refunds by Apple | N/A |
| Free-tier trap | Data older than 30 days **deleted**; sources unconnectable; 1 measurement/day; 5 metrics | None |

**Reading.** This is the one axis where Welltory's total cost of ownership can genuinely beat
AnxietyWatch: a user who owns a phone and nothing else gets real HRV analysis for $0 up front. But the
$0 tier cannot connect data sources, produces one measurement a day, shows 5 of 21 metrics, and **deletes
history at 30 days** — so the honest comparison is $99/year against AnxietyWatch's ~$90 one-time sensor
plus a Pi the user may already own. Over three years, Welltory's annual plan is ~$297 versus a one-time
hardware cost. Welltory's lifetime price of $599 exceeds it outright. And the headline AI feature
requires a **second** subscription to a different company, which no other candidate on this shortlist
does.

---

## 9. What cannot be concluded

**Blocked by the uninspected binary.** Nothing about Welltory's implementation was observed at runtime.
There is no packet capture, no log, no screenshot and no recorded interaction. Every statement about how
the app behaves in this report is a statement about **documented** behaviour. The vendor publishes no
client or analysis source — its 13 public repositories contain forks, small API wrappers and datasets,
and no product code — so the analysis engine, the baseline adaptation, the episode detector and the
fixed threshold are all opaque.

**Ten of fourteen reconciliation rows remain unresolved**, specifically rows 1, 2, 8, 9, 10, 12 — plus
the two absence findings (13, 14) which are absence-of-evidence rather than proof.

**Overstated by the vendor.**
- The store claim that phone-camera PPG is "as accurate as ECGs" exceeds the supporting evidence, which
  is a vendor-authored study of 21 recordings from 13 healthy 25–35 year-olds, at rest, finger on
  camera, 68–112 seconds each.
- Store copy implying Fitbit, Garmin, Oura and Mi Band supply "heart health data" conflicts with the
  vendor's own documentation that those devices cannot supply HRV at all (W1).
- "Backed by over 25,000 studies on PubMed" refers to HRV literature, not to Welltory.
- The April 2022 athlete validation and the IEEE EMBC presentation were **not located** (W11).

**Unsupported by any evidence found, and not to be asserted as absent.**
- No CPAP, AHI, leak or therapy-adherence capability (W3 domain).
- No medication, supplement or dose-tracking feature.
- No environmental-noise measurement; no aircraft-overflight context.
- No per-sample provenance; no documented missing-data semantics.
- No panic-triggered logging flow.

**Unresolved contradictions carried forward.**
- **W1** HRV device support: store copy versus support documentation.
- **W2** The fixed medical threshold — value, scope, interaction with the baseline, and whether a user
  can see which rule fired. **This is the most important open question in the report.**
- **W7** Which enumerated metrics are actually ingested per platform versus merely listed as categories.
- **W8** `My patterns` reliability — the vendor's ~7-occurrence threshold against one six-month user
  report of zero connections.
- **W9** The Play data-safety "3 others" allocations.
- **W10** Whether any clinician-shareable output exists beyond the printable BP report.

**Not established, and flagged so it is not assumed:** no iOS App Store data was retrieved; the science
page's papers were verified only as metadata, never read; the published `datatypes.md` schemas were not
parsed, which is the single most promising untried step for closing the CPAP and medication questions on
evidence rather than absence.

---

## 10. Balanced conclusion

**Classification: partial competitor to AnxietyWatch v1; a methodological reference more than a
competitor to AnxietyWatch2; and a possible complement to both.**

**Where Welltory competes directly.** The personal-baseline HRV idea, the paired before/after
measurement pattern, automatic environment context, and the anxiety-symptom self-tests. On these, it is
not a lesser version of AnxietyWatch — it is a shipped, polished, five-million-download product with a
research programme behind it, and where the two overlap, Welltory is the more mature product.

**Where it does not compete at all.** No CPAP, AHI, leak or therapy data. No medication feature. No
overnight continuous capture. No oximetry stream. No environmental noise. No aircraft context. No local
alarm. No self-hosting. No append-only store. These are not small gaps in a long feature list; several
of them are the reason AnxietyWatch exists.

**Where Welltory is straightforwardly ahead.**
- **Zero required hardware.** ~$0 versus a Polar strap and a Raspberry Pi. This is a real accessibility
  advantage over both AnxietyWatch versions, and it is available today.
- **HRV analysis depth.** 21 metrics, self-learning baseline, frequency-domain scores, episode
  classification, correlation mining — more analysis machinery than v1 has documented.
- **Breadth of context.** Roughly a hundred integrations including indoor air quality, productivity and
  geotracking; nothing on the AnxietyWatch side comes close.
- **Platform breadth and polish.** iOS, Android, web, AppGallery, Galaxy; documented refunds; an
  unusually specific privacy policy that names its advertising partners and addresses Washington MHMDA.
- **Scale as an evidence source.** Millions of users generate the edge cases that a small project only
  reads about — which is precisely why the fixed-threshold finding is so useful.

**Where AnxietyWatch is structurally ahead, and it is not a matter of degrees.**
- **The alarm path.** Welltory has no alarm, no escalation, no dead-man switch. v2's specified target is
  about the user who cannot act.
- **Phone- and network-independent operation.** The Pi keeps collecting when both are gone.
- **The apposite clinical domain.** CPAP and respiratory events, which is where an apnoeic anxiety
  patient's night actually happens.
- **Data permanence and honesty.** Append-only, never-deleted, three-state missingness — against a 30-day
  deletion cliff and no published missing-data semantics.
- **Provenance.** Raw RR at ~1 Hz with per-sample provenance against PPG-derived values with none.
- **Privacy by architecture.** Data that does not leave the device versus data that leaves it, however
  honestly disclosed.

**The design lesson is the vendor's own.** Welltory built the personal baseline, shipped it at scale, and
documented — in an article titled *"Why Welltory isn't really for people with chronic conditions yet"* —
that a fixed medical threshold overrides it and tells chronic-condition users that HRV "can't provide
meaningful insights" when cardiac regulation is impaired. Welltory's target user is explicitly the
healthy person who wants to stay that way: "80% of people don't have a chronic condition... That's the
gap we're filling." AnxietyWatch's user is the other 20%. That is not a competitive gap to be closed; it
is a different product for a different person, and Welltory says so itself.

**Best-fit users.**
- **Welltory** — a healthy or health-adjacent person with only a phone, who wants HRV insight, lifestyle
  correlations and daily guidance, and is comfortable with a US cloud, advertising pixels, $99/year, and
  a product the vendor does not intend for chronic conditions.
- **AnxietyWatch v1** — someone with a Polar H10 and CPAP data who wants a personal-baseline record,
  medication follow-up and a clinician-shareable report, locally.
- **AnxietyWatch2** — someone who needs continuous, phone-independent, append-only monitoring with a
  supervised alarm, and who is willing to run a Raspberry Pi.

**Complementarity.** The overlap is real enough that the two are not cleanly additive: a user would not
want to maintain two HRV baselines computed by different algorithms on different sensors. The clearer
integration story is the *zero-hardware measurement path* and the *context breadth* — the parts of
Welltory that have no AnxietyWatch equivalent — rather than a second baseline model. That said, no
integration work should be proposed on the strength of this report: it is a claims-and-documents
comparison with an uninspected binary, and the fixed-threshold question (W2) is unresolved. Until that
is answered, the safest use of Welltory in this project is as a **documented case study of a
personal-baseline system failing its edge case in public**, which is more valuable than a feature
checklist and should inform the decision-logic design directly.

---

## Related files

- Public claims: `welltory_CLAIMED_FEATURES_AND_PURPOSE.md`
- Implementation inspection: `welltory_IMPLEMENTATION_INSPECTION.md`
- Candidate-specific prompt: `PROMPT_2.md`
- Shortlist: `../CANDIDATE_SHORTLIST.md`
- Process state: `../PROCESS_STATE.md`
- Method and evidence classes: `../RESEARCH_METHOD.md`
