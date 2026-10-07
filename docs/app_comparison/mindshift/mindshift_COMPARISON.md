# MindShift CBT versus AnxietyWatch v1 and AnxietyWatch2 — comparison

| Field | Value |
|---|---|
| **Candidate** | MindShift CBT (`mindshift`, shortlist #10) |
| **Operator** | **247 Labs Inc** — a for-profit Toronto consultancy; Anxiety Canada is the *original* development partner, named in the past tense |
| **Platforms compared** | iOS `com.bstro.mindshift` v4.1.0 (first release 2013-06-06) · Android `com.bstro.MindShift` |
| **Research date** | 2026-10-06 |
| **Phase 2A** | **Performed — published web client and public records (no app binary; no runtime observation)** |
| **Companion files** | `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md`, `mindshift_IMPLEMENTATION_INSPECTION.md`, `PROMPT_2.md` |
| **Baselines** | AnxietyWatch v1 (`../AnxietyWatch`) · AnxietyWatch2 (`./`) |

> **Scope of the evidence.** Every statement about MindShift in this file is either a `[PUBLIC CLAIM]` by the
> vendor, a `[THIRD PARTY]` record, or a `[FOUND IN PUBLISHED CLIENT]` **static** reading of the site's served
> JavaScript. **Nothing about MindShift was observed running.** There are zero runtime observations for this
> candidate. Every statement about v1 or v2 is a `[SRC]` reading of the tree, with the file and line given.

---

## 1. Executive finding

**MindShift CBT is the most clinically-documented product in this shortlist and the least fully disclosed.**
Those two facts are not in tension — they are the same fact, seen from opposite ends of the vendor's own
publication.

The app has a real evidence base: a registered study (**NCT04562948**), a peer-reviewed paper
(PMID 40656972, *Journal of Mood and Anxiety Disorders* 2023), and a published corrigendum (PMID 40655914).
Very few consumer mental-health apps have that. The same vendor's marketing renders that evidence base as
"**Clinically proven**", "**Proven, not promised.**" and "**Most anxiety apps are guesswork.**" — while the
paper it cites reports that "**Improvements were unrelated to the frequency with which participants used the
MindShift app**", that the result was "consistent with response to treatment but **not remission**", and that
"**the lack of a comparison group who abstained from using the MindShift app limits the causal inferences one
can draw about the contribution of the MindShift app to participants' improved well-being over time**".

That is the package's clearest example of a **vendor's own funded study being the counter-surface to the
vendor's own marketing**. It is recorded as selective citation, with both sides quoted, and it is not an
allegation of misconduct — the paper is unusually candid, and crediting the authors for that candour is part of
the finding.

The second headline is an **absence**, and it is the strongest verified absence in the package:
`https://mindshiftcbt.com/privacy/`, the privacy policy declared by **both** storefronts, returns a genuine
**HTTP 404** — confirmed five independent ways, including a same-day re-fetch that returned **byte-identical**
content. The site's sitemap lists nine URLs and its router thirteen paths; **neither includes a privacy or
terms path**. A fixed-string sweep of the 741,896-byte published client returns **0** occurrences of
`Privacy Policy`, `Terms of Use` or `Terms`. Four `Privacy` strings survive, and all four are marketing cards or
FAQ headings.

The third is found only by reading the client, and it is a **static** finding. The same site that titles a
feature card "**Privacy respected**" ships its own visitor telemetry — `POST /api/analytics/session` and
`POST /api/analytics/events`, keyed to a per-session UUID, with `credentials:"include"`, covering page views,
resource opens, feature-panel exploration and client errors — and publishes **no cookie notice, no consent
banner and no analytics disclosure**, because it has no privacy page to publish a notice on. The code is
written to collect; whether any endpoint is live or any event was stored is `[UNKNOWN]`, and this report says
so rather than the reverse.

**Against the baselines, the comparison cuts both ways, and cleanly.**

- **Where MindShift is ahead:** crisis routing in the user's own language, at the point of use. Three named
  Canadian lines — **988 Suicide Crisis Helpline**, **Kids Help Phone**, **Talk Suicide Canada** — are present
  in the served client with `tel:` links and Canadian qualifiers, and the FAQ states the app is "not a medical
  device, and not a replacement for therapy or crisis care" and is for "Adolescents (13+) and adults with
  mild-to-moderate anxiety". A repository-wide search of AnxietyWatch2 for `crisis` across `*.rs`, `*.md`,
  `*.toml` and `*.swift` returns **0 files**. v1 carries a crisis notice in its `README.md` — "**If you are in
  crisis:**" and "This app is not a crisis intervention tool." — but no in-app crisis routing. **On the one
  safety surface that matters most at the worst moment, a free CBT app is ahead of both versions of this
  product.**
- **Where the baselines are ahead:** everything about data discipline. MindShift publishes no retention model,
  no export schema, no missingness semantics, no tombstone, no per-record provenance, and no measured-signal
  contract at all — its data is by design self-reported check-ins and CBT worksheets. v2 specifies an
  append-only `samples` table with tombstone removal, a registry with one canonical unit per type, sentinel
  rejection rather than clamping, and an explicit three-state honesty rule. MindShift has none of that because
  it is not that kind of product; the comparison should say so plainly rather than scoring a category the
  candidate never entered.
- **Where both are the same, and neither should be credited alone:** the scope boundary. v2's admin-console
  spec says "**This is explicitly not a medical device**, not FDA-cleared, and not on a path to clearance"
  (`docs/design/SPEC-admin-console.md:33`); MindShift's FAQ says "not a medical device, and not a replacement
  for therapy or crisis care". Two products in different domains arriving at the same honest sentence is worth
  recording as a shared property, not as a win.

**The single most useful thing this candidate teaches the project is neither a feature nor a defect — it is a
counting discipline.** MindShift is registered as `studyType: OBSERVATIONAL` with **no intervention module** and
**one** registered primary outcome; its paper is titled "trial" and reports "**four primary outcomes**"; its
site says "**open-label clinical trial**" and "**all four primary outcomes**". A registry is a third party; a
title and a marketing page are not. When a product's numbers come from its own surfaces, the registry is the
one that outranks them — and the same rule applies to AnxietyWatch2's own future claims about its CNS engine.

---

## 2. Product and availability

| Property | MindShift CBT | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| **What it is** | Free CBT self-help app: thought records, check-ins, guided tools, a moderated community | Personal anxiety journal with device-derived context and a CNS early-warning aid | Self-hosted vitals collector + daemon + console, with an iOS client |
| **Operator / licence** | 247 Labs Inc, for-profit; proprietary | Owner-built; repository not public | Owner-built; repository not public |
| **Price** | Free; a for-profit B2B surface (`/for-clinics`, `/institutions`) with **unpublished** pricing | Free (owner-built) | Free; self-hosted hardware cost |
| **iOS** | `com.bstro.mindshift` v4.1.0, 87,966,720 B, minOS 15.1 | Swift/SwiftUI app in the tree | `clients/apple/AnxietyWatchApp` |
| **Android** | `com.bstro.MindShift` | — | planned / not shipped |
| **First release** | **2013-06-06** | — | — |
| **Distribution** | App Store, Google Play | local build | local build + Pi deployment |
| **Rating (as retrieved)** | App Store **4.32402 / 537**; Play **3.3** (header "2.2K reviews", ratings block "2.08K"); site **"4.8"** | — | — |
| **Downloads (as retrieved)** | Play **"500K+"**; site **"More than 2 million"** / **"2M+"** | — | — |
| **Supported languages** | Site FAQ **"English and French"**; App Store `['EN','FR','ES']`; client language switch `en`/`fr` | — | — |
| **Age / content rating** | App Store page **9+**, iTunes `contentAdvisoryRating` **4+**; Play **Teen**, "Users Interact" | — | — |
| **Maintenance cadence** | v4.1.0 released 2026-09-19; listing updated 2026-09-18 | active development | active development |

**Availability is not the differentiator.** MindShift reaches hundreds of thousands of installs across two
storefronts and has been shipping since 2013 — thirteen years, in which the app changed handler from Anxiety
Canada to a for-profit consultancy. AnxietyWatch is a personal instrument with one operator and no distribution
platform. The comparison is between a widely distributed product and a bespoke one, and the useful question is
not reach; it is what each one is allowed to claim and what each one tells its user.

**One availability fact is recorded and not resolved:** the storefronts disagree with each other and with the
site about downloads and rating (§5.6 of the inspection). Both values are quoted with their retrieval dates and
**not averaged**. Where the *same* storefront shows two review counts on one page, that ambiguity is marked
**unresolved** and not attributed to the vendor — Apple's own page showing "9+" beside a record saying "4+" is
an Apple-side inconsistency and is recorded as such.

---

## 3. System boundary and architecture

### MindShift CBT — what the published client reveals

```
        user (browser)                        vendor infrastructure
   ┌───────────────────────┐              ┌──────────────────────────────┐
   │  served HTML shell    │              │  /api/resources              │  content
   │  + index-*.js (SPA)   │───GET────────▶│  /api/resources/:slug        │
   │                       │              │  /api/contact                │
   │  13 routes:           │              ├──────────────────────────────┤
   │   / /about /app       │              │  /api/admin/login|logout|    │
   │   /contact /faq       │              │         session|analytics    │
   │   /for-clinics        │───CRUD───────▶│  /api/admin/resources/...    │
   │   /institutions       │              ├──────────────────────────────┤
   │   /methodology        │              │  /api/analytics/session      │  ◀── the
   │   /resources[/:slug]  │───POST───────▶│  /api/analytics/events       │      finding
   │   /admin/*? /sign-in  │              ├──────────────────────────────┤
   │   /sign-up            │              │  /api/__clerk  (Clerk proxy) │  identity
   └───────────┬───────────┘              └──────────────────────────────┘
               │
               │  browser-local state
               ├── localStorage  "mindshift-lang" = en | fr          (language only)
               └── sessionStorage "s0" = UUIDv4, "mindshift.analytics.signed-in"
               │
               │  third-party code loaded by the client
               ├── Clerk  (auth, organizations, billing)   ── 505 case-insensitive hits
               ├── Stripe (via Clerk billing)              ── 3
               └── Sentry (error telemetry)                ── 129 case-insensitive

   robots.txt:  Disallow /admin  /sign-in  /sign-up  /api/
   sitemap.xml: 9 <loc> entries — no privacy, no terms
   router:      13 paths      — no privacy, no terms
   probe:       /privacy /privacy/ /terms /terms/ /privacy-policy[...] /legal[...] → all 404
```

**What this diagram is.** It is a **static map read out of published JavaScript**. It shows what the site is
written to do. The `POST` arrows out of the browser are drawn because the code contains those calls, **not**
because any call was made — no endpoint in this diagram was contacted, and none may be.

**What is not in the diagram, and why the omission matters more than the contents.** There is no privacy page,
no terms page, no cookie notice, no consent surface and no analytics disclosure anywhere in it. A site of this
scope would normally carry all five. The diagram is a boundary map in which the disclosure layer is simply
absent, and that absence is the finding.

### AnxietyWatch2 — the same view of the other product

```
   ┌────────────────────────┐        ┌──────────────────────────────────────────────┐
   │ collectors (Linux/Pi)  │        │  vitalsd: BlueZ · tokio · postgres           │
   │  AirSense 11 · Polar   │──spool─▶│                                              │
   │  EMAY oximeter · H10   │  COPY  │  TimescaleDB                                 │
   └────────────────────────┘ binary │   samples  — append-only, tombstone removal  │
                                     │   time_bucket_gapfill  — never locf/interp.  │
   ┌────────────────────────┐        │                                              │
   │ iOS client             │◀──API──│  admin console (Rust) · API (Rust)           │
   │  client root view ·    │        └──────────────────────────────────────────────┘
   │  HealthKit · journal   │
   └────────────────────────┘
        │
        └── v1: SwiftData/SQLite + HLC sync · ReportGenerator · DataExporter · ExportView
        └── v2 target: CNS early-warning engine, supervised alarm, hardware dead-man switch
                       ── specified only; the console spec states it must not be
                          designed as though CNS were the product (SPEC-admin-console.md:31)
```

**The two products are not substitutes and should not be scored as though they were.** MindShift is a
self-report psychotherapy adjunct with no measurement stream; AnxietyWatch is a measurement system with a
journal. The overlapping ground is narrow and specific: **what each one tells the user about the limits of what
it can see.** That is the ground on which this comparison is decided.

---

## 4. Data path and source comparison

| | MindShift CBT | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| **Primary input** | user self-report: check-ins, thought records, guided-tool completion | user journal + device data | device telemetry + user journal |
| **Physiological sources** | **none published** | HealthKit, Polar H10, EMAY oximeter, ResMed AirSense 11, Oura | registry-driven: AirSense 11 (17 types), Polar H10, EMAY, oximeter |
| **Measured quantity count** | 0 measured physiological quantities in the published record | multiple, per-integration | registry as single source of truth (`vitals/registry.toml`) |
| **Unit canonicalisation** | n/a — no measured quantities | per-integration | one canonical unit per type, converted once at the decoder |
| **Where data lives** | vendor servers (account) + on-device | on-device; HLC sync | local Pi (TimescaleDB) + iOS client |
| **Account required?** | optional for local use; "Creating an account lets you back up your data and sync it across devices" | no | no (local first) |
| **Export** | "A structured CSV of check-ins and the data collected across guided tools", patient-initiated | `DataExporter`, `ReportGenerator`, `ExportView` | report generator producing "something you can hand a clinician" (`SPEC-admin-console.md:28`) |
| **Deletion model** | "request deletion from the app settings, which permanently removes your data from our servers" — **no tombstone model published** | not published | append-only; **removal is a tombstone row**, never `DELETE` |
| **Derived measures published** | outcome scores in the paper; **no instrument list, units or windows published** | correlation engine | registry-defined types and ranges |

**The asymmetry is total and should be named, not scored.** MindShift has no measured signal to have a
canonical unit for. A comparison table in which AnxietyWatch "wins" eleven rows against a product that never
had those rows is a table that has stopped measuring anything. The honest reading is: **MindShift is not in the
measurement business, and the parts of the AnxietyWatch discipline that apply to it are disclosure,
provenance and the statement of limits — not sampling, units or gap-filling.**

**One row genuinely is comparable, and MindShift loses it.** `README.md`-level and spec-level, v2 states a
missingness rule: invalid or absent data contributes nothing and is never coerced into a number. MindShift's
data model is self-report, where absence is simply an unfilled day — a comfortable position that the vendor
nonetheless does not describe anywhere. Whether the app distinguishes a day that was not filled in from a day
filled in with no anxiety reported is `[UNKNOWN]`; no artifact in this package could settle it.

---

## 5. Claim-versus-documented-contract reconciliation

The full reconciliation is `mindshift_IMPLEMENTATION_INSPECTION.md` §7 — **56 rows, machine-counted**:
**20 MATCH · 12 DOCUMENTED ONLY · 12 DOES NOT MATCH · 10 PARTIAL · 2 OUTSIDE** (machine-counted from that
file's 56 rows; the tally in the inspection's §7 supersedes the earlier draft count and is authoritative).
This section gives the six that decide the comparison.

**5.1 The declared privacy URL does not exist.** Both storefronts point at
`https://mindshiftcbt.com/privacy/`. That path, and three plausible variants, return a genuine 404 document
whose readable body is "**Page not found**" / "**The requested MindShift CBT page could not be found.**" — not
a JS error shell, not a WAF challenge. Verified five ways: the probe, the nine-URL sitemap, the thirteen-path
router, the bundle string sweep, and a byte-identical re-fetch. The vendor's own release notes for **v3.0.1**
and **v3.0.3** say "**The Terms of Use and Privacy Policy have also been updated to reflect this new feature.**"
The documents existed. The URL that is still published does not resolve. **Recorded as a
documentation-lifecycle and discoverability defect — not as a data-handling finding and not as an allegation of
intent.**

**5.2 "Clinically proven" against the vendor's own funded paper.** Covered in §1. The three marketing strings
and the three paper sentences are quoted verbatim in the inspection §5.3. **Selective citation, recorded and
left standing.**

**5.3 The registry classifies the study differently from the paper and the site.** Registered:
`studyType: **OBSERVATIONAL**`, `observationalModel: ECOLOGIC_OR_COMMUNITY`, `timePerspective: PROSPECTIVE`,
**no `armsInterventionsModule`**, `enrollment 380 (ACTUAL)`, **one** primary outcome ("Linear and non-linear
change in generalized anxiety severity over 16 weeks") and **three** secondary. Reported by the paper: the
title's "**trial**" and the text's "**four primary outcomes**". Said by the site: "**open-label clinical
trial**" and "**all four primary outcomes**". **Two conflicts — design class and primary-outcome count — both
recorded unresolved, both quoted.** The primary-outcome conflict is the sharper of the two, because it changes
what the study was powered to answer.

**5.4 The verified-install subset is 156 of 380.** The paper's methods: "**the sample was restricted to the 156
participants for whom entrance of their study identification code into the MindShift smartphone app verified
that they downloaded the smartphone app**". **224 of 380 participants never demonstrably installed the
product**, and every headline surface carries 380. **Recorded as a claim-precision finding**; the paper reports
the restriction plainly.

**5.5 The funding disclosure exists in the paper and not on the surface that cites the paper.** The paper's
competing-interest statement: "**Lance M. Rappaport reports financial support was provided by Anxiety Canada.**"
Four of five authors "**volunteer on the Scientific Advisory Committee of Anxiety Canada**"; the
acknowledgements: "**This work was supported by a grant from Anxiety Canada to LMR.**" Anxiety Canada is the
app's original co-developer. **Recorded as a disclosure asymmetry between two surfaces by the same author.**

**5.6 A correctable error in one of the four headline outcomes.** The 2024 corrigendum: "**an unfortunate error
in the calculated total score for quality-of-life from the Q-LES-Q-SF**"; baseline corrected from (M=30.76,
SD=9.77) to (M=47.83, SD=15.39), reliability item count 16 → 14, "**the primary conclusions are unchanged**".
The corrigendum **does not restate** the quality-of-life effect size, so whether d = 0.31 moved is `[UNKNOWN]`.
Recorded as a **published correction**, which is the process working; the open question is whether the headline
number survived it.

**What is deliberately not in this section.** Two hypotheses were tested and **refuted** — a Segment CDP
integration and a Google `gtag` tag are both absent from the client. A refuted hypothesis is **not a finding**
and appears nowhere in this file. A hypothesised integration that isn't there is not evidence of good practice
either.

---

## 6. Feature matrix

Columns: **v1** (AnxietyWatch v1) · **v2 now** (exists in `./` today) · **v2 target** (specified in `openspec/`,
not shipped) · **MS** (MindShift CBT) · **evidence/maturity**.

| Capability | v1 | v2 now | v2 target | MS | Evidence / maturity |
|---|---|---|---|---|---|
| **Scope boundary stated to the user** | README only | `SPEC-admin-console.md:33` | — | **Yes** — "not a medical device…", "mild-to-moderate", 13+ | MindShift FAQ (client); v2 spec |
| **Crisis routing at the point of use** | **No** in app (README notice only) | **No** — `crisis` → **0 files** | — | **Yes** — 988, Kids Help Phone, Talk Suicide Canada | `[FOUND IN PUBLISHED CLIENT]` MindShift; `[SRC]` v2 |
| **Named emergency numbers with qualifiers** | README: 988 | none | — | 3 (Canada; Québec number given) | MindShift client |
| **Clinician role** | report to hand over | report generator (`:28`) | — | **explicitly refused**: "There's no clinician portal — each person owns and controls their export." | both: documented |
| **Patient-controlled export** | `ExportView`, `DataExporter` | report generator | — | **claimed**, CSV, no schema published | MindShift `/for-clinics` |
| **Published evidence base** | none | none | — | **Yes** — NCT04562948, PMID 40656972, corrigendum | `[THIRD PARTY]` |
| **Evidence classified honestly by the vendor** | — | **Yes** — CNS is "an in-progress early-warning aid, not the product" (`:31`) | — | **No** — "Clinically proven" vs the paper's own limits | both quoted |
| **Typed series, one canonical unit per type** | per-integration | registry + generated enums | — | **No** — no measured series | v2 registry |
| **Append-only store, tombstones, no `DELETE`** | No | **Yes** (`samples`) | — | **No** (self-report; deletion = "permanently removes") | v2 amendment 01 |
| **Sentinel rejection, never clamp** | No | **Yes** (`0x07FF`/`0x0800`) | — | n/a | v2 |
| **Three-state honesty (good / bad / not measured)** | No | **Yes** | — | **No** — no missingness semantics published | v2 |
| **Quarantine with a named reason** | No | **Yes** | — | **No** | v2 |
| **Per-record provenance** | partial | **Yes** | — | **No** | v2 |
| **Registry as single source of truth** | No | **Yes** (`vitals/registry.toml`) | — | **No** — no instrument list, units or windows published | v2 |
| **Real-time physiological capture** | **Yes** | **Yes** | — | **No** | v2 collectors |
| **Overnight / unattended capture** | partial (AirSense 11) | **Yes** (spool) | — | **No** | v2 |
| **Alert/escalation mechanism** | **Yes** — `CNSAlarmPresenter`, `CNSAlertTierMachine`, `CNSKlaxonFastPath` | partial | **supervised alarm + hardware dead-man switch** | **No alert path at all** | v1 `Services/CNSRisk/` |
| **Local-first operation** | **Yes** | **Yes** (Pi + local Postgres) | — | **No** — account + vendor servers | v2 |
| **No vendor account required** | **Yes** | **Yes** | — | optional-account claim; account needed for sync/backup | MindShift FAQ |
| **Visible third-party processors** | none | none | — | **3** — Clerk, Stripe, Sentry | `[FOUND IN PUBLISHED CLIENT]` |
| **First-party telemetry disclosed** | n/a (no site) | n/a | — | **No notice found** — the client collects; nothing discloses | `[FOUND IN PUBLISHED CLIENT]`, static |
| **Privacy policy reachable** | n/a | n/a | — | **No** — declared URL 404s, verified five ways | `[FOUND IN PUBLISHED CLIENT]` + probes |
| **B2B / organizational surface** | No | No | admin console | **Yes** — `/for-clinics`, `/institutions`, Clerk orgs, Stripe, unpriced | client + site |
| **Moderation / community policy published** | No | No | — | **No** on the web; a guidelines surface exists in-app per v4.1.0 notes | gap G-7 |

**Reading the matrix.** The right-hand column is not a score. MindShift has two rows in this table that
AnxietyWatch cannot match — **crisis routing** and a **published evidence base** — and those are the two rows
that would matter most to a person in trouble. AnxietyWatch has twelve rows MindShift cannot match, eleven of
which are consequences of MindShift not being a measurement product and one of which — the **crisis routing**
row — is a real gap in this project. **The rows where each is genuinely ahead are the same rows: what it tells
the user about what it cannot do.**

---

## 7. Derived values versus stored signals

**MindShift.** The paper reports four outcome measures (generalized anxiety severity, depressive symptom
severity, functional impairment, life satisfaction) and effect sizes d = 0.61 / 0.50 / 0.55 / 0.31. The
registry lists one primary and three secondary. The app itself publishes **no scoring instrument list, no
units, no valid ranges and no version identifiers**. Whether a given stored value is a raw response, a
subscale total, or a derived score is `[UNKNOWN]` from outside. **An undocumented value domain is a finding**,
and here it applies to every derived value the app produces.

**AnxietyWatch2.** Derived and stored are separated by construction: `vitals/registry.toml` defines the types,
units and valid ranges; generated enums mean a source id and a type id cannot be transposed at a call site; a
value outside range is rejected with a named reason rather than clamped. **Conversion happens once, at the
decoder.**

**v1.** Derived measures exist (a correlation engine, a report generator, CNS severity scoring) and sit on a
SwiftData/SQLite store with HLC-based sync. v1's known defect in this area is documented elsewhere in this
package and is not repeated here as a MindShift finding.

**The comparison that does transfer.** MindShift's outcome measures are published in a paper and not in its
data model; AnxietyWatch2's are published in a registry file and not in a paper. **Each one discloses what the
other does not**, and the project's advantage is not that it has a registry — it is that the registry is
checkable by the user without reading an academic appendix. That is a real, reproducible advantage and it is
the one thing worth carrying forward from this candidate.

**And the caution that also transfers.** MindShift's headline numbers are the ones its own corrigendum had to
correct. Any future AnxietyWatch2 claim about CNS sensitivity, specificity or alert accuracy must be
accompanied by the sample, the setting and the version of the algorithm that produced it — the discipline Oura
publishes as a per-record algorithm-version field, and the discipline MindShift's paper could not sustain
across a corrigendum.

---

## 8. Real-time, overnight and failure-mode comparison

| Scenario | MindShift CBT | v1 | v2 (now / target) |
|---|---|---|---|
| **Continuous physiological capture** | **None** — no sensor stream | yes, per-integration | yes (spool) |
| **Overnight unattended operation** | n/a — nothing to capture | AirSense 11 import | spool + daemon |
| **Signal absence** | no missingness semantics published | per-integration | three-state rule; absent contributes nothing |
| **Offline use** | claimed optional-account local use; whether check-ins work fully offline is `[UNKNOWN]` | yes | yes (local-first) |
| **Device/account failure** | app is a client of vendor servers for sync and community | local | local; degraded mode specified |
| **A user in crisis** | **three named crisis lines, at the point of use** | README notice only | **nothing** |
| **A claim with no reachable source** | "recommended by clinicians at academic medical centres", no citation | — | — |
| **A number that turns out wrong** | corrigendum published; whether the headline effect size moved is `[UNKNOWN]` | — | no such mechanism specified |
| **Alert fails to fire** | n/a — no alert path | `CNSAlarmPresenter` path exists | specified alarm + dead-man switch |
| **Site collects data and fails silently** | `.catch(()=>{})` on both analytics calls — **by design** | n/a | n/a |

**The failure-mode finding that belongs to this candidate.** Both of the site's analytics calls terminate in
`.catch(()=>{})`. That is a deliberate, correct engineering choice for non-essential telemetry — a page must
not break because measurement did. It is worth recording precisely because the *same pattern* in a *safety*
path would be a defect: the code that measures must never fail loudly, and the code that alarms must never fail
silently. **AnxietyWatch2 specifies the opposite behaviour for the alarm path, and MindShift illustrates the
right way to do the thing that is wrong for alarms.** That is a useful, checkable contrast and it is not a
criticism of the vendor.

**The failure mode this candidate illustrates about evidence.** A registered study whose classification, primary
outcomes and headline effect size cannot all be reconciled from the vendor's own surfaces is the worked example
of what happens when a product's numbers live in prose rather than in a schema. AnxietyWatch2's registry is the
structural answer to that failure mode — which is the reason this candidate is in the shortlist at all.

---

## 9. Provenance, missingness and reproducibility

| Property | MindShift CBT | v2 |
|---|---|---|
| **Per-record provenance** | none published | **Yes** — amendment 02 (integrity/provenance/encryption) |
| **Algorithm/method version on a record** | no | not per-record in v2 either; Oura publishes one (`PublicSleepAlgorithmVersion`) and is the comparator |
| **Missing vs zero distinction** | not published | **Yes** — the three-state rule |
| **Deletion** | "permanently removes your data from our servers" | **tombstone row**; `samples` is append-only |
| **Reproducible from outside** | no — no data model published | **Yes** — `vitals/registry.toml` is in the tree |
| **Evidence reproducible** | paper + registry + corrigendum, all reachable | internal tests and Verify commands |
| **Disclosure of funding/interests** | paper: **yes, verbatim**. Marketing: **no** | n/a |

**The provenance contrast is the sharpest structural difference in the package.** MindShift tells the user
where their data goes in two sentences — "**stored securely, and we don't sell your personal data to third
parties**" — and can say nothing else, because there is no published model behind the sentences. v2 can say
almost nothing in prose and almost everything in a schema. **Both are incomplete; neither is dishonest; the
difference is that v2's claims are checkable by anyone with the repository and MindShift's are checkable only by
reading the vendor's paper.** Which one a user should trust depends on a question this comparison cannot
answer: whether the user has the repository.

**One integrity fact is recorded as a positive.** The published client and every retrieved artifact in this
package is SHA-256-hashed and dated, and the same-day re-fetch of the 404 returned byte-identical content. That
is the standard the project applies to a **vendor's** artifacts; applying it to its own claims about its own CNS
engine is the transferable lesson.

---

## 10. Local/cloud/privacy and operational cost

| | MindShift CBT | v1 | v2 |
|---|---|---|---|
| **Where data lives** | vendor servers (account) + device | device; HLC sync | local Pi + local Postgres |
| **Cloud mandatory?** | no for local use; yes for sync, backup and community | no | no |
| **Operational cost** | free | free (owner-built) | hardware + electricity |
| **Privacy document reachable** | **No — declared URL 404s, verified five ways** | n/a | in-tree |
| **Analytics notice** | **none found**, while the client collects | n/a | n/a |
| **Third-party processors** | **Clerk, Stripe, Sentry** | none | none |
| **Data sale** | "we don't sell your personal data to third parties" | n/a | no third party exists |
| **Deletion mechanism** | app settings; "requests" per Play | local | tombstone |
| **Corporate entity contracting** | 247 Labs Inc (for-profit); Anxiety Canada named historically | owner | owner |

**The privacy finding, stated at the level the evidence supports.** A site that (a) declares a privacy policy
that does not resolve, (b) ships its own visitor telemetry, and (c) publishes no notice of that telemetry, is
the combination recorded here. **It is not recorded as a data-handling failure**, because nothing about the
vendor's handling of data was observed and the analytics may well be innocuous: a random per-session UUID,
four event categories, first-party origin, credentials included. It is recorded as a **disclosure** failure,
which is a different kind of finding with a different remedy — and the reason it matters to this project is
that **it is the failure mode AnxietyWatch2's own future web surfaces could drift into**. A local-first product
that later adds a marketing site with analytics has exactly this failure mode available to it.

**Operational cost is not comparable and should not be presented as one.** MindShift's cost is borne by a
for-profit consultancy and its institutional customers; whether that model survives is not the user's risk.
AnxietyWatch's cost is borne by the owner, in hardware, electricity and maintenance, and its failure mode is
that the owner must maintain it. **Both are free to the user and both depend on a party who could stop.**

**One cost asymmetry is real and should be named.** MindShift's consumer app is free and its B2B surface is
unpriced. AnxietyWatch2's operator is the same person as its user. A user of MindShift cannot know what
continues to be funded; the owner of AnxietyWatch knows exactly what is funded, because it is funded by them.
That is a genuine advantage of the bespoke product and it is not a claim about the commercial one.

---

## 11. Alerting and the alarm path

**This is the section where the comparison inverts, and it must be read in both directions.**

### 11.1 What MindShift has that neither baseline has

Three named crisis lines, present in the served client, each with a `tel:` link and a Canadian qualifier:

| Line | Offered as |
|---|---|
| **988 Suicide Crisis Helpline** | "Call or text 988, 24/7, anywhere in Canada." — `tel:988` |
| **Kids Help Phone** | "Call 1-800-668-6868 or text CONNECT to 686868." — `tel:18006686868` |
| **Talk Suicide Canada** | "Call 1-833-456-4566, 24/7 (Québec: 1-866-277-3553)." — `tel:18334564566` |

Alongside, in the FAQ, in the user's own language: the app is "**not a medical device, and not a replacement
for therapy or crisis care**" and is for "**Adolescents (13+) and adults with mild-to-moderate anxiety**".

**And the baseline it is compared against.** A repository-wide search of AnxietyWatch2 for `crisis` across
`*.rs`, `*.md`, `*.toml` and `*.swift`, excluding `docs/app_comparison/`, returns **0 files**. v1's `README.md`
carries one paragraph — "**If you are in crisis:**" and "(call or text 988) or your local emergency services.
This app is not a crisis intervention tool." — **a documentation notice, not an in-app affordance**, and v2's
README does not carry it.

**The finding, stated plainly:** a free CBT app routes a person in distress to three real phone numbers at the
moment of use, and this project — whose entire purpose is a person's mental health — does not. This is the
single clearest place in the whole package where a vendor is genuinely ahead of both versions of AnxietyWatch,
and it is a gap in **copy and routing**, not in engineering.

### 11.2 What the baselines have that MindShift does not

MindShift has **no alert path of any kind**. It measures nothing, monitors nothing, escalates nothing and has
no unattended mode — because it is a self-help app and that is a legitimate design for one. It is not a missing
feature; it is a different product.

v1 has a real alert stack: `AnxietyWatch/Services/CNSRisk/` contains `CNSAlarmPresenter.swift`,
`CNSAlertTierMachine.swift`, `CNSAlarmChannelPolicy.swift`, `CNSCriticalAlertPermission.swift`,
`CNSFusionEngine.swift`, `CNSSeverityScorer.swift`, `CNSQualityGate.swift`, `CNSDeviceStateMatrix.swift` and a
klaxon fast path with its own tests. v2 **specifies** a CNS early-warning engine, a supervised alarm and a
hardware dead-man switch — and the admin-console spec is explicit that CNS is "**one feature among many** — an
in-progress early-warning aid, not the product", and that the console "must not be designed as though it were".

**A caution that applies to this project, and not to the candidate.** v1's alert stack and v2's specified alarm
layer are the parts of this system with the highest consequence of being wrong. MindShift's `.catch(()=>{})` on
telemetry is the correct pattern for a measurement path and the **wrong** pattern for an alarm path. The
comparison worth drawing is not "MindShift has no alarm" — obviously it does not — but that **the discipline
which makes MindShift's telemetry good is the discipline that would make an alarm fatal, and this project must
not let one leak into the other.**

### 11.3 The irreducible difference

MindShift's safety surface is **a phone number a person can choose to call**. AnxietyWatch's safety surface is
**a machine deciding to wake someone**. Those are not comparable mechanisms, and the obligation each one carries
is different: an app that lists a crisis line cannot fail to list it, and a system that decides to alarm can
fail to alarm. **The second obligation is much heavier, and nothing in this candidate's evidence base
discharges it.** That is why the crisis-routing gap in §11.1 is a copy fix and the alarm-path question in
§11.2 is not.

---

## 12. User-facing consequences

**For a person using MindShift:**

- They can find help in a crisis from inside the app, in the language they are reading in. **This works and it
  is the most important thing in this file.**
- They are told the app is not a medical device and is for mild-to-moderate anxiety. That is honest and it is
  repeated.
- If they look for the privacy policy the store told them about, they will not find it. They will see a "Page
  not found" page. They cannot learn what the site collects about them, because the site publishes no notice —
  and the site does collect.
- If they read the marketing, they will be told the app is "**Clinically proven**". If they read the paper the
  site cites, they will find that improvement was unrelated to how much people used the app and that the study
  had no comparison group. **Both sentences are the vendor's.**
- If they want their data out, they are promised a structured CSV and told no clinician portal exists — a
  posture that is genuinely respectful, and one this project shares.

**For the owner of AnxietyWatch:**

- The most useful thing to take from MindShift is not a feature. It is the observation that **a product can
  have a registered study, a peer-reviewed paper, a corrigendum and a dead privacy page at the same time** —
  because the study and the page are maintained by different people for different reasons, and the page decays.
  AnxietyWatch2's registry and its `Verify:` commands are the structural answer to exactly that decay.
- The second is the crisis-routing gap. It is in copy, it is cheap, and this comparison is the artifact that
  records it. **This report does not propose a remediation**; it records that a free CBT app does this and this
  project does not.
- The third is a caution, not a lesson: the same discipline that makes good telemetry (`catch` and continue)
  must never reach the alarm path, where the correct behaviour is the opposite.

**What a user of AnxietyWatch does not get, and should not be told they do.** They do not get a crisis
affordance; they do not get a peer-reviewed evidence base; they do not get a moderated community; and they do
not get a product that someone else maintains. Those are real, and the comparison should not hide them behind
eleven rows about units and tombstones.

---

## 13. Limitations, unverified claims and evidence gaps

**Limitations of this comparison.**

1. **No app binary was inspected and no runtime was observed.** Phase 2A mode for this candidate is
   "Performed — published web client and public records (no app binary; no runtime observation)". The app the
   user actually installs was **not** examined. Everything attributed to the app is a storefront declaration, a
   vendor claim, or a string in the *website's* client.
2. **The website and the app are different artifacts and this file never merges them.** A finding from
   `index-BogFfh2L.js` is a finding about the **website**. The Play data-safety block is about the **app**. The
   App Store App Privacy panel is about the **app**. Keeping these apart is the single most important
   discipline applied in this candidate's work.
3. **No endpoint was called and no account was used.** The endpoints named in §3 exist as strings. Whether any
   of them is live is `[UNKNOWN]`.
4. **`anxietycanada.com` was unreadable** — every path, including `/robots.txt` and `/sitemap.xml`, returned a
   byte-identical 9,321-byte SPA shell. That is a **retrieval failure**, and Anxiety Canada's current
   statements about MindShift, and its own privacy policy, are `[UNKNOWN]` as a result.
5. **Apple's App Privacy panel is internally ambiguous** — "Contact Info" appears under both "Data Linked to
   You" (Name) and "Data Not Linked to You" (Email Address), while the collapsed summary lists it under
   "Linked". Whether the declared "Health & Fitness → Health" data is account-linked is `[UNKNOWN]`. **Both
   readings are recorded and neither is chosen.**
6. **The rating and download figures conflict across surfaces** and are quoted, not reconciled. Where the same
   storefront shows two counts, the ambiguity is marked unresolved.

**Unverified claims carried forward as unverified.**

| Claim | Class | Why it stays unverified |
|---|---|---|
| "recommended by clinicians at academic medical centres in Canada and the US, and used in research in outpatient and inpatient psychiatric settings" | `[PUBLIC CLAIM]` | no citation on the surface |
| "A structured CSV of check-ins and the data collected across guided tools" | `[PUBLIC CLAIM]` | no schema, no export performed |
| "stored securely" | `[PUBLIC CLAIM]` | undefined term; no encryption-at-rest statement |
| "we don't sell your personal data to third parties" / Play "No data shared with third parties" | `[PUBLIC CLAIM]` + developer self-declaration | Apple's own label says the app declaration "has not been verified by Apple" |
| "**Clinically proven**" | `[PUBLIC CLAIM]` | the vendor's own funded, uncontrolled paper does not carry it |
| "**Most anxiety apps are guesswork.**" | `[PUBLIC CLAIM]` | a comparative claim about unnamed products; no artifact could settle it |
| Quality-of-life effect size after the corrigendum | `[UNKNOWN]` | the corrigendum does not restate it |
| Whether a privacy/terms document exists **inside** the app | `[UNKNOWN]` — gap **G-1** | the binary was not inspected; release notes say the documents were updated |
| Which language set the shipped UI has | `[UNKNOWN]` — gap **G-4** | four surfaces, two answers |
| B2B pricing and terms | `[UNKNOWN]` — gap **G-6** | not published; the waitlist was not joined |
| Whether a moderation policy exists in-app | `[UNKNOWN]` — gap **G-7** | v4.1.0 notes imply a guidelines surface; nothing on the web |

**Evidence gaps.** The full list is `mindshift_IMPLEMENTATION_INSPECTION.md` §8 (**G-1**–**G-12**). The four
that would most change this file are: **G-1** (the privacy/terms documents inside the app), **G-3**
(`anxietycanada.com` unreadable), **G-4** (the shipped language set) and **G-11** (the post-corrigendum effect
size).

**What this comparison does not do.** It does not assess whether MindShift is clinically effective — that is a
question for controlled trials, and the vendor's own evidence is uncontrolled. It does not assess whether the
vendor's privacy practices are lawful — no regulator's finding was consulted and none is implied. It does not
compare the two products' therapeutic value, which is not a comparable quantity. And it does not present a
single runtime observation of the candidate, because there are none.

---

## 14. Balanced conclusion

**MindShift CBT is a legitimate product with a real evidence base, an honest statement of its own scope, and a
disclosure layer that has rotted while the product kept shipping.**

The good parts are not small and should be first. It routes people to help. It says out loud that it is "not a
medical device, and not a replacement for therapy or crisis care", that it is for "mild-to-moderate anxiety",
and that it is not for children under 13. It refuses a clinician portal in favour of a patient-controlled
export. It published a study, and then published a correction to it, which is more than most products in this
space do. **On the two rows of the feature matrix that would matter to a person in distress — a crisis
affordance, and an evidence base — MindShift is ahead of both versions of AnxietyWatch, and the crisis-routing
row is a real gap in this project, in copy rather than in engineering.**

The findings are also not small. The privacy policy both stores link to does not exist, verified five ways. The
site measures its visitors and tells no one. "**Clinically proven**" is rendered from a study whose own paper
reports that improvement was unrelated to app usage, that the result was not remission, and that the absence of
a comparison group "**limits the causal inferences**" — and the one sentence the site quotes from that paper,
"**all four primary outcomes**", is the sentence the independent registry does not support, since the
registration lists **one** primary outcome and classifies the whole thing `OBSERVATIONAL`. One of the four
headline outcomes needed a published corrigendum, and whether its effect size moved is still unknown. And 224
of the trial's 380 participants never demonstrably installed the app.

**These two lists are the same list.** A vendor that publishes a candid paper and a marketing page derived from
it, without maintaining the page or the policies around it, produces exactly this artifact: real evidence,
selectively rendered, sitting next to a dead link. **That is not a story about a bad actor; it is a story about
decay**, and decay is the failure mode this project is structurally built against — a registry that regenerates,
a diff that CI checks, a `Verify:` command whose exit code is the acceptance criterion.

**What this candidate contributes to the AnxietyWatch2 decision.**

1. **A structural lesson about claims.** The registry outranks the paper. A third-party registration outranks a
   vendor's own noun. Any future claim this project makes about CNS sensitivity, alert accuracy or false alarms
   should be accompanied by the sample, the setting and the algorithm version — and this project has no
   mechanism yet for the third of those, which Oura publishes per record and MindShift cannot publish at all.
2. **A safety gap, recorded and not remediated here:** a free CBT app offers three named crisis lines at the
   point of use and AnxietyWatch2 contains no crisis text anywhere in its tree.
3. **A boundary to keep:** telemetry should fail silently and alarms must not. MindShift's `.catch(()=>{})` is
   correct where it is and would be fatal in the alarm path.
4. **A caution about web surfaces.** A local-first product that later ships a marketing site with analytics and
   no notice reproduces this candidate's defect exactly. The finding here is disclosure, not collection.
5. **A positive to match, not to exceed.** MindShift tells its user what it cannot do. So does v2's
   admin-console spec. Doing it once, in a spec, is not the same as doing it at the point of use, in the user's
   language, where the crisis lines are — and that is the specific thing this candidate does better.

**Nothing in this file is a medical claim about either product, a finding of unlawful conduct by the vendor, or
an observation of MindShift running.** The candidate's Phase 2A outcome is *performed on a published client and
public records*, with **zero** runtime observations, and that limit governs everything above.

---

## Related files

- `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` — public-claims dossier; claim ledger **M-1–M-58**; artifact
  SHA-256 table; evidence gaps **G-1–G-12**; correction of the shortlist's operator attribution (§14.4).
- `mindshift_IMPLEMENTATION_INSPECTION.md` — the privacy-URL verification chain (§3.3), static architecture
  findings (§4–§5), **56-row** claim reconciliation (§7), and the explicit statement of **zero** runtime
  observations (§6).
- `PROMPT_2.md` — the candidate-specific second-stage brief (base `PROMPT2-EVIDENCE-FIRST.md` + supplementary
  `PROMPT2-PRIVACY-AND-OPERATING-MODE.md`), with questions **M1–M15**.
- `../PROCESS_STATE.md` — pipeline state and residual uncertainty.
- `../RESEARCH_METHOD.md` — evidence classes, inspection modes, and the standing sub-checks added by this pass.
- `../CANDIDATE_SHORTLIST.md` — shortlist entry #10; **corrected in place 2026-10-06** (operator and
  commercial model).
- `../../specs/AMENDMENTS.md` — amendments 01 (spool/TimescaleDB/tombstones), 02 (provenance/encryption),
  03 (multi-modal alerting), 07 (Rust API/admin), 09–11 (network trust/mobile enrolment).
