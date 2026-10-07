# Visible — authorized implementation inspection

**Candidate:** C11 — Visible (`com.makevisible.visible`), Visible Health Inc.
**Research date:** 2026-10-06/07.
**Governing method:** `../RESEARCH_METHOD.md` §6.1 (inspection modes), `../PROMPT1.md` Phase 2A,
`../FREE-ANDROID-TRAFFIC-INSPECTION.md`.
**Companion files:** `visible_CLAIMED_FEATURES_AND_PURPOSE.md` (claim ledger, artifact digests),
`PROMPT_2.md`, `visible_COMPARISON.md`.

---

## 1. Inspection mode

**Mode: `Performed — published client, vendor organisation, vendor documentation and peer-reviewed
publication.`**
**Mode for the app binary: `Not performed.`**

The package's five inspection modes are enumerated in `../RESEARCH_METHOD.md` §6.1. This candidate
uses the fifth (published client and public records) and the fourth (vendor-published artifacts), and
explicitly declines the app-binary mode.

| Surface | Performed? | What "performed" means here |
|---|---|---|
| Marketing site (server-rendered HTML + inline JS) | **Yes** | Read; retrieved bytes hashed and recorded |
| Help centre (95-article Intercom corpus) | **Yes** | 26 articles retrieved and read; the rest enumerated via `sitemap.xml` |
| Privacy policy, Terms of Use | **Yes** | Read in full from the served bytes |
| Store listings and Play Data Safety | **Yes** | Read from the store-served responses |
| Peer-reviewed publication + registry record | **Yes** | Read from the publisher, PMC and Europe PMC copies |
| Vendor GitHub organisation | **Yes** | Read from the GitHub API (metadata only, no vendor code exists there) |
| **Android APK / iOS IPA** | **No** | Not acquired, not decompiled, not launched — §2 |
| **Runtime network capture** | **No** | No build was installed; no mitmproxy session was run for this candidate |
| **Any observation of a running Visible instance** | **No** | — |

A static artifact reading is not a runtime observation. Nothing in this file asserts that a feature
works.

---

## 2. Why the app binary was not inspected

1. **Contract.** The vendor's Terms of Use state: "You may not reverse engineer, decompile, or
   otherwise attempt to extract the source code of the Software, unless expressly permitted or
   required by applicable law." The same document adds "You may not, however, copy or publish any
   part of the app or services." `[DOCUMENTED]`
2. **No authorized acquisition path.** The app is distributed free-of-charge through the App Store
   and Google Play under those terms. There is no lawful acquisition route that also authorizes
   decompilation (no explicit permission, no bug-bounty safe harbour published, no open licence).
   This is the same determination the package recorded for Bearable, SleepHQ, Empatica, Oura,
   MindShift, Welltory and Exist.
3. **The ruling effect.** The vendor's prohibition is a **counter-surface that strengthens the
   "not performed" decision** — the same reasoning the package applied to Empatica's EULA. A
   contract clause that prohibits the activity is a recordable fact about the boundary, not a
   criticism of the vendor.
4. **No runtime capture either.** The mitmproxy workflow in
   `../FREE-ANDROID-TRAFFIC-INSPECTION.md` requires installing the target app on a disposable
   emulator. Installing to observe traffic is not the same activity as decompiling, but it still
   produces the app's live requests and would require the same authorization decision, which has not
   been taken for this candidate. It was therefore not performed.

**Consequence:** every functional statement about Visible in this package is `[PUBLIC CLAIM]`,
`[DOCUMENTED]` or `[FOUND IN PUBLISHED CLIENT]`. There are **zero runtime observations** for this
candidate, consistent with the package-wide runtime-observation count of zero.

---

## 3. Artifact record

Full SHA-256 digests for every artifact cited in this file are in
`visible_CLAIMED_FEATURES_AND_PURPOSE.md` §20. Retrieval was by desktop HTTP client; all requested
resources were fetched once, and byte length and digest recorded in a retrieval manifest at the time
of fetch.

Status observations made with the client at the research date:

| URL | Result | Class |
|---|---|---|
| `https://www.makevisible.com/` | HTTP 200, 123486 bytes | `[OBSERVATION]` |
| `https://www.makevisible.com/pricing` | HTTP **404**, 13501 bytes (error document) | `[OBSERVATION]` |
| `https://www.makevisible.com/science` | HTTP **404**, 13501 bytes (error document) | `[OBSERVATION]` |
| `https://www.makevisible.com/privacy-policy` | HTTP 200 after redirect to `/privacy`, 68066 bytes (same digest as `/privacy`) | `[OBSERVATION]` |
| `https://www.makevisible.com/study` | HTTP 200, 13487 bytes — JavaScript shell only | `[OBSERVATION]` |
| `https://www.makevisible.com/research` and `/join-research` | HTTP 200, **byte-identical** (same SHA-256 `f0e17c9c…`) | `[OBSERVATION]` |
| `https://help.makevisible.com/*` (26 articles) | HTTP 200; individual digests recorded | `[OBSERVATION]` |

A 404 on a vendor's own navigation target (`/pricing`, `/science`) is a **documentation defect in the
site's routing**, recorded as an observation. A ~13 KB JavaScript shell is **not** a 404 and **not**
evidence of absence: the content could not be retrieved by a static client, which is `[UNKNOWN]`,
not a finding. This distinction is a standing package rule (`../RESEARCH_METHOD.md` §6.1).

---

## 4. Static findings from the published client

`[FOUND IN PUBLISHED CLIENT]` = a string or structure served to the browser by the vendor. Static.
Not a runtime observation, not a log, not a captured request.

### 4.1 Finding I-1 — Google Consent Mode v2 with an opt-out default outside the EEA/UK/CH

The marketing pages ship a Consent Mode v2 block whose own comments state the design:

```js
// EU/EEA + UK + CH → opt-in
gtag("consent", "default", { ad_storage: "denied", ad_user_data: "denied",
  ad_personalization: "denied", analytics_storage: "denied", functionality_storage: "denied",
  personalization_storage: "denied", security_storage: "granted", wait_for_update: 2000,
  region: ["AT","BE","BG","HR","CY","CZ","DK","EE","FI","FR","DE","GR","HU","IE","IT","LV","LT",
           "LU","MT","NL","PL","PT","RO","SK","SI","ES","SE","GB","IS","LI","NO","CH"] });
// Everyone else (US incl. CCPA states, Canada, etc.) → opt-out
gtag("consent", "default", { ad_storage: "granted", ad_user_data: "granted",
  ad_personalization: "granted", analytics_storage: "granted", functionality_storage: "granted",
  personalization_storage: "granted", security_storage: "granted", wait_for_update: 2000 });
gtag("set", "ads_data_redaction", true);
gtag("set", "url_passthrough", true);
// GPC = automatic CCPA opt-out
if (navigator.globalPrivacyControl) {
  gtag("consent", "update", { ad_storage: "denied", ad_user_data: "denied",
    ad_personalization: "denied" });
}
gtag('config', 'AW-11364136202');
```

Recorded properties, each directly readable from the served bytes:

1. **Region list is 33 entries**, covering the EU/EEA, the UK, Switzerland, Iceland, Liechtenstein
   and Norway; for those regions the non-essential storage signals default to `"denied"`.
2. **Outside that list, every storage signal defaults to `"granted"`** — the vendor's own comment
   labels this "Everyone else (US incl. CCPA states, Canada, etc.) → opt-out".
3. **`wait_for_update: 2000`** in both blocks: the tag waits 2 seconds for a consent update.
4. **`ads_data_redaction` and `url_passthrough` are enabled.**
5. **The Global Privacy Control branch denies the three advertising signals and does not update
   `analytics_storage`.** This is a scope observation about what the GPC path does, read from the
   code, not a claim about what it should do.
6. Google Ads conversion tag `AW-11364136202`; Google Tag Manager container `GTM-T2JXK5ZD`.

This is the customer-facing analytics/consent configuration of the **website**, not of the mobile app;
the app's telemetry is not observable without running it (`[UNKNOWN]`).

### 4.2 Finding I-2 — the marketing site is a Webflow-hosted static site with no server-rendered
application routes

Evidence: asset URLs served from `cdn.prod.website-files.com` with a site id
(`62ecf3a7c674b09fae0b1362`); two site paths return 404; two return the same bytes; two return a
~13 KB shell. `[OBSERVATION]`

