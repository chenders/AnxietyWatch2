# Exist — claimed features and purpose

**Slug:** `exist`
**Research date:** 2026-10-06
**Phase:** Phase 2 (public claims) + Phase 2A (published API documentation)
**Implementation inspection:** partially performed — see `exist_IMPLEMENTATION_INSPECTION.md`
**Shortlist rank:** 3 of 10 (`../CANDIDATE_SHORTLIST.md`)

---

## 1. Executive summary

Exist is a personal-analytics and correlation service. It ingests data automatically from roughly twenty
connected services, accepts manual entries and tags for everything a sensor cannot measure, and runs
statistical correlations across the lot. It has done this since 2014, is built by a two-person
bootstrapped company in Melbourne, and is the only candidate in this shortlist that publishes its own
revenue.

For this comparison Exist is the most directly relevant product on the list, and not because of its
feature count. Three findings make it matter:

1. **It publishes its statistics.** The public API documentation exposes the actual correlation record:
   a coefficient, a **p-value**, a **sample size** (`period`), a star confidence rating, plus a pair of
   filters named `strong` and `confident`. This is the only candidate whose approach to statistical
   honesty can be inspected rather than taken on trust, and it is materially more rigorous than the
   vendor's own marketing requires.

2. **It excludes missing data instead of coercing it.** Correlations require at least three weeks of
   data for an attribute "**excluding days with no data (shown as 'N/A')**", and the API's value lists
   simply omit days that have no value. Nothing is zero-filled. That is the same instinct as
   AnxietyWatch's three-state rule, arrived at independently and shipped.

3. **It is a small, transparent, privacy-first, no-AI business** — ~940 paying users, weekly-published
   revenue, no analytics, no advertising, no investors, and an explicit refusal to add large language
   models. Sound familiar.

Where it diverges from AnxietyWatch is equally clear: Exist **measures nothing**. It has no sensor, no
CPAP, no oximetry, no alarm, no real-time path, no local store, and no clinical output beyond an export.
Its mood tracking is once per day and its correlations refresh weekly. It is an analysis layer on other
people's data, and AnxietyWatch is a collection layer with its own analysis.

## 2. Product purpose and target user

- **Stated purpose.** "By combining manual tracking with automatic syncing from other services, we can
  help you understand and optimise your behaviour." And: "Exist is for you to track and understand
  yourself, not so others can find out how to better sell you things or influence your voting
  behaviour."
- **Target user.** The quantified-self user: someone already wearing a tracker and using multiple
  services, who wants their data unified and interrogated. Explicitly **not** a clinical population —
  there is no medical framing anywhere, and no medical-device claim to disclaim. The Play listing
  positions it around "symptom triggers", "what affects your sleep" and "what factors contribute to a
  productive day".
- **Vendor.** Hello Code Pty Ltd, Melbourne, Australia. Two founders, Belle B. Cooper and Josh Sharp,
  who also answer support. Copyright 2026. No investors — "We're making money to pay ourselves a living
  wage, not to make rich people richer."
- **Anxiety placement.** Exist has no anxiety product. Anxiety appears as an *example tag* on the manual
  tracking page ("Learn how your sleep is affected when you're anxious or stressed") and as a symptom
  the user can track. There is no panic flow, no anxiety instrument, and no anxiety-specific content.

## 3. Availability status

