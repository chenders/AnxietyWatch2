# Prompt 2 — Visible

**Candidate:** Visible (shortlist alternative C11) — Visible Health Inc. / Visible Health UK Ltd,
`com.makevisible.visible`
**Base variant:** `../PROMPT2-EVIDENCE-FIRST.md`
**Supplementary block:** `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md`
**Chosen because:** this candidate's central question is **whether the vendor's published evidence reaches
the vendor's paid product**, and **whether an absolute privacy sentence survives being read against the
vendor's own contract and its own storefront declaration**. Visible is the first candidate in the package that
publishes a labelled algorithm-version history (v1.1, v1.2) *and then* ships a score-changing update with no
version label; the first whose flagship peer-reviewed paper has a **verified internal age contradiction** and a
**dangling code-availability phrase**; and the first where the product's headline paid feature is **outside the
scope of its own published validation**. All of those are traceability questions, which is what
`PROMPT2-EVIDENCE-FIRST.md` is built for. The privacy/operating-mode block is added because the vendor
publishes an unqualified "isn't shared with any third parties" claim on four pages — in two wordings — while
its own privacy policy, its own Google Play Data Safety declaration and its own Terms say otherwise, and because retention,
research-data irreversibility and consent posture are first-order questions for an app whose paid tier is a
continuous physiological stream.

**Why not the other variants.** `PROMPT2-TECHNICAL-SYSTEMS.md` (used for `sleephq`, `nightscout`, `oscar`,
`homeassistant`) assumes an inspectable implementation surface — source, a published API contract, or a
runnable binary. Visible has **none of the three**: the app is closed-source under terms that prohibit reverse
engineering; the vendor's GitHub organisation publishes **only two unmodified forks of third-party code**
(zero vendor-authored commits); and **no API, OpenAPI document, webhook or integration contract is published
anywhere**. A systems-internals brief would generate speculation about a stack that no artifact describes.
`PROMPT2-USER-VALUE.md` is a poor fit because the user-value proposition is not in dispute — the app is
free-to-download, broadly reviewed, and describes a coherent pacing model; the open questions are evidence
scope, disclosure and provenance.

---

## Assignment

Produce a second-stage comparison of **Visible** against AnxietyWatch v1 (`../AnxietyWatch`) and AnxietyWatch2
(`./`) driven by **traceability**: every claim either reaches a primary artifact or is recorded as
unreachable.

`visible_CLAIMED_FEATURES_AND_PURPOSE.md` and `visible_IMPLEMENTATION_INSPECTION.md` are complete and are the
starting point, not drafts to redo. Close the gaps named in the inspection's §13 in the order given, then
answer the questions below. **Q1 (validation scope) and Q2 (the absolute privacy sentence) are the
assignment**; everything else is lower value.

**Do not present anything from the vendor's marketing pages, help centre or store listings as verified.**
Nothing about this candidate's runtime behaviour was observed. There are **no runtime observations** for
`visible` and you must not create the impression that there are. **A help-centre article is a vendor
document**, not a measurement. **A string in the served marketing HTML is a static artifact**, not a
behaviour.

**Hard limits, unchanged from PROMPT1.** This is an authorized research brief, not permission to act.
**No DRM, authentication, licensing, encryption, certificate pinning, anti-debugging or platform protection
may be bypassed.** **No credentials or private data may be extracted.** **No vendor endpoint may be called
without an explicit authorization decision recorded in `PROCESS_STATE.md`.** In particular, for this
candidate:

- **Do not obtain, decompile, disassemble or patch the app.** The Terms of Use state: "You may not reverse
  engineer, decompile, or otherwise attempt to extract the source code of the Software, unless expressly
  permitted or required by applicable law." `visible_IMPLEMENTATION_INSPECTION.md` §2 records this as the
  reason Phase 2A is closed. It remains closed.
- **Do not install the app on any device** to observe traffic. Running it is a separate authorization
  decision that has not been taken; `../FREE-ANDROID-TRAFFIC-INSPECTION.md` exists for when it is.
- **Do not create an account, join a study, enroll in the atmospheric-conditions study, or submit data.**
- **Do not probe any app or API endpoint.** None is documented; there is nothing to call and nothing to
  enumerate.

---

## Candidate identity

- **Operator under comparison:** **Visible Health Inc.**, 251 Little Falls Drive, Wilmington, Delaware 19808
  USA, with a wholly-owned UK subsidiary, **Visible Health UK Ltd**, 7 Savoy Court, London WC2R 0EX. The
  vendor's own trading-name sentence is the identity record.
