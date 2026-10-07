# MindShift CBT — claimed features and purpose

**Slug:** `mindshift`
**Candidate number:** 10 of 10 (`CANDIDATE_SHORTLIST.md`)
**Research date:** 2026-10-06
**Operator:** **247 Labs Inc** (Toronto, Canada) — see §3 and §14.4
**Surface of record:** `https://www.mindshiftcbt.com/` (React SPA served by Replit), the iOS App Store
listing, the Google Play listing, the ClinicalTrials.gov registration `NCT04562948`, and the peer-reviewed
publication that registration points to.

**Standing caveats for this file.**

- Everything here is a **vendor claim**, a **storefront or registry record**, or a **static reading of the
  vendor's published web client**. There are **no runtime observations** for this candidate: the app binary
  was not inspected, installed or run, and no traffic was captured. See
  `mindshift_IMPLEMENTATION_INSPECTION.md` §1 and §6.
- **Absence of evidence is not evidence of absence.** Where a surface could not be retrieved, this file says
  so and records `[UNKNOWN]`; it does not treat a 404, a JS shell or a failed fetch as proof that something
  does not exist. The single exception — a URL that returns a genuine "Page not found" document — is stated
  as what it is (§9.1) and no wider conclusion is drawn from it.
- **A documented contract is not an observed behaviour.** Nothing in this file is labelled *verified*,
  *working*, or *observed*. Store-panel declarations are *developer-supplied* and, in Apple's own words,
  "not been verified by Apple".
- **The vendor's own most conservative statement governs.** Where the site, the storefronts, the registry and
  the publication disagree, both statements are quoted and left unresolved.

---

## 1. Executive summary

MindShift CBT is a free, long-lived (released 2013-06-06) cognitive-behavioural self-help app for anxiety,
now operated by a for-profit Toronto software consultancy, **247 Labs Inc**. It is the most clinically
documented candidate in this package: it is the only candidate with a **peer-reviewed publication of a
16-week study of the app itself**, and that publication is a genuinely useful, unusually candid document.

Six findings dominate.

**Finding 1 — the operator is not the organisation the shortlist named, and the app is not non-commercial.**
The shortlist entry (§10 of that file) originally described this candidate as a non-profit's app and as
non-commercial. The App
Store seller is **247 Labs Inc** (`artistId` 1389070030), the site's own About surface says MindShift "is
designed, built, and cared for by the team at 247 Labs — the Toronto software firm that has developed the app
from the start", and the site runs a **B2B surface** (`/institutions`, `/for-clinics`) whose public client
contains **Clerk organizations and Stripe billing**. Anxiety Canada appears only in the **past tense**:
"**Originally developed in partnership with Anxiety Canada**". The consumer app is free; the operator is not
a non-profit. Both halves are recorded in §3 and §5.

**Finding 2 — the storefront-declared privacy policy URL returns "Page not found".** Google Play declares
the app's privacy policy at `https://mindshiftcbt.com/privacy/`; the App Store "Privacy Policy" link is the
**same URL**. On 2026-10-06 both return **HTTP 404** with a genuine "Page not found | MindShift CBT"
document (§3.3 of the inspection file walks the chain). The vendor's public client contains **zero**
occurrences of the strings `Privacy Policy`, `privacy policy`, `Terms of Use`, `Terms of Service` or
`Terms`; its sitemap lists nine URLs and its router thirteen routes, none of them privacy or terms; and eight
probed document paths all 404. Against this, the site says "**privacy-forward by design**", "**Privacy
respected**", names a "**Privacy & data**" FAQ category, and describes its FAQ as covering "**how it protects
your privacy**". The vendor's **own release notes** (v3.0.1, 2021-12-17 and v3.0.3, 2022-08-11) state that
"**The Terms of Use and Privacy Policy have also been updated**" — so the documents existed in some form at
those dates. **Whether they still exist inside the app is `[UNKNOWN]`; this inspection did not read the
binary.** §9 is the full record.

