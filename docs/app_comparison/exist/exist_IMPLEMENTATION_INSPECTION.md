# Exist — implementation inspection

**Slug:** `exist`
**Date:** 2026-10-06
**Outcome:** **PARTIALLY PERFORMED** — third inspection mode
**Binary inspected:** No
**Authorized public artifacts inspected:** Yes — public API documentation (primary), public data-syncing
attribute catalogue, public roadmap, published business statistics
**Runtime observation:** None
**App source code:** None published

---

## 1. Summary of outcome

Exist's app binary was **not** inspected, for the same reason as Bearable and Welltory: it is a
closed-source commercial app distributed through the App Store and Google Play, and obtaining the APK
would require circumventing a distribution control this project does not have authorization to bypass.
That path is closed and is not attempted (see §8).

Exist nevertheless yields the **strongest static evidence of any candidate examined so far**, and not
because of anything found in a binary. Hello Code publishes, deliberately and publicly:

1. **A complete API v2 reference**, including the *actual JSON record* for a correlation and an insight.
   This exposes the statistical model's output contract: a correlation coefficient, a **p-value**, a
   **sample size** (`period`), a 1–5 confidence rating, plain-language renderings with the sentence
   fragments that compose them, and a filter set including `strong` and `confident`.

2. **A public attribute catalogue** listing 81 attribute templates, their groups and their value types —
   including every field Exist can import, and by omission the fields it cannot.

3. **A public roadmap** and **published revenue figures**, including the paid-user count.

This is the **third inspection mode** introduced for Welltory and here confirmed as the norm for
privacy-conscious small vendors: *no binary, but authorized vendor-published artifacts that are
themselves implementation*. The distinction that made Welltory's inspection only partial — that a
dataset is not the product — does **not** apply here. The API reference *is* the analysis layer's
interface, and the correlation record *is* the analysis layer's output. Where Welltory's public
artifacts corroborated its marketing, Exist's public artifacts let us check its statistics directly.

**Assessment: the analysed surface is more rigorous than the marketing requires, and more rigorous than
any other candidate on the shortlist.**

## 2. Authorization basis

| Basis | Detail |
|---|---|
| Public API documentation | `https://developer.exist.io/` — publicly reachable, no authentication required to read the reference. Documented for third-party developers and explicitly inviting integrations |
| Public data-syncing page | `https://exist.io/apps-data-syncing/` — a marketing page that doubles as the attribute catalogue |
| Public roadmap | Changemap board, linked from the vendor's values page and referred to in help articles as a place to vote on suggestions |
| Published business statistics | `http://hellocode.co/stats/` — published by the vendor, linked from its own values page |
| Help centre | `https://kb.exist.io/` — public Knowledge Base, Help Scout |
| **Not performed** | App Store / Play binary acquisition; emulator install; traffic interception; any authenticated API call |

**No authentication was used, no credentials were created, and no user data was accessed.** Every
artifact inspected is a public web page or a public API reference page. The API reference documents
endpoints that require a token; **not one of those endpoints was called.**

## 3. Artifact record

| Artifact | URL | Type | Retrieved |
|---|---|---|---|
| Correlation reference | `https://developer.exist.io/reference/correlations/` | API reference (primary) | 2026-10-06, HTTP 200, 43,606 B |
| Attributes reference | `https://developer.exist.io/reference/attributes/` | API reference (primary) | 2026-10-06, HTTP 200, 69,161 B |
| Insights reference | `https://developer.exist.io/reference/insights/` | API reference (primary) | 2026-10-06, HTTP 200, 32,388 B |
| Averages reference | `https://developer.exist.io/reference/averages/` | API reference (primary) | 2026-10-06, HTTP 200, 32,736 B |
| Data-syncing / attribute catalogue | `https://exist.io/apps-data-syncing/` | Vendor web page | 2026-10-06 |
| Public statistics | `http://hellocode.co/stats/` | Vendor web page | 2026-10-06 |
| Developer landing | `https://developer.exist.io/` | Vendor web page | 2026-10-06 |
| Values / positioning | `https://exist.io/about/values/` | Vendor web page | 2026-10-06 |
| Privacy policy | `https://exist.io/privacy/` | Legal document | 2026-10-06 |
| Play listing | `.../details?id=co.hellocode.exist` | Platform page | 2026-10-06 |
| iOS metadata | iTunes Lookup API, `co.hellocode.Exist` | Platform API | 2026-10-06 |
| Help articles | `kb.exist.io` — articles 10, 12, 14, 17, 18, 21, 36, 37, 55, 59, 62, 63, 99, 116, 118 | Documentation | 2026-10-06 |