| Question | Finding |
|---|---|
| Status | **Active.** Android updated Sep 23, 2026; iOS v4.3 released Sep 25, 2026; support articles updated through Aug/Sep 2026; site copyright 2026 |
| Platforms | iOS, Android, **web** — "No pricing tiers, one plan gets you everything. Use Exist on Android, iOS, and web" |
| Android | `co.hellocode.exist` — **10K+ downloads**, 4.5★ from ~187 ratings |
| iOS | `co.hellocode.Exist`, trackId 986201088 — 4.02★ from **47 ratings**; v4.3; 14.7 MB; iOS 18.0+ |
| **Scale** | **~940 paying users** (vendor's own published figure). Total 4-week income across all three Hello Code products: $6,648.45 |
| Geography | Global; English only; priced in USD |
| History | Operating since 2014; privacy-policy update history dates from 2014-01-28 |

**Note on scale.** At ~940 paying users Exist is two to three orders of magnitude smaller than Bearable
(2M claimed) or Welltory (5M+ downloads). That is not a weakness for this comparison — it makes Exist
the candidate whose *business situation* most closely resembles an independent project's, and whose
privacy claims are consequently easier to take at face value.

## 4. User-facing workflows

- **Mood tracking.** Rate each day 1–9 (1 terrible, 9 excellent) in two taps, with an optional note
  explaining what made the day good or bad. Delivered by iOS/Android notification or by an email the
  user can reply to. Optional; can be turned off. Once enough data exists, the app shows the mood rating
  and note from the same day a year ago (or a few months ago if a year is not available).
- **Manual tracking.** Create attributes for anything: numeric (quantity, decimal, duration, time of
  day, 1–9 scale, percentage) or tags for day-level non-numeric facts (people, places, hobbies,
  medications, symptoms). Template attributes are offered; the user can also define their own. Manual
  *entry* is supported in the mobile apps; the web client is for review and analysis. **[A — documented]**
- **Custom tags.** Day-level labels. Suggested tags are provided; users can rename, merge and delete
  them.
- **Automatic sync.** Connect services; Exist attributes are fed by whichever service is assigned to
  them, and the user can change which service supplies a given attribute.
- **Correlations.** "Correlations are the most powerful part of Exist." Each correlation shows direction
  (positive/negative), a **strength** indicator and a **confidence** indicator of up to five stars.
- **Experiments.** "Run experiments on your behaviour, and Exist will confirm or rule out your hunches."
  The mechanism is the correlation engine run against a chosen pair; there is no separate experiment
  screen documented.
- **Insights.** Daily generated insights, e.g. "Last night's sleep was surprisingly long." / "Monday is
  usually your shortest sleep of the week." Insights carry a priority (1 = today … 4 = last month).
- **Averages and trends.** Weekly averages per attribute with a day-of-week breakdown; long-term trends;
  historical browsing by week and month.
- **Weekly summary email.**
- **Goals.** "How does Exist calculate goals" is a documented help topic.
- **Retrospective.** "Each day we show you what your mood rating and note was on the same day one year
  ago."

## 5. Data sources and supported hardware

**Connected services (as published on the data-syncing page):** Android (Health Connect), iPhone (Apple
Health), Calendar, Fitbit, **Foreground**, Garmin, GitHub, Instapaper, Last.fm, Mastodon, **Oura**,
RescueTime, Strava, Swarm, Todoist, Toggl, Trakt, **Withings**. Plus Samsung Health (via a documented
sync path), Apple Weather, and "many other music services via Last.fm".

**Attribute categories and importable fields (as published):**

| Group | Fields |
|---|---|
| Activity | Steps, active minutes, floors, elevation, active energy, stand hours, cycle distance, cycle time |
| Productivity | Productive time, distracting time, commits, keystrokes, tasks completed, words written, emails received/sent, mobile screen time, pomodoros |
| Mood | Mood, daily note, energy level, stress level |
| Sleep | Time asleep, time in bed, deep/light/REM sleep, bedtime, wake time, awakenings |
| Workouts | Workouts, workout distance, workout time |
| Events | Events, time in events |
| Finance | Money spent |
| Food and drink | Energy in, alcoholic drinks, coffees, water, carbohydrates, fat, fibre, protein, sodium, sugar, caffeine, cholesterol |
| Health and body | Weight, body fat, lean mass, time meditating, average heart rate, max heart rate, resting heart rate, **HRV**, menstrual flow, sexual activity |
| Location | Check-ins |
| Media | Tracks played, articles read, words read, time gaming, time watching movies, pages read, time watching TV |
| Social | Mastodon posts/replies/boosts/favourites |
| **Weather** | Max temp, min temp, precipitation, **air pressure**, cloud cover, humidity, wind speed, day length, weather summary |

**Hardware:** none of its own. Exist is a pure aggregator and requires at least one connected service or
manual entry to be useful. There is no BLE stack, no chest strap support, no sensor pairing.

**Not listed among importable attributes:** CPAP or PAP data, AHI, leak, therapy adherence, SpO₂,
respiratory rate, blood pressure, environmental noise, or aircraft overflight. Recorded as
**absence of evidence**, not as proof of absence.

## 6. Real-time, background, and overnight behaviour

- **No real-time path.** Data is pulled from services on a schedule. "We update today's data
  periodically throughout the day. Some services, like Fitbit and Garmin, update us with your data very
  regularly. Most others update every hour or every two hours." At less regular intervals the last
  week's data is refreshed. **[A — documented]**
- **No overnight capture of its own.** Sleep arrives from Oura, Fitbit, Garmin, Apple Health or Health
  Connect, after that service has processed it. Exist adds no measurement.
- **Correlations refresh weekly** — "each Monday you'll see new correlations available", and the API
  returns "all correlations generated in the last week". Analysis latency is therefore up to seven days
  by design. **[A — documented]**
- **No alarm, no escalation, no dead-man switch, no notification on a physiological threshold.** The
  only outbound prompts are the nightly mood reminder and the weekly summary email.
- **Backfill on connect:** typically the past 30 days, with a documented floor of "at least 14 days"
  for services that sync by item count rather than time period. Initial sync completes within one to two
  hours, up to 24 hours.

## 7. Anxiety, mood, symptom, medication and subjective input

| Capability | Finding |
|---|---|
| Mood | **Built in**, 1–9 daily, optional note, nightly reminder or email reply. No external service required |
| Anxiety-specific instrument | **None.** No GAD-7, no PHQ, no "clinically validated tests" of any kind |
| Panic-time interaction | **None.** Mood is captured once per day, retrospectively, by the user's own choice of when to answer |
| Symptoms | Supported **as manual attributes and tags** — "pain and illness symptoms" in the Play copy; user-defined, no clinical model |
| Stress / energy | 1–9 scale manual attributes, explicitly suggested |
| Free-text | Yes — the optional daily note attached to the mood rating |
| Tags | Day-level, user-defined, suggested tags supplied; renamable, mergeable, deletable |
| **Medication** | **As a custom tag or manual attribute only.** No medication model, no dose, no schedule, no reminder, no interaction data |
| Dose-triggered paired efficacy | **None.** No dose event exists to trigger anything |
| Subjective outcome as primary | **Mood is a first-class attribute** that can be either side of a correlation; this is the closest analogue to v1's model on the shortlist |

**Interpretation.** Exist's subjective layer is broader than AnxietyWatch v1's (any attribute, any tag,
any numeric shape) and shallower (one mood rating a day, no anxiety instrument, no panic flow, no
medication structure). Critically, because mood is an ordinary attribute, it can be a **dependent
variable** in a correlation — "what makes me happier" is Exist's headline question. That is a real
capability and a direct conceptual overlap with v1.

**Medication, precisely.** The custom-tracking page says users can track "medications" as tags and asks
"how medications affect your productivity". So medication *data* is supported while medication
*semantics* are not: it is a label the user invents, not a modelled entity with a dose and a schedule.
AnxietyWatch v1's dose-triggered follow-up has no equivalent.

## 8. Sleep, CPAP, respiratory, SpO₂, HRV and environmental data

| Capability | Finding | Evidence class |
|---|---|---|
| Sleep duration and stages | Imported: time asleep, time in bed, deep/light/REM, bedtime, wake time, awakenings | A (documented) |
| Sleep analysis | Averages, trends, day-of-week patterns; a documented insight type for sleep, e.g. "Monday is usually your shortest sleep of the week" | A (API + docs) |
| HRV | **Imported as an attribute** ("HRV" in Health and body). Not measured; no analysis beyond correlation and averaging | A |
| Resting / average / max heart rate | Imported attributes | A |
| **CPAP / PAP** | Not listed anywhere. No AHI, leak, pressure or adherence attribute | absence |
| **Respiratory rate** | Not listed | absence |
| **SpO₂** | Not listed | absence |
| Blood pressure | Not listed among importable attributes, despite the Withings integration | absence |
| **Barometric pressure / weather** | **Imported automatically** — air pressure, temperature (max/min), precipitation, cloud cover, humidity, wind speed, day length, weather summary, from Apple Weather | A |
| Indoor environment | Nothing found. No indoor air quality, CO₂ or temperature | absence |
| **Environmental noise** | Nothing found | absence |
| **Aircraft overflight** | Nothing found | absence |

**Interpretation.** Exist's environmental coverage is the **broadest weather set on the shortlist** — nine
distinct weather fields including air pressure, versus Bearable's manual factors and Welltory's
three-field automatic set (temperature, humidity, pressure). But it ends there: no indoor environment,
no noise, no aircraft. And its physiological coverage is strictly *imported summaries* — HRV as a daily
number, not RR intervals; sleep as stages, not raw signals. There is no CPAP domain at all.

**Why the weather breadth matters.** Air pressure is an AnxietyWatch v1 input, and it is the one
environmental variable all three candidates can compare. Exist imports it as an ordinary attribute, so
it can be correlated against mood, sleep and HRV with the same machinery as everything else. That is a
genuine and cheap architectural advantage of being a generic aggregator: a new environmental variable
costs Exist nothing, because everything is already an attribute.

## 9. Weather, aircraft, noise and sleep-context features

- **Weather: comprehensive and automatic.** Nine fields including air pressure; sourced from Apple
  Weather using approximate location; requires user opt-in to share location, and the privacy policy
  documents sharing latitude/longitude with Mapbox (for a suburb-level place name) and Apple Weather.
- **Sleep context:** the strongest of the three candidates in one narrow respect — day-of-week pattern
  insights ("Monday is usually your shortest sleep of the week") are a documented insight type, which is
  a low-cost, high-value framing AnxietyWatch does not currently present.
- **Aircraft overflight:** nothing. Third candidate in a row with no equivalent, which continues to
  suggest the feature is genuinely unclaimed territory.
- **Environmental noise:** nothing.

## 10. AI, statistical analysis, correlations and personalisation

**AI: explicitly refused.** This is the most distinctive finding in the whole candidate file. From the
vendor's values page:

> "**No AI** — We don't chase trends and won't be stuffing large language models into Exist to
> hallucinate what your data means. We stick with old-fashioned statistical methods."

That is a direct, deliberate, publicly stated rejection of the approach Welltory ships (OpenAI GPTs) and
the approach Bearable recommends to free-tier users (export CSV, use ChatGPT). It is also the closest
public statement of AnxietyWatch's own presumption that a health conclusion should be traceable rather
than generated.

**Correlation methodology — inspectable, and rigorous.** The public API documentation exposes the
correlation record itself. A real example from the vendor's own documentation:

```json
{
  "date": "2022-05-16",
  "period": 309,
  "offset": 0,
  "attribute": "sleep",
  "attribute2": "sleep_start",
  "value": -0.5577851552276848,
  "p": 1.1564227190131434e-26,
  "percentage": 55.77851552276848,
  "stars": 5,
  "second_person": "you spend more time asleep when you go to bed earlier.",
  "strength_description": "Quite often go together",
  "stars_description": "Certain to be related"
}
```

| Field | Meaning (as evidenced) |
|---|---|
| `value` | Correlation coefficient (≈ -0.558 here) |
| `p` | **A p-value**, exposed to the client |
| `period` | **Number of paired observations behind the correlation** (309) |
| `offset` | An offset field; `0` for same-day, consistent with the documented next-day lagged checks |
| `stars` (1–5) | Confidence rating shown to the user; the help centre says confidence reflects **how much data** was used, not how strong the relationship is |
| `strength_description` | Plain-language strength, e.g. "Quite often go together" |
| `stars_description` | Plain-language confidence, e.g. "Certain to be related" |
| `second_person` / `second_person_elements` | Auto-generated sentence with its fragments, so the claim is composed of identified parts |

**Strength and confidence are deliberately separate axes**, and the vendor explains why in user-facing
terms: "You might have a very strong relationship between two attributes, for example, with a **very low
confidence if we don't have much data** to use in our calculations." That statement matches the API
schema exactly — `value` is the strength, `period` and `p` are the evidence behind the confidence, and
`stars` is the confidence rendered for the user.

**Filters expose the same rigour to the client:** `strong=1` returns "only correlations above a certain
relationship strength"; `confident=1` returns "only correlations with a five-star confidence".

**Thresholds and windows (documented):**
- At least **three weeks** of data for an attribute before correlations are attempted.
- Days with no data are **excluded** — shown as "N/A", and simply absent from the API value lists.
- Correlations are **recomputed weekly**, every Monday.
- **Lagged correlations are checked**: "If a correlation between two attributes isn't found for the same
  day, Exist will check for correlations between attributes across two consecutive days. For example,
  does tracking more alcoholic drinks (or using the tag 'alcohol') affect your sleep the following
  night?"
- Users are told to log things as they happen, because Exist will look forward one day.

**Explicit causation discipline (documented, article 37):** "Correlations can't determine the cause of
the relationship—this is up to you to have a think about! One attribute changing might cause the other
to change or vice versa, or a third variable might affect both attributes, or the relationship might be
a coincidence." The article also states that a correlation "can be read with either attribute first,
because they don't imply any causation". **Confounding is named by the vendor, in the user-facing help
centre, not buried in a disclaimer.** This is the strongest false-attribution discipline of any candidate
examined.

**Personalisation / baselines.** There is no self-learning model. Personalisation is *configured*, not
learned: which service feeds which attribute, which attributes are active, which tags exist. However the
insight layer is baseline-relative — "Last night's sleep was **surprisingly long**" is a statement about
deviation from the user's own distribution, and averages are computed per user. **[A + D — the phrasing
is documented; the baseline mechanism is inferred]**