Consequence for the evidence model: the marketing site's "product" descriptions are marketing
copy, not descriptions of a running system. Nothing derived from it can be elevated to a functional
claim.

### 4.3 Finding I-3 — duplicate route serves byte-identical content

`/research` and `/join-research` returned the same 68637 bytes with the same SHA-256. Recorded as a
routing fact. It matters for retrieval discipline: two "different" sources are one source, and a
citation discipline that counts them as two would overstate corroboration.

### 4.4 Finding I-4 — no first-party analytics or third-party SDK strings beyond the Google tags

Within the retrieved static pages, third-party endpoints observed: Google Tag Manager, Google Ads,
and the Webflow CDN. **Not** observed in the retrieved static bytes: Sentry, Segment, Amplitude,
Mixpanel, Facebook/Meta pixel, Hotjar, or a first-party telemetry endpoint. `[ABSENT FROM CORPUS]`

This is a **limited negative**: it says only that those strings are not in the server-rendered
marketing HTML. The mobile app's telemetry is `[UNKNOWN]`, and the privacy policy explicitly
documents Meta conversion tracking for the website and Meta advertising sharing for the app with
marketing consent (§12.2 of the claim file), so the **policy** discloses third-party involvement the
**static page bytes** do not reveal.

---

## 5. Vendor GitHub organisation — static findings

`github.com/makevisible` (created 2022-01-11) publishes **two repositories, both forks, neither with
vendor-authored commits**:

| Repo | Upstream | Language | Created | Compare result |
|---|---|---|---|---|
| `makevisible/polar-ble-sdk` | `polarofficial/polar-ble-sdk` | Swift | 2025-01-30 | **23 commits behind, 0 ahead** |
| `makevisible/async-zip` | `korzonkiee/async-zip` | Dart | 2025-04-11 | identical to upstream |

`[STATIC VENDOR ARTIFACT]` facts about a published mirror — not about the app binary.

**Findings.**

- **I-5 — no first-party source is published.** The organisation's public surface is a mirror of
  third-party code. A reviewer cannot read any vendor algorithm, schema or client code. The
  organisation's existence therefore adds **no** source-inspection evidence for Visible, in contrast
  to `oscar`, `nightscout` and `homeassistant` in this package.
- **I-6 — the Polar fork is a dependency mirror, not a modified vendor tree.** `0 ahead` means the
  org has not published a patched Polar BLE SDK; whatever the app does with Polar's SDK is not
  disclosed by the fork.
- **I-7 — the "23 commits behind" state is a fact about the mirror's staleness**, not about the app.
  Recorded to prevent the common misreading that a vendor fork reflects the shipped dependency
  version. Whether the app's Polar integration tracks upstream is `[UNKNOWN]`.
- **I-8 — the fork itself is weak, labelled evidence that the mobile client talks to Polar hardware
  through Polar's official BLE SDK.** `[INFERRED]` The inference is supported by (a) the vendor's own
  statement that the band is "manufactured by Polar", (b) supported use of the retail Polar Verity
  Sense, and (c) the existence of a Polar BLE SDK fork in the vendor's own organisation. It is an
  inference: the fork's presence does not prove the app links it.
