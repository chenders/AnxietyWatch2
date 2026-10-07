# SleepHQ versus AnxietyWatch v1 and AnxietyWatch2

Candidate 5 of 10. Research date **2026-10-06**.
Base variant `PROMPT2-TECHNICAL-SYSTEMS.md` + `PROMPT2-PRIVACY-AND-OPERATING-MODE.md`.
Companions: `sleephq_CLAIMED_FEATURES_AND_PURPOSE.md`, `sleephq_IMPLEMENTATION_INSPECTION.md`, `PROMPT_2.md`.

**Evidence classes in this report.** `[PUBLIC CLAIM]` vendor's own statement · `[DOCUMENTED CONTRACT]`
stated in the vendor's published OpenAPI specification · `[STATIC]` read from a published artifact ·
`[INFERENCE]` my reasoning · `[UNKNOWN]` no evidence found. **There are no runtime observations in this
report.** No SleepHQ endpoint was called, no binary was inspected, no account was created.

**AnxietyWatch status labels.** `[V1 impl]` exists in `../AnxietyWatch` today · `[V2 current]` exists in
`./` today · `[V2 target]` fully specified in `openspec/`, not shipped · `[V2 unwired]` present and
tested but read by no consumer · `[unresolved]` no evidence either way.

---

## 1. Executive finding

**SleepHQ is a partial competitor to AnxietyWatch v1's CPAP-and-oximetry subsystem and a partial
competitor to AnxietyWatch2's AirSense 11 data path — but not a competitor to either product as a whole,
and not on the axis that gives AnxietyWatch its name.**

It is the first candidate in this package whose overlap with AnxietyWatch is on the *data* rather than on
the *category*. It reads the same machine family AnxietyWatch2 targets, adds the same class of oximetry,
produces the same class of nightly record, and is sold to the same person. Three of its properties are
directly comparable:

| | SleepHQ | AnxietyWatch2 | Which is ahead |
|---|---|---|---|
| Transport from the AirSense 11 | SD card; manual, phone OTG reader, or a **AUD $399 Wi-Fi dongle** requiring a Pro subscription `[PUBLIC CLAIM]` | **BLE streaming** — no card, no dongle, no subscription `[V2 current]` (physical gate open) | **V2 on friction and cost**; SleepHQ on proven end-to-end delivery |
| Oximeter | **O2 Ring Pro only** — "only SleepHQ branded devices are compatible" `[PUBLIC CLAIM]` | EMAY `SleepO2`, any BLE target `[V2 target]` | **V2 on replaceability**; SleepHQ on a shipped, measured integration |
| Derived values | `ahi`, `oa_count`, `ca_count`, `h_count`, `ua_count`, `*_95` percentiles, `duration_90`, `drop_90`, `drop_3`, `drop_4`, `o2_score` `[DOCUMENTED CONTRACT]` | **No AHI, no percentile computation anywhere** `[V2 current]`; device events **decoded and classified but read by nothing** `[V2 unwired]` | **Arguable, and argued in §7** |
| Alerting | A "sleep alarm" **shipped**, with "**Fixed three crashes in the sleep alarm**" in the vendor's own release notes `[STATIC]`; ring vibration alerts on SpO₂/HR thresholds `[PUBLIC CLAIM]` | Multi-modal alerting, supervised alarm and hardware dead-man switch are **V2 target** | **SleepHQ clearly ahead in production** |
| Clinician sharing | Live share links; clinician accounts; enterprise tier `[PUBLIC CLAIM]` | PDF reports `[V2 target]`; no sharing | **SleepHQ clearly ahead** |
| Ownership | Hosted cloud, account required, **no self-hosted option** `[PUBLIC CLAIM]` | Private LAN, self-hosted, no account `[V2 current]` | **Different products** |
| Anxiety, panic, medication | **Nothing found.** Subjective surface is one daily `feeling_score` and `notes` `[DOCUMENTED CONTRACT]` | Panic-time capture `[V1 impl]`; subjective capture `[V2 current, in progress]` | **AnxietyWatch — and SleepHQ does not contest it** |

**The single most consequential finding is not a feature comparison at all.** SleepHQ's founder publishes,
under his own name, that CPAP machines under-count hypopnoeas and that device AHI cannot be trusted — and
the same platform publishes an `ahi` field and four event counts on every session `[DOCUMENTED CONTRACT]`.
That tension is the best available external evidence that AnxietyWatch2's decision not to compute an AHI
is a defensible engineering position rather than an omission. It is **not** validation: the founder's
articles cite no study, name no journal and link to nothing, and AnxietyWatch2 has no published validation
of its own. §7 argues both sides.

**Verdict against v1:** a partial competitor within the CPAP/SpO₂ subsystem, and a **stronger product
inside it** — wider device support, better sharing, less manual work. v1's advantages are the ones not
about CPAP: panic-time capture, medication, environmental context, cost and ownership.

**Verdict against v2:** a partial competitor to the *ingestion path*, occupying a **different and largely
incompatible architecture**. SleepHQ is what a non-technical user buys today to analyse an AirSense 11 in
the cloud. AnxietyWatch2 is a self-hosted system that streams from the same machine and sells nothing.
They overlap on *the device and the night* and diverge on nearly everything else: SleepHQ has shipped
alerting and sharing that V2 has only specified, and V2 has an integrity and ownership posture SleepHQ
does not offer and does not claim.

---

## 2. Product and availability