**Failures recorded:** `exist.io/pricing/`, `/features/`, `/sources/`, `/faq/` → HTTP 404 (small bodies,
consistent with a catch-all). `developer.exist.io/guide/best-practices/`,
`/reference/important-values/`, `/reference/object-types-and-terminology/` → HTTP 404. The latter matters:
the **full `value_type` enumeration was not obtained**, so §4.2's value-type list is partial.

## 4. Static findings from authorized published artifacts

Every finding in this section is a **static finding from a vendor-published artifact**, not a runtime
observation and not a marketing claim. Findings are labelled `[STATIC]`. Where an artifact is quoted, the
wording is the vendor's.

### 4.1 The correlation record — the analysis layer's output contract `[STATIC]`

The single most informative artifact. The vendor's own documentation ships a real correlation object as
its response example:

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
  "second_person_elements": ["you spend more time asleep", "when", "you go to bed earlier"],
  "attribute_category": null,
  "strength_description": "Quite often go together",
  "stars_description": "Certain to be related",
  "description": null,
  "occurrence": null,
  "rating": null
}
```

Documented meanings and the structural conclusions that follow:

| Field | Vendor's definition | Implementation conclusion |
|---|---|---|
| `value` | Not separately described in the reference | A signed correlation coefficient (≈ −0.558). Sign carries direction `[STATIC]` |
| `p` | Not separately described | **A p-value is computed and stored per correlation.** The vendor chooses to expose it to clients `[STATIC]` |
| `percentage` | Not separately described | The coefficient rendered as a percentage (55.78 from 0.5578) — a presentation transform of `value` `[STATIC]` |
| `period` | Not separately described; example value 309 | **A paired-observation count.** This is the strongest reading: the help centre states that confidence depends on how much data was used, and 309 is implausible as anything but a sample size `[STATIC + INFERENCE]` |
| `offset` | Not separately described; example value 0 | Consistent with the documented next-day lag check. **Whether `offset` is a day count, and how many lags are tested, is not documented** → question **E10** `[INFERENCE]` |
| `stars` (1–5) | "shows a number of stars out of 5 to describe how sure we are" (help article 37) | A user-facing confidence derived from the statistics, distinct from strength `[STATIC]` |
| `second_person` | — | The correlation is rendered as a sentence, **with its constituent fragments stored alongside it** (`second_person_elements`). The natural-language claim is therefore built from identified parts, not generated free-form `[STATIC]` |
| `strength_description` | "Quite often go together" | Strength is bucketed into plain-language bands `[STATIC]` |
| `stars_description` | "Certain to be related" | Confidence is bucketed into plain-language bands `[STATIC]` |
| `attribute` / `attribute2` | — | **The schema is strictly pairwise.** There are exactly two attribute slots. There is no confounder slot, no covariate list and no model identifier. **This is structural evidence, stronger than any marketing statement, that Exist does not do multivariate analysis or confounder control** `[STATIC]` |
| `attribute_category` | null in the example | An optional grouping field; unused here `[STATIC]` |
| `description`, `occurrence`, `rating` | null in the example | Reserved/optional fields for a correlation's narrative form; not exercised `[STATIC]` |

**Why this matters more than a feature list.** A vendor can claim "correlations are the most powerful
part of Exist" in marketing. The API reference shows what a correlation *is*: coefficient, p-value,
sample size, lag offset, two confidence vocabularies, and a compositional sentence. That is checkable,
and it is the only candidate on this shortlist where the analytical claim can be assessed at the level of
its data contract.

### 4.2 The attribute catalogue and its value types `[STATIC]`

- **81 attribute templates** are published by `GET /api/2/attributes/templates/` (`count: 81` in the
  vendor's example).
- A user's own attribute list returned 134 items in the vendor's example, each carrying `template`,
  `name`, `label`, `group`, **`service`** (which service supplies it), `active`, `priority`, `manual`,
  `value_type`, `value_type_description`, and **`available_services`** — the list of connected services
  that claim they can provide this attribute.
- **Value types observed:** `0` = `"Integer"`, `3` = `"Period (min)"`. The full enumeration was not
  retrieved (404 on the terminology page), so **this list is partial** → question **E5**.
- Attribute groups observed: `activity` (priority 1), `sleep`, and others referenced by name in filters
  (`activity,workouts`). Group objects carry their own `priority` and `label`.

**Structural conclusions.** Exist has an explicit **unit type per attribute** (`Period (min)`,
`Integer`), and an explicit **ownership model** — one service is the designated supplier of an attribute,
and a documented fallback list exists. The "attribute ownership" reference section exists as its own
heading, and help article 21 documents takeover behaviour ("Once you disconnect Fitbit, steps will now
automatically come from Garmin"). That is a real, published data-provenance rule.

**But note the scope limit:** `value_type` describes a *shape* (integer, period, scale), not a canonical
unit with a validated range. There is no published evidence of a per-attribute valid range, no
out-of-range rejection rule, and no documented unit conversion — `Period (min)` is a type name, and
nothing suggests Exist prevents one service supplying minutes and another supplying seconds to the same
attribute. That is a **material difference from AnxietyWatch2's one-canonical-unit-per-type rule**, and
it is exactly what question **E5** targets.

### 4.3 Missing-data representation in the data model `[STATIC]`

This is the finding most directly comparable to AnxietyWatch's three-state rule.

- Help article 18: correlations require "at least three weeks' worth of data for an attribute,
  **excluding days with no data (shown as "N/A")**".
- The API's value lists (`GET /api/2/attributes/values/`, and `with-values/`) return **only date/value
  pairs for days that have a value**. In the vendor's own example, steps values run 2022-05-13, -12,
  -11, -10, -09 — consecutive here, but the record model is a sparse list keyed by date. **There is no
  zero-fill, no sentinel value and no null-value row for an absent day.** `[STATIC]`

**Conclusion.** Exist's data model **omits** absent observations rather than representing them as zero.
That is the correct instinct and it aligns with AnxietyWatch2's refusal to coerce a missing reading into a
number. The difference is granularity of intent:

| | AnxietyWatch2 (specified) | Exist |
|---|---|---|
| Absent observation | Third state, explicitly modelled and named ("not measured") | **Sparse: no row exists** |
| User-facing label | Never rendered as good or bad | Shown as "N/A" in the analysis view |
| Correlation treatment | Must contribute nothing | **Excluded pairwise** (documented) |
| Sentinel coercion | Forbidden (SFLOAT 0x07FF/0x0800 rejected) | **Not applicable** — no sensor, no sentinel to coerce |

Exist has the right *behaviour* (exclusion) without an explicit *third state* (a distinguishable,
queryable "not measured" marker). It has not needed one, because it never ingests a sentinel-bearing
sensor protocol. Whether it would survive one is **unknown** and is question **E2**.

### 4.4 The insight record — baseline-relative text, compositionally built `[STATIC]`

`GET /api/2/insights/` returns records with `created`, `target_date`, a `type` object
(`name`, `period`, `priority`, `attribute`), and both `html` and `text` renderings. Vendor example:

```json
{
  "created": "2022-05-16T13:17:03+08:00",
  "target_date": "2022-05-16",
  "type": { "name": "dow_sleep", "period": 1, "priority": 1,
            "attribute": { "name": "sleep", "label": "Time asleep",
                           "group": { "name": "sleep", "label": "Sleep", "priority": 3 },
                           "priority": 1, "value_type": 3,
                           "value_type_description": "Period (min)" } },
  "text": "Last night's sleep was surprisingly long.\r\nMonday is usually your shortest sleep of the week."
}
```

Findings:

- **Insight types are named and enumerated** (`dow_sleep` — day-of-week, sleep). Each type has a
  `period` and a `priority`. An insight is therefore a *typed* generated statement, not free text.
- **Priority is a documented 1–4 scale** where 1 = today and 4 = last month, and is a queryable filter
  (`GET /api/2/insights/?priority=1`).
- **The text is baseline-relative**: "**surprisingly long**" is a statement about deviation from the
  user's own distribution, and it is paired with a day-of-week baseline ("Monday is usually your shortest
  sleep"). This is the clearest evidence that Exist personalises against per-user baselines rather than
  population norms. `[STATIC]`
- **Insights are composed of two independent clauses** joined by a newline — one deviation observation
  and one pattern observation — which suggests a template/composition system rather than a language
  model. This is consistent with the vendor's explicit "no AI" position, though **the generator's
  internals are not published** → question **E8**. `[STATIC + INFERENCE]`
- Insights carry `created` timestamps **with a +08:00 offset in the vendor's example**, i.e. timestamps
  are timezone-aware. Whether the boundary logic is per-user-local or UTC is not documented.

### 4.5 Averages — the aggregate surface `[STATIC]`

`GET /api/2/averages/` returns, per attribute per week: `overall` plus **`monday` … `sunday`**. The
vendor's example: `{"attribute": "steps", "date": "2020-04-29", "overall": 4174.0, "monday": 4057.0,
..., "sunday": 3904.0}`.

- Day-of-week breakdown is a **first-class stored aggregate**, not a client-side computation. This is
  structural support for the `dow_*` insight family.
- `include_historical=1` retrieves the full weekly history for an attribute, so **the weekly average
  series is retained and queryable over time** — a genuine longitudinal record, unlike a dashboard that
  recomputes on demand.
- The averages record is keyed by attribute and date, with no per-value provenance field. Averaging
  across services is therefore possible in principle when attribute ownership changes — which is
  consistent with, though not proof of, the unit-mixing concern raised in §4.2.

### 4.6 Data ownership, correction and deletion rules `[STATIC]`

Published rules constitute an implementation of the data-integrity policy:

| Rule | Source | Implementation reading |
|---|---|---|
| Synced and template-based attributes **cannot be deleted**, only deactivated (data retained, hidden app-wide) | article 99 | Irreversible ingest; the vendor prefers retention over deletion, presumably for correlation continuity |
| Self-defined manual attributes and custom tags **can be permanently deleted**, including all related data | article 99 | Distinguishes vendor-defined schema from user-defined schema and applies different retention to each |
| A manual value for a date **cannot be removed**, only replaced | article 116 | Closest analogue to append-only: the date slot, once filled, is permanently occupied. The vendor's own example is to "track 0 to change the value" — i.e. substitute a real value, not a tombstone |
| Service-synced data follows a **different** correction path | article 116 (cross-reference) | Correction is delegated to the source service — the aggregator does not own the value |
| Disconnecting a service **does not delete history**; attributes reassign to another service or are disabled | article 21 | Explicit provenance/fallback model |
| Deletion removes personal information but **backups are retained for a commercially reasonable period**; payment records and support emails are kept | privacy policy | Retention carve-outs are legal/financial |

**Reading.** Exist's data-integrity model is best described as **fill-once-per-date, never-delete-vendor-
schema, delegate-correction-to-source**. It is neither append-only nor freely mutable. The rule that a
manual value cannot be removed is *closer* to AnxietyWatch's tombstone discipline than Bearable's editable
factors or Welltory's ordinary CRUD — but the mechanism is substitution, not an explicit tombstone, and
there is no audit trail of what the value was before.

### 4.7 API write surface — the integration finding `[STATIC]`

The reference contains dedicated sections **"Attribute ownership"**, **"Creating attributes"** and
**"Updating values"**, under a "Writing data" heading, alongside a "Sharing your integration" guide and a
"Migrating from API 1" note.

**This is the most actionable technical finding across the three candidates.** Exist publishes a
supported, documented, authenticated **write** path for arbitrary user-defined attributes and their
values. A third-party tool — including one built on AnxietyWatch-derived data — can push a daily value
into a user's Exist account without any reverse engineering, without touching Exist's app binary, and
within the vendor's explicit intent. Help article 118 states this outright as the supported alternative to
file import: "If you're a developer, you can also use our API to add data to your Exist account."
`[STATIC]`

### 4.8 Business and scale findings `[STATIC]`

Published by the vendor at `hellocode.co/stats/`, described in help/tone as generated from their payment
processor:

- **Exist: 940 paid users**, `$6,420.85` in the last 4 weeks.
- Across all three Hello Code products: `$6,648.45` in the last 4 weeks.
- A milestone tracker is shown (e.g. amounts at which a founder goes full-time, at which a junior
  developer could be hired).

**Reading.** Exist's paying population is roughly **940 users**. This is a vendor-published, machine-
derived figure — the strongest available evidence for any candidate's real scale, and the only one that is
not a platform download count or a marketing claim. It also implies revenue order-of-magnitude ~$80k/yr
for Exist, consistent with a two-person bootstrapped company and with the values page's claim that Exist
is not venture-funded.

**This is also the closest business-scale analogue to AnxietyWatch's own situation on the shortlist.**
The comparison value is not "Exist is small"; it is that Exist is a *privacy-first, no-AI, no-analytics,
solo-team, paid, non-VC* product that is nonetheless still running after twelve years at ~940 paying
users. Its architecture and its positions are therefore evidence about what that business shape actually
sustains.

## 5. Runtime observation

**None.** No app was installed, no emulator was used for Exist, no traffic was intercepted, no
authenticated API call was made. The validated mitmproxy toolchain from the Bearable pilot
(`FREE-ANDROID-TRAFFIC-INSPECTION.md`) was available and deliberately not applied, because the
authorization blocker from Bearable applies identically: the binary cannot be lawfully obtained. The
difference here is only that Exist's published artifacts are rich enough to substitute for it in the
analysis layer.

## 6. Claim reconciliation

Claims are the vendor's public marketing/help statements; findings are from authorized published
artifacts and platform declarations.

| # | Public claim | Finding | Verdict |
|---|---|---|---|
| X1 | "Correlations are the most powerful part of Exist" | Correlation records carry coefficient, p-value, sample size, offset, a 1–5 confidence rating, a strength band, and a compositional sentence (X-9/X-10) | **Corroborated and exceeded** — the implementation is more rigorous than the claim needs |
| X2 | "Each correlation includes indications of strength and confidence" | Strength (`value`, `strength_description`) and confidence (`stars`, `p`, `period`) are separate stored fields; filters `strong` and `confident` exist (X-9/X-10) | **Corroborated precisely** |
| X3 | "We need at least three weeks' worth of data… excluding days with no data (shown as 'N/A')" | Consistent with the sparse value model in the API: only date/value pairs for days with values, no zero-fill (X-11, X-29) | **Corroborated by schema** |
| X4 | "Exist will check for correlations… across two consecutive days" | `offset` field present in the correlation record, `0` in the example; the number of lags tested is **not** documented | **Partially corroborated** — the field exists; the range tested is unknown (E10) |
| X5 | "Correlations can't determine the cause… a third variable might affect both attributes" | Consistent with a **strictly pairwise** schema: exactly `attribute` and `attribute2`, no covariate slot, no model field | **Corroborated, and the limitation is structural** — the vendor's caveat is an honest statement of what the model cannot do |
| X6 | "No AI" — "won't be stuffing large language models into Exist to hallucinate what your data means. We stick with old-fashioned statistical methods." | Insights are **typed** (`dow_sleep`), carry a period and priority, and are composed of two clause fragments rather than free prose. No generative surface is documented anywhere in the API | **Consistent** — the insight generator's internals are not published, so this is consistency, not proof (E8) |
| X7 | "We finally turned off Google Analytics so we have no third-parties watching how you use the site" | Privacy-policy update history records the 2024-09-19 removal; no advertising or analytics partner is named in the policy; Play declares **no data shared** | **Corroborated by two independent sources** |
| X8 | "We don't, and won't ever, sell your data" | Privacy policy: "We will never sell personal information as part of the everyday process of running Exist, **but it may be transferred as part of the sale, merger, or dissolution of Exist or Hello Code**" | **Qualified** — the marketing claim is absolute, the legal text carves out an acquisition, disclosed with notice and opt-out (E1) |
| X9 | "Permanent deletion" … "Everything about you in the Exist database is permanently wiped" | Privacy policy: data may be kept "for a commercially reasonable period, for backup and archival purposes"; payment records and support emails are kept | **Qualified** — reconcilable (backups expire) but not identical (E1-adjacent) |
| X10 | "We're not building features to let your boss, colleagues, or coach see your data" | No sharing/coach/clinician surface appears anywhere in the API reference, the attribute catalogue or the help centre | **Consistent** |
| X11 | "Your data is yours… you can export it at any time" | Export supports **PDF, CSV and JSON**; the API adds programmatic read and write access | **Corroborated** |
| X12 | "No pricing tiers, one plan gets you everything" | The values page states one plan; the Play description separately says "US$6/month" | **Corroborated for structure; conflicting on price** (website $6.99 vs Play $6) (E-2/E-35) |
| X13 | "Host our Site outside the United States… additional safeguards" | Policy states non-US hosting; **the jurisdiction is not named**. Sub-processors are Help Scout, Stripe, Mapbox, Apple Weather, Campaign Monitor, Sentry | **Corroborated in intent, unverifiable in detail** (E9) |
| X14 | "We'll never ask for access to more of your data than we need" | Health Connect Limited Use statement is explicit in the policy; Play declares collection of **Location and App info and performance** only | **Corroborated as declared** — but see X15 |
| X15 | Play data-safety declaration | The app's entire purpose is to ingest health data, yet **Health and fitness is not among the declared collected types** | **Unresolved** — most likely governed by the separate Health Connect declaration regime; not verified (E4) |
| X16 | Mood tracking: 1–9 with optional note, shown "a year ago" once enough data exists | Help article 36 matches exactly; the 1–9 scale is **rationalised by the vendor** (5 as exact midpoint, enough range for symptom severity, energy, mood) (X-6/X-8) | **Corroborated, with a documented rationale** |
| X17 | "Run experiments on your behaviour, and Exist will confirm or rule out your hunches" | **No separate experiment object, endpoint or record exists in the API.** Experiments appear to be the correlation engine applied to a user-chosen pair | **Marketing term without a distinct implementation surface** — the capability is real; the "experiment" framing is a presentation layer (E-…, question E6/E3 adjacent) |
| X18 | "Exist can even uncover what happens when you don't track a tag" | No tag-absence field, endpoint or documented mechanism was found in the API or help centre | **Unverified** — plausible (tag presence/absence as a categorical level) but no mechanism is published (E3) |
| X19 | "No manual import" / "use our API to add data" | Export is three-format; import is unsupported and is a roadmap suggestion. The API has explicit "Creating attributes" and "Updating values" sections | **Corroborated, and the workaround is the documented one** (integration finding, §4.7) |
| X20 | Published scale | **940 paid Exist users**, $6,420.85 last 4 weeks, vendor-published | **Corroborated by vendor's own instrumentation** — the only non-marketing scale figure on the shortlist |

## 7. Evidence gaps

| # | Gap | Consequence |
|---|---|---|
| 1 | **App binary not inspected** | Nothing is known about the Android/iOS implementations: no APK surface, no network endpoints, no local storage, no SDK inventory, no permission set beyond the Play declaration |
| 2 | **No authenticated API call** | All API findings are from the *documentation's* examples. Whether a live response matches the documented schema, and what extra fields exist, is unverified |
| 3 | **Full `value_type` enumeration missing** | The terminology reference page 404'd. Only `0` = Integer and `3` = Period (min) are observed, so the unit model cannot be fully assessed (E5) |
| 4 | **No server-side visibility** | Nothing is known about how correlations are actually computed — the algorithm, the correlation statistic used (Pearson vs Spearman vs something else), the p-value's derivation, or the thresholds behind `strong`/`confident` |
| 5 | **`period` is inferred** | The API reference does not define `period`. Reading it as the paired sample size rests on its value (309) and on the help centre's statement that confidence depends on how much data was used (E2/E6) |
| 6 | **Insight generator unpublished** | Consistency with "no AI" is not proof of "no AI"; the internal generator is not documented (E8) |
| 7 | **Hosting jurisdiction unnamed** | Cannot assess jurisdictional risk beyond "not the US" (E9) |
| 8 | **Roadmap not read in detail** | The public roadmap was identified and its existence corroborated, but was not systematically enumerated. Feature-commitment claims are therefore not made |
| 9 | **No pricing-page artifact** | `exist.io/pricing/` is a 404; pricing rests on the homepage and values page (E11) |
| 10 | **Tests, coverage and engineering practice unknown** | No published CI, no open-source component, no changelog of substance was inspected. Engineering-quality claims are not made |

## 8. Ethical and legal limits applied

- **No DRM circumvention.** The app binary was not obtained by any means requiring bypass of a
  distribution or protection control.
- **No certificate pinning was challenged.** No traffic was intercepted and the mitmproxy CA was
  installed on no device for this candidate.
- **No authentication was created and no API endpoint was called.** Only public reference pages were
  read; the reference documents token-authenticated endpoints, none of which were invoked.
- **No scraping of user data, no account, no trial.** The 30-day trial was not started; no Exist account
  exists.
- **Private data untouched.** No personal, PHI, device-identifying or credential material was collected
  or committed. All recorded URLs are public vendor pages.
- **Claims separated from findings.** Every claim in §6 is quoted as a claim; every finding is drawn from
  a cited public artifact. No finding is presented as a runtime observation.
- **Absence framed as absence.** "Not found" is recorded as absence of evidence — never as proof that a
  capability does not exist.

## 9. Reproduction

```sh
# API reference pages (public, no auth)
for p in reference/correlations reference/attributes reference/insights reference/averages; do
  curl -sSL "https://developer.exist.io/$p/" -o "api_$(echo $p | tr '/' '_').html"