**Known limitation: correlations are pair-wise.** Every correlation record has exactly
`attribute` + `attribute2`. There is no multivariate model, no confounder control, and no adjustment for
a third variable — the vendor names confounding as something the *user* should consider, rather than
something the system controls. **[A — structural, from the API schema]**

## 11. Reports, clinician sharing, export and integrations

| Capability | Finding |
|---|---|
| **Export** | From the account page: **PDF report, CSV spreadsheet, or full JSON export of account data** |
| Import | **Not supported.** "No, data cannot be manually imported to Exist." CSV/JSON import is a roadmap suggestion users can vote on |
| API | **Public API v2**, read *and* write, with documented OAuth2 and token auth. Requires an Exist account. Community resources listed on GitHub |
| Clinician portal | None |
| Clinician-oriented output | A PDF report exists, but it is a personal-analytics report, not a clinical one. No medical framing |
| Self-hosting | **No.** Cloud service; hosting deliberately outside the United States |
| Public roadmap | Yes — a public, votable roadmap (Changemap) |
| Transparency | **Published revenue and paid-user counts**, updated weekly from the payment processor |

**The API as an integration path.** Because the API is read/write and attribute-based — with a documented
"Creating attributes" and "Updating values" surface — a third party can push arbitrary values into an
Exist account. That is a real, documented integration route for AnxietyWatch-derived data (for example a
nightly CPAP-derived summary attribute), and it is a materially lower-effort path than reverse-engineering
anything. It is the single most actionable technical finding across the three candidates.

