# Welltory — claimed features and purpose

**Slug:** `welltory`
**Research date:** 2026-10-06
**Phase:** Phase 2 (public claims)
**Implementation inspection:** not performed — see `welltory_IMPLEMENTATION_INSPECTION.md`
**Shortlist rank:** 2 of 10 (`../CANDIDATE_SHORTLIST.md`)

---

## 1. Executive summary

Welltory is a commercial HRV-first health intelligence app. Its core loop is: take a heart-rate-variability
measurement (phone camera, Apple Watch, Samsung Watch, or a compatible chest strap), get 21 HRV-derived
metrics plus Health / Battery / Stress scores, note what was happening, and receive daily insights and
recommendations. Around that it ingests roughly a hundred third-party sources — activity, sleep,
productivity, environment, nutrition, meditation — and correlates them against HRV.

It is the most **architecturally similar** candidate on the shortlist to AnxietyWatch, and specifically
to the part of AnxietyWatch that is hardest to get right: **it computes a personal baseline instead of
comparing against population norms**, and it says so as its central differentiating claim. It also
runs a paired before/after measurement design around user-logged activities, which is the closest
public analogue to AnxietyWatch v1's dose-triggered before/after prompt.

It is nonetheless a categorically different product from AnxietyWatch2. Welltory is a
**measurement-and-interpretation app**, not a monitoring platform: it has no continuous background
capture, no overnight recording, no local alarm, no CPAP or respiratory-event support, and no
self-hosting. Its data lives in a US company's cloud, its free tier **deletes data older than 30 days**,
and its privacy policy discloses advertising pixels and analytics SDKs.

The most important finding for AnxietyWatch is the vendor's own admission. In a support article titled
*"Why Welltory isn't really for people with chronic conditions yet"*, Welltory states that its
algorithm contains a **fixed medical threshold** below which it will flag regardless of the user's
personal baseline, that people with chronic conditions therefore "see constant alerts", and that when
cardiac regulation is impaired "HRV can't provide meaningful insights". That is an unusually candid
statement of exactly the failure mode a personal-baseline design is supposed to avoid — and it is
documented by the vendor, not inferred.

## 2. Product purpose and target user

- **Stated purpose.** "Read your body's signals... See what's draining your energy — and stop it before
  it drains your day." Marketing positions it as an energy-management and burnout-prevention tool.
- **Explicit target user.** The vendor is unusually specific, and it is *not* the AnxietyWatch user:
  "80% of people don't have a chronic condition, nor are they athletes... That's the gap we're filling."
  The article continues: **"if you're managing a serious medical condition, we're probably not the app
  for you (yet)."** Welltory targets high-achievers, workaholics and health-conscious people who want
  to avoid becoming patients.
- **Anxiety placement.** Anxiety is addressed as a symptom cluster and a set of content topics
  ("tips to beat insomnia... support a panic attack"), and the Premium tier includes "clinically
  validated tests: check in on possible anxiety, burnout, or depression symptoms". There is no
  panic-time interaction design comparable to AnxietyWatch v1's watchOS quick log.
- **Company.** Welltory Inc. (also rendered "Welltory Technologies, Inc." in the site footer), Redwood
  City, California. DPO is Ametros Group Ltd (UK); an EU representative is also appointed.

## 3. Availability status

| Question | Finding |
|---|---|
| Status | **Active.** Play Store listing updated Sep 14, 2026; support articles updated within weeks of the research date; privacy policy last updated September 1, 2026 |
| Platforms | iOS, Android, web app (`app.welltory.com`), Huawei AppGallery, Galaxy Store |
| Platform limits | Several features are documented as iOS-only (`My patterns`, `Today Screen`, `Insights`, `Self-Discovery Experiment`); an Android-specific Energy Forecast article exists. Feature parity is **not** established |
| Geography | Global; content and support are English, Russian and Spanish |
| Scale | Play Store: **5M+ downloads**, 4.6 rating, ~61K reviews. Site claims 17M users, 233K five-star ratings, 44 store features, 30+ university partnerships |

## 4. User-facing workflows

- **HRV measurement.** Phone-camera PPG (flash + finger, ~2 minutes), Apple Watch automatic, Samsung
  Watch, or a compatible chest strap.
- **Today screen.** Health, Battery and Stress scores; personalised sleep analysis, activity report,
  workout report; daily move goals.
- **Stress and recovery episodes.** Welltory "detects when your body goes through stress or recovery
  episodes throughout the day" from heart rate and physiological signals, then asks "What happened?" or
  "What helped you rest?" The answer — typed or spoken — is parsed automatically into tags such as
  `anxiety`, `deadline`, `coffee`, `walk`, `argument`. Tags are extracted by the system, not chosen from
  a fixed list, and the user can correct them before saving.