- **Product under comparison:** **Visible** — a cloud-based pacing and symptom app for energy-limiting illness.
  iOS `com.makevisible.visible`, track id 1624474919, **v26.9.3**, first released 2022-08-29, free to
  download, minimum iOS 15.0, 160,522,240 bytes, App Store advisory **12+**, seller "Visible Health Inc.";
  Android `com.makevisible.visible`, **100K+ Downloads**, Play content rating "Everyone".
- **Two tiers.** Free: 60-second phone-camera PPG reading, the daily Stability score, symptom tracking,
  research participation. Paid ("Membership" + **Visible Band 2.0**, a Polar-manufactured device): all-day
  heart rate, **PacePoints** energy budgeting, **real-time pacing notifications**, clinician dashboard.
  "Only available in US and UK."
- **The evidence artifact:** Aitken A, Sawyer A, Iwasaki A, Krumholz HM, Preston R, Calcraft P, Leeming H,
  Tosto-Mancuso J, Proal A, Osborne MA, Putrino D. "Digital physiological biomarkers predict within-person
  symptom changes in complex chronic illness." *npj Digital Medicine* **9**, 257 (2026). DOI
  `10.1038/s41746-026-02543-3`; PMID 41876851; PMC13022203. Received 2024-11-09, accepted 2026-03-03,
  published 2026-03-24, version of record 2026-03-27.
- **The strongest single structural finding:** the paper's biometric input is "a 60-s morning PPG assessment
  with evening symptom severity reports" — i.e. **the free tier's once-daily reading** — while the paid tier's
  headline features are continuous monitoring and real-time alerts, which the vendor's own blog frames as
  what comes "Next".
- **The strongest contractual-vs-marketing finding:** the claim appears on **four pages in two wordings** — the
  FAQ and `/research` say "All your Visible data is stored in an encrypted database and isn't shared with any
  third parties unless you choose to opt in to a research study", while the two help-centre articles say "All
  Visible data is stored in an encrypted database and is not shared with any third parties unless you
  explicitly opt in to a research study". The vendor's own privacy policy documents Meta conversion
  tracking and Meta advertising sharing, and the vendor's own Play Data Safety declaration lists
  **App activity → App interactions** as **shared**, for **Analytics**.

---

## Read first

In this order, and only these. `docs/specs/AMENDMENTS.md` is mandatory before any claim about AnxietyWatch2.

1. `visible_CLAIMED_FEATURES_AND_PURPOSE.md` — claim ledger (V-1…V-64), §6.1 missing-signal rule, §11 age
   contradiction, §12.3 the absolute sentence and its counter-surfaces, §13.2–§13.4 evidence scope and
   algorithm provenance, §16 ledger, §17 inconsistencies, §20 digests
2. `visible_IMPLEMENTATION_INSPECTION.md` — §1–§2 mode and the contract prohibition, §4 static findings,
   §7 the paper's verified defects, §8 algorithm provenance, §9 reconciliation table, §10 inferences
   (N-1…N-5), §12 what would unblock, §13 gaps, §14 machine-lifecycle facts
3. `../../specs/AMENDMENTS.md` — all of it
4. `../PROCESS_STATE.md` — pipeline state, authorization decisions, the machine-lifecycle record
5. `../RESEARCH_METHOD.md` — evidence classes, inspection modes, standing sub-checks, recovery procedure
6. `../CANDIDATE_SHORTLIST.md` — why Visible was moved from "alternatives considered" to candidate 11
   (**the entry was corrected in place; check the dated correction note before citing the older ranking text**)
7. `../PROMPT2-EVIDENCE-FIRST.md` and `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md` — the base and supplementary
   variants
8. `../mindshift/mindshift_COMPARISON.md` and `../oura/oura_COMPARISON.md` — the closest completed comparisons
   in **evidence posture** (a vendor-affiliated paper read as an artifact; a published proprietary contract
   read as a counter-surface) and in **structure and tone**

Do **not** read the AnxietyWatch2 source tree broadly. Read only what a specific claim requires, and cite the
file and line you read.

---

## AnxietyWatch status labels (use these exactly)

| Label | Means |
|---|---|
| **shipped** | code in the tree, with a test, and a consumer |
| **implemented and unwired** | code and tests present, read by nothing |
| **specified** (V2 target) | fully specified in `openspec/` and/or `vitals/registry.toml`, not shipped |
| **not present** | absent from the tree, verified by reading, naming the file or search that establishes it |
| **unknown** | not checked, or checked and inconclusive — say which |

