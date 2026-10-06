# Bearable — claimed features and purpose

**Slug:** `bearable`
**Research date:** 2026-10-06
**Phase:** Phase 2 (public claims)
**Implementation inspection:** not performed — see `bearable_IMPLEMENTATION_INSPECTION.md`
**Shortlist rank:** 1 of 10 (`../CANDIDATE_SHORTLIST.md`)

---

## 1. Executive summary

Bearable is a commercial, freemium symptom- and mood-tracking app for iOS and Android, built by
Bearable Ltd (UK) for people managing chronic physical and mental health conditions. It is the
closest single whole-product analogue to **AnxietyWatch v1**: it combines daily subjective logging
(mood, emotions, symptoms, energy, sleep, factors, medications, notes) with one-way import of
objective data from Apple Health or Google Health Connect, and it computes factor-versus-outcome
correlations that the user is then meant to take to a clinician. It is mature and widely used
(Play Store lists 500K+ downloads and 10.3K reviews; the App Store listing claims "over 2M patients
& doctors").

The two products diverge sharply on architecture and on what the data is *for*. Bearable requires an
account, stores health data on Bearable's own servers under server-side encryption, and performs its
analysis in the cloud. AnxietyWatch v1 keeps data local and self-hosted, keeps raw sensor provenance,
and generates a clinical PDF; Bearable exports CSV but has no PDF export, and exports are the user's
responsibility to interpret. Bearable has no live or overnight monitoring, no direct BLE sensor
support, no CPAP or oximetry ingestion, and no alarm path — so it competes with AnxietyWatch v1's
*journal-and-correlate* loop only, and not at all with the AnxietyWatch2 platform layer.

## 2. Product purpose and target user

- **Stated purpose.** "Discover what improves and worsens your symptoms so that you can find better
  ways to manage your health and well-being." (homepage)
- **Target user.** People with chronic or episodic conditions — the site and store copy enumerate
  chronic pain, migraine, chronic illness, ME/CFS, long Covid, PoTS, EDS, IBS/IBD, PCOS/PMDD,
  fibromyalgia, endometriosis, and the mental-health set (anxiety, depression, bipolar, BPD, ADHD,
  OCD, PTSD).
- **Explicit framing.** The product positions itself as charting the *relationship* between
  behaviours/treatments and symptoms, not as a treatment. Founder-led and community-driven since
  inception ("made by patients for patients").
- **Anxiety placement.** Anxiety is one of many listed conditions (`/depression-tracker/`,
  `/mood-tracker-app-journal/`, the App Store and Play Store copy). There is no anxiety-specific
  workflow, no panic-attack flow, and no anxiety-severity instrument — anxiety is a trackable
  symptom/emotion among others.

## 3. Availability status

| Question | Finding |
|---|---|
| Status | **Active.** Play Store listing shows an update dated Sep 14, 2026; support articles updated 2025–2026. |
| Platforms | iOS, iPadOS, macOS (the iOS app, not a native desktop build), Android. |
| Platform limits | **No browser or desktop-native version.** Support article states this is explicitly not planned as a near-term priority. |
| Geography | Not restricted in the sources examined; UK-based vendor with US store listings. |
| Consumer research product | A separate "Bearable Pro" research/ePRO offering exists with an admin panel and CSV export for study teams. |

## 4. User-facing workflows

From the homepage, support articles and store listings:

- **Daily check-in.** Mood, emotions, energy, sleep quality/quantity, symptoms (severity and
  frequency), factors, notes, gratitude. Support documents a "Health Check-in" that can be repeated
  multiple times per day.
- **Custom trackers.** Unlimited custom symptoms, symptom groups, emotions/feelings, and "Other
  Factors," each with configurable scales (binary or graded).
- **Medications and supplements.** Setup with name, unit and dose strength; per-dose logging with
  adjustable time and count; per-medication reminders; adherence review via an advanced medication
  report with a calendar overlay.
- **Trends and insights.** Trend charts, symptom-prevalence ranking, factor breakdown, calendar and
  timeline views, comparison graph, and (premium) the Impacts tab and Factor Effect reports.
- **Experiments and goals.** Structured "health experiments" — hypothesise that a habit changes a
  health outcome, track it, and review. Limited on Free, unlimited on Premium.
- **Appointment preparation.** A dedicated support article frames the product's clinical use as
  *preparation plus screenshots*, not report generation.
- **Period/menstrual-cycle tracking**, pain tracking, nutrition and hydration tracking, bowel
  movements.

## 5. Data sources and supported hardware

| Category | Detail |
|---|---|
| Native Bluetooth sensors | **None documented.** No BLE pairing, no chest strap, no oximeter, no CPAP, no blood-pressure cuff. |
| Health-platform import | Apple Health (iOS) and Google Health Connect (Android). Fitbit data arrives via Google Health after Google Fit's retirement. Import is **read-only and one-way**: the support article "Can I write or import data to another app from Bearable?" states Bearable "only reads data from other health apps — it doesn't write data back to them." |
| Imported metrics | Store copy names steps, sleep and heart rate; the privacy policy additionally lists blood pressure, blood glucose, temperature, activity, calories and weight. A user review also names HRV. The full authoritative metric list is not published. |
| Manual logging | Everything else — symptoms, factors, medications, notes, nutrition, mood. |
| Weather | **Not integrated.** "Right now, it's not possible to sync local weather data into Bearable." Weather can only be tracked by creating manual binary factors (humidity, UV, rainfall, wind, pollen, or banded AQI ranges). |

**Provenance.** Nothing in the public material describes per-sample source attribution (e.g.
distinguishing a cuff reading from a wrist reading). Google Health Connect's own source-priority
feature is referenced, which implies Bearable consumes a single de-duplicated value per metric rather
than a provenance-tagged stream.

## 6. Real-time, background, and overnight behaviour

- **Real-time monitoring: absent.** No live data stream, no live charts, no continuous recording.
- **Overnight behaviour: indirect at best.** Sleep and heart rate can arrive after the fact via
  Health Connect / Apple Health; Bearable itself does not run an overnight capture.
- **Background behaviour:** notifications and reminders only. No documented background BLE session
  and no documented always-on service.
- **Alarm path: none.** No alarm, no escalation, no caregiver alerting. Notification troubleshooting
  is about reminders not firing, not about missed physiological events.
- **Missing-data semantics: not documented.** A support article ("Are blank entries incorporated into
  insights?", "What happens to factors that I leave blank?") exists, which suggests the product makes
  a deliberate choice about blanks, but the content was not retrieved in this pass. What is clear is
  that there is no three-state "not measured" model: a day with no entry is simply absent.

## 7. Anxiety, mood, symptom, medication and subjective input

| Capability | Finding |
|---|---|
| Anxiety severity scale | Available as a user-created or default symptom; no dedicated validated instrument (no GAD-7, no PDSS) documented. |
| Panic-attack workflow | Not documented. |
| Mood/emotion logging | Core feature; custom emotions, custom feelings, mood by time of day. |
| Notes and journal | Notes, gratitude, significant events; "Extra Notes" unlimited only on Premium. |
| Medication logging | Core; dose, time, count, reminders, adherence calendar. |
| **Before/after dose efficacy** | **Claimed and documented**: "whether they are working for your pain, migraine or mental health and whether they cause side effects", plus correlation of medication against outcomes. This is *correlational* — there is **no paired prompt at dose time and again 30 minutes later**, which is the AnxietyWatch v1 N-of-1 dose-triggered design. |
| Random/unsolicited check-ins | Reminders are configurable, but support describes user-scheduled reminders ("mental health check-ins"), not randomised sampling. |
| Subjective data as ground truth | Subjective ratings are the *outcome variable* in the correlation engine, which is closer to AnxietyWatch than a physiology-first product. |

**Explicit safety caveat (vendor's own).** The medication article states: "because they're correlative
and not causal we recommend not making any decisions about your medication using Effect On reports in
Bearable", and directs users to speak to a clinician. This is a stronger and more specific disclaimer
than the generic "not medical advice" line in the store listing.

## 8. Sleep, CPAP, respiratory, SpO₂, HRV and environmental data

| Capability | Finding | Evidence class |
|---|---|---|
| Sleep quality/quantity | Native manual tracking; sleep also importable from health platforms | Publicly claimed + documented |
| Sleep staging (REM/deep) | Not documented | Unknown |
| CPAP / PAP therapy data | **No evidence of any support.** A site search for "cpap" returns no product article; no CPAP, AHI, leak or pressure feature appears in any store or support page | Not supported (absence of evidence across all public surfaces examined) |
| Respiratory rate | Not documented as a Bearable metric; would only arrive if exposed through the health platform | Unknown |
| SpO₂ | Not named in Bearable's own metric list; only reachable if Health Connect/Apple Health expose it | Unknown |
| HRV | Named by a user review as visible via the connected health app; not named in Bearable's own store or privacy copy | Observed (third-party/user report) — unverified |
| Blood pressure / glucose / weight / temperature | Named in the privacy policy as importable health data | Documented |
| Barometric pressure | Not documented | Not supported |
| Environmental noise | Not documented | Not supported |
| Aircraft overflight context | Not documented | Not supported |

**Interpretation.** Bearable treats physiology as *background context imported from elsewhere*, never
as a signal it captures, validates, or reasons about at sample level. There are no personal
physiological baselines in the AnxietyWatch sense — the statistical comparison is factor-present vs
factor-absent against a subjective outcome.

## 9. Weather, aircraft, noise and sleep-context features

- Weather: manual factor only (§5). No API, no automatic context, no pressure data.
- Aircraft/noise: nothing anywhere in the public material.
- Air quality: only as a user-authored banded factor, at the vendor's own suggestion.

AnxietyWatch's weather/pressure context and its aircraft-noise idea (C14, C25) have **no equivalent**
in Bearable beyond self-authored manual tags.

## 10. AI, statistical analysis, correlations and personalisation

- **AI: none claimed.** No AI, LLM or "insight engine" language appears in the homepage, store
  listings, privacy policy or support articles. The Free-tier feature list explicitly frames AI as
  *something the user does themselves*: "Export all of your data to .csv for manual analysis (e.g. in
  Excel) or using AI tools (e.g. ChatGPT)."
- **Correlation method (documented).** A factor appears in reports once there are at least three days
  with the factor and three days without, with outcome scores on those days. Timing is considered, so
  an evening factor is not matched to a morning outcome. A 30-day tracking window is recommended.
- **Factor Effect Report.** Compares average outcome on the same day as the factor and over a 1-day to
  7-day window afterwards, allowing for delayed effects; sortable best/worst; a same-day comparison
  with and without the factor.
- **Correlations grid (Premium).** Cross-compares factor correlations across multiple outcomes.
- **False-positive awareness.** The vendor's own guidance tells users to be critical: it gives the
  concrete worked example that "acts of kindness" may appear to worsen sleep quality because kindness
  days also carried junk food, caffeine and high stress. The recommended remedy is a "Favorite
  Correlations" chart — i.e. user judgement, not confounder control.
- **Personalisation** is UI/configuration-based (custom trackers, custom scales, home-screen layout),
  not model-based.

**Assessment.** There is no multi-variable model, no confounder adjustment, no confidence interval,
no multiple-comparison correction and no personal physiological baseline. Statistically this is a
straightforward two-group comparison that a user can already perform in a spreadsheet — which the
vendor acknowledges by pointing Free users at Excel and ChatGPT.

## 11. Reports, clinician sharing, export and integrations

| Capability | Finding |
|---|---|
| CSV export | **Yes, on Free.** "Export all of your data to .csv". In-app: Settings → Data + Security → My Data → Export My Data, with 3-month / 6-month / all-time ranges, then share via the OS share sheet. |
| Filtered export | Premium/timeline allows filtering by category (symptoms, moods, notes) before export. |
| **PDF report** | **Not available.** The no-desktop article lists "options for PDF export" as an outstanding community request. The recommended clinician workflow is *screenshots* of trend and impact views. |
| Clinician-facing summary | None. No clinician view, no share link, no fax/portal integration. |
| Write-back to health platforms | Not supported (documented). |
| Public API | Only in the research/ePRO ("Bearable Pro") offering, for study teams. |
| Data portability | CSV, and account deletion in-app. |

This is the single largest functional gap versus AnxietyWatch v1, which generates a clinical PDF
report as a first-class feature.

## 12. Privacy, hosting, account, telemetry and data ownership

| Aspect | Finding |
|---|---|
| Account required | **Yes.** Email + password, or Google/Apple/Facebook sign-in. The vendor's stated reasons are cloud restore, email change and device switching. |
| Hosting | Bearable's own servers. |
| Server-side encryption | Claimed: "any health data that you choose to track for your own purposes is encrypted before it is backed up on our servers." |
| Consent model | Explicit consent on account creation for storing sensitive health data; withdrawable by deleting the account. |
| Data sale | Denied repeatedly and specifically: "will never sell any personal data". HealthKit/Google Fit data is explicitly excluded from advertising use and data-broker resale. |
| Data minimisation | Deliberately does not collect name, age, sex or gender. |
| Third-party sharing | Only when the user opts into an integration ("e.g. with Google Fit or Apple Health"). |
| Telemetry | **Yes.** Device data (model, name, identifiers, device settings, app identifier, crash information) and event/usage data (which pages or tabs are opened) are collected; IP address is collected via cookies for approximate location "if you consent". Website analytics includes Google Tag Manager and Google Ads/DoubleClick endpoints (observed in page source). |
| Jurisdiction | UK company; the policy's February 2025 update was specifically to clarify that as a UK company it is "not required to have to comply with US criminal subpoenas requesting disclosure of data". |
| Subpoena/legal access | The above is a *claim about legal obligation*, not a claim that data cannot be produced. It should not be read as a technical guarantee. |
| Android data safety declaration | Play Store states the app "may share these data types with third parties: Location, Personal info and 2 others" and "may collect ... Location, Personal info and 5 others"; "Data is encrypted in transit"; deletion can be requested. The vendor's own declaration therefore does not claim a zero-sharing posture. |
| Local-first operation | **No.** The app is usable offline for data entry in practice, but the product is explicitly built around a server account, and correlation/insight features are server-side product features. |
| Self-hosting | **No.** |
| Anonymity posture | The vendor volunteers that "many Bearable community members use 'burner' or 'throwaway' accounts, and we have no problem with you doing the same" — an unusual and privacy-friendly acknowledgement. |

**Net position.** Bearable is a *reasonable* commercial actor — no data sale, deliberate data
minimisation, encryption at rest, no ads, and an unusually candid privacy stance. It is nevertheless
a cloud product with server-side health data, usage telemetry and a mandatory account, which is a
categorical difference from AnxietyWatch's local-first, no-account, self-hosted design.

## 13. Pricing, subscriptions, hardware and geographic limits

| Item | Finding |
|---|---|
| Model | Free tier plus paid Premium; no ads. |
| Prices published on the vendor site | **No.** The website does not publish a price. |
| Play Store IAP range (US, accessed 2026-10-06) | **$4.99 – $199.99 per item.** This is the declared range across all in-app products, not the price of a specific plan. |
| Lifetime subscription | **Not offered.** The vendor instead runs a monthly social-media draw for a free-forever Premium subscription. |
| Free tier (documented) | Unlimited custom symptoms, factors, mood/energy/sleep, custom emotions, bowel movements, nutrition, medications, reminders, significant events, default health measurements, default custom ratings, limited goals, limited experiments, unlimited symptom/mood notes, **30 days** of trends and comparison graph, calendar/timeline, widgets, **full CSV export**. |
| Premium (documented) | Impacts tab and correlation reports, correlations grid, 60/90/365-day report ranges, unlimited goals/experiments/extra notes/gratitudes, custom health measurements. |
| Hardware cost | **None.** No proprietary sensor, ring or strap is required. This is a meaningful structural advantage over Oura and Empatica. |
| Trial | Free trial, with documented cancellation and refund procedures. |
| Discounts / sponsored subscriptions | Both exist as support categories. |

**Interpretation.** The vendor's own free/premium split places the *correlation and multi-outcome
analysis layer* behind the paywall while leaving raw data export free. A user who will accept CSV
and a spreadsheet can stay free indefinitely; a user who wants the analysis in the app pays.

## 14. Clinical and regulatory claims

- **No medical-device claim.** No regulatory clearance or classification is claimed anywhere examined.
- Store disclaimers: "Insights from Bearable are not medical advice. Always consult your doctor before
  making medical decisions." (App Store listing)
- The medication support article adds the correlative-not-causal warning quoted in §7.
- **Backing evidence.** The site claims Bearable was "reviewed by clinicians at Cedar Sinai [sic] as
  one of the best mobile health applications to track patient outcomes", linking to:

  > Lu DJ, Girgis M, David JM, Chung EM, Atkins KM. *Evaluation of Mobile Health Applications to
  > Track Patient-Reported Outcomes for Oncology Patients: A Systematic Review.* Advances in
  > Radiation Oncology, 2021;6(1). DOI 10.1016/j.adro.2020.09.016

  **Scope caveat:** this is a systematic review of mobile apps for **oncology** patient-reported
  outcomes. It is evidence that Bearable was reviewed and rated favourably in that context. It is
  **not** evidence that Bearable's correlation engine is valid, that its anxiety tracking is
  clinically effective, or that it changes outcomes. The vendor's phrasing ("backed by scientific
  review ... to track patient-reported outcomes") is close to the paper's actual scope, which is to
  its credit, but it is easily read as broader than it is.
- The "4.6 on Play Store / 4.7 on App Store" figures are user ratings, not clinical evidence.

## 15. Evidence table

| # | Claim or fact | Source | Publisher | Date | Access | Confidence |
|---|---|---|---|---|---|---|
| B1 | Symptom/mood/factor/medication tracking, correlations, 30+ reports | https://bearable.app/ | Bearable Ltd | undated page | 2026-10-06 | High (vendor claim) |
| B2 | Free vs Premium feature split; correlation reports gated | https://bearable.app/support/common-questions/bearable-free-vs-premium-features/ | Bearable Ltd | updated 2025-07-23 | 2026-10-06 | High (documented) |
| B3 | Correlation method: ≥3 days with / 3 without; timing-aware | https://bearable.app/support/howto/how-to-find-correlations/ | Bearable Ltd | updated 2022-08-31 | 2026-10-06 | High (documented) |
| B4 | Factor Effect Report, 1–7 day windows, best/worst sorting | https://bearable.app/support/howto/the-factor-effect-report/ | Bearable Ltd | updated 2021-06-17 | 2026-10-06 | High (documented) |
| B5 | No PDF export; PDF is an outstanding community request | https://bearable.app/support/common-questions/is-there-a-browser-or-desktop-version-of-bearable/ | Bearable Ltd | updated 2025-07-22 | 2026-10-06 | High (documented) |
| B6 | CSV export path and ranges | https://bearable.app/support/common-questions/how-to-share-data-with-a-doctor-therapist-or-other-medical-team-member/ | Bearable Ltd | updated 2025-07-22 | 2026-10-06 | High (documented) |
| B7 | Read-only import; no write-back to health platforms | https://bearable.app/support/common-questions/can-i-write-or-import-data-to-another-app-from-bearable/ | Bearable Ltd | updated 2025-07-22 | 2026-10-06 | High (documented) |
| B8 | Account required (cloud restore, device switching) | https://bearable.app/support/common-questions/why-do-i-need-to-create-an-account/ | Bearable Ltd | updated 2025-07-23 | 2026-10-06 | High (documented) |
| B9 | No weather sync; manual banded factors instead | https://bearable.app/support/common-questions/how-to-track-the-weather-in-bearable/ | Bearable Ltd | updated 2025-07-22 | 2026-10-06 | High (documented) |
| B10 | Privacy policy: no data sale, server-side encryption, telemetry, UK jurisdiction, ICO ZB189459 | https://bearable.app/privacy-policy/ | Bearable Ltd | policy dated 2025-02-06 | 2026-10-06 | High (primary legal document) |
| B11 | Android data-safety declaration incl. third-party sharing | https://play.google.com/store/apps/details?id=com.bearable | Google Play | listing updated 2026-09-14 | 2026-10-06 | High (platform-declared) |
| B12 | 500K+ downloads, 4.6★, 10.3K reviews, IAP $4.99–$199.99 | https://play.google.com/store/apps/details?id=com.bearable | Google Play | as above | 2026-10-06 | High (platform-reported) |
| B13 | No lifetime subscription | https://bearable.app/support/account-subscription/lifetime-subscription/ | Bearable Ltd | updated 2025-07-24 | 2026-10-06 | High (documented) |
| B14 | Medication correlations exist but are explicitly not for medication decisions | https://bearable.app/support/howto/how-to-use-bearable-to-manage-your-medication/ | Bearable Ltd | updated 2023-11-03 | 2026-10-06 | High (documented) |
| B15 | Systematic review in oncology PROs | DOI 10.1016/j.adro.2020.09.016 | Advances in Radiation Oncology | 2021-01 | 2026-10-06 | High (peer-reviewed); scope narrow |
| B16 | "over 2M patients & doctors" | https://apps.apple.com/us/app/bearable-mood-symptoms/id1482581097 | Apple App Store | undated listing | 2026-10-06 | Low (marketing, unverifiable) |
| B17 | HRV visible via connected health app | Play Store user review text | end user | 2025 | 2026-10-06 | Low (single third-party report, unverified) |
| B18 | No CPAP feature | Site search for "cpap" returns no product article; no CPAP claim on any store or support page | Bearable Ltd | — | 2026-10-06 | Medium (absence of evidence, not proof of absence) |
| B19 | No AI feature claimed; CSV handed to the user's own AI tool | Free-vs-Premium article | Bearable Ltd | 2025-07-23 | 2026-10-06 | Medium-High |
| B20 | Public roadmap moved to Discord/feedback form | https://changemap.co/bearable-/bearable-roadmap/ | Bearable Ltd | undated | 2026-10-06 | Medium |

**Sources that could not be retrieved:** the ScienceDirect article page itself (HTTP 403; metadata
obtained via Crossref), and the contents of the blank-entry support articles. The `emoods.net`
comparison candidate was unreachable by DNS and is excluded from the shortlist.

## 16. Key similarities and differences versus AnxietyWatch

### Versus AnxietyWatch v1 (documented/implemented)

**Similar in intent**
- Subjective-first: mood/symptom severity plus free-text notes as the anchor data.
- Medication logging with reminders, and an explicit attempt to learn whether medication *helped*.
- A correlation engine that compares days-with vs days-without a factor.
- A "take this to your doctor" framing of the product's clinical value.
- Custom, user-defined trackers and scales, so the app adapts to the person.
- Strong privacy *rhetoric*: no data sale, no ads, no engagement metrics.

**Different in substance**
| Dimension | AnxietyWatch v1 | Bearable |
|---|---|---|
| Architecture | Local-first, self-hostable sync server | Cloud account, vendor servers |
| Physiology | Direct BLE capture (Polar H10 raw RR), HealthKit with provenance | Read-only health-platform import, no provenance |
| Baselines | Personal 30-day baselines, alerts on deviation from *your* baseline | Factor-vs-outcome group comparison; no physiological baseline |
| CPAP | AirSense 11 SD import, OSCE/EDF leak parsing | None |
| Oximetry | EMAY import + live BLE streaming | None |
| Dose efficacy | Paired before/after, dose-triggered prompt at 30 minutes | Correlational only |
| Clinical output | Generated PDF clinical report | Screenshots + CSV; PDF acknowledged as missing |
| Real-time | Live oximeter streaming, background H10 recording | None |
| Environmental context | Barometric pressure, environmental sound, song/lyric capture | Manual weather tags only |
| LLM analysis | Opt-in server-side Claude analysis over a chosen date range | None; user exports CSV to their own AI tool |
| Panic-specific UX | watchOS 5-second quick log designed for a panic moment | No panic flow |

**Where Bearable is genuinely stronger**
- Breadth of customisation and condition coverage (30+ report types, period tracking, pain tracking,
  nutrition).
- Polished onboarding and a large community; faster time-to-first-value.
- No hardware requirement at all.
- A working multi-device cloud restore that AnxietyWatch v1 only gets by self-hosting.

### Versus AnxietyWatch2 (current + specified target)

- **V2 current overlap: essentially none at the product level.** Bearable has no always-on collection,
  no BLE decoding, no provenance-preserving store, no CPAP live stream, no local console, no alarm
  path, and no three-state honesty model.
- **V2 specified target overlap: none.** The `cns-engine`/`klaxond` supervised-alarm ambition has no
  analogue in a logging app.
- **Complementary rather than competing:** Bearable is a plausible *source* of the subjective layer
  that AnxietyWatch2 currently lacks. If a user ran both, Bearable would provide the subjective
  context and AnxietyWatch2 the objective, continuous, provenance-backed signal. That is a
  coexistence story, not a substitution story.

## 17. Whole product or subsystem?

**Whole product — but only one half of it.** Bearable competes with the AnxietyWatch v1 *journal,
correlate and bring-to-the-clinician* loop. It does not compete with v1's sensor capture, CPAP,
oximetry, PDF reporting or self-hosting, and it does not compete with AnxietyWatch2 at all.

## 18. Weaknesses, omissions and unresolved questions

**Weaknesses and omissions (evidenced)**
1. No PDF or clinician-ready report; the recommended clinical workflow is screenshots.
2. No CPAP, oximetry, respiratory-rate or BLE sensor support.
3. Mandatory cloud account holding sensitive health data, with usage telemetry and an Android
   data-safety declaration that names third-party sharing.
4. Correlation layer is paywalled while the underlying data is free as CSV — the value-add is
   packaging, not method.
5. No anxiety-specific instrument or panic workflow despite marketing to anxiety users.
6. No confounder handling; the vendor's own example (kindness/sleep) shows a false attribution that
   the product surfaces to the user rather than correcting.
7. No desktop/web client and no self-hosting path.

**Unresolved questions for Prompt 2**
- U1: What is the actual current price of Premium (monthly/annual, per region)? Not published.
- U2: Can *arbitrary* imported HealthKit/Health Connect metrics (e.g. HRV, SpO₂, respiratory rate)
  enter the correlation engine as outcomes, or only the fixed Bearable outcome set?
- U3: How does the app behave when an imported metric is missing or stale for a given day — is a
  partially-covered day treated as "no data", as zero, or as a normal observation?
- U4: What exactly does "Are blank entries incorporated into insights?" answer? Partially-retrieved
  question; the answer determines whether blanks are excluded or treated as low/zero.
- U5: Is any statistical significance or confidence measure shown, or only effect direction and
  magnitude?
- U6: What are the precise contents of the CSV export (raw events vs daily aggregates; timezone and
  timestamp format; unit handling)?
- U7: What does the Android data-safety declaration's "2 others" / "5 others" resolve to concretely?
- U8: Does the app function acceptably with no account at all, or is the account an absolute gate?
- U9: Was the oncology systematic review's assessment of Bearable positive, neutral, or merely
  inclusive? The specific rating in the paper was not verified.
- U10: Is there any accessibility or data-accuracy documentation for the custom scales?

## 19. Related files

- Implementation inspection: `bearable_IMPLEMENTATION_INSPECTION.md`
- Candidate-specific comparison prompt: `PROMPT_2.md`
- Comparison report (produced by running Prompt 2): `bearable_COMPARISON.md`
- Shortlist: `../CANDIDATE_SHORTLIST.md`
- Method and limits: `../RESEARCH_METHOD.md`