- **My patterns.** Aggregates those tags into a stress/recovery ring, most-consistent-unwind tags,
  major stress-causing events, "worth paying attention to" tags (rare events with a strong body
  response), and a stress & recovery timeline. Per-tag detail shows frequency, weekly rhythm, monthly
  trend, heart rate during the episode, and the episode list. Insights require roughly seven or more
  occurrences.
- **Experiments.** Paired before/after measurement around an activity: measure, do the activity, tag it,
  measure again within three hours, receive a report on what changed. The vendor warns to experiment
  with one activity at a time because multiple tags cannot be disentangled.
- **Blood pressure.** Not measured by the app. Manual entry, or automatic sync from Qardio, Omron,
  iHealth or Medisana via Apple Health / Health Connect / Samsung Health, or Withings directly.
  Interpreted against 2024 ESC/ESH guidelines with additional derived indices, and the app alerts on
  alarming readings.
- **Sleep, activity, workouts, nutrition, meditation, productivity, lifelogging.** Ingested from
  connected apps and presented as reports and charts. Lifelogging covers calls, meetings, nights out,
  and geotracked time at home/office/on the road.
- **Echo / Sleep Flow.** Bedtime stories and calming audio, "uniquely tailored to your heartbeat".
- **AI Coach.** An external GPT built on OpenAI's ChatGPT platform, described below in §10.
- **Self-tests.** "Clinically validated tests" for possible anxiety, burnout or depression symptoms
  (Premium).
- **Breathing practices** and guided practices.

## 5. Data sources and supported hardware

**HRV-capable sources.** Phone camera (PPG); Apple Watch; Samsung Watch; and these chest straps:
Polar H7, Polar H10, 4iiii Innovations V100, Suunto Smart Belt, Suunto Smart Sensor, Garmin HRM-Dual,
Garmin HRM-Pro; also Zephyr HxM Smart, COOSPO, Topeak DB-HRM, CorSense, HeartMath Inner Balance,
Wahoo TICKR X, Wahoo TICKR.

**Explicitly incompatible for HRV.** Polar OH1, MyZone MZ-3, Scosche RHYTHM+, Wahoo TICKR Fit.
Also Withings and other heart-rate-only trackers, and — because the vendor's policy does not permit
data transfer — **Biostrap, Oura, Garmin, and Mi Band**. Fitbit supplies only daily HRV "summaries",
not RR intervals, which the vendor says is insufficient for its analysis.

**Marketing/documentation conflict.** The Play Store description says "Sync with FitBit, Samsung,
Garmin, MiFit, Polar, Mi Band, Oura, Withings and other wearables for more heart health data" and
"Sync 1,000+ supported apps & gadgets". The support documentation says Garmin, Oura, Mi Band and Fitbit
**cannot** supply HRV to Welltory. Both can be true only if "sync" means activity/sleep context rather
than HRV. The store copy does not make that distinction, and a prospective buyer could reasonably read
it as HRV support. Recorded as an unresolved reconciliation question (W1).

**Other sources.** Apple Health, Google Health Connect (with a documented Google Fit transition),
Samsung Health, Mi Band, RescueTime and Slack (productivity), Netatmo home weather stations (indoor
temperature, humidity, air quality), Withings Body+ (indoor CO₂, indoor/outdoor temperature), sleep
trackers via the health platforms (SleepCycle, Sleep As Android, Pillow, Sleep++, Fitbit, Misfits,
Runtastic), meditation and nutrition apps, and blood pressure monitors as listed above.

**Environment.** Weather — temperature, humidity, atmospheric pressure — is transferred automatically
from a weather service based on device location, requiring geolocation permission. **The data source is
connected for all users automatically.** This is a genuine overlap with AnxietyWatch v1's barometric
pressure context, and it is automatic rather than manual.

**Connecting data sources requires the paid version.** Documented: "Connecting data sources is only
possible in the full version of the app."

## 6. Real-time, background, and overnight behaviour

- **No continuous capture.** Welltory measures on demand. There is no overnight recording, no
  always-on session, and no continuous stream documented. Apple Watch and Samsung Watch supply
  automatic measurements through the health platform.
- **Episode detection is retrospective-ish, not an alarm.** "Welltory detects when your body goes
  through stress or recovery episodes throughout the day" and then prompts for context. This is
  closer to real-time insight than anything Bearable offers, but it is a notification and an
  interpretation, not a monitored alarm with an escalation path.
- **Sync latency is documented as non-trivial.** "Synchronization takes time... sometimes it can take a
  few hours or one day. For example, if you analyze sleep, the data won't be uploaded on the graph right
  after the night but the next day." This rules out any same-night analysis from synced sources.
- **Alarm path: none.** No alarm, no caregiver escalation, no dead-man switch, no technical alarm.
- **Background behaviour** is not documented beyond platform health sync.

## 7. Anxiety, mood, symptom, medication and subjective input