- **I-9 — the organisation is a corroborating identity artifact**: name, description ("Making
  invisible illness visible.") and location (United Kingdom) match the vendor's public positioning and
  the UK subsidiary. Recorded as identity corroboration only.
- An additional probe of a plausible sibling organisation name (`visible-health`) returned **0 public
  repositories**. Recorded as a negative on that name, not as a fact about the vendor.

---

## 6. Vendor documentation as an implementation-describing surface

The help centre is the most implementation-adjacent surface the vendor publishes: 95 articles, of
which 26 were retrieved and read in full. It describes behaviour in enough detail to make specific,
falsifiable statements — and it is the source of the package's strongest positive comparator for
AnxietyWatch2 (`visible_CLAIMED_FEATURES_AND_PURPOSE.md` §6.1).

**Rules applied to this surface** (`../RESEARCH_METHOD.md` §6.1):

- A documented behaviour is **labelled documented**, never "verified" or "working".
- Where documentation and marketing conflict, **the most conservative statement governs**.
- The documentation's own internal contradictions are recorded with both sides quoted (§7).
- A copy-pasted sentence appearing on three surfaces (the "encrypted database… not shared" sentence
  on the FAQ and two help articles) is **one vendor statement on three pages**, not three
  corroborating sources. This is the same rule that produced the package's shortlist-correction
  obligations.

---

## 7. The publication, read as an artifact

The npj Digital Medicine paper is the strongest evidence artifact Visible publishes, and it is
inspectable in full. Citation, digests and content summary are in
`visible_CLAIMED_FEATURES_AND_PURPOSE.md` §13.2. Three inspection-class findings:

### 7.1 Finding I-10 — intra-section age contradiction (verified)

Within the Participants section: eligibility states "People aged ≥18 years … were eligible for
inclusion", and the following sentence reports "Participants ranged in age from 13 to 83 years
(M = 46.3, SD = 12.4)." Both strings verified in the publisher copy and the PMC copy.

Recorded as a verified **within-artifact inconsistency**. It is not a claim about the science. Its
consequences are precise: either individuals below the stated eligibility floor were included (a
methods-reporting defect), or the reported age range is wrong (a data-reporting defect). The paper
does not say which, so the package records the contradiction and leaves the resolution `[UNKNOWN]`.

### 7.2 Finding I-11 — the Code availability statement does not identify a location (verified)

Verbatim: "Data analysis was completed in R. The analysis code is available at upon reasonable
request."

The phrase "available at upon reasonable request" contains a dangling preposition, so the statement
names no repository, no archive and no contact. The paper's reproducibility claim therefore has no
addressable artifact. Recorded as a verified documentation defect; the code's availability is
`[UNKNOWN]`.

### 7.3 Finding I-12 — vendor affiliation and the "independent validation" question

The competing-interests statement names three current-or-former employees among the authors, and one
of them ("H.L.") is the founder named on the vendor's own About page. The paper is therefore
**vendor-affiliated research**, and the package does not present it as independent validation of
Visible. It is evidence that the vendor's dataset and model exist and were peer-reviewed; it is not
evidence that an independent group reproduced the result.

**Additionally:** the vendor's 2025 blog describes the same work as a "breakthrough research paper
(currently in pre-print)" and the model as "a cutting-edge AI algorithm that predicts your
susceptibility to crashes". The published paper reports a walk-forward cross-validated within-person
prediction model. The package records both descriptors and adjudicates neither.

### 7.4 Finding I-13 — registry record is absent for the flagship evidence

A ClinicalTrials.gov query for the vendor as sponsor at the research date returned no registered
trial. The flagship evidence is a retrospective analysis of app-collected observational data, not a
registered interventional trial. Recorded as an independent-registry observation.

The vendor's own blog post announcing two further studies describes them accurately as an
observational survey and a **feasibility** randomised study, and states the honest limit in the
vendor's own words: the survey "can't establish cause and effect", and randomised controlled trials
are what the vendor is "continuing to collaborate with researchers to design". That self-limitation
is recorded as a **positive** — the vendor publishes the evidence-grade boundary it operates within.

---

## 8. Algorithm-provenance inspection

| Date | Artifact | Version label | Class |
|---|---|---|---|
| 2023-01-26 | "Introducing: Morning Stability" | original ("Morning Pace") | `[PUBLIC CLAIM]` |
| 2023-04-11 | "…Morning Stability Score v1.1" | **v1.1** | `[PUBLIC CLAIM]` |
| 2023-05-30 | "…Morning Stability Score v1.2" | **v1.2** | `[PUBLIC CLAIM]` |
| 2026-03-24 | npj Digital Medicine publication | no algorithm version named | `[PUBLISHED RESEARCH]` |
| 2026-07-14 | overnight mode / rename to "Stability score" | no version label; beta framing | `[PUBLIC CLAIM]` |
| 2026-09-23 | "An even more accurate Stability Score" | **none** | `[PUBLIC CLAIM]` |

**Refuted hypothesis (not recorded as a finding).** The initial hypothesis was that the vendor ships
algorithm changes with no versioning practice. The 2023 posts refute it: the vendor published v1.1
and v1.2. `Falsified hypotheses are not recorded as findings` (`../RESEARCH_METHOD.md` §6.1).

**Finding I-14 (the surviving, narrower claim).** The 2026-09-23 change is announced as altering
scores that users had already received ("a day that previously gave you a 4, might now give you a
3"), carries **no version identifier**, and postdates the validation the vendor's marketing relies on
by six months. No published artifact — not the paper, not the blog, not the help centre, not an API
document — maps a Stability score to the algorithm version that produced it.

Contrast surfaces that make this precise rather than generic: the vendor's own v1.1/v1.2 precedent;
Oura's documented `PublicSleepAlgorithmVersion` enum (package candidate 8); and AnxietyWatch2's
registry- and schema-version discipline. Recorded as a **provenance finding**, not misconduct.

---

## 9. Claim-reconciliation table

The full ledger (V-1 … V-64) is in `visible_CLAIMED_FEATURES_AND_PURPOSE.md` §16. This table
reconciles only the claims that a reconciliation exercise can settle, and separates what is settled
from what is not.

| Claim | Best available evidence | Verdict | Evidence class of the verdict |
|---|---|---|---|
| Band hardware is Polar-manufactured | Vendor page + supported retail Polar Verity Sense + Polar BLE SDK fork in vendor org | Match | `[PUBLIC CLAIM]` + `[STATIC VENDOR ARTIFACT]` |
| Missing overnight reading ⇒ no score, no substitute | Help-centre article | Match (documented rule) | `[DOCUMENTED]` |
| Raw continuous heart rate cannot be exported | Help-centre export article | Match (documented rule) | `[DOCUMENTED]` |
| Clinician dashboard omits empty sections | Help-centre article | Match (documented rule) | `[DOCUMENTED]` |
| "All your Visible data… isn't shared with any third parties…" | FAQ + `/research` + 2 help articles, **two wordings across four pages** **versus** privacy policy + Play Data Safety + Terms | **Unresolved** | `[PUBLIC CLAIM]` vs `[DOCUMENTED]` vs `[PUBLIC CLAIM]` |
| Minimum age 16 | Terms + help centre **versus** same help article's under-16 path | **Unresolved** | `[DOCUMENTED]` |
| Paper eligibility ≥18 | Paper **versus** paper's own reported age range | **Unresolved (internally inconsistent)** | `[PUBLISHED RESEARCH]` |
| Analysis code available | Paper's Code availability sentence | **Does not match** (no location identified) | `[PUBLISHED RESEARCH]` |
| Algorithm versions are published | Blog history v1.1/v1.2 | Match | `[PUBLIC CLAIM]` |
| The 2026-09-23 change is version-labelled | Blog | **Does not match** | `[PUBLIC CLAIM]` |
| Validation covers the paid tier's continuous monitoring | Paper (daily reading) vs blog ("Next…") | **Does not match** | `[PUBLISHED RESEARCH]` vs `[PUBLIC CLAIM]` |
| Apple Health / Garmin HRV is supported | Help-centre article | **Does not match** | `[DOCUMENTED]` |
| No CPAP / SpO₂ / sleep-staging support | Absent from the 95-article corpus and the site | Match (absent from corpus) | `[ABSENT FROM CORPUS]` |
| No public API, webhook or third-party integration | Absent from the corpus | Match (absent from corpus) | `[ABSENT FROM CORPUS]` |
| Offline operation exists | "offline" absent from corpus | Match (no documented offline mode) | `[ABSENT FROM CORPUS]` |
| The app's actual network behaviour | — | **`[UNKNOWN]`** | no runtime capture performed |

The three "unresolved" rows are recorded with both sides quoted and no adjudication; the vendoring of
an explanation (the guardian's age, the undefined term "your Visible data") is quoted where the
vendor supplies one and marked as **not adopted** by this package.

---

## 10. Inferences (explicitly labelled)

| # | Inference | Basis | Confidence |
|---|---|---|---|
| N-1 | The mobile client integrates Polar hardware via Polar's official BLE SDK (I-8) | Vendor Polar-OEM statement + supported retail Polar device + a Polar BLE SDK fork in the vendor's org | Moderate |
| N-2 | The free tier is a funnel for membership: the export article withholds the paid signal, and the band is the gate on PacePoints and alerts | Price page + export article + store "GET ALL DAY DATA" framing | Moderate |
| N-3 | The 4-day minimum for the first Stability score is a baseline-warmup constraint, not a data-volume constraint | 90-day rolling baseline + "no score that day" rule | Low-moderate |
| N-4 | Because the score's inputs include self-reported sleep and symptoms, the score is partly **self-referential**, and an improvement driven by changed self-reporting would be indistinguishable from an improvement driven by physiology on the user's own screen | Stability help-centre article's stated inputs | Moderate; the vendor does not discuss it |
| N-5 | The age-field provenance hazard (§11 of the claim file) is the most likely explanation of the paper's 13-year-old participant under ≥18 eligibility | Vendor's own "Create an account using your own age" instruction | Moderate; not stated by the vendor |

Inferences are recorded separately so that no downstream reader can mistake them for observations.

---

## 11. Non-feature findings

| # | Finding | Class |
|---|---|---|
| D-1 | Terms: "The Agreement controls the relationship between you and and does not establish…" (repeated conjunction) | Verified text defect |
| D-2 | Terms: the phrase "Privacy and Policy" | Verified text defect |
| D-3 | Terms: one document is called both "Terms of Use" and "Terms of Service" | Naming inconsistency |
| D-4 | Paper: "The analysis code is available at upon reasonable request." | Verified text defect with an evidence consequence |
| D-5 | Paper: eligibility age vs reported age range | Verified internal inconsistency |
| D-6 | Site: `/pricing` and `/science` return HTTP 404 while being plausible navigation targets | Verified routing defect |
| D-7 | Site: `/research` and `/join-research` serve byte-identical documents | Verified duplication; an anti-corroboration fact |
| D-8 | Price table: two headline "Save N%" figures computed on different bases without stating the base (US "Save 25%" is the membership-only basis, 12 × $19.99 = $239.88 → $179.88; UK "Save 31%" is the combined membership-plus-band basis, £264.58 → £181.58). The UK membership component is additionally labelled "(25% off)" while its own two figures (£14.99/month, £131.88/year) imply ≈26.7%, and the UK band label "(41% off)" matches its own figures as stated | Verified presentation/rounding observation, unresolved. The four tables' totals are arithmetically consistent, so **no arithmetic defect is recorded**; the earlier arithmetic-defect hypothesis concerned the totals and remains refuted |
| D-9 | Terms dated 2022-07-14 govern a product tier (band, clinician dashboard, overnight mode) they do not describe | Lifecycle fact |
| D-10 | The vendor's most absolute privacy claim appears on **four pages** (FAQ, `/research`, and the "What happens to my data?" and "How is my data stored?" help articles) in **two different wordings** — the possessive and the consent verb both vary | Four pages, two wordings — an anti-corroboration fact, and a wording drift on the vendor's strongest claim. The drift is recorded, not resolved, and carries no inference about data handling |
| D-11 | The vendor's own code comments in the served Consent Mode block label the non-EEA default "Everyone else (US incl. CCPA states, Canada, etc.) → opt-out" | Verified static finding |

The package records these because they are directly observable properties of vendor artifacts. None
is presented as a legal or compliance conclusion.

---

## 12. What would unblock further inspection

Ranked by evidence value per unit of authorization required:

1. **An owner authorization decision for a runtime capture** of a Visible build on a disposable,
   authorized emulator, with the mitmproxy workflow in `../FREE-ANDROID-TRAFFIC-INSPECTION.md`. This
   would answer the package's largest `[UNKNOWN]`: whether the documented connectivity dependency,
   the alert path and the export limits behave as documented. **Not to be attempted without an
   explicit decision**, because the Terms prohibit reverse engineering and the vendor's contract
   governs.
2. **A vendor-authored OpenAPI/spec artifact.** Oura (candidate 8) and SleepHQ (candidate 5) publish
   structured contracts; Visible publishes none. Absent a spec, the documentation remains the ceiling
   on Phase-1 evidence.
3. **A published algorithm-version manifest** for the Stability score — the single change that would
   close finding I-14 without any inspection at all.
4. **A privacy-policy statement that names the term "your Visible data"** — the single change that
   would close the §12.3 conflict without any inspection.
5. **An independent (non-vendor-affiliated) replication** of the published model. This is the only
   route to converting the paper from vendor-affiliated evidence into independent validation.

---

## 13. Evidence gaps created by not inspecting the app binary

| Question | Status | Why the documentation cannot answer it |
|---|---|---|
| What is the Stability formula, and which version produced a given day's score? | `[UNKNOWN]` | No formula or version manifest published |
| What is the camera-PPG signal-quality model? | `[UNKNOWN]` | No threshold, error model or rejection rule published |
| Does the app function at all without connectivity? | `[UNKNOWN]` | Vendor documents that a *reading* may not start or complete without a strong connection; behaviour unmeasured |
| Are alerts delivered reliably, and what is the escalation/retry path? | `[UNKNOWN]` | Notification semantics are documented only as "Get a phone notification…" |
| Does the app read or write Apple Health / Health Connect? | Partly known | The vendor says HRV from Apple Health is not supported; other HealthKit surfaces are `[UNKNOWN]` |
| What telemetry does the app emit, and to whom? | `[UNKNOWN]` | Static page bytes do not describe app telemetry; the privacy policy names categories, not endpoints |
| Does the app respect an OS-level permission deny for BLE geolocation? | `[UNKNOWN]` | No runtime observation |
| Is the band's data stored locally, in the cloud, or both, and for how long? | Partly known | Retention is documented as "as long as necessary"; no period |

Each row is an **unknown**, not a suspected defect.

---

## 14. Machine-lifecycle facts affecting this candidate

Recorded here because they changed the research process for this candidate and are part of the
evidence record (`../PROCESS_STATE.md` carries the same facts).

- **L-1.** The host rebooted twice on 2026-10-06 (~21:44 and ~22:14), and the operating system cleared
  its temporary directory on each boot. The package's scratch corpus — including this candidate's
  previously fetched artifact tree, the quote checker and the sweep logs for all ten closed
  candidates — was destroyed. **No committed deliverable was lost**: every deliverable records its
  artifact URLs and SHA-256 digests, which is what made re-fetch and re-verification possible.
- **L-2.** Recovery procedure: the candidate's artifacts were re-fetched into a scratch root outside
  the repository (so it cannot be committed), re-hashed, and **every quotation in these deliverables
  was re-verified against the freshly retrieved bytes** rather than carried over from the destroyed
  copy.
- **L-3.** The recovery procedure is the one documented in `../RESEARCH_METHOD.md` §9: re-fetch from
  the URLs recorded in the deliverables, verify the recorded digests, re-run the two-mode
  quote-verification sweep, and record any digest drift. Digest drift was observed for pages that
  embed dynamic content (related-article lists, view counters); the freshly retrieved bytes are the
  authority for quotations in this file.
- **L-4.** The reboot also destroyed the sweep tooling. The checker was rebuilt in the persistent
  scratch root before the sweep for these deliverables, so the sweep result for this candidate is
  produced by a checker that exists and can be re-run, not by a script that vanished.

---

## 15. Ethical and legal limits applied

- **No DRM, authentication, encryption, certificate-pinning, anti-debugging or platform protection
  was circumvented.** No attempt was made to bypass any control.
- **No credential, session token, private key or personal data was obtained.** The only
  account-shaped data in this file is public vendor information.
- **No server action was taken** against any vendor endpoint: all requests were ordinary `GET`
  requests for public documents, at a low rate, with a single browser-like user-agent.
- **No production login** was attempted.
- **The vendor's prohibition on reverse engineering was honoured, not tested.** Where the contract
  forbids an activity, the package records the prohibition and declines the activity.
- **No CA certificate was installed anywhere.** No mitmproxy session was run for this candidate.
- **Third-party paths and personal data:** the vendor's GitHub fork content is third-party open-source
  code; no third-party developer's machine path is reproduced in this package. Where a public source
  comment contains a personal path, the package cites it as "a tester's machine path (redacted here)"
  and never reproduces it.
- **No machine-specific path is committed.** Deliverables cite URLs and digests; scratch locations are
  never cited as evidence (`../RESEARCH_METHOD.md` §9).

---

## 16. Related files

- `visible_CLAIMED_FEATURES_AND_PURPOSE.md` — claims, ledger, artifact digests
- `PROMPT_2.md` — candidate-specific second-stage prompt
- `visible_COMPARISON.md` — 14-section comparison against v1 and V2
- `../FREE-ANDROID-TRAFFIC-INSPECTION.md` — the (unused, for now) runtime-capture workflow
- `../RESEARCH_METHOD.md` — §6.1 inspection modes, §9 recovery
- `../PROCESS_STATE.md` — authorization decisions and sequencing