**The PDF export is also notable:** Exist offers PDF, CSV *and* JSON. Bearable offers CSV only and has an
outstanding, unmet community request for PDF; Welltory documents a printable blood-pressure report only.
On export breadth, Exist leads all three.

## 12. Privacy, hosting, account, telemetry and data ownership

| Aspect | Finding |
|---|---|
| Account required | **Yes.** "The API can only be accessed by user accounts"; the Android app requires a paid Exist account |
| **Data sale** | "We don't, and won't ever, sell your data. You pay for Exist. You (and your data) are not the product." |
| **Qualification on the above** | The privacy policy: "We will never sell personal information as part of the everyday process of running Exist, **but it may be transferred as part of the sale, merger, or dissolution of Exist or Hello Code**", with advance notice and an opt-out. The 2018 update entry records this clarification explicitly |
| Hosting | "We have deliberately chosen to host our Site **outside the United States** in order to provide additional safeguards for the security of your data." Specific jurisdiction not stated |
| **Analytics** | **None.** "We finally turned off Google Analytics so we have no third-parties watching how you use the site." The 2024-09-19 policy update records the removal |
| **Third parties** | Help Scout (support email), Stripe (subscriptions), Mapbox (suburb name from coordinates), Apple Weather (coordinates → weather), Campaign Monitor (product emails), Sentry (crash/error reporting incl. username, phone model and OS version, and **possibly attribute-related data in error logs**) |
| Advertising | **None.** No pixels, no audiences, no ad partners named |
| Aggregated disclosure | "We may generate and disclose aggregated and anonymised information statistics... for marketing purposes", provided it does not identify individuals |
| Health Connect | "The use of information received from Health Connect will adhere to the Health Connect Permissions policy, including the Limited Use requirements" |
| **Deletion** | Account deletion removes all personal information from the site, "However, we may keep your information for a commercially reasonable period, for backup and archival purposes." Payment records and support email cannot be deleted (legal/accounting requirement) |
| Disconnecting a service | Does **not** delete history; only stops new collection |
| Local-only mode | **None** |
| "Your data is for you" | "We're not building features to let your boss, colleagues, or coach see your data" |
| Minimal access | "When you connect another service, we'll never ask for access to more of your data than we need" |

