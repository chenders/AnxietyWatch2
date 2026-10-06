# Prompt 2 — MindShift CBT

**Candidate:** MindShift CBT (shortlist #10) — 247 Labs Inc, `com.bstro.mindshift` / `com.bstro.MindShift`
**Base variant:** `../PROMPT2-EVIDENCE-FIRST.md`
**Supplementary block:** `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md`
**Chosen because:** this candidate's central question is not what the product does — that is described at
length — but **whether the vendor's published evidence reaches the vendor's published claims, and whether a
privacy posture survives being read against the vendor's own client**. MindShift is the first candidate in the
package where the strongest artifact is the vendor's **own peer-reviewed paper**, and the first where a
**storefront-declared privacy URL is verifiably dead**. Both are traceability questions, which is what
`PROMPT2-EVIDENCE-FIRST.md` is built for. The privacy/operating-mode block is added because the site ships its
own visitor telemetry with no notice, publishes no privacy document, and runs a for-profit B2B surface beside a
"Always Free" consumer app — so **where the data lives, which entity owns it, and what is disclosed** is a
first-order question here, not a secondary one.

**Why not the other variants.** `PROMPT2-TECHNICAL-SYSTEMS.md` (used for `sleephq` and `nightscout`) assumes an
inspectable implementation surface — source, a published API, or a runnable binary. MindShift has **none of the
three**: the client is minified, no API contract is published, and the app binary cannot lawfully be obtained.
A systems-internals brief would generate speculation about endpoints that were deliberately never called.
`PROMPT2-USER-VALUE.md` is a poor fit because the user value proposition is not in dispute — the app is free,
broadly distributed and clinically oriented, and the question is what backs the words "**Clinically proven**".
The open question is the evidence and the disclosure, not the value.

---

## Assignment

Produce a second-stage comparison of **MindShift CBT** against AnxietyWatch v1 (`../AnxietyWatch`) and
AnxietyWatch2 (`./`) driven by **traceability**: every claim either reaches a primary artifact or is recorded
as unreachable.

`mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` and `mindshift_IMPLEMENTATION_INSPECTION.md` are complete and are
the starting point, not drafts to redo. Close the gaps named in the inspection's §8 in the order given, then
answer the questions below. **Closing gap G-1 (whether the privacy and terms documents exist inside the app)
is worth more than everything else in this prompt.**

**Do not present anything from the vendor's marketing pages as verified.** Nothing about this candidate's
runtime behaviour was observed. There are no runtime observations for `mindshift` and you must not create the
impression that there are. **The 2026-10-06 bundle is a text artifact, not a running site.**

**Hard limits, unchanged from PROMPT1.** This is an authorized research brief, not permission to act.
**No DRM, authentication, licensing, encryption, certificate pinning, anti-debugging or platform protection
may be bypassed.** **No credentials or private data may be extracted.** **No vendor endpoint may be called
without an explicit authorization decision recorded in `PROCESS_STATE.md`.** In particular: do **not** call
`/api/analytics/session`, `/api/analytics/events`, `/api/admin/*` or `/api/resources/*`, do not create an
account, and do not sign up to the `/institutions` waitlist. Do not intercept or decrypt traffic from the app.

---

## Candidate identity

- **Operator under comparison:** **247 Labs Inc** — a for-profit Toronto consultancy, `sellerName` on the App
  Store and developer on Google Play. The shortlist's "Anxiety Canada" attribution is **superseded**; the site
  says the app was "**originally developed in partnership with** Anxiety Canada" (past tense).
- **Product under comparison:** **MindShift CBT** — a free, clinically framed CBT self-help app; iOS
  `com.bstro.mindshift` v4.1.0, first released **2013-06-06**, `fileSizeBytes` 87,966,720; Android
  `com.bstro.MindShift`. Storefronts disagree on downloads (Play "500K+" vs site "2M+") and rating
  (App Store 4.32402/537 vs Play 3.3 vs site "4.8").
- **Commercial surfaces:** consumer app Free; `/for-clinics` and `/institutions` are organizational surfaces
  with **unpublished** B2B pricing. The published client ships **Clerk organizations and Stripe billing**.
- **The evidence artifact:** the vendor's own funded, peer-reviewed paper — Rappaport LM, Jerome E, Van
  Ameringen M, Whittal M, McLean CP, "North American open-label 16-week trial of the MindShift smartphone app
  for adult anxiety", *Journal of Mood and Anxiety Disorders* 2023;4:100036, PMID 40656972, PMCID PMC12244177
  — plus its 2024 **corrigendum** (PMID 40655914) and the independent registration **NCT04562948**.
- **The strongest finding is an absence:** `https://mindshiftcbt.com/privacy/`, declared as the privacy policy
  by **both** storefronts, returns a genuine HTTP **404**, verified five ways.

---

## Read first

In this order, and only these. `docs/specs/AMENDMENTS.md` is mandatory before any claim about AnxietyWatch2.

1. `mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` — claim ledger (M-1…M-58), §9.1 the 404 finding, §14.5 gaps
2. `mindshift_IMPLEMENTATION_INSPECTION.md` — §3.3 the privacy-URL verification chain, §5 reconciliation
   points, §7 claim reconciliation, §8 gaps
3. `../../specs/AMENDMENTS.md` — all of it
4. `../PROCESS_STATE.md` — pipeline state, residual uncertainty, active-candidate line
5. `../RESEARCH_METHOD.md` — evidence classes, inspection modes, standing sub-checks
6. `../CANDIDATE_SHORTLIST.md` — why this candidate is ranked where it is (**carries superseded operator
   wording; see the claimed-features §14.4**)
7. `../PROMPT2-EVIDENCE-FIRST.md` and `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md` — the base and supplementary
   variants
8. `../empatica/empatica_COMPARISON.md` — the closest completed comparison in **evidence posture** (a vendor
   with a regulated artifact and a contractual counter-surface) and in **structure and tone**

Do **not** read the AnxietyWatch2 source tree broadly. Read only what a specific claim requires, and cite the
file and line you read.

---

## AnxietyWatch status labels (use these exactly)

| Label | Means |
|---|---|
| **shipped** | code in the tree, with a test, and a consumer |
| **implemented and unwired** | code and tests present, read by nothing (the `EventKind` case) |
| **specified** (V2 target) | fully specified in `openspec/` and/or `vitals/registry.toml`, not shipped |
| **not present** | absent from the tree, verified by reading, naming the file or search that establishes it |
| **unknown** | not checked, or checked and inconclusive — say which |

AnxietyWatch2's CNS engine, supervised alarm and hardware dead-man switch are **specified (V2 target)**.
Never write them as shipped. Never write a V2 "does not X" claim without checking the tree first.

**Two V2 facts established for this prompt, so that you do not have to re-derive them and so that you do not
overstate them:** a repository-wide search for `crisis` across `*.rs`, `*.md`, `*.toml` and `*.swift`,
excluding `docs/app_comparison/`, returns **0 files** — V2 has **no crisis routing**, and MindShift has three
named Canadian lines. Conversely, `docs/design/SPEC-admin-console.md:28` says the report generator produces
"something you can hand a clinician" and `:33` says "**This is explicitly not a medical device**, not
FDA-cleared, and not on a path to clearance" — the same posture MindShift takes with "not a medical device".
**Record both; do not let the crisis gap disappear because the medical-device boundary matches.**

---

## Evidence rules

1. **Five evidence classes, never merged:** `[PUBLIC CLAIM]` · `[THIRD PARTY]` · `[FOUND IN PUBLISHED CLIENT]`
   · `[INFERENCE]` · `[UNKNOWN]`. Plus `[runtime observation]` where it applies — of which this candidate has
   **none**. Do not manufacture one by describing a vendor screenshot or a `fetch()` call as behaviour.
2. **`[FOUND IN PUBLISHED CLIENT]` is a static class.** A bundled React SPA served to every visitor is a
   **published client**, not a binary and not a runtime. A string in it establishes what the code is *written
   to do*; it never establishes that an endpoint is live or that data was stored. Write it that way, every
   time.
3. **A documented contract is not observed behaviour**, and there is no contract here anyway. The site's own
   endpoints are **undocumented**: record them as found-in-client, never as working.
4. **The storefront-declared privacy URL is `[404]`, not `[UNKNOWN]`.** A genuine Not-Found document, returned
   byte-identically on re-fetch, on a site whose control paths return 200, is **verified absence** — the
   strongest class available. Contrast sharply with the `anxietycanada.com` case, which is a **retrieval
   failure** (`[UNKNOWN]`) because every path returned the same 9,321-byte SPA shell. **A 404 and a JS shell
   are different findings and must never be merged.**
5. **A vendor-authored, vendor-funded paper is not independent validation.** The venue is independent; the
   work, the funding and the authors are the vendor's. Quote the paper's **own** competing-interest
   statement — "Lance M. Rappaport reports financial support was provided by Anxiety Canada" — rather than
   inferring the funder's role.
6. **An independent clinical registry outranks the vendor's own noun.** `studyType: OBSERVATIONAL` is a
   third-party classification; "trial" in the paper's title and "clinical trial" on the site are the vendor's
   nouns. Record the conflict; do not adjudicate it in the vendor's favour or against it.
7. **Quote the most conservative statement when surfaces conflict**, and record the conflict. The site's own
   three-versus-four outcome counts and its three-way rating disagreement are tabulated in the
   claimed-features deliverable — use them, do not re-litigate them.
8. **An unresolved ambiguity is not a discrepancy.** Where two panels of the *same* storefront show different
   review counts (Play header "2.2K" vs ratings block "2.08K"), or where Apple's own page shows 9+ while the
   iTunes record shows 4+, record both and mark **unresolved**. Do not attribute Apple-side inconsistency to
   the vendor.
9. **A correct quotation is mandatory; a paraphrase wearing quotation marks is the specific failure this
   package exists to prevent.** Every quoted fragment must verify against the retrieved bytes. Run the quote
   checker (`/tmp/aw2research/mindshift/quotecheck.py`) as a separate pre-commit step and report its result,
   including a zero.
10. **Never state float equality.** Use tolerances.
11. **Do not reproduce third-party machine paths or personal identifiers** found in public artifacts; cite as
    "a tester's machine path (redacted here)". **Do not reproduce the error-telemetry ingest identifier found
    in the bundle**, and do not name individual staff members who are not the authors or the published
    clinical contributors.
12. **An inspection mode is not an observation.** "Performed — published web client and public records" means
    *read*, not *ran*.
13. **Standing sub-checks — apply each, and record the negative results too:** undocumented value domain ·
    write-only field · generated `example` blocks · vendor material that undercuts the headline · two vendor
    surfaces in conflict · intra-page conflict · honest boundary statement · documented default the code does
    not implement · an undefined term in a decisive table · a falsified hypothesis (do not record it as a
    finding) · independent registry artifact · contract/EULA as counter-surface · **an artifact the vendor
    offers as evidence that cannot bear the adjacent claim** · **retrieval failure is not absence** ·
    **HTML-shell string is not bundle string** · **a count must state its match mode (case-sensitive vs
    insensitive)** · **a quote checker must pair quotes per paragraph and per table row.**
14. **Verify a failure-mode hypothesis against the evidence before recording it.** If you hypothesise what the
    app does under a failure condition and cannot observe the app, record it as an **open question**, not a
    finding. Falsify your own hypothesis and drop it silently if the evidence contradicts you. Two
    hypotheses — a Segment CDP integration and a Google `gtag` tag — were tested for this candidate and
    **refuted**; neither appears anywhere as a finding, and neither should be invented by you.
15. **A tally a reader cannot reproduce is worse than no tally.** Any verdict count must be machine-counted
    from its own table, not written from impression.

---

## Candidate-specific unresolved questions

Answer in order. **Q1 and Q2 are the assignment**; the rest are lower value.

| # | Question | Why it matters | What would resolve it |
|---|---|---|---|
| **Q1** | **Do the privacy policy and terms of use still exist, and where?** | Both storefronts declare `https://mindshiftcbt.com/privacy/`, which returns a verified 404; the bundle contains 0 occurrences of `Privacy Policy`, `Terms of Use` or `Terms`; the sitemap and the 13-path router contain neither. Yet the App Store's own version history says, for v3.0.1 (2021-12-17) and v3.0.3 (2022-08-11), "**The Terms of Use and Privacy Policy have also been updated**". So the documents existed and a URL that was once valid is now dead. | The app binary (not lawfully available), or a vendor statement, or an archived copy of the document **plus a statement that it is no longer operative**. Note: an archived copy shows what the document *said*, never that it is *current* — say so if you find one. |
| **Q2** | **What is the basis of "Clinically proven", "Proven, not promised" and "Most anxiety apps are guesswork"?** | The vendor's own funded paper reports that "**Improvements were unrelated to the frequency with which participants used the MindShift app**", that effects were moderate, that the result was "**not remission**", and that "**the lack of a comparison group … limits the causal inferences**". The paper is unusually candid. The marketing is the surface that keeps only the outcome count — and the outcome count is the one the registry does not support. Either a comparing source exists, or a medical-adjacent app makes a proof claim its own evidence does not carry. | The paper's own text (already extracted); any subsequent trial with a control arm; a vendor methodology page (the site has `/methodology`); a search of the literature for a controlled MindShift study. Report the search **and** its result either way. |
| Q3 | Is there a controlled or comparative study of MindShift, and does the vendor claim one? | It decides whether "Clinically proven" is a present-tense claim with a source or a retrospective reading of one uncontrolled study. The paper cites prior MindShift work as "[35], [36]" — read those, they may be the closest thing to replication. | Europe PMC or PubMed for later citations of the 2023 paper; `api/itunes_search.json`-style metadata sweeps; the vendor's `/methodology` page. |
| Q4 | Was the quality-of-life effect size restated after the corrigendum? | The 2024 corrigendum corrected the Q-LES-Q-SF total-score calculation (baseline M corrected 30.76 → 47.83, SD 9.77 → 15.39) and says the primary conclusions are unchanged, but **does not restate** the reported d = 0.31. So whether one of the four headline numbers moved is `[UNKNOWN]`. | A corrected table, a republication, or an erratum addendum. If none exists, record `[UNKNOWN]` rather than assuming the number survived. |
| Q5 | **Who owns and operates the site, and what does the for-profit surface change?** | The shortlist said "Anxiety Canada"; the App Store `sellerName` and the Play developer are **247 Labs Inc**, and the site says "originally developed in partnership with Anxiety Canada" — past tense. A for-profit operator runs a B2B surface (`/for-clinics`, `/institutions`) with unpublished pricing beside a "Always Free" consumer app. | The site's About/team surfaces (already read); corporate registration; the `/institutions` terms — **do not contact them**. Record the entity mapping and the missing pricing as findings, not as inferences about intent. |
| Q6 | Does the site's own analytics collection have a disclosed purpose, retention period or lawful basis, on any surface? | The client posts a per-session UUID and four event categories to its own origin with `credentials:"include"`, silently. The site publishes no cookie notice, no consent banner, no analytics disclosure and — per Q1 — no privacy page. The Play data-safety block describes the **app**, not the website, and is a developer self-declaration. | Any vendor notice; the Play data-safety text read carefully for scope (it is about the app). **Do not call the endpoints.** Record the absence of a notice as the finding and state the static class explicitly. |
| Q7 | Which surface reflects the shipped UI language set? | The site FAQ says "currently available in English and French"; the App Store declares `languageCodesISO2A ['EN','FR','ES']`; the bundle's language switch accepts only `en` and `fr`; a **v2.0.9** release note says the app was "made it bilingual (French and English)". Four surfaces, two answers. | The app binary, or a vendor statement. Otherwise `[UNKNOWN]`, with the bundle's two-language constraint recorded as found-in-client. |
| Q8 | Is there a moderation policy or community terms anywhere a user can read? | The site promotes "a safe, actively moderated discussion board" and "Actively moderated for safety"; Play flags "Users Interact" and the App Store "Contains User-Generated Content"; the bundle contains 0 occurrences of `guidelines`. The v4.1.0 release note says "the guidelines no longer interrupt you or send you back to the home screen", which implies a guidelines surface **inside the app**. | The in-app guidelines (binary), or a published moderation policy. Otherwise: a publication-placement finding, exactly as recorded. |
| Q9 | Does the app implement the accessibility standard the vendor's own team surface claims? | A site role description says "designing to AODA and WCAG standards"; the App Store Accessibility section says "The developer has not yet indicated which accessibility features this app supports", while v4.1.0 notes claim "better screen reader support and clearer contrast". A design standard is not a declared feature — keep the two objects apart. | The store's accessibility section being populated; the app itself; or the vendor's own accessibility statement. |
| Q10 | Is the registry-versus-paper conflict about **design class** resolvable from a sponsor document? | `studyType: OBSERVATIONAL` with `observationalModel: ECOLOGIC_OR_COMMUNITY` and **no** intervention module, against a paper titled "trial" and a site saying "clinical trial", is a registered-vs-reported conflict. A protocol or statistical analysis plan could show the sponsor's own classification. | ClinicalTrials.gov document sections, a sponsor protocol on a registry, or the paper's supplementary material. Quote both and leave unresolved if nothing is found. |
| Q11 | Is the registry-versus-paper conflict about **primary outcome count** resolvable? | The registry lists **one** primary outcome and **three** secondary; the paper and the site both say "**four primary outcomes**". This is sharper than the design-class conflict because it changes what the study was powered to answer. | The registered statistical analysis plan; a later registration amendment (check `history`/`version` fields); the paper's methods. Both quotes are already in hand. |
| Q12 | What does AnxietyWatch2 have, or not have, against MindShift's two genuine positives? | (a) MindShift states its scope boundary in the user's own language: "not a medical device, and not a replacement for therapy or crisis care", "mild-to-moderate", "Adolescents (13+)", and routes to three named crisis lines. (b) It refuses a clinician portal in favour of patient-controlled export. V2 matches the first half of (a) (`SPEC-admin-console.md:33`) and the spirit of (b) (`:28`), but a repository-wide search for `crisis` returns **0 files**. | Read the V2 tree for the claims; cite file and line; record the crisis gap as **not present** with the search that establishes it. Do **not** propose a remediation. |
| Q13 | Does V2's own documentation classify its evidence honestly, the way the registry does for MindShift? | V2's CNS engine is "an in-progress early-warning aid, not the product" (`SPEC-admin-console.md:31`) and the console spec is explicit that it must not be designed as though it were. That is the same discipline the registry imposes on MindShift. It is a V2 **positive** and it belongs in §6, not buried. | Read `openspec/` CNS specs; cite file and line. |
| Q14 | Is the app's data export what `/for-clinics` says it is? | "A structured CSV of check-ins and the data collected across guided tools", patient-initiated. No schema is published and no export was performed, so the format is documented-only. It is the direct analogue of V2's report generator and the comparison should say so. | The app binary, or a vendor format description. Otherwise documented-only. |
| Q15 | Does the free-consumer-plus-paid-B2B structure matter for V2's own positioning? | V2 positions as free and open source; MindShift's consumer app is genuinely free while its operator is a for-profit firm monetising organizations. The two are not the same business, and the comparison should say which costs fall on whom rather than treating "free" as a single property. | The `/institutions` and `/for-clinics` copy (already read); published pricing (**none**); V2's own cost model, read from the tree where it exists. |

---

## Comparison taxonomy for this candidate

Use these dimensions, in this order, so the report lines up with the other candidates' comparisons.

1. **Evidence traceability per claim** — for every performance or outcome figure, which artifact carries it
   and whether it is reachable. The registry-versus-paper-versus-site triangle is the worked example.
2. **Funding and interest disclosure** — where the disclosure exists (the paper), where it does not (the
   marketing), and what that changes about how the claim should be read.
3. **Sample honesty** — 380 enrolled against **156** with a verified install; the headline figure everywhere.
4. **Disclosure surface completeness** — the site's inventory (sitemap, router, bundle sweep) against what a
   site of this scope would normally carry. The missing privacy document is the anchor.
5. **First-party telemetry versus stated posture** — what the client collects, under what notices, and the
   exact class of the finding (static).
6. **Entity mapping and commercial model** — who contracts with whom; free consumer surface against an
   unpriced B2B one.
7. **Missing-data and absent-signal posture** — how each side represents "not measured", applied to both
   V1/V2 and to MindShift. MindShift's scale is self-report by design; V2's is measured-and-reported.
8. **Alerting and the escalation path** — V2's specified alarm layer and dead-man switch against a consumer
   CBT app with **no alert path at all**, and MindShift's crisis routing against V2's absent crisis routing.
   This cuts both ways and the report must show both directions.
9. **Scope-boundary language at the point of use** — "not a medical device", "mild-to-moderate", crisis
   routing, age floor, as against V2's written framing.
10. **Derived measures and reproducibility** — a closed scoring app with no published instrument list against
    `vitals/registry.toml` as a single source of truth.
11. **What transfers and what does not** — the intervention is entirely different; the evidence discipline and
    the disclosure discipline are the transferable part. Be explicit about which is which.

---

## Failure-mode scenarios to walk through

For each: state what the **documented** behaviour is, then state whether that documentation exists at all.

| Scenario | What to establish |
|---|---|
| A user in crisis opens the app | MindShift routes to three named crisis lines (988, Kids Help Phone, Talk Suicide Canada) with Canadian qualifiers, **found in the client**. V2 has no crisis routing — `crisis` returns **0 files** in the tree. This is the one scenario where the candidate is plainly ahead. |
| A user's anxiety is severe, not mild-to-moderate | The FAQ states the intended scope ("mild-to-moderate"). Whether the app is gated for that, or only described that way, is **open** without the binary. |
| The site's analytics endpoint fails | `.catch(()=>{})` — the site's own telemetry fails silently by design. Recorded as found-in-client, with no claim about the endpoints' liveness. |
| The privacy page is requested | Verified 404, byte-identical on re-fetch. **This is the finding**, not a hypothesis. |
| A resource page is missing | The client has a localized not-found component and a resource-load failure string ("We couldn't load the resources right now. Please try again shortly."). A string is not a behaviour; the state is modelled in the UI layer and no more. |
| The user wants their data out | `/for-clinics` promises a structured CSV, patient-initiated, and explicitly no clinician portal. **Documented-only**; no export was performed and no schema is published. |
| The user asks for deletion | FAQ: deletion from app settings "permanently removes your data from our servers"; Play: "You can request that data be deleted". Documented-only; no tombstone or retention model is published — contrast with V2's append-only/tombstone rule, which is a **specified** discipline the candidate does not describe. |
| The funding source is asked about | The paper discloses it; the marketing does not. Record both quotes and leave it standing. |
| The study is asked to carry the word "proven" | It cannot: no comparison group, moderate effects, improvement unrelated to app use. The failure mode of the *evidence*, and the reason this candidate is in the shortlist. |
| A claim has no reachable source | Q2 and the "recommended by clinicians at academic medical centres" FAQ line, which carries no citation. Record as documented-only, not as false. |

---

## Required report structure

Match `../empatica/empatica_COMPARISON.md`, `../nightscout/nightscout_COMPARISON.md` and
`../sleephq/sleephq_COMPARISON.md` — fourteen sections:

1. Executive finding · 2. Product and availability · 3. System boundary and architecture (ASCII) ·
4. Data path and source comparison · 5. Claim-versus-documented-contract reconciliation · 6. Feature matrix ·
7. Derived values versus stored signals · 8. Real-time / overnight / failure-mode · 9. Provenance,
missingness, reproducibility · 10. Local/cloud/privacy and operational cost · 11. Alerting and the alarm
path · 12. User-facing consequences · 13. Limitations, unverified claims and evidence gaps ·
14. Balanced conclusion. Plus a "Related files" footer.

Because this candidate has **no** alert path of its own, §11 must carry both directions: what MindShift lacks
that V2 specifies (any escalation mechanism, any monitored signal, any dead-man switch), and what MindShift has
that V2 does not (crisis routing in the user's own language, at the point of use).

---

## Tone and claim discipline

- No medical advice; no claim that any product is unsafe or that a mental-health app is harmful.
- **No catastrophising about the missing privacy page.** A dead declared URL is a documentation-lifecycle and
  discoverability defect. It is **not** evidence of ungoverned data handling, not a regulatory conclusion, and
  not an allegation of intent. State it at the level the evidence supports and no higher.
- **No catastrophising about the funding.** A funded study is normal. The finding is a **disclosure
  asymmetry between two surfaces by the same author** — that is all it is, and the paper's own candour is a
  point in the vendor's favour.
- **No catastrophising about the analytics.** Absence of a notice on a site with no privacy page is a
  recordable tension, not an accusation. The finding is static: it says what the code is written to do.
- No unsupported superlatives, in either direction. A phrasing that names a specific, checkable property is
  fine; one that ranks the vendor on character is not.
- Distinguish **observed** from **documented** from **found in the client** from **inferred** in the prose,
  not only in the tables.
- Where the evidence does not support a conclusion, write that. A recorded gap is a valid result.
- **Do not overstate the comparison in AnxietyWatch2's favour.** MindShift has crisis routing V2 does not, a
  published evidence base V2 does not, and a patient-controlled export posture V2 only matches in spirit.
  Say so in §6, §11 and §14. Equally, correct every V2 "does not X" claim against the tree before writing it.

---

## Sources to use

**Primary, already retrieved (re-verify the hashes before quoting — the inspection's §3.2 has all of them):**

- `web/index-BogFfh2L.js` — the published client, 741,896 B, SHA-256 `1bb16811…`
- `web/probe_log.txt` — the eleven-path status log; `web/ms_privacy_20261006.html` — the byte-identical
  re-fetch
- `api/ct_NCT04562948.json` — the ClinicalTrials.gov v2 record
- `api/epmc_40656972.json`, `api/fulltext_PMC12244177.xml` — the paper; `api/epmc_40655914.json`,
  `web/pmc_12244218.html` — the corrigendum
- `web/appstore_ms.html`, `api/itunes_com.bstro.mindshift.json`, `web/play_ms.html` — storefront records
- `web/sitemap.xml`, `web/robots.txt` — the site's own inventory

**To seek, in priority order:** a **current** privacy policy or terms document, or a vendor statement that
none exists · any controlled or comparative MindShift study (including the paper's own references [35] and
[36]) · a restatement of the QoL effect size after the corrigendum · the ClinicalTrials.gov record's
`history` / version fields, for a registration amendment · a published moderation policy · the app's own
accessibility declaration.

**Do not use:** review-aggregator summaries; secondary pages that restate "4.8" or "2M+" without a date;
anything paywalled; anything that would require bypassing a control to reach; **`anxietycanada.com`**, which
returned a byte-identical SPA shell for every path on the research date and is therefore unusable, not empty.

**Do not call:** any `/api/` path on `mindshiftcbt.com`, including the analytics, admin and resources
endpoints. They appear in this package as **strings read from published JavaScript** and must stay that way.

---

## Output

`mindshift_COMPARISON.md` in this directory, following the fourteen-section structure above.

When you finish: run the quote checker on every file you wrote
(`python3 /tmp/aw2research/mindshift/quotecheck.py <file>`) and report its result **including a zero**; update
`../PROCESS_STATE.md` (candidate 10 dossier, residual uncertainty turned into numbered items, active-candidate
line) and `../RESEARCH_METHOD.md` if the inspection mode or a standing sub-check changed. Then run
`scripts/check-public-info.py --self-test` and the gate, `git add` the files **by name**, and commit with the
`Co-Authored-By: OpenAI <noreply@openai.com>` trailer. Verify `git show --check` is clean.

**Acceptance is the exit code of the gate, not your own assessment.** Do not mark candidate 10 complete on the
strength of a written report; record the commands you ran and what they returned.