| Item | Finding | Class |
|---|---|---|
| Vendor | SleepHQ Pty Ltd, Melbourne / Phillip Island, AU | `[PUBLIC CLAIM]` |
| Founders | Nick Dunn (sleep technologist, biomedical science) and Adam (web developer) | `[PUBLIC CLAIM]` |
| Platforms | Web, iOS (`com.sleephq.ios`), Android (`com.sleephq.android`) | `[PUBLIC CLAIM]` |
| iOS | v3.0.34, 2026-10-05; 4.73★ / 204 ratings; 17+; iOS 18.1+; 17.4 MB | `[PUBLIC CLAIM]` |
| Android | 10K+ installs; 4.4★ / 76 reviews; updated 2026-09-13; "Everyone"; "Users Interact" | `[PUBLIC CLAIM]` |
| Free tier | "Free to use forever" — **1 device, 1 member, most recent 30 days viewable**, breath-by-breath, share links, professional connections | `[PUBLIC CLAIM]` |
| Pro | **US $14.90/mo or US $149.00/yr** (17% off; $12.42/mo effective); 14-day trial; unlimited data/members/devices; journal with notes/flags/advanced trends; **custom PDF reports with AI Insights**; Academy; Apple/Google Health sync; Community; **API access** | `[PUBLIC CLAIM]` |
| Hardware | **Magic Uploader Pro AUD $399** (ResMed 10/11; requires Pro); **O2 Ring Pro AUD $299** (the only oximeter supported) | `[PUBLIC CLAIM]` |
| Currency | Selectable AUD/CAD/EUR/GBP/USD; regional values not captured | `[PUBLIC CLAIM]` |
| Refunds | **"No Refunds on Subscriptions or products."** | `[PUBLIC CLAIM]` |
| Self-hosting | **None offered.** Hosted service; account required | `[PUBLIC CLAIM]` |
| Developer access | A public REST API is **included in the Pro tier** | `[PUBLIC CLAIM]` |

**Total cost of ownership, stated plainly.** A user who wants the automated path pays **US $149/year plus
AUD $399 plus AUD $299** in the first year (≈ US $590 at the AUD figures as list prices, before any bundle
discount), recurring only in the subscription; the shop offers 40% off the ring when bought with the
uploader. A user on the free tier pays nothing and gets 1 device, 1 member and 30 days of history. An
AnxietyWatch user pays for a Raspberry Pi and electricity and their own time. **These are not comparable
in the same currency, and the report does not pretend they are** — but the honest framing is that SleepHQ
sells convenience and AnxietyWatch sells control, and each is expensive on the other's axis.