**Two documented tensions worth preserving.**

1. **"Permanent deletion" versus backup retention.** The values page promises "Everything about you in
   the Exist database is permanently wiped." The privacy policy says data may be kept "for a commercially
   reasonable period, for backup and archival purposes." Both are the vendor's words and both are
   recorded here; they are reconcilable (backups expire) but not identical.

2. **"Won't ever sell" versus an acquisition carve-out.** The marketing claim is absolute; the legal text
   carves out a sale, merger or dissolution of the company, with notice and an opt-out, and the update
   history shows this was a deliberate 2018 clarification. The qualifier is honest and disclosed — it is
   still a qualifier the marketing page omits. Recorded as an **unresolved reconciliation question (E1)**.

**Android data-safety declaration.** Play reports: **"No data shared with third parties"**; collects
**"Location and App info and performance"**; **"Data is encrypted in transit"**; **"You can request that
data be deleted"**. This is coherent with the privacy policy (Sentry → App info and performance; Mapbox
and Weather → Location). One observation is recorded without accusation: despite the app's entire
function being to ingest health data, **Health and fitness is not among the declared collected types**.
Given the policy's explicit Health Connect Limited Use statement, the most likely explanation is that
Health Connect data is governed by that separate declaration regime — but this was **not verified**, and
it is carried forward as **E4**, not asserted.

**Assessment.** Exist's privacy posture is the strongest on the shortlist by a wide margin, and unusually
it is *architectural and financial* rather than merely rhetorical: a no-AI stance, no analytics, no
advertising, a bootstrapped company with published accounts, deliberate non-US hosting, and a support
model where the founders answer. The two tensions above are minor and disclosed. The substantive gap
versus AnxietyWatch is not privacy policy quality — it is that Exist's data is still *in someone else's
cloud*, with a mandatory account, whereas AnxietyWatch's does not leave the device.

## 13. Pricing, subscriptions, hardware and geographic limits

| Item | Finding |
|---|---|
| Model | **One plan, no tiers.** Free 30-day trial, then paid |
| **Price** | **$6.99/month USD**, or **$62.90/year (25% off)** = $5.24/month effective |
| Trial | **30 days**; "We ask for a credit card up-front, but we give you plenty of warning before your trial ends" |
| Platform cost | The app is free to download but **requires a paid Exist account** |
| Play-description discrepancy | The Play listing says "an account costs **US$6/month**" while the website says $6.99/month |
| Hardware cost | **$0 of its own** — but Exist is nearly useless without at least one other service, so the true cost includes whatever tracker the user already pays for |
| API cost | Free for account holders |
| Refunds | Not documented in the pages examined |

**Cost position.** Exist is the **cheapest paid candidate on the shortlist** — $62.90/year or $6.99/month,
versus Welltory's $99/year or $599 lifetime, and it has no mandatory hardware. It is also the only
candidate offering both PDF and JSON export on a single plan, and the only one offering a documented
write API at no extra cost.

## 14. Clinical and regulatory claims

- **No medical-device claim anywhere.** Exist does not present itself as a health product, does not use
  clinical language, and does not claim to diagnose, monitor or treat anything. There is no disclaimer to
  find because there is no claim to disclaim.
- **No clinical instruments.** No anxiety, depression, burnout or sleep-apnea screening.
- **No clinical validation claims.** No studies, no citations, no "clinically validated" language.
- **No emergency or monitoring capability claimed.**

**Assessment.** Exist's regulatory position is the cleanest of the three candidates precisely because it
makes no health claims at all. But this cuts both ways for the comparison: Exist is honest about being a
*personal analytics* tool, and it offers no clinical pathway of any kind. AnxietyWatch's clinical
reporting, CPAP domain and (specified) alarm are outside Exist's scope entirely.

## 15. Evidence table