AnxietyWatch2's CNS engine, supervised alarm and hardware dead-man switch are **specified (V2 target)**. Never
write them as shipped. Never write a V2 "does not X" claim without checking the tree first.

**Three V2 facts established for this prompt, so that you do not have to re-derive them and so that you do not
overstate them.** (a) V2 is **registry-driven**: `vitals/registry.toml` is the single source of truth for
sources, types, units, ranges and thresholds, and `vitals/generated/` is regenerated and diffed in CI. (b) V2's
**third state** — measured-and-good / measured-and-bad / **not measured** — is a stated project rule, and the
third state is never rendered, logged or scored as the first. (c) The alarm path is **specified**, not shipped,
and `docs/design/SPEC-admin-console.md` says the product is "explicitly not a medical device". **Record all
three with file:line citations; do not let V2's unshipped status disappear because Visible's shipped alert path
is the opposite case.** Conversely, do not let Visible's genuine positives disappear: it ships a real-time
consumer alert path, it publishes labelled algorithm versions, and it has a peer-reviewed evidence base, none
of which V2 has.

---

## Evidence rules

1. **Six evidence classes, never merged:** `[PUBLIC CLAIM]` · `[DOCUMENTED]` · `[FOUND IN PUBLISHED CLIENT]` ·
   `[PUBLISHED RESEARCH]` · `[INFERENCE]` · `[UNKNOWN]`. Plus `[runtime observation]` where it applies — of
   which this candidate has **none**. Do not manufacture one by describing a help-centre screenshot or a
   documented rule as behaviour.
2. **`[DOCUMENTED]` is the vendor's own description of its own behaviour.** It is neither verified nor
   observed. Never write that Visible does X; write that Visible's documentation states X. The one place this
   package allows a stronger verb is a **static artifact fact** (the served page contains Y).
3. **`[FOUND IN PUBLISHED CLIENT]` is a static class.** The served Consent Mode block is code the vendor ships
   to every browser, read as text. It establishes what the code is *written to do*; it never establishes that a
   consent banner appeared, or that a tag fired. Write it that way, every time.