| Capability | Finding |
|---|---|
| Anxiety-specific instrument | Premium includes "clinically validated tests: check in on possible anxiety, burnout, or depression symptoms" (specific instruments not named) |
| Panic-time interaction | Not documented. Marketing offers "tips to... support a panic attack"; there is no panic-triggered logging flow |
| Mood logging | Manual mood entry; "fill out mood and health askers" on the free tier |
| Symptoms | Presents as symptom tracking in store copy; depth of symptom modelling is not documented |
| Free-text context | Strong: episode prompts accept typed or spoken free text, auto-tagged into short labels |
| Tags | System-extracted from free text, correctable by the user. Documented as not limited to a fixed list |
| Medication tracking | **Not documented.** No medication feature, dosing, or reminder appears in the feature lists examined |
| Dose-triggered before/after efficacy | **Not present for medication.** The paired before/after *Experiments* design exists but is user-initiated around arbitrary activities, not triggered by a dose |
| Weather/subjective correlation | Manual tags include "rainy weather" as an example, plus automatic weather data |

**Interpretation.** Subjective input in Welltory exists to *explain detected physiological episodes*,
which is the inverse of AnxietyWatch v1's model where the subjective rating is the primary outcome and
physiology is the context. Both designs are defensible; they answer different questions.

**Independently reported weakness.** A Play Store reviewer, having used the app for six months with
twice-daily measurements, reports that it "still hasn't made a single connection", that tags cannot be
edited or reorganised, and that entry timestamps cannot be corrected. The tag-editability point is in
direct tension with the documentation's claim that the user can correct tags before saving, though the
reviewer may be describing post-hoc editing. Recorded as **[observed]**, single report, unverified.

## 8. Sleep, CPAP, respiratory, SpO₂, HRV and environmental data

| Capability | Finding | Evidence class |
|---|---|---|
| HRV | **The core product.** 21 metrics per measurement; a self-learning algorithm adapting to the individual baseline | Publicly claimed + documented |
| Personal physiological baseline | **Yes** — the central claim: "adapts to your individual baseline instead of applying one-size-fits-all thresholds" | Publicly claimed |
| Personal baseline — caveat | A fixed "medical threshold" still applies; users whose HRV sits below it get flagged regardless of baseline | Vendor-documented limitation |
| Sleep staging | Sleep duration, stages (REM/deep/light), quality metrics, timing, interruptions, snoring, respiration rate in sleep — enumerated in the privacy policy as ingested categories | Documented (privacy policy) |
| Sleep analysis | Personalised sleep analysis and Sleep Flow on Premium; sourced from wearables and health platforms | Publicly claimed |
| CPAP / PAP therapy | **No support found.** No CPAP, AHI, leak, pressure or therapy-adherence feature appears anywhere examined | Not supported (absence of evidence) |
| Obstructive sleep apnea | Mentioned only in editorial blog content and as an *explanation* for SpO₂/BP relationships in the blood-pressure article. Not a feature | Publicly documented as context only |
| Respiratory rate | Enumerated among ingested vital signs and sleep metrics in the privacy policy | Documented |
| SpO₂ / oxygen saturation | Enumerated as an ingestible vital sign, listed in the data-sources article among ingestible metrics, and discussed at length in the blood-pressure article | Documented |
| Barometric pressure | **Yes** — automatic via weather service, by location | Documented |
| Environmental/indoor context | Netatmo indoor temperature, humidity, air quality; Withings Body+ CO₂ | Documented |
| Environmental noise | **No support found.** Audio is used for content (Echo/Sleep Flow), not measurement | Not supported (absence of evidence) |
| Aircraft overflight context | Not supported | Not supported |

**Interpretation.** Welltory holds a genuine personal-baseline HRV design — the same *idea* as
AnxietyWatch v1's rolling personal baseline. But the sensors are different in kind: PPG or a
consumer watch rather than Polar H10 RR at ~1 Hz with per-sample provenance, and there is no
overnight capture at all. AnxietyWatch's CPAP, oximetry and respiratory-event support has no
Welltory equivalent.

## 9. Weather, aircraft, noise and sleep-context features

- **Weather: automatic and real.** Temperature, humidity and atmospheric pressure arrive from a weather
  service based on location, with the source enabled for all users. This is closer to AnxietyWatch v1's
  barometric-pressure context than Bearable's manual factors.
- **Indoor environment:** supported through third-party hardware (Netatmo, Withings Body+), which is a
  coverage AnxietyWatch does not claim.
- **Environmental noise:** not a Welltory capability. Audio is playback only.
- **Aircraft overflight:** nothing. As with every candidate so far, this remains an AnxietyWatch idea
  with no competitor equivalent.

## 10. AI, statistical analysis, correlations and personalisation

- **AI: yes, but external and gated.** "Welltory AI Coach is an AI assistant based on OpenAI's GPTs
  technology." It is **not part of the Welltory app** — the privacy policy states it "is available as an
  external feature hosted on OpenAI's ChatGPT platform and is not part of the Welltory App or Services."
  Connecting it requires a consent redirect through a Welltory authorisation page, after which OpenAI
  receives access to selected Welltory data via a secure API endpoint.