**A conflict in the vendor's own pricing copy.** The pricing block lists "Magic Uploader Integration
(requires separate purchase)" under the **free** tier; the shop product page states "To use the Magic
Uploader Pro, you need to have a SleepHQ Pro subscription." `[PUBLIC CLAIM]` ×2. The most conservative
statement governs: **the uploader requires Pro.** Two current vendor surfaces disagree — the second such
unreconciled conflict in this package (after Bearable's data-safety panel).

---

## 3. System boundary and architecture

```text
SleepHQ                                    AnxietyWatch2 [V2 current + target]

  ResMed AirSense 11                         ResMed AirSense 11
        |                                          |
        | SD card (removed)                        | BLE notifications (no card)
        v                                          v
  [manual copy | phone OTG | Wi-Fi SD card]   vitals-decode (no_std, pure)
        |             ^                            |
        |             | AUD $399 uploader          | COPY-binary spool (attempt first)
        |             | requires Pro               v
        |             |                        TimescaleDB `samples` hypertable
        v             |                        PK (source,type,session_id,seq)
  HTTPS upload ----------------------------+   append-only; tombstone delete
        |                                  |        |
        v                                  |        v
  SleepHQ cloud (account, Stripe)          |   read views on the private LAN
   - two-stage: POST /imports              |
     then POST /imports/{id}/process_files |
   - per-night machine_date (11 jsonb      |   [V2 target] alerting:
     summary blocks) + per-session record  |    multi-modal + supervised alarm
   - derived: ahi, counts, *_95,           |    + hardware dead-man switch
     duration_90/drop_*, o2_score          |   [V2 target] PDF reports
   - share links; clinician accounts       |   [V2 target] ezShare ingest
        |                                  |   [V2 in progress] subjective capture
        v                                  |   [V2 in progress] third-party read
  O2 Ring Pro --BLE--> phone or uploader   |
   (proprietary: 1 s samples, vibration    |   EMAY SleepO2 --BLE--> collector
    threshold alerts)                      |    (replaceable sensor)
```

**The boundary that matters is the cloud boundary.** SleepHQ's analysis happens on the vendor's servers;
its derived values are produced there and published back as JSON. V2's analysis happens on a machine the
owner controls, and the derived layer is not yet written. Everything else in this section follows from
that one difference.

Two secondary boundaries are also structural rather than cosmetic:

- **The hardware boundary.** SleepHQ's automated path requires two pieces of vendor hardware; its
  oximetry is single-sourced. V2 requires none and targets a third-party oximeter.
- **The two-stage processing contract.** `POST /v1/imports/{import_id}/files` uploads;
  `POST /v1/imports/{id}/process_files` starts processing `[DOCUMENTED CONTRACT]`. Work is explicitly
  triggered, and an import record carries `status`, `progress`, `failed_reason` and `processed_at`.
  V2's `cpap-ezshare-ingest` change specifies the same principle in its own words — "attempts precede
  work" (invariant 7) `[V2 target]`. The two systems reached the same conclusion about ingest
  observability independently.

---

## 4. Data path and source comparison

| Stage | SleepHQ | AnxietyWatch2 | Class |
|---|---|---|---|
| Acquisition from the device | SD card, removed from the machine | BLE notifications, card untouched | `[PUBLIC CLAIM]` / `[V2 current]` |
| Automation | Wi-Fi SD card in the Magic Uploader Pro; "uploads your ResMed data overnight"; App Store copy claims a **10-minute** sync cadence | Continuous while the collector is connected | `[PUBLIC CLAIM]` / `[V2 current]` |
| Phone-only path | OTG SD card reader in the app | Not a path — the Pi is the collector | `[PUBLIC CLAIM]` |
| Oximetry | O2 Ring Pro over Bluetooth, to the phone or to the uploader | EMAY `SleepO2` over BLE to the collector | `[PUBLIC CLAIM]` / `[V2 target]` |
| Other health data | Apple Health / Health Connect (Pro) | Not a source; v2 reads its own BLE sensors | `[PUBLIC CLAIM]` / `[V2 current]` |
| Device coverage | ~19 families, enumerated, with the unsupported ones listed too | One source family: `resmed_air11`, **17 types** in `vitals/registry.toml` | `[PUBLIC CLAIM]` / `[V2 current]` |
| Ingestion observability | Import record with `status`, `progress`, `failed_reason`, `processed_at`, `content_hash`, `fingerprint_base64` | Raw-frame journal before decode; attempt ledger `[V2 target]` | `[DOCUMENTED CONTRACT]` / `[V2 target]` |
| Raw data retrievable | Yes — per-file `download_url`, **signed, 5-minute expiry** | Yes — the `samples` table is the raw store, and the spool holds frames | `[DOCUMENTED CONTRACT]` / `[V2 current]` |
| Waveform/breath-level access | **No documented endpoint carries breath-level data**, despite the marketing claim of a breath-by-breath view | Flow/pressure/leak are stored as samples in the hypertable | `[DOCUMENTED CONTRACT]` + `[UNKNOWN]` / `[V2 current]` |

**The most interesting asymmetry in this table is the last row.** SleepHQ advertises "breath-by-breath"
analysis and publishes `flow_limit` and `flow_limit_95` per session `[PUBLIC CLAIM]` + `[DOCUMENTED
CONTRACT]` — but **no documented endpoint returns a waveform or a high-resolution series**. Whether the
breath-by-breath view is served by an unpublished clinic endpoint, by the apps directly, or by something
else is `[UNKNOWN]`. Meanwhile V2 stores the raw signal and has no view layer at all. One product can
display what it may not expose; the other stores what it cannot yet display. Neither is complete.

**The documented hash contract is a genuine integration strength.** `content_hash` is published as **MD5
of (file content + file name), with the path excluded**, with a worked example and a validation endpoint
`[DOCUMENTED CONTRACT]`. That is precise enough for a third party to implement against and check. It is
also MD5, and a second identifier, `fingerprint_base64`, has **no published algorithm**.

---

## 5. Claim-versus-documented-contract reconciliation

| # | Claim | Documented contract / static evidence | Reconciliation |
|---|---|---|---|
| 1 | "Track your AHI" | `SessionSerializer.ahi` (decimal) + `oa_count`/`ca_count`/`h_count`/`ua_count` | **Backed.** Whether it is relayed or re-derived is **not documented** |
| 2 | "Advanced statistics… how your pressure changes over time" | `pressure_set(_95)`, `ipap_set(_95)`, `epap_set(_95)` | **Backed** |
| 3 | "Pan through the night… zoom to analyse airflow limitation and respiratory arousals" | `flow_limit`, `flow_limit_95`; **no waveform endpoint** | **Not reconciled.** No documented resource carries breath-level data |
| 4 | "Custom PDF reports with AI Insights" | **No AI resource, field or description anywhere in the API surface** | **Not reconciled.** Absence in the contract is not absence in the product |
| 5 | "set personalised alerts to receive notifications for specific oxygen & pulse rate level events" | **No alert, threshold or notification resource** | **Not reconciled**; app/device-level, outside the public API |
| 6 | "Automatically syncs… every 10 minutes" | No documented schedule; no webhook or push resource | **Not reconciled** |
| 7 | "Get detailed statistics on your therapy for every night" | Eleven `jsonb` summary blocks on the nightly record | **Exists; contents `[UNKNOWN]`** |
| 8 | "Track your feelings with fun emojis & create custom flags…" | `feeling_score` (integer, no range), `notes`, `sleep_stages` (text); **no flag field** | **Partially reconciled** |
| 9 | "Share… no personally identifiable information is shared" | **No documented share resource** | **Unfalsifiable from outside** |
| 10 | "Download all data at any time" | Per-file `download_url`, **signed, 5-minute expiry**; no bulk-archive endpoint | **Backed, with a caveat** |
| 11 | "SleepHQ is not a medical device" | `ahi`, event counts and an undefined `o2_score` published per session | **Compatible, and worth naming**: the platform derives clinical-looking indices while disclaiming medical-device status |
| 12 | "encrypted both at rest and in flight… completely secure at all times" | The specification says nothing about transport or storage; the privacy policy says "progressing towards SOC 2 Type II"; the Play panel says only "encrypted in transit" | **Most conservative statement governs** — see §10 |
| 13 | "SleepHQ reads AirSense 11" | 8 AirSense 11 / AirCurve 11 variants listed | **Backed** |
| 14 | "Only SleepHQ branded devices are compatible" (oximetry) | SD-root-relative `path` on every file upload; no generic SpO₂ import; no device write path | **Backed and deliberate** |

**No claim was contradicted outright.** The contract corroborates the analysis-related claims in shape and
is silent on the three that a buyer would weigh most: the AI feature, the alerting, and the
breath-by-breath view. That silence is itself the finding — the strongest implementation-grade artifact
this vendor publishes cannot substantiate its three most distinctive capabilities.

---

## 6. Feature matrix

| Capability | v1 | V2 current | V2 target | SleepHQ | Evidence / maturity |
|---|---|---|---|---|---|
| Read AirSense 11 | Via OSCAR CSV `[V1 impl]` | **BLE, 17 types** `[V2 current]` | — | **SD card** `[PUBLIC CLAIM]` | SleepHQ shipped; V2 gate open |
| Read other CPAP brands | Via OSCAR `[V1 impl]` | None | — | **~19 families** `[PUBLIC CLAIM]` | SleepHQ far ahead |
| Oximetry | CSV columns `[V1 impl]` | EMAY `SleepO2` decoder `[V2 current]` | — | **O2 Ring Pro only** `[PUBLIC CLAIM]` | Divergent philosophy |
| CPAP event classification | From OSCAR CSV | **`EventKind` decoded, read by nothing** `[V2 unwired]` | — | Modelled as per-session counts `[DOCUMENTED CONTRACT]` | V2 implemented-and-unwired |
| AHI | From OSCAR `[V1 impl]` | **Not computed** `[V2 current]` | — | `ahi` + 4 counts `[DOCUMENTED CONTRACT]` | See §7 |
| Percentiles | From OSCAR | **None** `[V2 current]` | — | `*_95` fields `[DOCUMENTED CONTRACT]` | Undefined estimator |
| Oximetry-derived score | None | None | — | `o2_score` `[DOCUMENTED CONTRACT]` | **No published definition** |
| Real-time alerting | None | None | Multi-modal `[V2 target]` | **"sleep alarm"** `[STATIC]` | SleepHQ shipped, was crashing |
| Supervised alarm / dead-man switch | None | None | `[V2 target]` | `[UNKNOWN]` | V2 design only |
| Clinician sharing | PDF reports `[V1 impl]` | None | PDF `[V2 target]` | **Share links + clinical accounts** `[PUBLIC CLAIM]` | SleepHQ ahead |
| Panic-time capture | Yes `[V1 impl]` | In progress (`ios-client-stage7`) `[V2 current]` | — | **No** | Daily journal only |
| Daily subjective entry | Yes `[V1 impl]` | In progress `[V2 current]` | — | `feeling_score` + `notes` `[DOCUMENTED CONTRACT]` | Both present, different timing |
| Medication tracking | Yes `[V1 impl]` | — | — | **Not found** `[UNKNOWN]` | V1 ahead |
| Weather / aircraft / song context | Yes `[V1 impl]` | — | — | **Not found** `[UNKNOWN]` | V1 ahead |
| Public API for third parties | None | — | Read access `[V2 target]` | **Pro-tier REST API** `[PUBLIC CLAIM]` | SleepHQ ahead |
| Corrections to past data | Local edits `[V1 impl]` | Tombstone rows; versioned clock transform `[V2 current]` | — | `PUT machine_dates time_offset`; sessions record `moved_from_*` `[DOCUMENTED CONTRACT]` | Both handle it; different ergonomics |
| Self-hosted / no account | Optional sync `[V1 impl]` | **Yes, required model** `[V2 current]` | — | **No** `[PUBLIC CLAIM]` | Structural divergence |
| Cost | Free `[V1 impl]` | Hardware + electricity | — | $149/yr + $399 + $299 `[PUBLIC CLAIM]` | Different value propositions |

---

## 7. Derived values versus stored signals — the decision both products answer, oppositely

This section is the reason SleepHQ is on the shortlist.

### 7.1 What SleepHQ does

It publishes, per session, an **AHI and its four component counts**, pressure and flow percentiles
labelled `_95`, oximetry-derived quantities named `duration_90`, `drop_90`, `drop_3` and `drop_4`, and an
integer `o2_score` `[DOCUMENTED CONTRACT]`. Of these, only the four event counts have an evident meaning
(they are the machine's own obstructive, central, hypopnoea and unclassified counts). The rest:

- **`_95` is undefined.** No estimator, no window, no interpolation, no statement of whether it is over
  the session or the night. A percentile is published whose definition exists only in its name.
- **`duration_90`, `drop_90`, `drop_3`, `drop_4` are undefined.** No units, no thresholds, no equations.
  The suffix pattern suggests an oxygen threshold and a duration.
- **`o2_score` is undefined.** An integer score with no published method at all.

### 7.2 What AnxietyWatch2 does

It computes **no AHI and no percentile anywhere in the tree** `[V2 current]` — the only occurrence of the string "AHI" in the Rust sources is a doc comment recording the opposite: `EventKind::Rera` "contributes to RDI, not AHI". It stores raw flow,
pressure, leak and oximetry samples. It **does** decode and classify the device's own event
notifications — `HypopneaEnd`, `CentralApneaEnd`, `ObstructiveApneaEnd`, `ApneaEnd`, `ReraEnd`, `CsrStart`,
`CsrEnd` — into `EventKind` (`crates/vitals-decode/src/events.rs`), but that classification is **read by
no consumer**: the collector passes only the raw event name through `observe_event`, and the session
lifecycle acts on `TherapyStart`/`MaskOn`/`TherapyStop`/`MaskOff`/`StandbyStarted` `[V2 unwired]`.

So the accurate statement is neither "V2 has no event handling" nor "V2 computes an AHI". It is:
**V2 classifies device events correctly and then does nothing with the classification, and publishes no
index.**

### 7.3 The case for SleepHQ's position

- **A user gets an answer.** *My AHI last night was 4.2* is actionable and discussable; *your flow signal
  is in a hypertable* is not. AnxietyWatch2 currently produces **no nightly summary of any kind** — the
  analysis layer is unbuilt.
- **The numbers are at least labelled as derived.** A consumer can see them as outputs of an analysis
  layer, not as measurements.
- **`summary_calculated` exists**, so a record can say that its own summary is not there yet. That is a
  real three-state instinct, whatever the semantics of `false` turn out to be.
- **Device event counts are the device's own scoring**, relayed with attribution to a session. Surfacing
  what the machine already decided, and teaching the user its limits, is a coherent position.

### 7.4 The case for AnxietyWatch2's position

- **The vendor disputes its own headline number.** SleepHQ's founder publishes that machine AHI
  under-counts hypopnoeas, that "if the device scored an AHI of zero, it could actually be closer to a
  10, and if it showed a 16, it could have been more like 25-30", and recommends reviewing flow data and
  oximetry instead `[PUBLIC CLAIM]`. The same platform publishes `ahi`. Both are the vendor's own
  statements.
- **The founder's own worked example** — machine 51 events / AHI 6.32, manual review 115 events / AHI 14.3
  — is a **more than twofold** divergence on the platform's headline metric `[PUBLIC CLAIM]`.
- **On the platform's own evidence, an AHI is least trustworthy exactly where it matters most.** The
  under-count is described as worst at low values, which is the region a treated patient lives in.
- **An undefined score is worse than no score.** `o2_score` is an integer with no published method. A
  number that looks like a measurement and is an unstated algorithm is precisely what this project's
  rules forbid.
- **The percentile labels are undocumented.** A `_95` whose estimator is unknown is a value a downstream
  system can only consume on faith.

### 7.5 What the honest reading is

**The two positions are not symmetrical, and the comparison should not pretend they are.** SleepHQ's
derived fields are *inspectable in shape and undefined in method*; V2's absence of derived fields is
*complete and internally consistent*. Neither has published validation. Neither can claim its numbers are
correct.

What the SleepHQ case establishes, and nothing more, is this: **a commercial competitor with roughly
100,000 claimed users also believes the device's AHI is unreliable, and still ships it.** That is evidence
that shipping an AHI has product value. It is *not* evidence that the AHI is trustworthy, and it is *not*
independent validation of V2's choice not to ship one.

**The cost of V2's refusal is real and should be named.** A V2 user today gets no nightly index, no
percentile, no score — nothing to look at. The refusal is only defensible if it is paired with a
replacement, and the replacement (raw signal plus a *documented* analysis, with the device's own
classification consumed rather than discarded) is `[V2 unwired]` and `[V2 target]`. Until then, V2's
position is a promise, not a product.

**And one point favours V2 that SleepHQ's own contract makes visible.** SleepHQ's derived layer is
**read-only**: there is no documented write path to any session or any derived value; the only correction
the API permits is a night's `time_offset` `[DOCUMENTED CONTRACT]`. A user cannot fix a wrong AHI, and
cannot mark one as untrusted. V2's append-only, tombstone-based model with a versioned clock transform is
built for exactly that class of correction.

---

## 8. Real-time, overnight and failure-mode comparison

| Scenario | SleepHQ | AnxietyWatch2 | Reading |
|---|---|---|---|
| **A sensor stream drops overnight** | The card still holds the night's data; the next upload recovers it `[PUBLIC CLAIM]`. The ring's BLE link is to the phone or uploader; a drop there loses the night unless the ring buffers | BLE disconnect mid-notification; the session lifecycle acts on lifecycle events only; the raw-frame journal bounds what is lost `[V2 current]` | **SleepHQ has the structural advantage**: an SD card is a store-and-forward buffer, so a radio failure is not a data failure. V2's radio *is* its data path |
| **The phone is unavailable** | CPAP ingestion via the uploader is independent; the ring pairs to the phone or the uploader `[PUBLIC CLAIM]`. Phone-only path unavailable | The Pi collects; the phone is a client `[V2 current]` | V2 unaffected; SleepHQ partly unaffected |
| **The internet is unavailable** | The uploader's Wi-Fi SD card "stores your data until you return to your home network, then syncs automatically" `[PUBLIC CLAIM]` | Nothing depends on the internet `[V2 current]` | Both degrade gracefully; SleepHQ reconverges, V2 never needs to |
| **The vendor's cloud is unavailable** | No dashboard, no analysis, no alerts, no sharing. **The derived layer is entirely server-side** `[INFERENCE from architecture]` | No such dependency `[V2 current]` | **V2 structurally ahead.** SleepHQ has a single point of failure that is not the user's |
| **A reading is stale, malformed, out of range, or absent** | **Undocumented.** Eleven opaque `jsonb` blocks; `omitted` unenumerated; `summary_calculated` semantics unstated `[UNKNOWN]` | Absent ≠ invalid; gaps become explicit NULLs; `locf`/interpolation forbidden (amendment 01) `[V2 current]` | V2's policy is documented; SleepHQ's is unknown — **and unknown is not the same as worse** |
| **A clinician needs an export with source and quality context** | Share link (contents unverified), or per-file `download_url` (signed, 5-min expiry), or a Pro custom PDF with AI Insights `[DOCUMENTED CONTRACT]` / `[PUBLIC CLAIM]` | PDF report `[V2 target]`; no sharing `[V2 current]` | **SleepHQ ahead, decisively, today** |
| **The alert path fails** | The alarm is shipped and "**Fixed three crashes in the sleep alarm**" `[STATIC]`; the vendor also added "You're now told when a night's recording fails to save" `[STATIC]` | Supervised alarm and hardware dead-man switch are `[V2 target]`; nothing shipped | **SleepHQ ahead in production, V2 ahead in intent.** Design intent with no shipped evidence is not a capability |

**Two conclusions from this table, both uncomfortable for AnxietyWatch.**

First, **an SD card is a better failure-tolerant substrate than a radio link for the overnight window.**
SleepHQ's transport is clumsy, expensive and old-fashioned, and it survives a dropped connection because
the data was already written to flash by the machine. V2's continuous BLE stream is the more elegant
design and the more fragile one: collect nothing during a disconnect and the night is partly gone. V2
mitigates this with the raw-frame journal, but the journal only records what arrived.

Second, **SleepHQ's own release notes are the best available evidence about the cost of shipping the
alerting features V2 specifies.** Three crash fixes in a single release, plus a retrofit of an explicit
recording-failure notification, is what the alarm path cost this vendor. V2's alerting design (multi-modal
alerting, supervised alarm, hardware dead-man switch) is more conservative in intent and has never run in
production.

---

## 9. Provenance, data quality, missingness and reproducibility

| Property | SleepHQ | AnxietyWatch2 |
|---|---|---|
| Identity of raw artifacts | `content_hash` (**MD5 of content+name, path excluded**, documented with a worked example) + `fingerprint_base64` (**algorithm undisclosed**) | Sequence numbers within a session; PK `(source,type,session_id,seq)`; raw-frame journal |
| Duplicate handling | Content hash + fingerprint, documented as the dedupe mechanism | Sequence-ordered append-only inserts |
| Failed work recorded | `failed_reason`, `status`, `progress`, `processed_at` on the import `[DOCUMENTED CONTRACT]` | Attempt ledger `[V2 target]`; rejection reasons currently printed to stderr, not persisted (`crates/vitals-collectors/src/bin/h10_collector.rs:330–332`) |
| Correction of past data | `PUT machine_dates/{id}` `time_offset` only; sessions carry `moved_from_machine_date_id` / `moved_from_date`; no session write path | Versioned clock transform; raw device time preserved; tombstone rows for removal |
| Deletion | `DELETE /v1/imports/{id}?purge_files=true`; import and file deletion documented | **No `UPDATE`, no `DELETE`** against `samples`; removal is a tombstone row |
| Missingness | **Undocumented** (`jsonb`, `omitted`) | Amendment 01 forbids `locf()` and `interpolate()` on a physiological series and requires `time_bucket_gapfill` so gaps return as explicit NULLs |
| Replayability of the source | SD-card files are retained server-side and downloadable | Raw frames are stored; decode is pure and replayable |
| Retention | Pro claims **"unlimited data"** `[PUBLIC CLAIM]`; the free tier's 30-day limit is a *view* limit with no retention statement `[UNKNOWN]` | Raw `samples` dropped after **2 years** (amendment 01), compressed after 7 days, with a `samples_1min` continuous aggregate carrying min/max |
| Time identity | `time_zone` + `time_offset` per night; **night-boundary convention undocumented** | UTC epoch seconds only; local time exists only in presentation; versioned transform |

**One clear win each.** SleepHQ's **file-level provenance is more complete than V2's**: a documented hash
algorithm, a second fingerprint, an explicit failure reason, and the original files retained and
downloadable. That is a genuinely strong audit story at the file layer, and V2's attempt ledger is still
a target. V2's **correction and deletion semantics are stricter and better specified**: append-only,
tombstoned, with the raw device time preserved so a bad clock is a transform rather than a rewrite.
Neither system's missingness handling is comparable, because SleepHQ's is not documented at all.

---

## 10. Local/cloud/privacy and operational cost

### 10.1 Data flow, and where the vendor's statements conflict

```text
SleepHQ:  sensor/card -> app or uploader -> HTTPS -> SleepHQ cloud (Stripe, Oneleet)
          -> analysis -> jsonb summaries + derived session fields -> dashboard/share link
          -> third parties? [UNKNOWN: no subprocessors named]

AnxietyWatch2:  BLE -> Pi (private LAN) -> spool -> TimescaleDB -> views
          cloud: none.  account: none.  vendor: none.
```

| Item | SleepHQ | Class |
|---|---|---|
| Data collected | Name, Email, **Birth date**, Gender, Time Zone, CPAP data, Apple Health data, Health Connect data, O2 Ring data | `[PUBLIC CLAIM]` |
| Stated purpose | "To build, improve and provide technical support"; **"We also perform anonymised statistical analysis and research to develop new products and services"** | `[PUBLIC CLAIM]` |
| Sharing | "We don't share it without your permission" | `[PUBLIC CLAIM]` |
| Named subprocessors | **None named** — no analytics, crash-reporting or advertising vendor | `[PUBLIC CLAIM]` (by omission) |
| Play data safety | Collects "Personal info and Health and fitness"; **"No data shared with third parties"**; "encrypted in transit"; deletion requestable | `[PUBLIC CLAIM]` |
| Security posture | "**working with** Oneleet… **progressing towards** SOC 2 Type II"; HIPAA controls "**designed to meet**"; independent pen testing | `[PUBLIC CLAIM]` |
| Security claim, FAQ | "encrypted **both at rest and in flight** ensuring your data is **completely secure at all times**" | `[PUBLIC CLAIM]` |
| Policy metadata | **No date, no governing law, no retention period, no named jurisdiction** | `[STATIC]` |
| Portability | Files downloadable via a **signed URL expiring in 5 minutes**; "download all data at any time" | `[DOCUMENTED CONTRACT]` / `[PUBLIC CLAIM]` |
| Erasure | "completely delete your account & all associated data" | `[PUBLIC CLAIM]` |
| Free tier history | **View** limit of 30 days; **retention vs deletion `[UNKNOWN]`** | `[PUBLIC CLAIM]` + `[UNKNOWN]` |

**The conflict, resolved conservatively.** "Completely secure at all times" is an absolute claim no
system can support, and it is the kind of formulation this project's own rules forbid. The privacy policy
— a document with legal weight — says only that the vendor is *progressing towards* an attestation. The
Play panel says only "encrypted in transit". Following the rule that **the most conservative vendor
statement governs**, this report records: **SleepHQ states encryption in transit; it asserts encryption
at rest in an FAQ but not in a store declaration; and its security programme is described by the vendor
as in progress, not attested.** SleepHQ is recorded as **not claiming SOC 2 Type II certification** at
the research date. No inference is drawn about its actual security in either direction.

**The genuinely notable privacy finding is a negative one, and it favours SleepHQ.** Unlike Bearable
(which names Meta/Google/TikTok as recipients) and Welltory (which names Meta, Google and TikTok in its
policy), **SleepHQ names no analytics, advertising or crash-reporting subprocessor at all**, and its Play
data-safety declaration agrees with its policy that data is not shared with third parties. Two independent
vendor surfaces agreeing is worth recording precisely because Bearable's equivalent pair did not. That is
not the same as proving no sharing occurs; it is the vendor's statement, and it is the most privacy-
favourable statement any commercial candidate has made so far.

**And the counterweight is the AI feature.** Pro includes "Custom PDF reports with **AI Insights**"
`[PUBLIC CLAIM]`, and the API surface contains **no AI resource, field or description** `[DOCUMENTED
CONTRACT]`. The privacy policy permits "anonymised statistical analysis and research to develop new
products and services". Which model, where data is processed, whether it is opt-in, and whether CPAP
records leave SleepHQ's infrastructure are all `[UNKNOWN]`. For a platform holding continuous respiratory
and oximetry data, that is the most material unresolved privacy question in this candidate.

### 10.2 Total cost and lock-in

| Item | SleepHQ | AnxietyWatch2 |
|---|---|---|
| Recurring | **US $149/yr Pro** (or $14.90/mo) — required in practice for the uploader, history beyond 30 days, Health integration, and API access `[PUBLIC CLAIM]` | None |
| One-off hardware | **AUD $399** uploader + **AUD $299** ring (40% off the ring when bundled) `[PUBLIC CLAIM]` | A single-board computer and its power supply `[INFERENCE]` |
| Time cost | Low: plug in the uploader, wear the ring | **High**: build, provision, maintain, and until the analysis layer exists, read raw data |
| Data lock-in | Free to leave — but the derived layer is only meaningful inside SleepHQ, and the API is a **Pro** entitlement `[PUBLIC CLAIM]` | None in principle: the store is a local PostgreSQL/TimescaleDB the owner controls. Note that V2 itself drops raw `samples` after 2 years (amendment 01), keeping a 1-minute aggregate — so "unlimited data" on Pro is a broader retention *claim* than V2's own rule |
| Hardware lock-in | `[PUBLIC CLAIM]` **High** for oximetry — only SleepHQ-branded devices; the uploader is ResMed 10/11 only, and needs Pro | None; the oximeter is a replaceable BLE target |
| Subscription trap | Auto-renewing, **no refunds**, price changes reserved `[PUBLIC CLAIM]` | N/A |
| Exit | Original files downloadable (5-minute signed URLs); derived data readable via a Pro API | Everything is local |

**The honest asymmetry:** SleepHQ is cheap in time and expensive in money and lock-in; AnxietyWatch2 is
free in money and expensive in time, and until recently also in capability. A user who values the
clinician-sharing workflow and the shipped alarm is better served by SleepHQ today and should know they
are renting it. A user who values not having their respiratory data in a vendor cloud is better served by
AnxietyWatch2 today and should know they are, for now, operating an unfinished analysis layer.

---

## 11. Alerting and the alarm path

This section exists because **SleepHQ has shipped the feature AnxietyWatch2 has specified**, which makes
it the reference case rather than a competitor on this axis.

| Property | SleepHQ | AnxietyWatch2 |
|---|---|---|
| Real-time alerting | **Shipped** — a "sleep alarm" in the mobile apps `[STATIC]` | Multi-modal alerting `[V2 target]` |
| Device-level alert | O2 Ring Pro vibration on SpO₂/HR threshold breach `[PUBLIC CLAIM]` | Hardware dead-man switch `[V2 target]` |
| App-level alert | "set personalised alerts… for specific oxygen & pulse rate level events" `[PUBLIC CLAIM]` | Supervised alarm `[V2 target]` |
| Reliability evidence | "**Fixed three crashes in the sleep alarm**"; "You're now told when a night's recording fails to save" `[STATIC]` | **None** — not shipped |
| Supervision model | `[UNKNOWN]` — no SleepHQ source states whether the alarm distinguishes unauthorised, unmeasured or failed states | Specified as supervised, with a dead-man switch `[V2 target]` |
| Threshold provenance | Preset thresholds `[PUBLIC CLAIM]`; not documented in the API | Registry-defined thresholds (`vitals/registry.toml`) `[V2 current]` |

**What this comparison legitimately concludes.** SleepHQ's release notes are evidence that shipping a
real-time alert path with a phone component costs several crash fixes and a retrofitted failure
notification. V2's alerting specification is more conservative in intent — a supervised alarm and a
hardware dead-man switch are exactly the answer to *what if the alert path itself fails* — and is entirely
unproven. **Design intent is not a capability.** The correct statement is: *SleepHQ has the feature and
has paid for it in defects; AnxietyWatch2 has the better-specified feature and has paid nothing yet.*

**One point where SleepHQ's own design appears weaker than V2's specification, on the evidence
available.** Nothing in SleepHQ's sources indicates that its alert distinguishes *not measured* from
*measured and good*, and `omitted` — the one field that might — is unenumerated `[UNKNOWN]`. V2 makes
that distinction a rule. But this is a statement about documentation and specification, not about
behaviour, and it must not be written as though SleepHQ's alarm were known to false-reassure.

---

## 12. User-facing consequences

**What a SleepHQ user can do that an AnxietyWatch2 user cannot, today:**

1. Put on a ring, plug in a dongle, and have the previous night analysed without touching a card or a
   terminal.
2. Send a link to a sleep physician and have them look at the data.
3. Get a vibration on the wrist when oxygen drops below a threshold overnight.
4. Read a nightly AHI and think about whether therapy is working.
5. Run a CPAP machine from any of ~19 supported families.
6. Use an API on a Pro plan and build on the data.

**What an AnxietyWatch2 user can do that a SleepHQ user cannot:**

1. Keep every sample on hardware they own, with no account, no subscription and no vendor cloud.
2. Change oximeter without changing platform.
3. Correct the clock and keep the raw device time as evidence, with append-only integrity.
4. Log a panic at the moment it happens, alongside medication and environmental context — and, in v1,
   have weather, songs and aircraft noise in the same store.
5. Read the raw signal rather than a derived index.

**What neither can do:** validate any of its own numbers. SleepHQ publishes no study; AnxietyWatch2
publishes no study. Both disclaim medical-device status — SleepHQ explicitly ("are not a diagnosis and
are not medical advice"; "is not a medical device"), AnxietyWatch2 through its scope and rules. Neither should be
described as clinically validated or diagnostic, and no local notification on either platform is an
emergency monitor.

---

## 13. Limitations, unverified claims, evidence gaps and blocked inspection

**Inspection performed:** the vendor's published OpenAPI specification and reference were read in full
(21 paths, 11 models), together with the marketing site, shop pages, support/FAQ pages, privacy policy,
terms, supported-devices page, App Store metadata, Play listing and the founder's articles.

**Blocked or not performed:**

- **No runtime observation.** No endpoint was called; no account, OAuth client or trial was created.
- **No binary inspection.** The iOS and Android applications were not obtained, decompiled or executed.
- **No source inspection.** No SleepHQ-authored source repository exists; the third-party projects found
  (a Raspberry Pi uploader, an API client) are unaffiliated user work and were not treated as vendor
  evidence.
- **No hardware.** Neither the O2 Ring Pro nor the Magic Uploader Pro was handled; all hardware properties
  are the vendor's statements.
- **No share link was created**, so the de-identification claim is unverified.
- **The clinic/enterprise API surface** is documented as existing and is explicitly unpublished.

**Material evidence gaps:** contents of the eleven `jsonb` blocks; definitions of `o2_score`,
`duration_90`, `drop_90`, `drop_3`, `drop_4`, and the `_95` estimators; the import `status`/`progress`
domains; the meaning of `omitted`; whether the free tier hides or deletes data past 30 days; whether the
session AHI is relayed or re-derived; what "AI Insights" is; whether the sleep alarm is supervised;
whether it distinguishes unmeasured from good; the night-boundary convention; the `fingerprint_base64`
algorithm; whether `active_energy_joules` is ever returned.

**Claims that cannot be checked from outside:** the 100,000-user and 160-country figures; "the first ever
cloud based cpap data analytics platform"; the ring's "medical grade" description; "completely secure at
all times"; "encrypted at rest"; "no personally identifiable information is shared"; the 10-minute sync
cadence; the 1-second sampling rate; the bundle discount.

**Vendor statements recorded but not treated as validation:** the founder's two articles on AHI
under-counting, which name no study, author, journal or link.

**A note on the diligence question.** Nothing in this report should be read as a criticism of SleepHQ for
using the SD card. The CPAP industry's data access has historically required either vendor cloud services
or card extraction; SleepHQ's card-based approach is a lawful, robust and widely used method, and it is
strictly more capable than anything available without BLE. AnxietyWatch2's BLE path is only possible
because of `resmed-air11-ble-stream`, and it is not yet fully gated.

---

## 14. Balanced conclusion

**SleepHQ is a subsystem competitor, not a product competitor — and on that subsystem it is the most
relevant candidate researched so far.**

Against **v1**, it wins on device breadth, on getting data off the machine, and decisively on clinician
sharing. It loses on everything that makes v1 an anxiety tool: panic-time capture, medication,
environmental context, cost and ownership. A user who wants to understand their CPAP therapy should use
SleepHQ; a user who wants to understand their anxiety should not.

Against **v2**, the relationship is stranger. They read the same machine, and they answer the same design
question in opposite ways. SleepHQ reaches the device through an SD card and two pieces of hardware and
computes every derived value it can name. AnxietyWatch2 reaches the device over the radio, uses no vendor
hardware, and computes nothing derived at all. **SleepHQ's derived layer is broader and its methods are
undocumented; V2's derived layer does not exist.** Neither has published validation.

**What AnxietyWatch should take from this candidate:**

1. **A shipped alert path is expensive and the vendor's own release notes prove it.** Three crash fixes in
   one release, and a retrofitted "your night failed to save" notification. Budget for the alerting
   milestone accordingly.
2. **An SD card is a better store-and-forward buffer than a radio.** V2's data path is the radio. The raw-
   frame journal, not the BLE link, is what protects the overnight window, and it deserves to be treated
   as the primary mitigation rather than a detail.
3. **File-level provenance with a documented hash algorithm is a real feature.** SleepHQ publishes the
   MD5 construction precisely enough to implement and test. V2's attempt ledger should be at least as
   explicit about what it records and how it is checked.
4. **Sharing with a clinician is a capability, not a nicety — and SleepHQ's model for it works.** If V2
   wants PDF reports to matter, the share path deserves a design that does not require an account.
5. **An `omitted` field with no enumeration is a cautionary example.** If V2 marks a session as excluded,
   the value domain must be published and closed. Same for the misspelled `time_over_leak_threshhold`:
   an interface name is a contract, and typos in it are permanent.

**What AnxietyWatch should refuse to copy:**

1. **Vendor-hardware lock-in in the sensor path.** SleepHQ's own FAQ states only its own oximeter works.
   V2's replaceable BLE target is the better decision and should stay one.
2. **Publishing derived indices whose methods are undocumented.** `o2_score` is an integer with no
   published definition. V2's emerging rule — measured-and-good, measured-and-bad, not measured, and
   nothing in between — is incompatible with shipping a score nobody can reproduce.
3. **An absolute security claim.** "Completely secure at all times" is unsupportable and it is
   contradicted by the vendor's own privacy policy. V2's guards are required to name their blocker and
   the value that blocked it, for the same reason.

**What AnxietyWatch should watch, without acting:**

The founder's AHI argument is the most useful external evidence obtained so far for V2's decision not to
publish an index — and it is a vendor statement in a vendor's interest, with no cited study, and it must
be used that way. **The strongest defensible use of SleepHQ in this package is as a precedent, not as a
source of truth:** a company that serves a large CPAP population also distrusts device AHI, and has
nonetheless shipped it. That tells AnxietyWatch what the market expects, and it does not tell AnxietyWatch
what is true.

**In one sentence:** SleepHQ is what a CPAP user buys when they want the numbers now and do not mind
renting them; AnxietyWatch2 is what they build when they want the samples and do not mind waiting — and
if AnxietyWatch2 intends to keep refusing to compute the numbers, it owes its users a documented
replacement analysis, because *the raw signal is in the database* is not yet a product.

---

## Related files

- `docs/app_comparison/sleephq/sleephq_CLAIMED_FEATURES_AND_PURPOSE.md`
- `docs/app_comparison/sleephq/sleephq_IMPLEMENTATION_INSPECTION.md`
- `docs/app_comparison/sleephq/PROMPT_2.md`
- `docs/app_comparison/oscar/oscar_COMPARISON.md` — the source-inspection candidate and v1's format authority
- `docs/app_comparison/exist/exist_COMPARISON.md` — the other documented-contract candidate
- `docs/app_comparison/CANDIDATE_SHORTLIST.md`, `RESEARCH_METHOD.md`, `PROCESS_STATE.md`