4. **A vendor-authored, vendor-affiliated paper is not independent validation.** The venue is independent; the
   cohort, the data, the funding context and three of the authors (one of them the founder named on the
   vendor's own About page) are the vendor's. Quote the paper's **own** competing-interest sentence rather
   than characterising it.
5. **An internal contradiction is recorded with both sides quoted and no adjudication.** The paper's
   "People aged ≥18 years" against its own "Participants ranged in age from 13 to 83 years" is a verified
   contradiction. So is the help centre's 16-year floor against its own under-16 guardian path. Do not pick a
   winner; state what each side says and what would resolve it.
6. **A plausible explanation the vendor itself supplies is quoted as an explanation, not adopted as a
   finding.** "Create an account using your own age" is the vendor's own instruction and a mechanism that
   would put a guardian's age in the account field. Record it as a **documented provenance hazard**; do not
   convert it into a conclusion about the cohort.
7. **Quote the most conservative statement when surfaces conflict**, and record the conflict. Where a term is
   undefined — "your Visible data" — say so, and state that any narrow reading is **not stated by the
   vendor**. Do not supply the vendor's defence for it.
8. **A documented missing-signal rule is a positive comparator, not a deficiency.** Visible's "you will not
   receive a Stability Score that day. There is no option to take a morning reading instead" is the closest
   published analogue in this package to V2's third state. **Its limit is equally important**: the third state
   is implemented at the *score* and absent at the *clinician dashboard*, which "only displays sections when
   there is information to share". Record both halves.
9. **An unresolved ambiguity is not a discrepancy.** Where two storefronts report different rating counts or
   different install bands, record both and mark them unresolved; they are different quantities measured
   differently. Do not average, reconcile or subtract them.
10. **A falsified hypothesis is not a finding.** Two hypotheses for this candidate were tested and dropped:
    (a) an arithmetic defect in the published price table — the four stated totals are arithmetically
    consistent, so **no defect is recorded**; (b) an absence of algorithm-versioning practice — the vendor
    demonstrably published v1.1 and v1.2, so **that hypothesis is refuted** and only the narrower
    unlabelled-2026-change finding survives. Do not resurrect either.
11. **A correct quotation is mandatory; a paraphrase wearing quotation marks is the specific failure this
    package exists to prevent.** Every quoted fragment must verify against the retrieved bytes. Run the quote
    checker (described in `../RESEARCH_METHOD.md` §1, with commands and expected results in §9) in **both
    modes** as a separate pre-commit step and report the result, **including a zero**. This candidate's
    quotations cross a help centre, three legal documents, two stores and a peer-reviewed paper, so the
    vendor-only mode matters especially.
12. **Never state float equality.** Use tolerances.
13. **Do not reproduce third-party machine paths or personal identifiers.** The vendor's GitHub forks contain
    third-party open-source code; if a public source comment contains a developer's personal path, cite it as
    "a tester's machine path (redacted here)". Do not name individual non-author employees.
14. **An inspection mode is not an observation.** "Performed — published client, vendor organisation, vendor
    documentation and peer-reviewed publication" means *read*, not *ran*.
15. **Standing sub-checks — apply each, and record the negative results too:** undocumented value domain ·
    write-only field · generated `example` blocks · vendor material that undercuts the headline · two vendor
    surfaces in conflict · intra-page conflict · honest boundary statement · documented default the code does
    not implement · an undefined term in a decisive table · a falsified hypothesis (do not record it) ·
    independent registry artifact · contract/EULA as counter-surface · **an artifact the vendor offers as
    evidence that cannot bear the adjacent claim** · **retrieval failure is not absence** · **HTML-shell
    string is not app-bundle string** · **a count must state its match mode** · **a quote checker must pair
    quotes per paragraph and per table row** · **a copy-pasted sentence on three pages is one statement, not
    three corroborating sources**.
16. **Verify a failure-mode hypothesis against the evidence before recording it.** If you hypothesise what the
    app does under a failure condition and cannot observe the app, record it as an **open question**, not a
    finding. The connectivity statement ("the heart rate reading may not start or complete") is the vendor's
    own sentence about *measurement*, and it is not a test result: do not upgrade it into an observed failure,
    and do not downgrade it into a sync footnote.
17. **A tally a reader cannot reproduce is worse than no tally.** Any verdict count must be machine-counted
    from its own table, not written from impression. The claim ledger's 64 rows are counted; recount them if
    you cite a total.

---

## Candidate-specific unresolved questions

Answer in order. **Q1 and Q2 are the assignment**; the rest are lower value.

| # | Question | Why it matters | What would resolve it |
|---|---|---|---|
| **Q1** | **Does the vendor's published evidence reach the paid product's headline features?** | The paper's input is "a 60-s morning PPG assessment with evening symptom severity reports" — a once-daily reading. The paid tier sells continuous all-day heart rate, **PacePoints** and **real-time pacing notifications**. The vendor's own blog says of the paper's findings: "Next, we'll build on these findings by bringing in continuous monitoring…". So the strongest evidence behind the product validates the **free tier**. | Any published validation of the continuous-monitoring path: an algorithm white paper, a second paper, a device-validation study, or a published method for the PacePoints curve. Report the search **and** its result either way. If nothing exists, record the scope gap as the finding — not as misconduct. |
| **Q2** | **Can the absolute privacy sentence be true as written?** | The claim appears on **four pages in two wordings**: the FAQ and `/research` say "All your Visible data is stored in an encrypted database and isn't shared with any third parties unless you choose to opt in to a research study", while the "What happens to my data?" and "How is my data stored?" help articles say "All Visible data is stored in an encrypted database and is not shared with any third parties unless you explicitly opt in to a research study". The vendor's own privacy policy documents Meta conversion tracking and Meta advertising sharing with marketing consent; the vendor's own Play Data Safety declaration lists **App activity → App interactions** as **shared**, purpose **Analytics**; the Terms say "By using Visible, you give us permission to use your anonymous information to improve the Service for all users." | A vendor definition of "your Visible data", a corrected sentence, a single canonical wording, or a policy statement reconciling the surfaces. Absent that, record all four pages and both wordings with dates, mark unresolved, and state that the narrow reading is **not stated by the vendor**. **Do not call it a false statement** — the term is undefined, and the package records the conflict, not a verdict. |
| Q3 | **Which algorithm version produced a given day's Stability score?** | The vendor published labelled versions (v1.1 on 2023-04-11, v1.2 on 2023-05-30) but the 2026-09-23 change announces that already-recorded scores move ("a day that previously gave you a 4, might now give you a 3") with **no version identifier**, six months after the 2026-03-24 validation. | A published version manifest, a per-record version field, or a change log. Contrast directly with Oura's documented `PublicSleepAlgorithmVersion` enum (candidate 8) and with V2's registry/schema discipline. |
| Q4 | **What is the provenance of the age field, and can the cohort's age range be trusted?** | Four surfaces disagree: Terms 16, help centre 16, help centre under-16-with-guardian, paper ≥18 eligibility against a reported 13–83 range. The vendor's own onboarding instruction ("Create an account using your own age") names a mechanism that would record a guardian's age. | The vendor's own definition of the account age field; a data dictionary; a statement about how under-18 accounts are represented. Otherwise state the hazard and the contradiction and leave both open. |
| Q5 | **What exactly is exported, and what is deliberately withheld?** | The CSV export covers Stability, check-ins, notes, medications, FUNCAP answers (not the final average), breathing minutes and PacePoints, but "the data export does not include your live continuous heart rate data or Heart Rate Zones", and a clinician sees the raw stream only through the vendor's revocable live link. This is the **inverse** of V2's posture and one of the package's clearest §7 axes. | A published export schema, or a vendor statement about why the raw signal is excluded beyond the clinical-sharing route. |
| Q6 | **Does the product have a third state at the presentation layer?** | The score layer does (no score on a missed overnight measurement). The clinician dashboard does not: it "only displays sections when there is information to share". So an absent month is indistinguishable from a month with no data on that surface. | A vendor statement about how the dashboard represents gaps, or a screenshot in the vendor's own documentation showing an empty state. **No screenshot seen at the research date means `[UNKNOWN]`, not absence.** |
| Q7 | **Does Visible measure or merely document the environmental confounder?** | Two surfaces: the PacePoints article tells the user to attribute changes to "changes in the temperature, e.g., there has been recent hot weather"; the `/research` page recruits for "Effect of atmospheric conditions on complex chronic illness" with the quote "Many people notice their symptoms change with the weather. This study helps us investigate how environmental conditions and symptom patterns relate to one another." **No atmospheric data is ingested.** | The study's protocol or results, if published; or any product surface showing weather/air-quality data in the app. This is the sharpest comparison to V2's specified environmental-context work: same hypothesis, different treatment. |
| Q8 | **Is the measurement dependency on connectivity real, and what is its failure mode?** | The help centre says "If you don't [have a strong and stable internet connection], the heart rate reading may not start or complete." That would make a *measurement*, not merely a sync, network-gated. | A runtime observation (requires an authorization decision) or a clearer vendor statement. Until then: `[DOCUMENTED]`, vendor's own words, not tested. |
| Q9 | **What happens to a PacePoints-based alert when the user takes a beta-blocker?** | The help centre states the alert under-fires: "you won't generate as many PacePoints or receive exertion notifications, even if it feels like you are exerting yourself", and can over-fire "even when at complete rest". The mitigation is a manual zone retune by the user. | A vendor statement on automatic detection or a documented alert-quality caveat inside the alert surface itself. Otherwise record the documented degradation, quoted, with no inference about how often it occurs. |
| Q10 | **Is there any third-party health-data ingestion at all?** | The help centre states the negative directly: "We don't currently support HRV data from Apple Health or Garmin and this is something a lot of people have asked for." No Apple Health, Health Connect, CPAP, SpO₂ or sleep-staging support is documented anywhere in the corpus. | Any newer vendor statement, or a release note. Record the documented absence; do not assert the binary lacks code. |
| Q11 | **Is "AI" doing what a reader would assume?** | The vendor's own 2025 blog calls it "a cutting-edge AI algorithm that predicts your susceptibility to crashes"; the published paper reports a walk-forward cross-validated within-person prediction model with "(AUC = 0.82–0.85 vs. 0.73–0.83)". A group-level discrimination statistic is not a per-user accuracy and is not a validated real-time prediction. | The model's own methods (already in hand) plus any vendor statement about in-product prediction claims. Keep the vendor's noun and the artifact's statistic apart. |
| Q12 | **Why is the paper's code-availability statement unusable, and does it matter?** | Verbatim: "The analysis code is available at upon reasonable request." The dangling phrase leaves no repository, archive or contact, so the reproducibility claim has no addressable artifact. | A corrected statement, a repository, or a supplementary file. Otherwise the code's availability is `[UNKNOWN]` and the finding stands as a documentation defect with an evidence consequence. |
| Q13 | **What does AnxietyWatch2 have, or not have, against Visible's genuine positives?** | Visible ships a real-time consumer alert path on continuous heart rate; publishes labelled algorithm versions; has a peer-reviewed evidence base with ethics, competing-interest and limitations sections; publishes an anti-anxiety rationale for its measurement cadence; and offers a clinician-shareable live surface. V2's alarm path is **specified, not shipped**; V2 has no published paper; V2 has no clinician view at this date. | Read the V2 tree for the claims; cite file and line; record each as **specified**, **shipped** or **not present**. Do **not** propose a remediation. |
| Q14 | **Does V2's own documentation classify its evidence honestly, the way Visible's paper's limitations section does?** | The paper publishes its own limitation — "Due to the retrospective study design, limited information was collected on demographics, making the generalizability of the findings uncertain" — and the vendor's blog publishes its own boundary: "Importantly, this doesn't mean a single reading can tell you exactly when a crash is coming…". V2's CNS engine is described as in-progress and the console spec is explicit about the non-medical-device boundary. | Read `openspec/` and `docs/design/` for the framing; cite file and line. This is a **shared positive** and belongs in §6, not buried. |
| Q15 | **What are the two further studies, and does the vendor classify them accurately?** | The vendor's own blog describes an observational survey ("This was an observational, descriptive study… can't establish cause and effect") and a **feasibility** randomised study ("No serious safety issues were reported"; 89% still using the monitor at eight weeks, 66% at six months), and states that randomised controlled trials are what it is "continuing to collaborate with researchers to design". | The two papers themselves if published: their venues, designs, outcome counts and funders. Report both; the vendor's self-classification is a **positive** to record, not to discount. |

---

## Comparison taxonomy for this candidate

Use these dimensions, in this order, so the report lines up with the other candidates' comparisons.

1. **Evidence scope versus product scope** — the paper validates the daily reading; the paid tier sells the
   continuous path. This is the report's spine and the worked example of an artifact that cannot bear the
   adjacent claim.
2. **Evidence attribution** — vendor-affiliated authorship, the founder's presence among the authors, the
   exempt-determination ethics record, the absent registry entry, and the "AI" descriptor against the
   published statistic.
3. **Algorithm provenance** — labelled v1.1/v1.2 against the unlabelled 2026-09-23 change that moves existing
   scores; contrasted with Oura's documented version enum and V2's registry discipline.
4. **Disclosure surface completeness and conflict** — the absolute sentence, its three pages, and its four
   counter-surfaces; retention without a period; anonymised research data that cannot be deleted.
5. **First-party telemetry and consent posture** — what the served client is written to do (Consent Mode:
   opt-in inside the EEA/UK/CH, opt-out outside; GPC denying advertising signals but not analytics storage),
   and the Play Data Safety declaration read for **scope** (the app) against the website behaviour.
6. **Missing-signal posture, layer by layer** — the third state at the score, its absence at the clinician
   dashboard, and the modality-switch rule, applied to both V2 and Visible.
7. **Derived values versus stored signals** — which side of the line each product's export falls on, and what
   a clinician can and cannot obtain.
8. **Real-time, overnight and failure behaviour** — the shipped alert path, the documented medication
   confounder, the documented connectivity dependency, the overnight window, and V2's specified alarm and
   hold-state rule.
9. **Environmental and contextual data** — a documented confounder plus a recruiting study on one side; an
   ingested, stored signal on the other; and the honest statement that the two are at different stages.
10. **Sensor plane and openness** — a vendor-branded Polar-OEM band as the paid gate, against V2's
    device-agnostic, registry-driven ingestion of Polar H10, BLE and CPAP sources; and what each choice costs
    the user.
11. **What transfers and what does not** — the clinical domain is different (pacing for energy-limiting
    illness vs. anxiety/vitals collection); the evidence discipline, the versioning discipline and the
    missing-signal rule are the transferable parts. Be explicit about which is which.

---

## Failure-mode scenarios to walk through

For each: state what the **documented** behaviour is, then state whether that documentation exists at all.

| Scenario | What to establish |
|---|---|
| A user forgets to wear the band overnight | Documented: no Stability score that day, and "There is no option to take a morning reading instead." This is the candidate's strongest positive and the closest published analogue to V2's third state. |
| A user switches between morning-camera and overnight measurement | Documented: "It's not possible to accurately switch between the two [modalities]", and switching creates a **new baseline**. Record as a documented discontinuity, not a defect. |
| A user takes a beta-blocker | Documented under-alerting ("you won't generate as many PacePoints or receive exertion notifications, even if it feels like you are exerting yourself"), documented over-alerting at rest, and a **manual** mitigation. The real-time alert path degrades, and the vendor says so in a help article rather than in the alert. |
| The phone has a poor connection during the morning reading | Documented: "the heart rate reading may not start or complete." A measurement gated on connectivity, in the vendor's own words; **not tested**. Contrast with V2's spool-and-sync design. |
| A clinician opens the shared dashboard for a month with no data | Documented: the dashboard "only displays sections when there is information to share" and shows trackers "logged at least once". Missing data is **omitted**, not marked unknown. This is the scenario where V2's rule is stricter. |
| A user asks for their raw continuous heart rate | Documented: it "can't be exported or downloaded"; the clinician route is a live vendor-hosted link. Derived out, raw withheld. |
| A user closes their account | Documented: access, wellness and wearable data are erased from servers — **but** "In certain cases, such as where you have opted into share your data with third parties for research purposes, data has been anonymised and cannot be deleted as it cannot be linked back to you." |
| A user's score changes after an app update | Documented: the 2026-09-23 change states "a day that previously gave you a 4, might now give you a 3". No version identifier is published, so which algorithm produced a historical score is `[UNKNOWN]`. |
| A user wants Visible data in another app | Documented: no Apple Health, Health Connect, Garmin or Fitbit path; "We don't currently support HRV data from Apple Health or Garmin…". No API, webhook or integration is published. |
| A user's symptoms are affected by heat or weather | Documented as a **user-interpretable confounder** ("You are sensitive to changes in the temperature, e.g., there has been recent hot weather") and as a **recruiting study** on atmospheric conditions. No environmental data is ingested. |
| A study participant withdraws | Documented: "You can withdraw from a study at any time, immediately ending any future data collection", with the irreversibility of already-anonymised data stated separately. Both true; the two statements answer different questions. |
| The vendor's own claim is asked for its source | The paper discloses competing interests and funding; the marketing carries the outcome framing. Record the disclosure asymmetry and leave both surfaces standing. |

---

## Required report structure

Match `../mindshift/mindshift_COMPARISON.md`, `../oura/oura_COMPARISON.md`,
`../empatica/empatica_COMPARISON.md` and `../nightscout/nightscout_COMPARISON.md` — fourteen sections:

1. Executive finding · 2. Product and availability · 3. System boundary and architecture (ASCII) ·
4. Data path and source comparison · 5. Claim-versus-documented-contract reconciliation · 6. Feature matrix ·
7. Derived values versus stored signals · 8. Real-time / overnight / failure-mode · 9. Provenance,
missingness, reproducibility · 10. Local/cloud/privacy and operational cost · 11. Alerting and the alarm
path · 12. User-facing consequences · 13. Limitations, unverified claims and evidence gaps ·
14. Balanced conclusion. Plus a "Related files" footer.

Section 3's ASCII diagram must be drawn from **documented** components only, labelled as such, and must not
invent a server topology the vendor does not describe. Section 11 must carry both directions: V2 specifies an
alarm layer and a hold-state rule that Visible does not describe, and Visible ships a real-time pacing alert
on continuous heart rate that V2 has not shipped.

---

## Tone and claim discipline

- No medical advice; no claim that any product is unsafe, and no claim that a pacing app is harmful.
- **No catastrophising about the privacy sentence.** An absolute sentence with an undefined term and four
  counter-surfaces is a **disclosure conflict**. It is not evidence of unlawful processing, not a regulatory
  conclusion, and not an allegation of intent. State it at the level the evidence supports and no higher.
- **No catastrophising about the paper.** Vendor-affiliated research is normal and the paper is unusually
  well documented, with an ethics determination, a competing-interests statement, a limitations section and a
  data-availability statement. The findings are a **scope gap between evidence and the paid product**, an
  **internal age contradiction** and **a code-availability phrase that names no location** — that is all they
  are. The vendor's own candour is a point in its favour and must be recorded as one.
- **No catastrophising about the unlabelled algorithm change.** The vendor labelled versions before; this
  change is not labelled; the practical consequence is that a historical score's provenance is `[UNKNOWN]`.
  It is a provenance finding, not misconduct.
- **Do not treat the absence of CPAP, SpO₂ or weather ingestion as a deficiency.** Visible does not claim
  them, does not sell them, and is aimed at a different condition. Record them as **scope differences**.
- Distinguish **public claim** from **documented** from **found in the served client** from **already
  published research** from **inferred** in the prose, not only in the tables.
- Where the evidence does not support a conclusion, write that. A recorded gap is a valid result.
- **Do not overstate the comparison in AnxietyWatch2's favour.** Visible ships a real-time alert path V2 has
  not, publishes a peer-reviewed evidence base V2 does not, labels algorithm versions where V2 labels schemas,
  and offers a clinician surface V2 lacks. Say so in §6, §11 and §14. Equally, correct every V2 "does not X"
  claim against the tree before writing it.
- **Do not overstate the comparison in Visible's favour either.** V2's third state reaches the presentation
  layer, its store is append-only, its export posture is signal-first, and its ingestion is device-agnostic.
  Those are real differences and the report must show them without ranking the vendors on character.

---

## Sources to use

**Primary, already retrieved.** Re-verify every digest against
`visible_CLAIMED_FEATURES_AND_PURPOSE.md` §20 before quoting; the artifact record is the authority. The
retrieval root used a scratch directory outside the repository and is **never cited as evidence** — cite the
URL and the digest.

- **Marketing and legal:** `https://www.makevisible.com/` · `/how-it-works` · `/pacing` · `/faqs` ·
  `/join-research` (byte-identical to `/research`) · `/privacy` · `/terms` · `/about` · `/sitemap.xml`
- **Stores:** the iTunes lookup records (US and GB) · the Google Play listing · the Google Play **Data Safety**
  page
- **Help centre (`help.makevisible.com`, 95 articles; 26 read in full):** age requirements · Stability score
  introduction and explanation · overnight measurement and its HR/HRV method · consistent morning
  measurements · normal range · HRV · PacePoints and PaceSetter · pacing notifications · heart-rate-monitored
  pacing · heart-rate-altering medications · symptom-cadence rationale · data export · health report PDF ·
  share with a clinician · what happens to my data · how is my data stored · what data is shared when I enroll ·
  Polar Verity Sense FAQs · what conditions does Visible support · early access hub · your pacing day · camera
  check-in help
- **Publication and registry:** the npj Digital Medicine article · the PMC full text · the Europe PMC API
  record · the ClinicalTrials.gov query for the vendor as sponsor
- **Vendor organisation:** the `makevisible` GitHub org metadata, repo list, and the two compare records
- **Blog history:** the Stability-version posts (2023-01-26, 2023-04-11, 2023-05-30), the overnight/rename
  post, "An even more accurate Stability Score" (2026-09-23), the Armband 2.0 post, the nature-paper post, the
  two-studies post, and the 2025-plans post

**To seek, in priority order:** a published validation of the **continuous** monitoring path or of the
PacePoints curve · a Stability **version manifest** or per-record version field · a vendor definition of "your
Visible data" · any framework, standard or certification claim beyond "an encrypted database" and
encryption in transit · the two further studies once published (venues, designs, outcome counts, funders) ·
the atmospheric-conditions study protocol or results · a published export schema.

**Do not use:** review-aggregator summaries; secondary pages that restate a member count or rating without a
date; anything paywalled; anything that would require bypassing a control to reach; **`/pricing` and
`/science`**, which return genuine 404s and are therefore **verified absent**, not empty; and **`/study` and
`/product-demo`**, which returned ~13 KB JavaScript shells — a retrieval failure, recorded as `[UNKNOWN]`,
never as absence.

**Do not call:** any vendor endpoint. None is documented, and none may be probed. Do not install the app.

---

## Output

`visible_COMPARISON.md` in this directory, following the fourteen-section structure above.

When you finish: run the quote checker on every file you wrote — both the normal corpus and the
`AW2_VENDOR_ONLY=1` mode (re-create the checker from `../RESEARCH_METHOD.md` §1; commands and expected results
in §9) — and report the result **including a zero**, with every MISS triaged into checker artifact, author
paraphrase wearing quotation marks, or a real defect fixed against the vendor bytes. Then update
`../PROCESS_STATE.md` (candidate 11 dossier, residual uncertainty turned into numbered items, the
machine-lifecycle facts, the active-candidate line), `../CANDIDATE_SHORTLIST.md` (Visible's §4 entry, dated)
and `../RESEARCH_METHOD.md` if an inspection mode or a standing sub-check changed. Then run
`scripts/check-public-info.py --self-test` and the full pre-push gate, `git add` the files **by name**, and
commit with the `Co-Authored-By: OpenAI <noreply@openai.com>` trailer. Verify `git show --check` is clean.

**Acceptance is the exit code of the gate, not your own assessment.** Do not mark candidate 11 complete on the
strength of a written report; record the commands you ran and what they returned.