**Finding 3 — the study the marketing rests on is registered as observational, not as a trial, and its
registered primary outcome is one, not four.** The site's strongest evidence sentence is "A 16-week
**open-label clinical trial** of 380 adults showed significant improvement across **all four primary
outcomes**." The registration `NCT04562948` — which the vendor itself cites — records `studyType:
OBSERVATIONAL`, `observationalModel: ECOLOGIC_OR_COMMUNITY`, **no `armsInterventionsModule` at all**, one
primary outcome ("Linear and non-linear change in generalized anxiety severity over 16 weeks") and three
secondary outcomes. The publication's own title and keywords say "open-label ... trial", and its paper
reports "four primary outcomes". **The registry and the paper disagree with each other about the design class
and about which outcomes are primary.** Both are quoted verbatim in §11. This file does not adjudicate.

**Finding 4 — the same publication states that improvement was unrelated to app use, and the marketing omits
it.** The paper's abstract: "**Improvements were unrelated to the frequency with which participants used the
MindShift app.**" Its limitations section: "**the lack of a comparison group who abstained from using the
MindShift app limits the causal inferences one can draw about the contribution of the MindShift app to
participants' improved well-being over time. For example, participants who reported increased app use did not
demonstrate greater improvements in well-being.**" The paper also reports that improvement "did not differ as
a function of whether participants engaged in psychotherapy (ps > 0.28)" or pharmacotherapy, that effects
were **moderate** (d = 0.61, 0.50, 0.55, 0.31), that the result was "consistent with response to treatment but
**not remission**", and that only **156 of 380** participants ever verified installing the app. The site
renders "**Clinically proven**" as a footer badge, "**Proven, not promised.**" in its meta description, and
"**Most anxiety apps are guesswork.**" §11.4 sets the two surfaces side by side and leaves the tension
standing.

**Finding 5 — an Anxiety Canada relationship network is disclosed in the source and not on the marketing
surface.** The publication's competing-interest statement reads: "**Lance M. Rappaport reports financial
support was provided by Anxiety Canada**", and "**Lance M. Rappaport, Michael Van Ameringen, Maureen Whittal,
and Carmen P. McLean volunteer on the Scientific Advisory Committee of Anxiety Canada.**" The
acknowledgements read: "**This work was supported by a grant from Anxiety Canada to LMR.**" Anxiety Canada is
the app's original co-developer. The site's four named clinical contributors all hold or held Anxiety Canada
roles. **The disclosure exists in the peer-reviewed record — which is to the authors' credit — and is absent
from the marketing surface that cites the study.** §11.5 is the record. This is a
transparency-placement observation, not an allegation: see §14.3.

**Finding 6 — the site runs its own visitor analytics and publishes no notice for it.** The 741,896-byte
published client contains a first-party telemetry pipeline: `POST /api/analytics/session` and
`POST /api/analytics/events`, both with `credentials:"include"`, keyed by a UUIDv4 held in `sessionStorage`
(`mindshift.analytics.signed-in`), carrying exactly four event names — `page_view`, `client_error`,
`resource_opened`, `features_explored` — and truncated page paths, with failures swallowed by
`.catch(()=>{})`. Identity traffic is proxied through the site's own origin (`/api/__clerk`). The same
bundle contains **no cookie notice, no consent banner and no analytics disclosure** (`consent` → 0;
the six `cookie` hits are all inside the bundled Sentry SDK; the 16 `Consent` hits are Clerk's
`OAuthConsent` component) — and there is no privacy page on which one could live (§9.1). **This is a
static finding from a published client: it says what the site's code is written to do, not that any endpoint
is live or that any data was stored.** §9.4 is the record.

**Two things this candidate does better than most in the package.** The boundary language is unusually
disciplined ("not a medical device, and not a replacement for therapy or crisis care"; "There's no clinician
portal — each person owns and controls their export"), and the app pipes **988** and **Kids Help Phone**
through a one-tap crisis affordance rather than pretending the product covers crisis. §12.6 records both as
positives.

---

## 2. Positioning

The positioning changed with the operator. Historically MindShift was Anxiety Canada's public-facing
self-help app. The current site positions it as an **evidence-backed, clinician-recommendable, permanently
free CBT tool**, and — separately — as a **B2B/organizational product** aimed at clinics, campuses,
workplaces and people on waiting lists.

| Surface | Positioning sentence (verbatim) |
|---|---|
| Site meta description | "Navigate your anxiety with real CBT tools. **Proven, not promised.** MindShift gives you science-backed strategies to shift your anxious thinking, settle your body, and approach your fears one step at a time. Developed in Canada by experts in the field." |
| Site hero claim | "**Most anxiety apps are guesswork.**" / "Real tools, not guesswork" |
| Site comparison block | "**Most mental-health apps have no published evidence. MindShift is different.** A 16-week open-label clinical trial of 380 adults showed significant improvement across all four primary outcomes." |
| Site footer badge | "**Clinically proven**" · "**2M+ downloads**" · "**Made in Canada**" |
| App Store description | "MindShift CBT is a free mental health app **developed by experts** to provide you with **proven tools** for anxiety relief." |
| Google Play description | "Take control of anxiety, stress, and panic with MindShift CBT. Get **free, science-backed tools** based on Cognitive Behavioral Therapy (CBT)…" |
| `/for-clinics` | "Recommend a free, **peer-reviewed** CBT app. Patients and clients can export their check-in and Thought Record data to share in session. **Printable waiting-room poster included.**" |
| `/institutions` | "Extend **free, evidence-based** anxiety support to communities, campuses, workplaces, and people waiting for mental health care across Canada and beyond." |

The positioning is a **category claim** — that MindShift is the anxiety app with published evidence — and — unlike several
candidates in this package — it is a claim the vendor can largely point at: there *is* a published study of
this app, in a *Journal of Mood and Anxiety Disorders* paper with a PMID. The open questions are not whether
evidence exists but **what class of evidence it is** (§11) and **what the vendor's own storefront compliance
surfaces say** (§9).

---

## 3. Governance, corporate and legal

### 3.1 Operator

| Fact | Value | Source |
|---|---|---|
| iOS seller | **247 Labs Inc** | App Store listing; `api/itunes_com.bstro.mindshift.json` |
| Google Play developer | **247 Labs Inc** | Play listing |
| Bundle identifier (iOS) | `com.bstro.mindshift` | iTunes Lookup |
| Android package | `com.bstro.MindShift` (**case-sensitive**; the vendor's own download link) | site client |
| Legacy bundle identifiers | `com.anxietybc.mindshift`, `ca.anxietycanada.mindshift` → **resultCount 0** | iTunes Lookup |
| Corporate site | `https://247labs.com` | site footer / team block |
| Company description (vendor's own) | "the Toronto software firm that has developed the app from the start" | site About |
| Copyright line | "Copyright © 247 Labs Inc." | App Store page |

Note on the Android package: a naive lookup of `com.bstro.mindshift` (lower-case) returns **404** on Google
Play. The correct, case-sensitive identifier appears in the vendor's own download link inside the published
client: `https://play.google.com/store/apps/details?id=com.bstro.MindShift&hl=en_CA`. Recorded because the
package's convention is that a failed retrieval is not an absence, and because the near-miss is a trap a
later reader could repeat.

### 3.2 Anxiety Canada's role — the vendor's own wording, in the past tense

Three vendor surfaces, all past-tense:

- Site footer/about: "**Originally developed in partnership with Anxiety Canada.** MindShift CBT is a
  self-help and self-management tool — not a medical device, and not a replacement for therapy or crisis
  care."
- Site About meta description: "Learn about MindShift CBT, **originally developed with Anxiety Canada** and
  guided by clinical experts in anxiety and CBT, serving millions worldwide."
- Publication (2023): "the freely available MindShift smartphone application (**Anxiety Canada**)" and the
  study objective naming "the MindShift app (Anxiety Canada)".

**Anxiety Canada's own current statements could not be retrieved.** Every path fetched from
`https://www.anxietycanada.com/` — `/`, `/mindshift/`, `/resources/mindshift/`, `/privacy-policy/`,
`/terms-of-use/`, `/robots.txt`, `/sitemap.xml`, `/sitemap_index.xml` — returned a **byte-identical
9,321-byte HTML shell** (`bacd434ff31aed1e42b609a2233d84b3b0d16a0a7b2b42f04b417acef179a8d6`, `<title>Anxiety
Canada</title>`, client-rendered). The host serves the same document for every path, so **curl cannot read
that site's content**. This is a **retrieval limitation, not evidence of absence**: whether Anxiety Canada
still promotes, hosts a policy for, or claims any current role in MindShift is **`[UNKNOWN]`** here.

### 3.3 The relationship network, as disclosed in the peer-reviewed record

Recorded as the source states it, in §11.5. In summary: the trial's funder is Anxiety Canada; four of its
five authors volunteer on Anxiety Canada's Scientific Advisory Committee; Anxiety Canada is the app's
original development partner; and all four of the site's named clinical contributors hold or held Anxiety
Canada roles. The source **does** carry these disclosures, including one footnote stating that "**A member of
the Editorial Board is an author of this article**" with the journal's standard independent-peer-review
assurance. Recorded as a placement fact.

---

## 4. Availability, platforms and hardware

| Item | Value | Source |
|---|---|---|
| iOS | v**4.1.0**, released **2026-09-19**; minimum iOS **15.1**; 88 MB; 4+ (lookup) / age rating 9+ (page) | iTunes Lookup + App Store page |
| Android | Google Play listing; updated **Sep 18, 2026**; "**500K+ Downloads**" | Play listing |
| First release | **2013-06-06** | iTunes Lookup `releaseDate` |
| iPad / Mac / Vision | iPadOS 15.1+, macOS 12.0+ (Apple silicon), visionOS 1.0+ | App Store page |
| Languages (store) | English, French, **Spanish** | App Store page; `languageCodesISO2A` = `[EN, FR, ES]` |
| Languages (vendor FAQ) | "MindShift is currently available in **English and French**." | site FAQ |
| Hardware | none — phone/tablet only; no wearable, no sensor pairing | site, store listings |
| Community | in-app "Community" discussion board | site; release notes |
| Account | **optional**; "You can use the core tools right away. Creating an account lets you back up your data and sync it across devices." | site FAQ |

**Language disagreement (§14.1 M-41).** The storefront lists Spanish; the vendor's own FAQ says English and
French. A **v2.0.9** release note reads: "Thank you for all your patience and support! We have improved our
app's performance and made it bilingual (French and English). Switch the language of your device to French
if you want to use MindShift in French." Which surface reflects
the shipped UI is **unresolved**; both are quoted.

**Accessibility disagreement (§14.1 M-42).** The site's team surface describes its **UI/UX Designer** role as
"Leads MindShift's interface and user experience with a focus on accessibility — designing to **AODA and
WCAG** standards so the app stays clear, calming, and usable for everyone." The App Store's
developer-supplied Accessibility section reads, verbatim: "**The developer has not yet indicated which
accessibility features this app supports.**" A design standard is not the same object as a declared
accessibility feature list, so this is recorded as **two surfaces that do not corroborate each other**, not
as a contradiction. The release notes (v4.1.0) do claim "Improved accessibility — better screen reader
support and clearer contrast throughout the app".

### 4.1 Ratings — three surfaces, recorded unresolved

The package's rule, fixed during the Oura candidate, is that an **unresolved rating/review-count difference
is not recorded as a discrepancy**. Applied here:

| Surface | Value at 2026-10-06 |
|---|---|
| Vendor site (hero statistics block) | the literal rendered string "**4.8 rating**", drawn beside **five filled star glyphs** |
| App Store (US) | `averageUserRating` **4.32402** from **537** ratings |
| Google Play | **3.3** stars; the header block shows "**2.2K reviews**" and the ratings block shows "**2.08K reviews**" |

All three are recorded; no average is computed and no motive is inferred. One additional, separable
observation: the site's statistic is **unqualified** — it names no storefront, no storefront region and no
as-of date — so it **cannot be reproduced from either storefront at the research date**. That
qualifier-absence is itself the recordable fact (see the standing rule on a decisive statistic whose
qualifier is unresolvable).

---

## 5. Pricing

| Surface | Statement |
|---|---|
| App Store | `formattedPrice` = **Free**; no in-app-purchase section in the rendered page |
| Google Play | description: "**Always Free: No subscriptions, no hidden costs.** Just effective, accessible support." |
| Site FAQ | "MindShift is **free to download** on the App Store and Google Play." |
| `/for-clinics` | "Download the waiting-room poster above, or contact `support@mindshiftcbt.com` for **other formats and bulk materials**." |
| `/institutions` | "Part of MindShift's next-generation rebuild. **Talk to us about early access for your organization.**" |

**Two facts recorded together, without editorialising.**

1. The consumer app's price is **free**, consistently, across both storefronts and the site. There is no
   subscription and no advertised in-app purchase. (The package does not treat the *absence* of an IAP
   section in a rendered store page as proof that none exists; the vendor's own "no subscriptions, no hidden
   costs" statement is the governing claim, and it is the vendor's.)
2. The **published web client contains billing infrastructure for a separate, unpriced organizational
   product**: Clerk's organization and billing modules (`clerk.com` appears 14 times, 4 of them
   `dashboard.clerk.com`) and Stripe's payment element (`PaymentElementContext`, `StripeUtilsContext`,
   `__internal_loadStripeJs`). The organizational surface is explicitly **"early access"** and is
   **`[UNKNOWN]` as to price** — no price is published anywhere retrieved.

So: **the "free" claim applies to the consumer app and is not contradicted by anything retrieved; the
operator is a for-profit firm with an unpriced B2B offering.** The shortlist's phrase "Free, non-commercial"
is **half right** — free, not non-commercial (see §14.4).

---

## 6. Measured signals

MindShift is a **self-report CBT tool**. There is no sensor, no wearable integration, no passive measurement
and no physiological signal of any kind. What it stores is what the user types.

From `/for-clinics` (the most explicit data inventory on any surface):

> "**Daily Check-ins and anxiety tracking, Thought Records, Facing Fears Ladders, Behavioural Experiments,
> Comfort Zone Challenges, and Goal Setting.**"

and the export description:

> "**A structured CSV of check-ins and the data collected across guided tools**, ready for review or
> sharing."