| # | Claim or fact | Source | Publisher | Date | Access | Confidence |
|---|---|---|---|---|---|---|
| E-1 | Purpose, positioning, "correlations are the most powerful part" | https://exist.io/ | Hello Code | live | 2026-10-06 | High (vendor claim) |
| E-2 | **Pricing: $6.99/mo or $62.90/yr (25% off), one plan, 30-day trial** | https://exist.io/ and /about/values/ | Hello Code | live | 2026-10-06 | High |
| E-3 | Platforms iOS/Android/web | https://exist.io/ | Hello Code | live | 2026-10-06 | High |
| E-4 | Values: no data sale, export any time, minimal scopes, no boss/coach access, **no AI**, no investors, founders only, transparent earnings, public roadmap, **no mobile or web analytics**, permanent deletion | https://exist.io/about/values/ | Hello Code | live | 2026-10-06 | High (vendor claim) |
| E-5 | **Published revenue: Exist 940 paid users, $6,420.85 in the last 4 weeks**; total $6,648.45 | http://hellocode.co/stats/ | Hello Code | live at access | 2026-10-06 | High (vendor-published, from payment processor) |
| E-6 | Mood tracking: 1–9 daily, optional note, notification or email reply, anniversary view | https://kb.exist.io/article/36-what-is-mood-tracking + /about/mood/ | Hello Code | 2020-12-07 / live | 2026-10-06 | High (documented) |
| E-7 | Manual tracking value types: quantity, decimal, duration, time of day, scale 1–9, percentage | https://kb.exist.io/article/59-what-is-manual-tracking | Hello Code | 2023-02-26 | 2026-10-06 | High (documented) |
| E-8 | **1–9 scale rationale**: 5 is an exact midpoint; used for symptom severity, energy, mood, stress | https://kb.exist.io/article/62-why-are-scale-values-1-9 | Hello Code | 2024-01-24 | 2026-10-06 | High (documented) |
| E-9 | **Correlation record: coefficient, p-value, period (n=309), offset, stars, plain-language strength and confidence** | https://developer.exist.io/reference/correlations/ | Hello Code | live | 2026-10-06 | **High — primary API documentation** |
| E-10 | Correlation filters `strong` and `confident` (five-star) | same as E-9 | Hello Code | live | 2026-10-06 | High |
| E-11 | **≥3 weeks of data required; days with no data excluded and shown as "N/A"; weekly refresh each Monday** | https://kb.exist.io/article/18-how-long-does-it-take-to-find-correlations | Hello Code | 2023-09-20 | 2026-10-06 | High (documented) |
| E-12 | **Lagged/cross-day correlations checked (two consecutive days); log as things happen** | https://kb.exist.io/article/55-will-exist-find-correlations-across-multiple-days | Hello Code | 2022-02-09 | 2026-10-06 | High (documented) |
| E-13 | **Causation disclaimer and explicit mention of confounding (a third variable)** | https://kb.exist.io/article/37-what-are-correlations | Hello Code | updated | 2026-10-06 | High (documented) |
| E-14 | "Exist can even uncover what happens when you don't track a tag" | https://exist.io/about/custom-tags/ | Hello Code | live | 2026-10-06 | Medium (marketing; mechanism not documented) |
| E-15 | **Export: PDF report, CSV spreadsheet, full JSON** | https://kb.exist.io/article/14-how-do-i-export-my-data | Hello Code | 2026-04-23 | 2026-10-06 | High (documented) |
| E-16 | **No manual file import; CSV/JSON import only a roadmap suggestion** | https://kb.exist.io/article/118-can-i-import-data-from-a-file | Hello Code | 2024-01-24 | 2026-10-06 | High (documented) |
| E-17 | **Manual value for a date cannot be removed, only replaced**; a different correction path applies to service-synced data | https://kb.exist.io/article/116-can-i-remove-a-value-for-a-manual-attribute | Hello Code | 2026-08-26 | 2026-10-06 | High (documented) |
| E-18 | Synced/template attributes cannot be deleted, only deactivated; self-defined manual attributes and tags can be permanently deleted with their data | https://kb.exist.io/article/99-how-can-i-delete-or-deactivate-an-attribute | Hello Code | 2024-03-13 | 2026-10-06 | High (documented) |
| E-19 | Disconnecting a service keeps history; attributes fall back to another service or are disabled | https://kb.exist.io/article/21-what-happens-when-i-disconnect-a-service | Hello Code | 2020-10-23 | 2026-10-06 | High (documented) |
| E-20 | Update cadence: today's data periodically; most services hourly/2-hourly; last week refreshed less often | https://kb.exist.io/article/17-how-often-is-my-data-updated | Hello Code | 2024-04-24 | 2026-10-06 | High (documented) |
| E-21 | Backfill on connect: typically 30 days, floor of 14 days; done within 1–2 h, up to 24 h | https://kb.exist.io/article/63-how-much-data-will-sync-when-i-connect-a-service | Hello Code | 2022-09-29 | 2026-10-06 | High (documented) |
| E-22 | Importable attributes by group, incl. HRV, resting HR, sleep stages, **air pressure**, 9 weather fields | https://exist.io/apps-data-syncing/ | Hello Code | live | 2026-10-06 | High (documented) |
| E-23 | Connected-service list (~20 services) | same as E-22 | Hello Code | live | 2026-10-06 | High |
| E-24 | **No CPAP, SpO₂, respiratory rate or blood-pressure attribute listed** | same as E-22 | Hello Code | live | 2026-10-06 | Medium (absence of evidence) |
| E-25 | **No indoor environment, no noise, no aircraft context** | all vendor surfaces examined | Hello Code | — | 2026-10-06 | Medium (absence of evidence) |
| E-26 | **API v2 is read/write**, requires an account, supports creating attributes and updating values | https://developer.exist.io/ | Hello Code | live | 2026-10-06 | High (primary docs) |
| E-27 | **Insight records** carry type, period, priority (1–4), attribute, value_type and rendered text | https://developer.exist.io/reference/insights/ | Hello Code | live | 2026-10-06 | High (primary docs) |
| E-28 | **Averages** per attribute per week with day-of-week breakdown | https://developer.exist.io/reference/averages/ | Hello Code | live | 2026-10-06 | High (primary docs) |
| E-29 | 81 attribute templates; value types include Integer (0) and Period min (3); attribute objects expose `available_services` | https://developer.exist.io/reference/attributes/ | Hello Code | live | 2026-10-06 | High (primary docs) |
| E-30 | Privacy policy: third parties (Help Scout, Stripe, Mapbox, Apple Weather, Campaign Monitor, Sentry), non-US hosting, acquisition carve-out, backup retention, aggregated-statistics disclosure, Health Connect Limited Use | https://exist.io/privacy/ | Hello Code | 2023-10-06 (updates to 2024-09-19) | 2026-10-06 | High (primary legal document) |
| E-31 | Google Analytics removal recorded in the policy update history (2024-09-19) | same as E-30 | Hello Code | 2024-09-19 | 2026-10-06 | High |
| E-32 | Android: 10K+ downloads, 4.5★, ~187 ratings, updated 2026-09-23 | https://play.google.com/store/apps/details?id=co.hellocode.exist | Google Play | 2026-09-23 | 2026-10-06 | High (platform-reported) |
| E-33 | Android data safety: **no data shared**; collects Location + App info and performance; encrypted in transit; deletion requestable | same as E-32 | Google Play | 2026-09-23 | 2026-10-06 | High (platform-declared) |
| E-34 | iOS: `co.hellocode.Exist`, 4.02★, 47 ratings, v4.3, 14.7 MB, iOS 18.0+, released 2026-09-25 | iTunes Lookup API | Apple | 2026-09-25 | 2026-10-06 | High (platform-reported) |
| E-35 | Play description states **US$6/month** (conflicts with E-2) | Play listing | Google Play | 2026-09-23 | 2026-10-06 | High (documented discrepancy) |
| E-36 | No medical, clinical or diagnostic claims; no clinical instruments; no validation claims | all vendor surfaces examined | Hello Code | — | 2026-10-06 | High (absence + consistent framing) |
| E-37 | Founders Belle B. Cooper and Josh Sharp handle support; two-person company | https://exist.io/about/values/ , http://hellocode.co/stats/ | Hello Code | live | 2026-10-06 | High |