- **Dual paywall.** "Full access to Welltory AI Coach's features requires a subscription to Welltory
  Premium **and ChatGPT Plus version**." So the AI feature requires *two* paid subscriptions, one of
  them to a different vendor.
- **AI is explicitly beta and unreliable.** The vendor's own notice: experimental beta, "not a
  substitute for professional medical advice", may "produce 'hallucinations' or inaccurate answers",
  and the user "accept[s] the responsibility to verify the information provided". This is a more
  explicit warning than most products give, and it should be weighted accordingly.
- **Privacy claim for AI.** Welltory states it does not receive or access ChatGPT conversations, that the
  AI Coach cannot send data back to Welltory, that it implements Zero Data Retention settings where
  technically feasible, and that it does not sell data to third parties. It also states that users'
  data is not used "to train or improve their models" — i.e. the external AI provider's models.
- **Correlation engine.** "Our algorithms search for correlations in this data, like the stats of your
  workouts, meditation, sleep and productivity." Correlational analysis over ingested lifestyle data
  against HRV.
- **Episode detection.** A signal-processing layer that classifies stress and recovery episodes from
  heart rate and physiological signals. This is a genuine detection feature, not just a chart.
- **Insights.** Generated from patterns once sufficient data exists (roughly seven occurrences).
  At most one My patterns insight appears at a time.
- **Personalisation.** Two layers: the self-learning HRV baseline (model-based) and user tags/
  experiments (configuration-based). This is more model-driven personalisation than Bearable offers.
- **Validation claims.** The site claims PPG-based phone-camera measurement was shown to equal Polar
  chest straps "which are ECG-accurate" in the vendor's own study; that Apple Watch HRV was validated
  against Polar H7 and Samsung HRM; that research in April 2022 validated Welltory for HRV monitoring
  in elite endurance athletes; and that results were presented at the 41st International Engineering in
  Medicine and Biology Conference in Berlin. These are vendor statements about vendor-affiliated
  research and were **not independently verified** in this pass.

**Assessment.** Statistically, Welltory is more methodologically serious than Bearable: it has a
self-learning baseline model, episode detection, and a paired before/after design. But its own
documentation concedes a fixed threshold that overrides the personal baseline, and the AI layer is a
gated beta on a third-party platform. The claims about "hundreds of millions of data points" and
"world's first self-learning HRV algorithm" are unverifiable marketing.

## 11. Reports, clinician sharing, export and integrations

| Capability | Finding |
|---|---|
| Export | Documented as a user right: "You can export your data for your own use and share it with third parties if you wish (data portability)" |
| Blood-pressure report | Premium: "send your data to your doctor or print out neat reports with all of your readings" |
| Clinical PDF | A printable BP report is documented. A general clinical PDF across all data is **not** documented |
| Clinician portal | None |
| Write-back to health platforms | Not documented |
| Public API | Available for research partners ("Welltory for research"), not as a general consumer API |
| Open source | The site footer links an "Open Source" page, implying published components; not examined in this pass |
| Data loss risk | **Free tier deletes data older than 30 days.** The vendor advises exporting in advance. Premium retains full history |

The free-tier 30-day deletion is the single most consequential export finding. AnxietyWatch's store is
append-only by design and never deletes samples; Welltory's free tier is a **retention cliff**.

## 12. Privacy, hosting, account, telemetry and data ownership

| Aspect | Finding |
|---|---|
| Account required | **Yes** — email/password, or Apple/Google/Facebook sign-in (required: "email address and password (encrypted)") |
| Hosting | Welltory's own servers; US data controller with UK DPO and EU representative |
| Encryption | "robust security measures"; passwords encrypted. Specific at-rest encryption of health data is not stated in the terms examined |
| Data sale | Denied repeatedly: "never sell your personal data", "We do not sell your personal data for monetary consideration and do not share health or wellness data for advertising purposes" |
| **Advertising disclosure** | **Explicit and material.** Welltory discloses hashed email address, online identifiers, device information and internet activity data to **Meta Platforms (Meta Pixel, Custom/Lookalike Audiences, Remarketing), Google LLC (Google Ads, Google Ad Manager) and TikTok** for conversion tracking, ad-campaign optimisation and audience-segment building. Analytics and attribution providers named include **Amplitude, AppsFlyer, Firebase and Crashlytics** |
| Health data in advertising | Stated as excluded: not shared for advertising purposes |
| Consent model | Health-data processing is consent-based; withdrawing consent deletes the account because there is no remaining legal basis |
| Research sharing | Aggregated and anonymised data is shared for research on a legitimate-interest basis; "No individual-level data is shared with external researchers unless we have your additional consent" |
| Prior open-data project | A 2020 COVID-19 open-data research project released a collected dataset (with Apple Watch, Garmin, Fitbit data) to researchers |
| HIPAA | **Not a covered entity or business associate.** The policy states HIPAA requirements do not apply |
| Jurisdiction-specific | CCPA/CPRA, VCDPA and **Washington MHMDA** rights are addressed, including the right to a list of consumer health data categories and recipients |
| Local-first operation | **No.** Core functionality "relies on the analysis of health data" in the vendor's service |
| Self-hosting | **No** |
| Data location | US company; international transfers addressed in the policy |
| Deletion | Available in app and web app, but **not self-serviceable while a Premium subscription is active** — the user must wait for expiry or contact support |
| Platform data-safety declaration | Play Store: "may share... with third parties: Location, Personal info and 3 others"; "may collect: Personal info, Health and fitness and 3 others"; "Data is encrypted in transit" |