From the site's tool list, the full named feature set is: Daily anxiety check-in; Thought Records
(catch/challenge/reframe); Facing Fears Ladders (graded exposure); Relaxation and Mindfulness audio
(2–6 minutes, "Meditation and mindfulness audio", "Relaxation and sleep support"); Quick Tips;
Behavioural Experiments; Comfort Zone Challenges; Goal Setting; Progress tracking ("Visualize your mental
health journey with daily check-ins and insights"); Community.

**Not present on any surface retrieved:** heart rate, HRV, SpO₂, respiratory rate, sleep staging, CPAP data,
temperature, glucose, blood pressure, activity, or any derived physiological score. This candidate has **no
overlap with V2's measured-signal set**; the overlap is entirely at the level of self-report instruments and
the derived-trend layer.

---

## 7. Derived measures and analysis

MindShift's analysis surface is deliberately thin, and the vendor makes that a selling point:

- "**Progress Tracking**: Visualize your mental health journey with daily check-ins and insights."
- "Track progress and what's working".
- `/for-clinics`, under "Your data, your clinician": "Export your thought records and progress to share with
  your own therapist or doctor. **No algorithm sitting in the middle.**" (French: "**Aucun algorithme ne
  s'interpose.**")
- `/for-clinics` "Data patients bring in": "**Anxiety trajectory between sessions**", "Exposure adherence and
  outcomes", "Prediction testing and learning review", "Confidence-building practice and progress",
  "Symptom trajectory between sessions".
- "See actual check-in patterns and thought records — not a retrospective 'how was your week?'"

There is **no published scoring algorithm, no published index, no published threshold, and no published
alert logic** anywhere on the surfaces retrieved. "Insights" is named but never specified. The only
statistical claim made about the app's own analysis is negative: the vendor says there is no algorithm
between the user and the clinician, which is a **data-path claim**, not a feature claim, and is recorded as
such.

**This is the clearest structural contrast with V2 in the whole package.** V2's difficulty is that it must
never let a derived value masquerade as a measurement; MindShift has almost no derived values to
misrepresent, and its product risk sits instead in **claim discipline** (§11) and **document hygiene** (§9).

---

## 8. Published interfaces

| Interface | Status | Evidence |
|---|---|---|
| **CSV export of the user's own data** | Claimed | "A structured CSV of check-ins and the data collected across guided tools"; "Export your thought records and progress…" |
| **Clinician portal** | **Explicitly absent** | "There's **no clinician portal** — each person owns and controls their export." |
| Export initiation | Patient-controlled | "**Export is initiated and controlled by the patient or client — privacy-forward by design.**" |
| Public API | **None found** | no API documentation, no developer surface, no third-party data API on any retrieved page |
| Webhooks / integrations | **None found** | — |
| Third-party data import | **None found** | — |
| Printable artefacts | Claimed | "The same tools from the app, as clean printable PDFs — **free to download and use**"; waiting-room poster (Letter or A4, EN or FR) |
| Support contact | Published | `support@mindshiftcbt.com` |
| **Site analytics (website only)** | **Present in the published client** | `POST /api/analytics/session` and `POST /api/analytics/events`; four event names; see §9.4 |
| **Site CMS / admin API (website only)** | **Present in the published client** | `POST /api/admin/login`, `POST /api/admin/logout`, `GET /api/admin/session`, `GET /api/admin/analytics`, `POST`/`PUT`/`DELETE /api/admin/resources/…` |
| **Waitlist + identity** | **Present in the published client** | Clerk is configured with a `waitlist` root (`/waitlist`) and its frontend API is proxied first-party at `/api/__clerk` |

**The no-clinician-portal, patient-controlled-export boundary is a positive comparator** for V2 and is
recorded as one in §12.6. It is a **public claim**, not an observed behaviour: no export was performed and no
CSV was examined.

---

## 9. Privacy

This is the candidate's second centre of gravity, and the record is unusually lopsided: **a lot of privacy
language, and no privacy document.**

### 9.1 The finding — the declared privacy policy URL returns a genuine "Page not found"

**What the storefronts say.**

- **Google Play** declares the app's privacy policy as **`https://mindshiftcbt.com/privacy/`**. (Retrieved
  from the rendered Play listing; the string appears once in the page's app metadata.)
- **The App Store** page's "Privacy Policy" link under Information is **the same URL**:
  `https://mindshiftcbt.com/privacy/`.

**What that URL returns.** Retrieved 2026-10-06:

| Path | HTTP | Bytes | Document |
|---|---|---|---|
| `/privacy` | **404** | 3,362 | `<title>Page not found \| MindShift CBT</title>` |
| `/privacy/` | **404** | 3,362 | same |
| `/terms` | **404** | 3,358 | same |
| `/terms/` | **404** | 3,358 | same |
| `/privacy-policy` | **404** | 3,376 | same |
| `/privacy-policy/` | **404** | 3,376 | same |
| `/legal` | **404** | 3,358 | same |
| `/legal/` | **404** | 3,358 | same |

**These are genuine "Page not found" documents, not JS error shells and not WAF challenges.** That
distinction matters and was fixed as a package rule during the Oura candidate (where a Next.js error shell
was *not* a 404 and *not* evidence of absence). Here the server returns a real 404 status with a real
Not-Found document. The served HTML carries `<title>Page not found | MindShift CBT</title>` and the visible
body reads, in full: "Page not found" / "The requested MindShift CBT page could not be found." The
published client separately contains a second, localized 404 component for the SPA's own routing — "The
page you're looking for moved or never existed. Let's get you back to something helpful." (with the French
"La page que vous cherchez a été déplacée ou n'a jamais existé…") — which is a *client* string, not part of
the served document. Re-fetching `/privacy` later the same day returned a **byte-identical** 3,362-byte
document (SHA-256 `4a9ba979…`, unchanged), so the finding is stable, not a transient. For **this URL**,
"there is no document here" is a verified fact.

**What the rest of the site says.**

| Check | Result |
|---|---|
| `sitemap.xml` (9 `<loc>` entries) | `/`, `/methodology`, `/app`, `/institutions`, `/for-clinics`, `/about`, `/resources`, `/faq`, `/contact` — **no privacy, no terms** |
| Router path table in the published client | 13 paths: the nine above plus `/resources/:slug`, `/sign-in/*?`, `/sign-up/*?`, `/admin/*?` — **no privacy, no terms** |
| `robots.txt` | disallows `/admin`, `/sign-in`, `/sign-up`, `/api/` — a real application surface exists behind those |
| String sweep of the 741,896-byte published bundle | `Privacy Policy` → **0**; `privacy policy` → **0**; `Terms of Use` → **0**; `Terms of Service` → **0**; `terms of use` → **0**; `Terms` → **0** |
| The four surviving `Privacy` strings | "**Privacy respected**" (card title) with the body "Your data is yours — securely stored, never sold, and exportable to your own clinician."; the FAQ category heading "**Privacy & data**"; the FAQ meta "…and **how it protects your privacy**."; the feature-card title "Privacy respected" |

**What the vendor's own release notes say.** The App Store version history contains, for **v3.0.1
(2021-12-17)** and again for **v3.0.3 (2022-08-11)**: "We have integrated an online forum: MindShift CBT
Community… **The Terms of Use and Privacy Policy have also been updated to reflect this new feature.**"

**What is verified, and what is not.**

- **Verified:** both storefronts point at a URL that currently returns a genuine Not-Found page; no privacy or
  terms document is reachable from the public site by any route, link, sitemap entry or router path; and the
  documents were referred to as existing by the vendor's own notes in 2021–2022.
- **Not verified, recorded `[UNKNOWN]`:** whether a privacy policy and terms of use still exist **inside the
  app binary**. The vendor's own release notes suggest documents existed; this inspection read the published
  web client and the public records, and **did not inspect the app**. It is entirely possible the documents
  live behind a settings screen. That possibility is the reason this is recorded as a **public-surface and
  storefront-linkage finding**, not as "the app has no privacy policy".

### 9.2 What the store panels declare

**Google Play "Data safety" (developer-supplied), verbatim:**

> "No data shared with third parties"
> "This app may collect these data types **Personal info and Health and fitness**"
> "**Data is encrypted in transit**"
> "You can request that data be deleted"