**Retrieval failures recorded:** `exist.io/pricing/`, `/features/`, `/sources/` and `/faq/` each returned
HTTP 404 (`/pricing/` is not a page — pricing is on the homepage and the values page). Several
`developer.exist.io` sub-paths (`guide/best-practices/`, `reference/important-values/`,
`reference/object-types-and-terminology/`) returned 404 and were not retrieved; the value-type
enumeration is therefore **partial** — two types (Integer, Period) are observed in examples and the full
list was not obtained. No App Store listing page was retrieved (the iTunes Lookup API was used instead).

## 16. Key similarities and differences versus AnxietyWatch

### Versus AnxietyWatch v1 (documented/implemented)

**Similar in intent**
- **Subjective ratings are first-class data.** Mood 1–9 alongside objective attributes; v1 rates anxiety
  alongside physiology.
- **Personal baselines rather than population norms.** Exist computes per-user averages and its insight
  phrasing is baseline-relative ("surprisingly long"); v1 computes a rolling personal baseline.
- **Correlation as the core analytical claim.** "Correlations are the most powerful part of Exist";
  v1's correlation engine is the same idea.
- **Explicit refusal to imply causation.** Exist names confounding in its help centre; v1's design notes
  show the same caution. Both refuse to treat a correlation as an answer.
- **Weather and air pressure as context.** Both import atmospheric pressure automatically.
- **Medication and symptoms are tracked** — as free-form data in Exist, as structured dose events in v1.

**Different in substance**

| Dimension | AnxietyWatch v1 | Exist |
|---|---|---|
| Measures anything itself | Yes — Polar H10 RR, CPAP SD, EMAY oximeter, barometer | **No.** Pure aggregator |
| Raw vs summary physiology | Raw RR at ~1 Hz | Imported daily HRV/HR numbers only |
| CPAP / AHI / leak / adherence | Yes, SD import | **Nothing** |
| Oximetry / SpO₂ | Yes, EMAY import + BLE stream | **Nothing listed** |
| Respiratory rate | Via CPAP/oximeter | **Nothing listed** |
| Medication semantics | Dose events with paired follow-up | A user-invented tag or number |
| Panic-time capture | watchOS quick log | **None** — one mood rating per day |
| Analysis latency | Immediate, local | **Correlations weekly; data hourly-to-daily** |
| Missing-data handling | Three-state: measured-good / measured-bad / **not measured** | **Days with no data excluded and shown "N/A"**; no third state named |
| Statistical disclosure | — | **Coefficient, p-value, sample size and confidence exposed via API** |
| Multivariate / confounder control | Not claimed | **None** — strictly pair-wise; user is told to consider confounders |
| Immutability | Append-only; removal is a tombstone | Manual values **cannot be removed**, only replaced ("you can track 0 to change the value"); **template and service-synced attributes cannot be deleted at all**, only deactivated |
| Storage | Local-first, self-hosted | Vendor cloud, mandatory account, non-US hosting |
| Telemetry | None claimed | **None** — no analytics, no advertising, by explicit policy |
| AI | Anthropic-based analysis (v1) | **Explicitly refused** |
| Export | PDF report | **PDF + CSV + JSON**, plus a read/write API |
| Alarm | Local alerts with dedup | **None** |

**Where Exist is genuinely stronger**
- **Statistical transparency.** p-values and sample sizes exposed to clients is more than any other
  candidate does, and more than most users would ask for.
- **Correlation hygiene by construction.** Strength *and* confidence are separate axes, a correlation is
  suppressed until there are three weeks of data, missing days are excluded rather than zero-filled, and
  the causation caveat is user-facing.
- **Export breadth.** PDF, CSV and JSON on the base plan, plus a documented write API.
- **Breadth of context** including nine weather fields, calendar, productivity, media, finance and
  location — nothing in v1 approaches this.
- **Privacy posture and business transparency.** No analytics, no advertising, deliberate non-US hosting,
  no AI, published revenue.