**Net position.** Welltory's health-data handling rhetoric is strong and its policy is detailed and
comparatively transparent — it names its advertising partners and its DPO, addresses MHMDA explicitly,
and separates health data from advertising. But by its own disclosure it operates a **substantial
advertising and analytics SDK footprint** with hashed identifiers leaving the device, and it is a
US-hosted cloud service with no local-only mode. That is a categorical difference from AnxietyWatch's
no-SDK, no-telemetry, local-first design, and it is documented by the vendor rather than inferred.

## 13. Pricing, subscriptions, hardware and geographic limits

| Item | Finding |
|---|---|
| Model | Free tier + Premium subscription + separate Lifetime purchase. A 3-day (72-hour) free trial begins after the first measurement |
| **Annual** | **$99/year**, billed once, presented as **$8.25/month** |
| **Lifetime** | **$599** one-time payment, "full access forever" |
| Monthly plan | **Not offered** on the plans page examined. Only Annual and Lifetime were shown |
| Play Store IAP range | **$4.99 – $79.00 per item** |
| Discounting | A 35% off annual promo was active on the site at the research date; promo-code activation and gift certificates both exist; Black Friday FAQ article exists |
| Refunds | 14 days from first payment; 48 hours for a renewal charge; full refunds only for annual plans; App Store purchases refunded by Apple |
| Hardware cost | **$0 required.** Phone camera works. Chest strap (~$50–$90) and smartwatch are optional upgrades. Netatmo/Withings sensors are optional extras |
| Hidden requirement | AI Coach needs **ChatGPT Plus in addition to Premium** |

**Free tier limits (documented).** One HRV measurement per day, 5 metrics instead of 21, simplified
sleep and blood-pressure reports, experiments disabled, **history limited to the past 30 days** with
anything older **deleted** unless exported, and data-source connections unavailable.

**Interpretation.** Welltory is the most expensive candidate examined on a recurring basis: $99/year is
roughly double a typical consumer tracker subscription, the lifetime price is $599, and the headline AI
feature requires a second subscription to a different company. Against AnxietyWatch's
free-software-plus-hardware model, and against Bearable's generous free CSV export, Welltory's free
tier is deliberately crippled by a data-retention cliff.

## 14. Clinical and regulatory claims

- **No medical-device claim.** The vendor states plainly: "Welltory is not a medical app and can't be
  used for diagnostic purposes or as a substitute for professional medical advice."
- The compatible-devices article repeats: the devices "are not medical devices, and the data provided by
  them is not intended to be utilized for medical purposes".
- Blood-pressure analysis is explicitly bounded: "any medical assessment or recommendations can only be
  given by your physician. Our interpretations cannot be considered a medical assessment or used as
  recommendations to start, change, or end any treatment."
- **HRV's limits are stated unusually honestly:** "HRV isn't a universal diagnostic tool — it shifts with
  any health condition, signaling that something is off but not pinpointing exactly what." And: if the
  heart does not respond normally to regulation (heart block, rate-affecting medication, pacemaker,
  arrhythmia), "HRV can't provide meaningful insights".
- **Backing evidence.** Vendor-affiliated research and conference presentations; "backed by over 25,000
  studies on PubMed" (referring to HRV generally, not to Welltory); "quoted by Johns Hopkins University,
  Harvard Medical School, TechCrunch" (media/institutional mention, not endorsement or validation);
  30+ university partnerships. The specific peer-reviewed papers were **not verified** in this pass.

**Assessment.** The on-label claims are appropriately conservative, and the chronic-conditions article
is more honest about HRV's limits than most competitors. But store copy simultaneously asserts that
"Welltory's PPG-based phone camera measurements are equal to measurements taken with Polar chest
straps" and that "studies show that our hrv measurement method is as accurate as ECGs" — claims a user
will read as validation of the product's conclusions, not merely of its pulse detection.

## 15. Evidence table