**App Store "App Privacy" (developer-supplied; Apple's own label: "This information has not been verified by
Apple"), verbatim structure:**

| Class | Category | Purpose |
|---|---|---|
| **Data Linked to You** | Contact Info → **Name** | App Functionality |
| **Data Not Linked to You** | Analytics: User Content → Other User Content | Analytics |
| **Data Not Linked to You** | Analytics: Usage Data → Product Interaction | Analytics |
| **Data Not Linked to You** | App Functionality: **Health & Fitness → Health** | App Functionality |
| **Data Not Linked to You** | App Functionality: Contact Info → **Email Address** | App Functionality |
| **Data Not Linked to You** | App Functionality: Identifiers → **User ID** | App Functionality |

with the collapsed summary repeating: "Data Linked to You" → "**Contact Info**" / "Data Not Linked to You" →
**Health & Fitness, Contact Info, User Content, Identifiers, Usage Data**".

**Three observations, recorded as observations.**

1. **Health data is declared *not* linked to the user's identity**, while an account exists and the vendor's
   FAQ says an account "lets you back up your data and sync it across devices", and the same panel declares
   a **User ID**. Whether health data is in fact unlinkable from that User ID is **not verifiable from
   outside** and is not asserted either way.
2. **`Contact Info` appears in both linkage classes.** The detailed view puts *Name* under "Linked to You"
   and *Email Address* under "Not Linked to You"; the collapsed summary lists *Contact Info* under both
   headings. The panel is **internally mixed** about the same data category. Recorded verbatim; the
   resolution is left to the reader and marked unresolved (§14.5 G-4).
3. **"No data shared with third parties" vs a client that ships third-party processors.** The published
   client loads **Sentry** (error telemetry; 129 occurrences of `sentry`, case-insensitive), **Clerk**
   (authentication/organization), and every served document's HTML shell — including the 404 pages — loads a
   **Replit** script (`i.replit.com/script.js`; **0 occurrences in the app bundle**). Under store definitions a *service provider* processing data on the
   developer's behalf is generally not "sharing", so this is **not a contradiction** — it is the same
   *interpretation hazard* recorded for Oura's identical panel, where a platform self-declaration and a
   vendor's own stack appear to disagree until the definitions are read. Recorded as a hazard, not a
   discrepancy, and the definitions are not resolved here.

### 9.3 What the site says about privacy

| Claim (verbatim) | Where |
|---|---|
| "**Privacy respected**" — card body: "Your data is yours — securely stored, never sold, and exportable to your own clinician." | feature card |
| "**Privacy & data**" (FAQ category) | FAQ |
| "**Everything you might want to know about MindShift CBT, how it works, and how it protects your privacy.**" | FAQ meta description |
| "Yes. Your Thought Records, check-ins, and personal data are **stored securely, and we don't sell your personal data to third parties**." | FAQ |
| "If you have an account, you can **request deletion from the app settings, which permanently removes your data from our servers**." | FAQ |
| "**Export is initiated and controlled by the patient or client — privacy-forward by design.**" | `/for-clinics` |
| "**Anonymous and judgment-free**" (Community) | feature block |

**"Stored securely" is undefined** — no standard, no encryption claim at rest, no retention period, no
sub-processor list, no jurisdiction, no controller identity, no contact for a privacy request. The Play
panel's "**Data is encrypted in transit**" is the only concrete security statement retrieved, and it says
nothing about data at rest. **An undefined security claim beside a missing privacy document is the
combination this file records**; it is not upgraded into a finding that the data is insecure.

### 9.4 The website runs its own analytics, and publishes no notice for it

The published client is not a static brochure. Read from the 741,896-byte bundle at
`web/index-BogFfh2L.js`:

| Element | What the client contains |
|---|---|
| Session identity | `function Nh()` reads/validates a UUIDv4 in `sessionStorage` under key `s0`, minting one with `crypto.randomUUID()` when absent |
| Session registration | `fetch("/api/analytics/session", {method:"POST", credentials:"include", keepalive:true, body:{sessionId}})` — gated by a second `sessionStorage` key, `mindshift.analytics.signed-in`, which stores `<state>:<sessionId>` so the call is skipped for a repeat state |
| Event transport | `fetch("/api/analytics/events", {method:"POST", credentials:"include", keepalive:true})` with a payload of `{category, name, path, sessionId}`, `path` truncated to 240 characters |
| Event taxonomy | exactly four call sites: `{category:"page_view", name:"page_view"}`, `{category:"client_error", name:<error>}`, `{category:"feature_use", name:"resource_opened"}`, `{category:"feature_use", name:"features_explored"}` |
| Failure handling | both calls end in `.catch(()=>{})` — the site's own telemetry fails silently and cannot surface an error to the user |
| Identity provider | Clerk's frontend API is proxied through the site's **own origin** at `/api/__clerk` (`VITE_CLERK_PROXY_URL`), so identity traffic appears first-party in the browser |
| Admin surface | `/api/admin/login`, `/api/admin/logout`, `/api/admin/session`, `/api/admin/analytics`, and resource CRUD at `/api/admin/resources/…` (`POST`/`PUT`/`DELETE`); `robots.txt` disallows `/admin`, `/sign-in`, `/sign-up` and **`/api/`** |

**Why this is recorded as a finding rather than a footnote.** The site's own copy says "**privacy-forward by
design**", and its feature card titled "**Privacy respected**" reads "Your data is yours — securely stored,
never sold, and exportable to your own clinician.". The same site measures its visitors — page
views, which resource pages were opened, which feature panels were explored, and client-side errors — with a
per-session identifier, posting to its own origin with credentials included. There is **no cookie notice, no
consent banner and no analytics disclosure anywhere in the published client**: a fixed-string sweep returns
`consent` → 0, `cookie` → 6, and all six are inside the bundled **Sentry** SDK's own data-collection
configuration, while all 16 `Consent` occurrences belong to Clerk's `OAuthConsent` component. And there is
no privacy page to disclose it on (§9.1).

**Stated exactly, and no further.** This is a **static** finding from a published client: it establishes
what the site's code is *written to do*, not that any endpoint is live, that any event was stored, or that
any data was shared. `credentials:"include"` plus a first-party Clerk proxy means the browser treats this
traffic as first-party, which is precisely why a notice would be expected — that is an observation about the
absence of a notice, **not** an allegation about what the vendor does with the data. It also does not touch
the *app*, which is measured separately in §9.2 and was not inspected.

### 9.5 What the site does not say

Recorded as an **absence of published material**, with the standing caveat that non-publication is not the
same as non-existence:

- no privacy policy (§9.1);
- no terms of use (§9.1);
- no **cookie or analytics notice**, for a site that ships its own page-view and feature-use telemetry (§9.4);
- no **community guidelines**, for a board the app's own release notes say has "guidelines" and a surface
  that declares "**Actively moderated for safety**";
- no data-retention period;
- no sub-processor list;
- no data controller or DPO identity;
- no age-verification or parental-consent statement, for an app the site says is for "**Adolescents (13+)**";
- no security disclosure or vulnerability-reporting policy;
- no published account-deletion SLA ("you can **request** deletion").

---

## 10. Security and reliability surfaces

| Topic | Record |
|---|---|
| Transport | Play: "Data is encrypted in transit". No further detail published. |
| At rest | **No published statement.** The site's "stored securely" is undefined. |
| Authentication | **Clerk** in the published client (`clerk.com`, `dashboard.clerk.com`); the app's own auth internals were not inspected. |
| Payments | **Stripe** via Clerk billing, in the published client (organizational product). |
| Error telemetry | **Sentry** — a third-party ingest DSN is embedded in the public client (not reproduced here). |
| Hosting | **Replit** — `i.replit.com/script.js` is in the served HTML shell of every document (including the 404 pages) and **0 times in the JS bundle**; the marketing site is a Replit-served SPA. |
| Failure states in the client | the published bundle ships user-visible failure copy: "**We couldn't load the resources right now. Please try again shortly.**" and "**Please refresh the page and try again.**" — i.e. a not-loaded/retry state is modelled in the UI. **Static reading only; a string is not a behaviour.** |
| Content provenance affordance | content pages render "**Last reviewed and updated**" with a `<time datetime=…>` element. **A per-page review-date affordance exists.** The date values themselves are supplied from data not present in the fetched shell → `[UNKNOWN]`. |
| Incident/vulnerability disclosure | **none found** |
| Status page | **none found** |
| Uptime or reliability claim | **none found** |

---

## 11. Clinical, regulatory and validation posture

**MindShift is not a regulated medical device and the vendor says so**, on the site and in the store
listings: "**not a medical device, and not a replacement for therapy or crisis care**". No FDA clearance, no
CE mark, no product-code, no notified body, no clinical-investigation record was sought or found. **The
entire clinical posture of this candidate is one 16-week study, its registration, and its corrigendum.**

### 11.1 The registration — `NCT04562948`, the independent registry artifact

Retrieved from the ClinicalTrials.gov v2 API on 2026-10-06 (20,342 bytes; SHA-256 in §14.2). Verbatim values:

| Field | Value |
|---|---|
| briefTitle / officialTitle | "Community Applications of the MindShift App" |
| orgStudyId | "MindShift" |
| Organization / lead sponsor | **University of Windsor** (`class: OTHER`) |
| Collaborator | **Anxiety Canada** |
| Responsible party / PI | **Lance M. Rappaport**, Assistant Professor, University of Windsor |
| Location | University of Windsor, Windsor, Ontario, Canada |
| Status | **COMPLETED** |
| Study first posted | 2020-09-24 |
| Start | 2020-11-11 |
| Primary completion / completion | **2022-04-12** |
| **Last update posted** | **2022-11-02** |
| **`studyType`** | **`OBSERVATIONAL`** |
| `designInfo` | `observationalModel: ECOLOGIC_OR_COMMUNITY`; `timePerspective: PROSPECTIVE` — **no `allocation`, no `interventionModel`, no `maskingInfo`** |
| `armsInterventionsModule` | **absent** |
| `interventionBrowseModule` (derived) | **`None`** — the registry indexes no intervention |
| **Primary outcome (1)** | "**Linear and non-linear change in generalized anxiety severity over 16 weeks**" (timeFrame: 16 weeks) |
| **Secondary outcomes (3)** | "…depressive symptom severity…"; "…functional impairment…"; "**…life satisfaction** over 16 weeks" |
| Enrollment | **380 (`ACTUAL`)** |
| `hasResults` / `resultsSection` | **false / absent** |
| `oversightHasDmc` | false |
| `isFdaRegulatedDrug` / `isFdaRegulatedDevice` | false / false |
| Eligibility | Inclusion: resident of Canada or the USA; "Report any distress related to anxiety"; comfortable with English; regular smartphone access; semi-regular internet. **Exclusion Criteria: "None"** |
| `healthyVolunteers` / sex / ages | true / ALL / 18 Years – none |
| IPD sharing | **"NO"** — with a stated rationale: "The research team has planned to make data available to other researchers through request to the study principal investigator. Given funding provided by Anxiety Canada and the focused nature of the present study, this IPD plan was considered a careful way to protect potentially identifiable information…" |
| References | **18 entries, every one typed `BACKGROUND`; none typed `RESULTS`** |

**Three facts about the record itself**, stated separately from any judgement:

1. **The registration is titled "Community Applications of the MindShift App" — the word "trial" does not
   appear in it**; it appears in the *publication's* title and keywords.
2. **The registration was last updated 2022-11-02.** The publication (2023) and the corrigendum (2024) are
   not linked from it. A reader who follows the NCT number printed on the vendor's site lands on a record
   that has never been updated with the study's results.
3. **The design has no comparator and no intervention.** The registration's own brief summary states the
   study "will examine the utility" of the app, and the publication's comparison is "as compared to anxiety
   and distress prior to app use" — a **within-person pre-post comparison**.

### 11.2 The publication

**"North American open-label 16-week trial of the MindShift smartphone app for adult anxiety."**
*Journal of Mood and Anxiety Disorders*, vol. 4 (2023), article 100036.
PMID **40656972**; DOI `10.1016/j.xjmad.2023.100036`; Open Access; PMC12244177. Authors: Rappaport LM,
Jerome E, Van Ameringen M, Whittal M, McLean CP. Europe PMC reports **1 citing article** (recorded as
reported, with the caveat that citation coverage for a 2023 paper in this journal may be incomplete).

Verbatim from the abstract and body — the material a claim check has to engage with:

- Methods: "Adults ages 18–80 (N = 380) participated in an **online open-label trial** to evaluate change in
  anxiety and related distress while using the MindShift smartphone app… Participants reported the severity
  of **four primary outcomes** at baseline and 2-, 4-, 8-, 12-, and 16-weeks…"
- Results: "**All four primary outcomes improved**… **Improvements were unrelated to the frequency with which
  participants used the MindShift app.** Effect sizes indicated **moderate** change in anxiety symptoms
  (d = 0.61, p < 0.0001), depressive symptoms (d = 0.50, p < 0.0001), functional impairment (d = 0.55,
  p < 0.0001), and quality-of-life (d = 0.31, p < 0.0001)…; improvements were **consistent with response to
  treatment but not remission**."
- Conclusions: "the MindShift app **may** provide a ready to scale low-cost resource…"
- Design: "Participants were instructed to enter a study-specific ID code into the MindShift app to verify
  that it was downloaded and set up… the **freely available MindShift smartphone application (Anxiety
  Canada)**… cleared by the Research Ethics Board of the University of Windsor (#20–127)."
- Results, verification subset: "Similar patterns of change in all four primary outcomes were evident when
  the sample was restricted to the **156 participants** for whom entrance of their study identification code
  into the MindShift smartphone app verified that they downloaded the smartphone app."
- Results, moderators: "improvement in all four primary outcomes **did not differ** as a function of whether
  participants engaged in psychotherapy (ps >0.28), pharmacotherapy for any mental health condition
  (ps >0.37), or pharmacotherapy for anxiety (ps >0.26) in the past 2 months nor did primary outcomes differ
  as a function of the **frequency with which participants reported using the app** over the 16-week study
  period (ps >0.17)."
- Discussion: "**While comparison against a sample who abstained from accessing mental health resources was
  deemed incompatible with the ethical principle of equipoise**, the present data indicate that users'
  anxiety and depressive symptom severity, quality-of-life, and functional impairment improved over 16-weeks
  of using the MindShift app **relative to baseline assessment**."
- Limitations, first: "**the lack of a comparison group who abstained from using the MindShift app limits the
  causal inferences one can draw about the contribution of the MindShift app to participants' improved
  well-being over time. For example, participants who reported increased app use did not demonstrate greater
  improvements in well-being.** … Similarly, results may indicate **regression-to-the-mean**."
- Limitations, third: "**White women were overrepresented in the present sample.**"
- Limitations, fourth: "assessment of participants' symptomatology, well-being, and app use **relied on
  self-report**."
- Limitations, fifth: "data for the present study were collected **during the COVID-19 pandemic**."
- Limitations, finally: "**study attrition** is consistent with other research on digital interventions."
- Keywords: "Anxiety, MHealth, Smartphone app, **Open-label trial**, Cognitive behavioral therapy."
- In-text: "This study was **registered** on ClinicalTrials.gov (NCT04562948)." — **the paper says
  "registered", not "pre-registered".**

**This is a candid paper.** It publishes its own central weakness, its own null moderator finding, and its
own sampling limits. Nothing in this file characterises the study as invalid; what it records is that the
**marketing surface cites the study's results and none of its qualifications**, including the one finding
that most directly bears on the product claim.

### 11.3 The corrigendum

**"Corrigendum to 'North American open-label 16-week trial of the MindShift smartphone app for adult
anxiety'"**, *Journal of Mood and Anxiety Disorders* vol. 8 (2024) 100093; PMID **40655914**; DOI
`10.1016/j.xjmad.2024.100093`; PMC12244218. Same five authors. Verbatim:

> "The authors regret an **unfortunate error in the calculated total score for quality-of-life from the
> Q-LES-Q-SF** in the 2023 paper… In response, we corrected the scoring and identified that the **primary
> conclusions are unchanged**… We initially reported the interitem reliability and unidimensionality of all
> **16 items** of the Q-LES-Q-SF. However, the **14 items** traditionally included in the total score also
> demonstrate high interitem reliability (α = 0.90 – 0.94)…
> The overall mean quality-of-life score at baseline **changed from (M = 30.76, SD = 9.77) to (M = 47.83,
> SD = 15.39)** following a correction in the calculation of the total score from the Q-LES-Q-SF."

Recorded as: **one of the four headline outcomes required a published correction a year later**; the
corrigendum states the conclusions are unchanged; and the corrigendum does **not** restate the
quality-of-life effect size, so whether d = 0.31 changed is **`[UNKNOWN]`**. The vendor's marketing surface
does not mention the corrigendum.

### 11.4 What the site says, set beside what the source says

| Site claim (verbatim) | Same-source counter-statement (verbatim) |
|---|---|
| "A 16-week **open-label clinical trial** of 380 adults showed significant improvement across **all four primary outcomes**." | Registry: `studyType: OBSERVATIONAL`; **one** primary outcome, three secondary. Publication: "an online **open-label trial**" with "**four primary outcomes**". |
| "**Clinically proven**" (footer badge) | "the MindShift app **may** provide a ready to scale low-cost resource"; effect sizes **moderate**; "**not remission**"; "**the lack of a comparison group… limits the causal inferences one can draw about the contribution of the MindShift app**". |
| "**Proven, not promised.**" | "**Improvements were unrelated to the frequency with which participants used the MindShift app**"; "participants who reported increased app use did not demonstrate greater improvements in well-being". |
| "**Most anxiety apps are guesswork.**" | The publication's own result set includes a null usage–outcome relationship and possible regression to the mean. |
| "A 16-week clinical trial showed significant improvements in **anxiety, functioning, and quality of life**" | "**four** primary outcomes" elsewhere on the *same site*, and "four primary outcomes" in the *publication*. |
| "MindShift has also been **recommended by clinicians at academic medical centres in Canada and the US**, and **used in research in outpatient and inpatient psychiatric settings**." | No citation is given on the surface. The publication cites "prior research on the MindShift app [35], [36]" but the two claims above are not sourced where they are made. |
| "12–16 weeks" waitlist framing, `/for-clinics` | The study had **no exclusion criteria** and enrolled any adult self-identifying anxiety-related distress; its conclusion scopes the result to "particularly those who report **mild or moderate** symptom severity". |

**The reconciliation is left standing.** The site's evidence sentences are, on their own terms, faithful to
the abstract. The finding is that the surface is **selective**: it quotes the favourable half of a document
whose other half is the reason the document is credible.

### 11.5 Funding, affiliations and disclosure — as the source itself states them

- "**Lance M. Rappaport reports financial support was provided by Anxiety Canada.**"
- "**Lance M. Rappaport, Michael Van Ameringen, Maureen Whittal, and Carmen P. McLean volunteer on the
  Scientific Advisory Committee of Anxiety Canada.**"
- Acknowledgements: "**This work was supported by a grant from Anxiety Canada to LMR.**"
- "A member of the Editorial Board is an author of this article. Editorial Board members are not involved in
  decisions about papers which they have written themselves or have been written by family members or
  colleagues… Any such submission is subject to **all of** the journal's usual procedures, with peer review
  handled independently of the relevant editor and their research groups."
- "Research findings were presented at the annual meeting of the Canadian Psychological Association in
  Toronto, ON, June 24, 2023."
- One author discloses relationships with 13 pharmaceutical companies (AbbVie, Bausch, Biogen, Boehringer
  Ingelheim, Elvium/Purdue, Jazz, Vistagen, Lundbeck, Otsuka, Pfizer, Sunovion, Takeda, Biron).

**The vendor's named clinical contributors, from the site's own team surface:**

| Name (site title) | Site text relevant to the relationship |
|---|---|
| **Kristin Buhr, Ph.D., Psychologist** — "Clinical lead, MindShift CBT" | "served as the **lead content developer for the award-winning MindShift CBT app**. She now acts as **MindShift's Clinical Lead**…"; "has developed numerous self-help resources and programs **for Anxiety Canada**"; Director, North Shore Stress and Anxiety Clinic |
| **Melanie Badali, Ph.D., Psychologist** | "an **award-winning** psychologist…"; "Since 2013, she has championed MindShift CBT as an accessible, evidence-based digital tool…" |
| **Melisa Robichaud, Ph.D., Psychologist** | co-director and founder, Vancouver CBT Centre; "**former President of CACBT**"; "served on multiple boards and advisory committees, **including Anxiety Canada**" |
| **Sarah Newth, Ph.D., Psychologist** | Director, North Shore Stress and Anxiety Clinic; "**past roles** at the Anxiety Disorders Clinic at UBC Hospital in Vancouver and **as the Director of Programs for Anxiety Canada**" |

**The overlap is total: every named clinical contributor has a current or past Anxiety Canada affiliation,
and the trial's funder is Anxiety Canada, which is also the app's original co-developer.** The relationship
is **disclosed in the peer-reviewed record**. It is **not disclosed on the marketing surface** that invokes
the study. Both facts are recorded, in that order, and neither is characterised.

---

## 12. What MindShift does not do — in the places that matter to V2

### 12.1 There is no measured signal of any kind

No sensor, no wearable, no passive or physiological measurement. V2's entire measured-signal layer — Polar
H10 ECG, ResMed Air11 therapy data, SpO₂, respiratory rate, sleep, and the append-only `samples` store —
has **no counterpart** here. A comparison must not treat MindShift's self-report check-ins as a measurement
pipeline; they are user-typed records, and the vendor presents them that way.

### 12.2 There is no real-time path, and nothing that reports overnight

MindShift is an on-demand, interactive app. There is no continuous acquisition, no overnight window, no
background collection, and no state machine that can fail while the user sleeps. **The failure modes that
dominate this package for V2 — a collector that stops, a sensor that goes quiet, a value that is absent
rather than good — do not exist in this product**, because nothing is being acquired in the background to
fail. V2's absent-state discipline has no analogue to be tested against here.

### 12.3 There is no alerting of any kind

No alarm, no notification path for a condition, no escalation, no caregiver contact, no dead-man switch.
Recorded while noting a documented **boundary decision**: the vendor routes crisis to external resources
(`988`, Kids Help Phone) rather than claiming any alerting role — "not itself a crisis service". `[UNKNOWN]`
whether the app sends any local notification at all (the binary was not inspected, and no notification claim
appears on any surface retrieved).

### 12.4 There is no derived index, threshold or score published anywhere

No scoring algorithm, no index, no threshold, no risk band. "Insights" is named and unspecified.

### 12.5 There is no documented data model, no API, and no published contract

A CSV export is claimed; its schema is not published. There is no user-facing API, no webhook, no
integration and no data-model documentation. (The *website* does ship its own private endpoints — analytics
and an admin CMS, §9.4 — but they are undocumented, carry no contract, and expose nothing about the app's
data model.) Consequently **no claim on this candidate can be reconciled against a contract** — the only
reconciliation surfaces available are the registry, the publication, the storefront panels and the published
client.

### 12.6 Two things it does well, recorded as positives

1. **Boundary discipline in prose.** "not a medical device, and not a replacement for therapy or crisis
   care"; "Adolescents (13+) and adults with mild-to-moderate anxiety. **It is not designed for severe
   conditions or crisis intervention — it routes users to crisis resources and is not itself a crisis
   service.**" This is the three-state honesty rule applied to scope, in plain language, unprompted.
2. **A patient-controlled export boundary with the clinician portal explicitly refused.** "There's **no
   clinician portal** — each person owns and controls their export." and "Export is initiated and controlled
   by the patient or client — privacy-forward by design." Whatever else is unresolved, **the vendor names the
   boundary of its own claim** rather than leaving the reader to infer it. This is the same class of positive
   as Home Assistant naming its own paid cloud route.

Two smaller positives: the published client models a **failure state** ("We couldn't load the resources right
now") rather than assuming success, and content pages carry a **"Last reviewed and updated"** affordance.

---

## 13. Relationship to AnxietyWatch v1 and v2

**To v1.** No relationship. v1 is a Swift/SwiftUI iOS app with HealthKit ingestion, a CNS risk pipeline,
correlation engine and OSCAR/AirSense import. MindShift shares none of those surfaces; the only shared noun
is "anxiety". MindShift is nonetheless the **only candidate in this package whose primary evidence artifact
is a study of anxiety self-report outcomes**, so it is the closest thing to a same-domain comparator for
v1's user-facing framing — and its most useful contribution to v1 is the **scope-boundary language** in
§12.6 and the **three-outcome/four-outcome** inconsistency in §14.1 M-9.

**To v2.** Four contacts, none of them architectural:

1. **Claim discipline.** V2 does not make a clinical-efficacy claim and must not start; MindShift shows both
   how a vendor documents a study honestly in the source and how easily the marketing surface sheds the
   qualifications. Read §11.4 as a template for what not to write.
2. **`unknown` is not `good`.** The vendor's crisis boundary ("not itself a crisis service") and its
   mild-to-moderate scope are the self-limiting statements V2 needs in the same places — particularly around
   the supervised alarm and the CNS engine, where the failure mode is a reassuring value that was never
   measured.
3. **Provenance affordances.** The "Last reviewed and updated" element and the patient-controlled export are
   small, cheap provenance features V2 can copy.
4. **The privacy-document gap is a warning, not a comparison.** V2's own privacy posture depends on
   documents existing and being reachable. This candidate is the package's clearest example of a vendor whose
   *privacy language* and whose *privacy documents* came apart: a storefront compliance link pointing at a
   404, a sitemap with no policy, and a client with the strings `Privacy Policy` and `Terms` appearing zero
   times. Whatever else V2 does, it should be able to answer "where is your policy" with a URL that resolves.
   **MindShift's in-app document status is `[UNKNOWN]` and is not being used to claim otherwise.**

**The honest framing:** MindShift is not a competitor to V2 and not a source of design ideas. It is the
package's best worked example of **evidence-claim hygiene** — of how a candid study and a confident marketing
page can be built from the same data.

---

## 14. Evidence posture

### 14.1 Claim ledger

Verdict vocabulary is unchanged from the rest of the package. Classes used here:

- `[PUBLIC CLAIM]` — a statement by the vendor on a vendor-controlled surface.
- `[THIRD PARTY]` — a record produced by someone other than the vendor (storefront panel, ClinicalTrials.gov,
  the journal/PubMed record).
- `[FOUND IN PUBLISHED CLIENT]` — a static reading of the vendor's publicly served web client
  (`index-BogFfh2L.js`, 741,896 bytes). **This is a static finding class, not a runtime observation.**
- `[FOUND IN PUBLISHED DOCUMENT]` — a static reading of a PDF/HTML document served by a third party.

| # | Claim / fact | Surface | Class | Status |
|---|---|---|---|---|
| M-1 | "MindShift is designed, built, and cared for by the team at **247 Labs** — the Toronto software firm that has developed the app from the start." | site | `[PUBLIC CLAIM]` | consistent with storefront seller name |
| M-2 | "**Originally developed in partnership with Anxiety Canada**" | site | `[PUBLIC CLAIM]` | past tense; consistent with registry collaborator and the paper's attribution |
| M-3 | "the **award-winning** MindShift CBT app" | site | `[PUBLIC CLAIM]` | **no award is named anywhere on any surface retrieved** |
| M-4 | App Store: seller **247 Labs Inc**; `com.bstro.mindshift`; v4.1.0; 2026-09-19; 87,966,720 B; minOS 15.1; Free; 4+ | iTunes Lookup | `[THIRD PARTY]` | retrieved 2026-10-06 |
| M-5 | Play: developer **247 Labs Inc**; package `com.bstro.MindShift`; updated Sep 18, 2026; "500K+ Downloads"; 3.3★ | Play listing | `[THIRD PARTY]` | retrieved 2026-10-06 |
| M-6 | `com.anxietybc.mindshift` and `ca.anxietycanada.mindshift` → resultCount 0 | iTunes Lookup | `[THIRD PARTY]` | legacy ids absent from the US store |
| M-7 | "A 16-week **open-label clinical trial** of 380 adults showed significant improvement across **all four primary outcomes**." | site | `[PUBLIC CLAIM]` | registry says `OBSERVATIONAL`, 1 primary + 3 secondary; publication says "four primary outcomes" — **source conflict, unresolved** |
| M-8 | "Yes. A **peer-reviewed** 16-week open-label trial (N=380 adults; ClinicalTrials.gov NCT04562948) showed significant improvement in anxiety, depression, functional impairment, and quality of life." | site FAQ | `[PUBLIC CLAIM]` | "peer-reviewed" supported by PMID 40656972 |
| M-9 | "A 16-week clinical trial showed significant improvements in **anxiety, functioning, and quality of life** - making MindShift one of the few anxiety apps with published, peer-reviewed evidence." | site | `[PUBLIC CLAIM]` | **three outcomes here vs four** in M-7/M-8 on the same surface |
| M-10 | "**Most mental-health apps have no published evidence. MindShift is different.**" | site | `[PUBLIC CLAIM]` | comparative; not verifiable here |
| M-11 | "**Most anxiety apps are guesswork.**" / "Real tools, not guesswork" | site | `[PUBLIC CLAIM]` | comparative |
| M-12 | "**Proven, not promised.**" | site meta | `[PUBLIC CLAIM]` | contrasts with the source's "may provide" and its causal-inference limitation |
| M-13 | "**Clinically proven**" | site footer badge | `[PUBLIC CLAIM]` | same contrast as M-12 |
| M-14 | "**Backed by a 16-week clinical trial.**" | site | `[PUBLIC CLAIM]` | — |
| M-15 | "Grounded in CBT. **Proven in a clinical trial.**" | site | `[PUBLIC CLAIM]` | — |
| M-16 | "Open-label trial · N=380 adults (18–80) · ClinicalTrials.gov NCT04562948" | site | `[PUBLIC CLAIM]` | matches registry enrollment and publication age range |
| M-17 | "MindShift has also been **recommended by clinicians at academic medical centres in Canada and the US**, and **used in research in outpatient and inpatient psychiatric settings**." | site FAQ | `[PUBLIC CLAIM]` | **no citation given** |
| M-18 | "developed by experts" / "Structured CBT exercises designed by experts" | store + site | `[PUBLIC CLAIM]` | four named clinicians on the team surface |
| M-19 | `studyType OBSERVATIONAL`; `ECOLOGIC_OR_COMMUNITY`; no arms/interventions; 380 ACTUAL; sponsor University of Windsor; collaborator Anxiety Canada; `hasResults` false | ClinicalTrials.gov | `[THIRD PARTY]` | §11.1 |
| M-20 | Paper: "online open-label trial"; "four primary outcomes"; d = 0.61/0.50/0.55/0.31; "**Improvements were unrelated to the frequency with which participants used the MindShift app**"; "not remission" | journal | `[FOUND IN PUBLISHED DOCUMENT]` | §11.2 |
| M-21 | Paper: "**the lack of a comparison group who abstained from using the MindShift app limits the causal inferences…**"; "may indicate regression-to-the-mean" | journal | `[FOUND IN PUBLISHED DOCUMENT]` | §11.2 |
| M-22 | Paper: only **156 of 380** participants verified app installation | journal | `[FOUND IN PUBLISHED DOCUMENT]` | §11.2 |
| M-23 | Paper: funding "provided by **Anxiety Canada**"; four of five authors "**volunteer on the Scientific Advisory Committee of Anxiety Canada**"; "supported by a grant from Anxiety Canada to LMR" | journal | `[FOUND IN PUBLISHED DOCUMENT]` | §11.5; **disclosed in the source** |
| M-24 | Corrigendum: Q-LES-Q-SF total-score error; baseline QoL (M 30.76, SD 9.77) → (M 47.83, SD 15.39); "primary conclusions are unchanged" | journal | `[FOUND IN PUBLISHED DOCUMENT]` | §11.3; not referenced on the marketing surface |
| M-25 | Registry references: 18 entries, **all `BACKGROUND`, none `RESULTS`** | ClinicalTrials.gov | `[THIRD PARTY]` | §11.1 |
| M-26 | "**Privacy respected**" / "**Privacy & data**" / "**how it protects your privacy**" / "privacy-forward by design" | site | `[PUBLIC CLAIM]` | §9.3 |
| M-27 | Play Data safety: "**No data shared with third parties**"; collects "**Personal info and Health and fitness**"; "**Data is encrypted in transit**"; "**You can request that data be deleted**" | Play panel | `[THIRD PARTY]` | developer-supplied; §9.2 |
| M-28 | Play declares the privacy policy at `https://mindshiftcbt.com/privacy/` | Play panel | `[THIRD PARTY]` | **that URL returns a genuine 404** (§9.1) |
| M-29 | App Store "Privacy Policy" link = `https://mindshiftcbt.com/privacy/` | App Store page | `[THIRD PARTY]` | **same 404 URL** |
| M-30 | App Privacy: Linked to You = Contact Info → Name; Not Linked to You = User Content, Usage Data, **Health & Fitness → Health**, Contact Info → Email, Identifiers → User ID | App Store panel | `[THIRD PARTY]` | "has not been verified by Apple"; **`Contact Info` appears in both classes** |
| M-31 | "Your Thought Records, check-ins, and personal data are **stored securely, and we don't sell your personal data to third parties**." | site FAQ | `[PUBLIC CLAIM]` | "stored securely" undefined |
| M-32 | deletion is "**request** deletion from the app settings, which **permanently removes your data from our servers**" | site FAQ | `[PUBLIC CLAIM]` | request-based; no SLA |
| M-33 | "**Creating an account lets you back up your data and sync it across devices.**" | site FAQ | `[PUBLIC CLAIM]` | account optional |
| M-34 | Release notes v3.0.1 (2021-12-17) and v3.0.3 (2022-08-11): "**The Terms of Use and Privacy Policy have also been updated**" | App Store version history | `[THIRD PARTY]` | vendor-authored; documents referred to as existing |
| M-35 | Public site has **no** privacy or terms document: sitemap 9 URLs; router 13 paths; **0** occurrences of `Privacy Policy`/`privacy policy`/`Terms of Use`/`Terms of Service`/`terms of use`/`Terms` in the 741,896-byte bundle; 8 probed paths 404 | site + client | `[FOUND IN PUBLISHED CLIENT]` | §9.1 |
| M-36 | Tool set: Daily Check-ins, Thought Records, Facing Fears Ladders, Behavioural Experiments, Comfort Zone Challenges, Goal Setting, Relaxation/Mindfulness audio, Quick Tips, Community | site | `[PUBLIC CLAIM]` | — |
| M-37 | CSV export; "**no clinician portal**"; export "initiated and controlled by the patient or client" | site | `[PUBLIC CLAIM]` | schema not published |
| M-38 | "**Always Free: No subscriptions, no hidden costs.**" | Play description | `[PUBLIC CLAIM]` | consistent with App Store `formattedPrice: Free` |
| M-39 | "**500K+ Downloads**" (Play) vs "**More than 2 million downloads across 180+ countries and all 13 Canadian provinces and territories**" and footer badge "**2M+ downloads**" | Play + site | mixed | **same quantity, two surfaces, different values — recorded without averaging** |
| M-40 | "**4.8 rating**" (with five filled star glyphs) vs App Store 4.32402/537 vs Play 3.3/2.2K | site + stores | mixed | recorded unresolved; **the site figure names no storefront or date** |
| M-41 | FAQ "currently available in **English and French**" vs store `[EN, FR, ES]` | site + store | mixed | unresolved |
| M-42 | "designing to **AODA and WCAG** standards" vs "**The developer has not yet indicated which accessibility features this app supports.**" | site vs App Store panel | mixed | two surfaces that do not corroborate each other |
| M-43 | "**not a medical device, and not a replacement for therapy or crisis care**"; "It is **not designed for severe conditions or crisis intervention** — it routes users to crisis resources and is not itself a crisis service." | site | `[PUBLIC CLAIM]` | **positive: boundary is named** |
| M-44 | Crisis list in the published client: "**988 Suicide Crisis Helpline**" ("Call or text 988, 24/7, anywhere in Canada."), "**Kids Help Phone**" ("Call 1-800-668-6868 or text CONNECT to 686868."), "**Talk Suicide Canada**" ("Call 1-833-456-4566, 24/7 (Québec: 1-866-277-3553).") | published client | `[FOUND IN PUBLISHED CLIENT]` | **all three Canada-specific**; resources meta says "plus **Canadian** crisis support lines" |
| M-45 | "**A safe, actively moderated discussion board**"; "**Actively moderated for safety**"; "**Anonymous and judgment-free**"; Play rating flag "**Users Interact**"; App Store "Contains User-Generated Content"; **no published community guidelines or terms** (0 occurrences of `guidelines`/`Terms` in the bundle) | site + stores | mixed | moderated UGC with no public moderation terms on this surface |
| M-46 | Published client loads **Clerk** (auth/organizations/billing; `clerk.com` ×14, of which `dashboard.clerk.com` ×4), **Stripe** (via Clerk billing: `PaymentElementContext`, `StripeUtilsContext`, `__internal_loadStripeJs`), **Sentry** (error telemetry; `sentry` ×129 case-insensitive); **Replit** (`i.replit.com/script.js`) is in the served HTML shell of every document and **0 times** in the bundle | published client + shell | `[FOUND IN PUBLISHED CLIENT]` | §10 |
| M-47 | Published client ships user-visible failure copy: "We couldn't load the resources right now. Please try again shortly." / "Please refresh the page and try again." | published client | `[FOUND IN PUBLISHED CLIENT]` | a not-loaded state is modelled; **a string is not a behaviour** |
| M-48 | Content pages render "**Last reviewed and updated**" with `<time datetime>` | published client | `[FOUND IN PUBLISHED CLIENT]` | **positive provenance affordance**; date values `[UNKNOWN]` |
| M-49 | B2B surface: "**Talk to us about early access for your organization**"; "bulk materials and organizational deployments"; `robots.txt` disallows `/admin`, `/sign-in`, `/sign-up`, `/api/` | site + client | `[PUBLIC CLAIM]` + `[FOUND IN PUBLISHED CLIENT]` | **price not published → `[UNKNOWN]`** |
| M-50 | Four named clinicians on the team surface; Buhr "Clinical lead, MindShift CBT" who "served as the lead content developer"; Newth past "Director of Programs for Anxiety Canada"; Robichaud "including Anxiety Canada"; Badali "Since 2013, she has championed MindShift CBT" | site | `[PUBLIC CLAIM]` | §11.5 |
| M-51 | `anxietycanada.com` returns a byte-identical 9,321-byte shell for every path, including `/robots.txt` and `/sitemap.xml` | retrieved | **retrieval failure** | **not evidence of absence — `[UNKNOWN]`** |
| M-52 | App Store page age rating "**9+**" vs iTunes Lookup `contentAdvisoryRating` "**4+**" | App Store (two surfaces) | `[THIRD PARTY]` | Apple-side; recorded unresolved, not attributed to the vendor |
| M-53 | App Store Accessibility section: "The developer has not yet indicated which accessibility features this app supports." | App Store panel | `[THIRD PARTY]` | developer-supplied field, unpopulated |
| M-54 | Published client contains `POST /api/analytics/session` (`{sessionId}`, `credentials:"include"`, `keepalive:true`) and `POST /api/analytics/events` (`{category, name, path, sessionId}`) | published client | `[FOUND IN PUBLISHED CLIENT]` | **static**: the code is written to post; no endpoint was called |
| M-55 | Session id is a UUIDv4 (`crypto.randomUUID()`) held in `sessionStorage`, validated against a UUID regex, and paired with sign-in state in `mindshift.analytics.signed-in` | published client | `[FOUND IN PUBLISHED CLIENT]` | per-session, not per-user-by-name; cleared with the tab's session storage |
| M-56 | Exactly four event call sites: `page_view` / `page_view`, `client_error`, `feature_use` / `resource_opened`, `feature_use` / `features_explored`; page path truncated to 240 characters; both fetches end `.catch(()=>{})` | published client | `[FOUND IN PUBLISHED CLIENT]` | telemetry fails silently and is invisible to the user |
| M-57 | No cookie notice, consent banner or analytics disclosure in the bundle: `consent` → 0; the six `cookie` hits are inside the bundled Sentry SDK's data-collection config; all 16 `Consent` hits are Clerk's `OAuthConsent` component | published client | `[FOUND IN PUBLISHED CLIENT]` | **paired with §9.1**: there is no privacy page on which a notice could live |
| M-58 | Identity provider proxied first-party: `VITE_CLERK_PROXY_URL` = `/api/__clerk`; admin surface `/api/admin/login`, `/api/admin/logout`, `/api/admin/session`, `/api/admin/analytics`, and resource CRUD at `/api/admin/resources/…` | published client + `robots.txt` | `[FOUND IN PUBLISHED CLIENT]` | a real CMS exists behind a disallowed path |

### 14.2 Artifact record

All artifacts retrieved 2026-10-06 with a Chrome user-agent. SHA-256:

**Vendor surfaces and public client**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/home.html` (SEO shell) | 3,918 | `37687122e030377d2e72f62fc61f4388c3104c92b95556c4bca4fecbf2b379f5` |
| `web/index-BogFfh2L.js` (**published client**) | 741,896 | `1bb1681128467a90803d37652bc7eb8d65e209a8dbdd867a5a260e8cba1ef828` |
| `web/bundle_strings.txt` (extracted strings) | 57,225 | `a15a83604b7dbcb1aabbb8ce95d5686bd1cf6e1ea89dc7d6b19e5d221bab7dd0` |
| `web/sitemap.xml` | 1,160 | `646c9740caa92c3166fea8afca74784bee07f2349d7bcfc22c878b2cfeca06c2` |
| `web/robots.txt` | 144 | `d119e9352b5f47b53876b2e9cf9d00d0de205cbc45143f12ea3dc47f8118a943` |
| `web/ms_privacy.html` (**404 document**) | 3,362 | `4a9ba979a9d4a5f0525e3f61f343fea0818a109cee05334ef237c952767fa3a7` |
| `web/ms_terms.html` (**404 document**) | 3,358 | `bc9ec2ddd69856c171265819821f84cb3380c518b5011ac36ed7aa78ac4fbb47` |
| `web/ms_faq.html` | 6,828 | `dea8a01f057f3e4102676fcbf74f0d0922f20b6da9eab67b7f1265cc1fdb9c8c` |
| `web/ms_app.html` | 4,339 | `36b5d99a8286a1042ad4f01242af943bdaa990c5ec87e3284dd47ba9cc12ebe8` |
| `web/ms_for-clinics.html` | 5,645 | `8e2cbe785e798aa9f92ae9c095e85191e09c0c111dff5c5f16463bf9a7c4e30c` |
| `web/ms_institutions.html` | 3,919 | `3c09ffdd1d23b7b04a9c9dfbecc9f70ab88ae01ae6e094675d7a63c26a1712a6` |
| `web/ms_methodology.html` | 3,889 | `ee5cff8e804c60908db3695f4e412ab3a3e6761b98bb183f12de1fcffbd50049` |
| `web/ms_resources.html` | 4,643 | `3e538958e617ab74eaa40ef1281833c61379b2220d03e0ab32cf818f42e54f4a` |
| `web/ms_about.html` | 3,861 | `ab58ecd4cf38e369a5bdfa2775e4003ee9e49b391a9c38c3b3d3e05614853b16` |
| `web/ms_contact.html` | 3,876 | `71eab6dd36ee84d9b21d39b89433d9453bfc8d56069daf9cdf338613edf55dc7` |
| `web/tx.py` (extractor) | 344 | `97b9257115be5e6a31378bc8506c4099580b03c69c2a84ab4802f37c715afa50` |
| `web/ms_privacy_20261006.html` (independent re-fetch of `/privacy`) | 3,362 | `4a9ba979a9d4a5f0525e3f61f343fea0818a109cee05334ef237c952767fa3a7` — **byte-identical to `web/ms_privacy.html`** |
| `web/probe_log.txt` (status codes for 11 probed paths) | 852 | `d2eae211983b770c1f2f9301f3350a18c288b666ee954cf86377feec8bc08957` |

**Anxiety Canada — retrieval failure, byte-identical for every path**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/ac_home.html` = `ac_mindshift.html` = `ac_ms1.html` = `ac_ms2.html` = `ac_privacy.html` = `ac_terms.html` (all six identical) | 9,321 | `bacd434ff31aed1e42b609a2233d84b3b0d16a0a7b2b42f04b417acef179a8d6` |

**Storefront records**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `web/appstore_ms.html` | 689,610 | `35a83c387d3f567eb99217742356d34359eb7d3c7153b2b26b780270fd47fd45` |
| `web/play_ms.html` | 1,206,296 | `8017dbc088c331449572be32adaf7b148bacc37420218b7d3516e5f141050a5c` |
| `api/itunes_com.bstro.mindshift.json` (= `itunes_bs.json`) | 9,253 | `535edc20149c50737b1c8e4d1a6ee22f52bc31973b967255cbfe1365061d89af` |
| `api/itunes_ms.json` / `api/itunes_com.anxietybc.mindshift.json` / `api/itunes_ca.anxietycanada.mindshift.json` (all three identical — `resultCount 0`) | 42 | `7f5ce663b726607eae2fde2a6dc438052ac0d1681e636de7cc3377c0bb77e047` |
| `api/itunes_search.json` (24 results) | 250,197 | `5a2ed0783efc50abcf466e8461954d40a9da424a4d6a3f3e01c99520f20e50f6` |
| `api/itunes_ac.json` (18 results for "anxiety canada") | 185,910 | `0593422301d41fb9912e7db0ee4d942a8bf2e7e4b94e13be75ca1d08285c91db` |

**Independent and third-party records**

| Artifact | Bytes | SHA-256 |
|---|---|---|
| `api/ct_NCT04562948.json` (ClinicalTrials.gov v2) | 20,342 | `55cb272ec50d5399801660e7fea173084fd90eb9df7c83c5c85ec317d9f402c4` |
| `api/epmc_mindshift.json` (Europe PMC search, 55 hits) | 164,775 | `07fc5916262506f2d68ddea99e42e76b89d2d82b1cb221344d277df9e74b82c0` |
| `api/epmc_40656972.json` (the paper) | 5,854 | `11a12ce0d4813a79ea65165d34b45eef02802ee68c4d80bc777c02a98b6b3b7d` |
| `api/epmc_40655914.json` (the corrigendum) | 3,950 | `901919c5e3ed61d278b005532f09b2cfa1fc20a1cf9551597801c4c1d2852778` |
| `api/fulltext_PMC12244177.xml` (paper full text) | 139,072 | `d39e93328326b88c5e77d70b443776f1f90116b4bc7beff00036de47e7976c7e` |
| `web/pmc_12244177.html` | 235,623 | `cdbfc0a90bae0750f84c737694ced9343ff77eb6709b973ef71b7fdf0a7eba83` |
| `web/pmc_12244218.html` (corrigendum) | 96,303 | `f541152e15c43be8efe890bb4182eceed99c88a4ae887baa7749b5f3c2f430d5` |

Two failed retrievals are recorded as such, not as absent content: `api/fulltext_40656972.xml` and
`api/fulltext_40655914.xml` each contain a **148-byte HTTP 500 error body** from the Europe PMC
`/fullTextXML` endpoint. The fetch was retried against the PMC id (`PMC12244177`) and succeeded. **A 500 is a
retrieval failure, not an absence.** The corrigendum's full text was obtained from its PMC page rather than
from the XML endpoint.

### 14.3 What this file is not

- It is **not** an allegation of research misconduct. The publication discloses its funding, its
  affiliations, its editorial-board relationship, its missing comparison group and its null usage finding.
  **A study that publishes its own limitation is doing what it should.** What this file records is where a
  *marketing surface* diverges from a *source* that is itself candid.
- It is **not** a finding that MindShift has no privacy policy. It records that **the URL both storefronts
  publish as the privacy policy returns a Not-Found page**, that no privacy or terms document is reachable
  from the public site, and that the vendor's own notes referred to such documents in 2021–2022. **The
  in-app state is `[UNKNOWN]`.**
- It is **not** a claim that the app is insecure. No security claim is made or refuted; the only concrete
  statement retrieved is "encrypted in transit".
- It is **not** a clinical judgement. This file does not assess whether CBT is effective, whether MindShift
  helps, or whether the study's design is appropriate. It records what the sources say.
- It is **not** derived from runtime observation. There are none for this candidate.

### 14.4 Correction carried into this file

**The shortlist premise was materially altered. Its entry for this candidate has since been corrected in
place, and the superseded wording is recorded here for traceability.**

`CANDIDATE_SHORTLIST.md` originally attributed this candidate to **Anxiety Canada (non-profit)** and
described it as **free and non-commercial**, adding that MindShift is **non-profit-backed**. **That wording
is superseded.** The entry now reads **247 Labs Inc** (for-profit, Toronto) as the operator and records a
free-to-consumer app with a paid B2B surface; the correction is marked inline in
`CANDIDATE_SHORTLIST.md` §10 with the date it was applied. The storefront seller, the site's own About
surface and the site's team block all name **247 Labs Inc** — "the Toronto software firm that has developed
the app from the start" — as the operator; Anxiety Canada appears only in the **past tense** as the original
development partner; and the site operates an **unpriced B2B surface** whose published client ships **Clerk
organizations and Stripe billing**.

**Corrected position.** MindShift is a **free-to-the-consumer app operated by a for-profit software firm**,
not a non-profit's product. The two halves must be kept apart: "free" is correct and consistent across both
storefronts; "non-commercial" is not.

This correction is recorded here, in `PROCESS_STATE.md`, in `CANDIDATE_SHORTLIST.md` §10, and as a standing
sub-check in `RESEARCH_METHOD.md`. See gap G-8.

### 14.5 Evidence gaps

| Gap | What is missing | Why it matters | Status |
|---|---|---|---|
| **G-1** | The app binary was not inspected | whether the privacy policy and terms of use still exist in-app is the one question that would resolve §9.1 fully | **not attempted** — proprietary binary, no lawful acquisition route, no authorization from 247 Labs |
| **G-2** | No runtime observation of any kind | nothing about the app's actual behaviour, network calls or failure handling is known | **not attempted** — no emulator, no proxy, no account |
| **G-3** | CSV export schema unpublished and unexamined | the package cannot compare MindShift's data model to V2's | blocked by G-1 |
| **G-4** | App Privacy panel is internally mixed about `Contact Info` linkage | affects any claim about how health data relates to identity | **unresolved from outside** |
| **G-5** | Whether Spanish is shipped in the UI | store says EN/FR/ES; vendor FAQ says EN/FR | unresolved |
| **G-6** | `anxietycanada.com` is unreadable to non-browser clients | Anxiety Canada's current role, and whether it hosts a MindShift policy, is `[UNKNOWN]` | **retrieval limitation** |
| **G-7** | Whether the corrigendum changed the quality-of-life effect size | one of four headline outcomes had a scoring error | corrigendum does not restate it — `[UNKNOWN]` |
| **G-8** | `CANDIDATE_SHORTLIST.md` carried the superseded operator wording | a reader of the shortlist alone would have got the wrong operator | **closed 2026-10-06** — entry #10 corrected in place, with the superseded wording preserved in the correction block |
| **G-9** | The "4.8 rating" figure cannot be reproduced | no storefront, region or date is named on the surface | unresolved |
| **G-10** | One citing article reported for the paper | citation impact is not assessed here and coverage may be incomplete | recorded as reported only |
| **G-11** | No peer review of the *registration* record | the registry is a sponsor-supplied document, not an independent verification of the study | scope limit, not a defect |
| **G-12** | Any current Anxiety Canada–MindShift agreement is not public | whether a live partnership exists beyond the historical one is unknown | `[UNKNOWN]` |

### 14.6 Weaknesses and omissions in this file

1. **The strongest evidence in this file is a PDF and an HTML page, not source code.** Everything in §11
   rests on two published documents and one registry record, read once, on one date.
2. **I did not search for other publications about MindShift.** The Europe PMC sweep was targeted; a
   systematic search for every study using the app was not performed, so "the only study" is **not** a claim
   this file makes (the paper itself cites "[35], [36]" as prior data on the app).
3. **The registry-versus-paper conflict is reported, not resolved.** I did not contact the authors, the
   sponsor, or ClinicalTrials.gov; I did not compare the registration against a statistical analysis plan.
4. **The published client is a marketing site, not the app.** Findings from `index-BogFfh2L.js` describe the
   *website*, and I have been careful to attribute them that way throughout.
5. **The string sweep is exact-match only.** `Privacy Policy` returning 0 does not rule out a link rendered
   from a URL constant, a translated string, or a dynamically composed label. The 404 sweep and the
   sitemap/router checks are what make the finding stand; the string sweep is corroboration, not the proof.
6. **Nothing here establishes intent.** Every divergence recorded in this file is compatible with an
   unmaintained policy page, a site rebuild in progress ("next-generation rebuild" is the vendor's own
   phrase), or a document that lives in the app. All are stated where relevant.
7. **A quote-verification sweep was run against the fetched bytes** before this file was committed
   (`/tmp/aw2research/mindshift/quotecheck.py`, line- and paragraph-scoped pairing, HTML/XML tag-stripped,
   JS escapes decoded, elisions split into fragments; 158 distinct quoted fragments checked). The sweep
   **found and fixed five real misquotations** in draft: (a) a dropped "all of" in the editorial-board
   footnote; (b) a spurious "your" at the head of the FAQ data sentence (draft read `your personal data are
   stored securely`; the source is "Your Thought Records, check-ins, and personal data are stored
   securely"); (c) "outpatient
   and inpatient psychiatric settings" mis-transcribed as "ambulatory and inpatient"; (d) "registered"
   misquoted as "pre-registered"; and (e) a missing "ClinicalTrials.gov" inside the quoted FAQ trial
   sentence. Two
   fragments the checker reports as unfound are prose, not vendor quotations (one quotes
   `CANDIDATE_SHORTLIST.md`, which is outside the artifact corpus).

---

## Related files

- `mindshift_IMPLEMENTATION_INSPECTION.md` — Phase 2A record: artifacts, hashes, the 404 verification chain,
  and the claim-reconciliation table.
- `PROMPT_2.md` — the candidate-specific second-stage brief.
- `mindshift_COMPARISON.md` — the v1/v2 comparison.
- `../CANDIDATE_SHORTLIST.md` — shortlist entry #10 — **corrected in place 2026-10-06**; the superseded
  wording is preserved in that entry's correction block.
- `../PROCESS_STATE.md` — candidate dossier and residual uncertainty `R1`–`Rn`.
- `../RESEARCH_METHOD.md` — inspection modes and standing sub-checks.