- **Zero hardware requirement** and the lowest price on the shortlist.

### Versus AnxietyWatch2 (current + specified target)

- **V2 current overlap: minimal.** Exist has no continuous collection, no provenance model, no local
  console, no alarm and no sensor. It shares only the general idea of correlating environmental and
  behavioural context against a personal baseline.
- **V2 specified target overlap: essentially none.** The supervised alarm, dead-man switch,
  provenance-preserving append-only store and local-first operation have no Exist counterpart.
- **One genuine architectural affinity.** Exist independently arrived at *excluding* missing days rather
  than coercing them, and at separating strength from confidence. Both are design instincts AnxietyWatch2
  formalises further — Exist has two states where v2 specifies three, but the direction is the same, and
  Exist's shipped experience with it is evidence that the approach is practical at consumer scale.
- **The complementarity case is the strongest of any candidate so far.** Exist publishes a read/write,
  attribute-based API with a documented "creating attributes / updating values" surface, a JSON export,
  and PDF/CSV export. That is a real, documented, low-effort path by which AnxietyWatch-derived objective
  data could reach a user's existing Exist account — for example a nightly CPAP-derived summary attribute
  correlated against an existing mood series. Nothing needs to be reverse-engineered.

## 17. Whole product or subsystem?

**Whole product for "aggregate everything I already track and find relationships in it" — and a partial
competitor to v1's analysis layer only.** Exist competes with v1's correlation-and-context function and
with nothing else: it does not measure, does not record overnight, has no CPAP or oximetry domain, no
medication semantics, no alarm, no local store, and no clinical output beyond a generic report. Against
AnxietyWatch2 it is not a competitor at all; it is the most plausible **integration partner** on the
shortlist, and the most useful **methodological precedent** among the commercial candidates.

## 18. Weaknesses, omissions and unresolved questions

**Weaknesses and omissions (evidenced)**
1. **Measures nothing.** Every physiological number is somebody else's summary. There is no raw signal,
   no provenance, and no path to a live measurement.
2. **Correlations are strictly pair-wise.** The API schema has exactly two attributes per correlation.
   No confounder control, no multivariate model — the vendor explicitly leaves confounding to the user.
3. **Weekly analysis latency.** Correlations refresh on Mondays; today's data may be one to two hours old
   at best. Nothing is real-time.
4. **No CPAP, SpO₂, respiratory rate, or blood-pressure attribute listed** — the entire respiratory and
   therapy domain is absent, and so is the oximetry layer v1 relies on.
5. **No panic-time interaction.** Mood is one retrospective rating per day.
6. **No medication semantics** — a self-invented tag, not a dose event.
7. **No manual import.** CPAP SD-card data cannot be loaded without writing to the API.
8. **Removing a manual value for a date is impossible** — only replacement. Convenient for data
   integrity, awkward when a unit or value type was wrong.
9. **Very small user base** (~940 paying users), so community knowledge, third-party integrations and
   edge-case hardening are all correspondingly thin.
10. **Pricing inconsistency** between the Play listing ($6/month) and the website ($6.99/month).
11. No clinical instruments, no clinician report, no alarm — by design, but they are capabilities
    AnxietyWatch has or specifies.

**Unresolved questions for Prompt 2**
- **E1** How are the marketing claim "we don't, and won't ever, sell your data" and the privacy policy's
  acquisition carve-out reconciled in practice? What notice and opt-out would a user actually receive?
- **E2** How are the "N/A" days handled *inside* the correlation computation — dropped pairwise, or does
  an attribute with intermittent coverage skew the effective sample (`period`)? The exclusion rule is
  documented; its statistical consequences are not.
- **E3** What exactly does "Exist can even uncover what happens when you don't track a tag" mean
  mechanically — is tag absence treated as a categorical level, and is that reported with the same
  strength/confidence treatment?
- **E4** Why does the Play data-safety declaration not list Health and fitness among collected types,
  given the app's function? Is Health Connect data governed by a separate declaration regime?
- **E5** What is the full `value_type` enumeration, and does Exist enforce one canonical unit per
  attribute — or can the same attribute carry mixed units from different services?
- **E6** What are the exact thresholds behind `strong=1` and `confident=1`, and are they user-visible?
  The star meanings are described in words; the numeric cut-offs are not published.
- **E7** Does the PDF export include correlations with their strength/confidence, or only charts and
  averages?
- **E8** Are insight texts ("surprisingly long") generated from a documented personal-baseline rule, and
  is the rule disclosed? The baseline mechanism is inferred, not documented.
- **E9** What is the actual hosting jurisdiction, given only "outside the United States" is stated?
- **E10** Does the `offset` field represent a day lag, and does Exist check lags beyond one day? Marketing
  says "two consecutive days"; the API exposes an offset.
- **E11** Is there any refund or cancellation policy, and how does the up-front credit card requirement
  interact with the 30-day trial in jurisdictions with cooling-off rights?
- **E12** How does Exist behave for a user whose connected service stops supplying data — does the
  attribute silently freeze at its last value, or go to "N/A"?

## 19. Related files

- Implementation inspection: `exist_IMPLEMENTATION_INSPECTION.md`
- Candidate-specific comparison prompt: `PROMPT_2.md`
- Comparison report: `exist_COMPARISON.md`
- Shortlist: `../CANDIDATE_SHORTLIST.md`
- Method and limits: `../RESEARCH_METHOD.md`