| # | Claim or fact | Source | Publisher | Date | Access | Confidence |
|---|---|---|---|---|---|---|
| W1 | Purpose, energy/burnout positioning, scale claims | https://welltory.com/ | Welltory Inc. | undated | 2026-10-06 | High (vendor claim) |
| W2 | Not for people with chronic conditions; fixed medical threshold; HRV limits | https://help.welltory.com/en/articles/12927837-why-welltory-isn-t-really-for-people-with-chronic-conditions-yet | Welltory Inc. | 2026-01-23 | 2026-10-06 | High (vendor-documented limitation) |
| W3 | Personal-baseline algorithm claim | https://help.welltory.com/en/articles/3352436-what-s-special-about-welltory-s-analysis | Welltory Inc. | updated ~2026-09 | 2026-10-06 | High (vendor claim) |
| W4 | Free trial contents; free-tier 30-day deletion; Premium feature list | https://help.welltory.com/en/articles/11659476-welltory-subscription-which-one-to-choose-how-to-cancel-or-get-a-refund-and-how-to-delete-your-data | Welltory Inc. | 2026-01-23 | 2026-10-06 | High (documented) |
| W5 | Premium description and 30-day free-tier deletion restated | https://help.welltory.com/en/articles/4007331-welltory-premium-personalized-guidance-habit-analytics-and-so-much-more | Welltory Inc. | updated ~2026-09 | 2026-10-06 | High (documented) |
| W6 | Annual $99 / Lifetime $599 pricing | https://welltory.com/plans/ | Welltory Inc. | live at access | 2026-10-06 | High (vendor pricing page) |
| W7 | Play Store: 5M+ downloads, 4.6★, ~61K reviews, IAP $4.99–$79.00, updated 2026-09-14 | https://play.google.com/store/apps/details?id=com.welltory.client.android | Google Play | 2026-09-14 | 2026-10-06 | High (platform-reported) |
| W8 | Android data-safety declaration: sharing and collection categories | same as W7 | Google Play | 2026-09-14 | 2026-10-06 | High (platform-declared) |
| W9 | HRV-capable and incompatible devices | https://help.welltory.com/en/articles/4412301-compatible-sports-heart-rate-trackers-and-other-devices | Welltory Inc. | 2024-06-19 | 2026-10-06 | High (documented) |
| W10 | Garmin/Oura/Mi Band/Fitbit cannot supply HRV | https://help.welltory.com/en/articles/3379398-why-you-can-t-use-most-fitness-trackers-and-smartwatches-to-measure-stress-and-energy | Welltory Inc. | updated ~2026-09 | 2026-10-06 | High (documented) |
| W11 | Store copy claiming Fitbit/Garmin/Oura/Mi Band sync (conflict with W10) | https://play.google.com/store/apps/details?id=com.welltory.client.android | Google Play | 2026-09-14 | 2026-10-06 | High (vendor copy) |
| W12 | Weather data automatic by location; geolocation required | https://help.welltory.com/en/articles/3364385-how-to-analyze-weather-data | Welltory Inc. | 2022-07-05 | 2026-10-06 | High (documented) |
| W13 | Netatmo indoor stations; Withings Body+ CO₂ | https://help.welltory.com/en/articles/3364378-gadgets-for-tracking-surroundings | Welltory Inc. | 2025-01-27 | 2026-10-06 | High (documented) |
| W14 | Blood pressure: not measured; monitors supported; ESC/ESH 2024; not a medical assessment | https://help.welltory.com/en/articles/3878866-how-to-analyze-blood-pressure-with-welltory | Welltory Inc. | updated ~2026-09 | 2026-10-06 | High (documented) |
| W15 | My patterns: episode detection, voice/typed free text, auto-extracted tags, ≥7 occurrences | https://help.welltory.com/en/articles/16012012-my-patterns-ios | Welltory Inc. | 2026-09-03 | 2026-10-06 | High (documented) |
| W16 | Experiments: paired before/after, 3-hour window, one activity at a time | https://help.welltory.com/en/articles/5714460-experiments | Welltory Inc. | 2023-05-15 | 2026-10-06 | High (documented) |
| W17 | AI Coach is external on OpenAI ChatGPT; requires Premium **and** ChatGPT Plus; beta; hallucination warning | https://help.welltory.com/en/articles/8727843-how-to-use-welltory-ai-coach | Welltory Inc. | updated ~2026-09 | 2026-10-06 | High (documented) |
| W18 | Privacy policy: advertising/analytics partner disclosure, no data sale, HIPAA non-coverage, MHMDA, research sharing, AI data flow | https://welltory.com/privacy-policy/ | Welltory Inc. | 2026-09-01 | 2026-10-06 | High (primary legal document) |
| W19 | Sync latency: hours to a day; sleep not available the next morning's graph | https://help.welltory.com/en/articles/3364884-data-we-collect-from-fitness-trackers-and-smartwatches | Welltory Inc. | 2025-10-02 | 2026-10-06 | High (documented) |
| W20 | Connecting data sources requires the full version | https://help.welltory.com/en/articles/3364292-gadgets-and-apps-for-sleep-tracking | Welltory Inc. | 2025-06-30 | 2026-10-06 | High (documented) |
| W21 | PPG claim: phone camera equal to Polar chest straps; ECG-equivalence claim in store copy | https://help.welltory.com/en/articles/3352614-why-trust-photoplethysmography-or-ppg + Play listing | Welltory Inc. | 2025-10-02 / 2026-09-14 | 2026-10-06 | Medium (vendor-affiliated research, not verified) |
| W22 | Research claims: April 2022 athlete validation; 41st IEEE EMBC Berlin; 25,000 PubMed studies; university partnerships | https://welltory.com/science/ | Welltory Inc. | undated | 2026-10-06 | Low-Medium (unverified vendor claims; papers not located) |
| W23 | Account deletion unavailable while Premium is active | same as W4 | Welltory Inc. | 2026-01-23 | 2026-10-06 | High (documented) |
| W24 | Independent user report: no correlations after 6 months; tags not editable or organisable; times not editable | Play Store review text | end user | undated | 2026-10-06 | Low (single third-party report) |
| W25 | Ingestion of respiratory rate, SpO₂, ECG, EDA, peripheral perfusion index, sleep stages, snoring | https://welltory.com/privacy-policy/ | Welltory Inc. | 2026-09-01 | 2026-10-06 | Medium-High (category list in a legal document; availability per platform unverified) |
| W26 | No CPAP feature | Absent from all pages, support articles, plans and store listings examined; apnea appears only in editorial content | Welltory Inc. | — | 2026-10-06 | Medium (absence of evidence, not proof of absence) |