done
# -> 200 for all four; 404 for reference/important-values, reference/object-types-and-terminology,
#    guide/best-practices

# Vendor pages
UA="Mozilla/5.0 ... Chrome/120.0"   # any ordinary browser UA
curl -sSL -A "$UA" https://exist.io/apps-data-syncing/ -o apps-data-syncing.html
curl -sSL -A "$UA" https://exist.io/about/values/      -o values.html
curl -sSL -A "$UA" http://hellocode.co/stats/          -o stats.html

# Help centre (public Help Scout KB)
# articles 10,12,14,17,18,21,36,37,55,59,62,63,99,116,118

# Platform metadata
curl -sS "https://itunes.apple.com/search?term=exist+self+tracking&entity=software&limit=10&country=us"
# -> bundleId co.hellocode.Exist, trackId 986201088, 4.02* from 47 ratings, v4.3
```

## 10. Related files

- Claims and purpose: `exist_CLAIMED_FEATURES_AND_PURPOSE.md`
- Candidate-specific comparison prompt: `PROMPT_2.md`
- Comparison report: `exist_COMPARISON.md`
- Inspection method and modes: `../RESEARCH_METHOD.md`
- Free inspection procedure (not applied here): `../FREE-ANDROID-TRAFFIC-INSPECTION.md`