**Retrieval failures:** `welltory.com/pricing/` and `welltory.com/terms-of-use/` returned HTTP 404;
pricing was obtained from `welltory.com/plans/` instead. The iOS App Store listing was not retrieved
(the guessed URL 404'd); Apple-side rating and IAP data are therefore **not included**, and only
Play-Store figures are cited. The peer-reviewed papers behind the science-page claims were not located.

## 16. Key similarities and differences versus AnxietyWatch

### Versus AnxietyWatch v1 (documented/implemented)

**Similar in intent**
- **Personal baseline vs population norms.** Both compute baselines against the individual rather than
  the population, and both market that as the core methodological choice.
- **Paired before/after measurement.** Welltory's Experiments measure HRV before and after an activity
  and report the change; v1 prompts a paired rating around a medication dose.
- **Free-text context turned into structure.** Welltory auto-extracts tags from typed or spoken text;
  v1 captures notes and tags alongside readings.
- **Weather/barometric context.** Both bring atmospheric data in automatically, though from different
  sources (weather service by location vs Core Motion barometer).
- **Physiological episode detection.** Welltory classifies stress/recovery episodes; v1 computes HRV
  metrics and below-baseline alerts.
- **Both warn that they are not medical devices** and both are explicit about it.

**Different in substance**

| Dimension | AnxietyWatch v1 | Welltory |
|---|---|---|
| Primary outcome | Subjective anxiety rating | HRV measurement |
| Primary sensor | Polar H10 raw RR (~1 Hz), HealthKit, CPAP SD, EMAY oximeter | Phone-camera PPG, Apple/Samsung Watch |
| Overnight capture | Yes — Polar H10 background recording | **None** |
| CPAP / respiratory events | SD-card import, AHI, leak, usage | **None found** |
| Oximetry | EMAY import + live BLE streaming | SpO₂ only as an ingestible health-platform metric |
| Medication support | Dose logging + dose-triggered paired follow-up | **No medication feature found** |
| Baseline model | Rolling personal baseline; alert on deviation from *your* baseline; no fixed override documented | Personal baseline **plus** a fixed medical threshold that flags regardless — per the vendor's own article |
| Data retention | Append-only store, samples never deleted; removal is a tombstone | **Free tier deletes data older than 30 days** |
| Storage | Local-first, self-hosted sync | Vendor cloud, account required |
| Telemetry | None claimed; no third-party SDKs, no analytics | Advertising pixels (Meta, Google, TikTok) and analytics/attribution SDKs (Amplitude, AppsFlyer, Firebase, Crashlytics) |
| AI | Opt-in server-side analysis on your own server | External GPT requiring Premium **and** ChatGPT Plus |
| Clinical report | Generated PDF over a chosen date range | Printable blood-pressure report; general PDF not documented |
| Real-time alarm | Local alerts with dedup/uploader | None |
| Cost | Free software; hardware optional | $99/year or $599 lifetime |

**Where Welltory is genuinely stronger**
- **HRV analysis depth**: 21 metrics, self-learning baseline, episode detection, frequency-domain scores.
- **Zero required hardware.** The phone camera alone is sufficient, which is a real accessibility
  advantage — and directly relevant to any AnxietyWatch user who does not own a Polar strap.
- **Automatic weather/environment integration** with no user effort, including indoor sensors.
- **Breadth of ingestible context** (productivity, nutrition, meditation, lifelogging, geotracking).
- **Polished, shipped, at scale** with a documented refund policy and self-service deletion (except
  while subscribed).

### Versus AnxietyWatch2 (current + specified target)

- **V2 current overlap: minimal but non-zero.** Welltory has episode *detection* from physiological
  signals — the closest thing on the shortlist so far to AnxietyWatch2's detector ambition. It has no
  continuous collection, no provenance model, no three-state missing-data rule, no local console and no
  alarm.
- **V2 specified target overlap: none.** The supervised alarm, dead-man switch and
  provenance-preserving store have no Welltory equivalent.
- **Critical architectural difference: what happens to data over time.** AnxietyWatch's append-only
  store and "never coerce, never clamp, never delete" rules are the inverse of a free tier that deletes
  history at 30 days and an algorithm with a fixed threshold that overrides the personal baseline.
- **Welltory is a complement, not a substitute.** It provides a zero-hardware HRV measurement and a
  rich contextual layer; AnxietyWatch2 provides continuous overnight capture and a monitored alarm path.

## 17. Whole product or subsystem?

**Whole product for the "measure HRV, explain it, and correlate it with lifestyle" problem — and a
partial competitor to v1 there.** It does not compete with v1's CPAP, oximetry, overnight capture,
medication follow-up, PDF reporting or self-hosting, and it does not compete with AnxietyWatch2 at all.
The overlap that matters is the **baseline methodology**, where Welltory is the closest public analogue
and, on the strength of its own chronic-conditions article, also a cautionary example.

## 18. Weaknesses, omissions and unresolved questions

**Weaknesses and omissions (evidenced)**
1. **Free tier deletes data older than 30 days**, with the vendor advising users to export first. A
   retention cliff on health data is a severe limitation for longitudinal self-study.
2. **Fixed medical threshold overrides the personal baseline**, producing "constant alerts" for people
   with chronic conditions — per the vendor's own documentation. HRV is also unusable when cardiac
   regulation is impaired.
3. **No CPAP, no respiratory-event data, no overnight capture.** For an apnoeic anxiety patient the
   product is missing the capability entirely.
4. **No medication feature at all**, so the v1 medication-efficacy workflow has no equivalent.
5. **Advertising and analytics SDKs** (Meta, Google, TikTok pixels; Amplitude, AppsFlyer, Firebase,
   Crashlytics) with hashed identifiers, devices and online activity disclosed to advertising partners.
6. **AI requires two subscriptions from two vendors** and is a beta with an explicit hallucination
   warning.
7. **Store marketing conflicts with support documentation** on which wearables supply HRV (Fitbit,
   Garmin, Oura, Mi Band).
8. **Expensive and non-standard billing**: $99/year or $599 lifetime, no monthly plan, and account
   deletion is blocked while a subscription is active.
9. **Sync latency of hours to a day** rules out same-night analysis from synced sources.
10. **Platform feature asymmetry** — several key features are documented as iOS-only.

**Unresolved questions for Prompt 2**
- **W1** Does "sync with Fitbit, Garmin, Oura, Mi Band" in store copy mean HRV, or only activity/sleep
  context? Documentation says those devices cannot supply HRV. Which is operative?
- **W2** What is the fixed "medical threshold" and how does it interact with the personal baseline?
  Under what conditions does the baseline actually govern the verdict?
- **W3** Is there any medication, supplement or dose-tracking feature at all?
- **W4** What exactly does the export produce — an API, a file, per-metric granularity; and can it be
  automated before the 30-day deletion?
- **W5** Are the 21 HRV metrics computed on-device or server-side, and what happens to measurements when
  offline?
- **W6** Which specific "clinically validated tests" are offered for anxiety, and are they named and
  referenced?
- **W7** What are the exact sleep metrics ingested versus merely accepted as categories (the privacy
  policy enumerates many that may not be reachable per platform)?
- **W8** What is the real-world reliability of `My patterns`? The vendor requires ~7 occurrences; one
  long-term user reported zero connections in six months. Is there a documented minimum-data rule?
- **W9** What do the "3 others" shared and "3 others" collected in the Play data-safety declaration
  resolve to?
- **W10** Does any Welltory output constitute a clinician-shareable report beyond the printable
  blood-pressure report?
- **W11** Are the science-page validation claims backed by retrievable peer-reviewed papers?
- **W12** Is the periodic HRV "episode detection" preventive in any sense, or purely descriptive after
  the fact?

## 19. Related files

- Implementation inspection: `welltory_IMPLEMENTATION_INSPECTION.md`
- Candidate-specific comparison prompt: `PROMPT_2.md`
- Comparison report: `welltory_COMPARISON.md`
- Shortlist: `../CANDIDATE_SHORTLIST.md`
- Method and limits: `../RESEARCH_METHOD.md`
