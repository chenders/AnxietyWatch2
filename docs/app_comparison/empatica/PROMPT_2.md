# Prompt 2 — Empatica EpiMonitor

**Candidate:** Empatica EpiMonitor (and the Empatica Health Monitoring Platform it sits on)
**Base variant:** `../PROMPT2-EVIDENCE-FIRST.md`
**Supplementary block:** `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md`
**Chosen because:** this candidate's central question is not what the product does — that is largely
documented — but **whether the vendor's published evidence supports the vendor's published claims**, and at
what contractual and operating cost. Empatica is the first candidate in the package where an **independent
regulator's record of the vendor's own submission** is the strongest available artifact, and the first where
a **licence** is a primary evidence surface. That is what `PROMPT2-EVIDENCE-FIRST.md` is built for. The
privacy/operating-mode block is added because the alert path is cloud-mandatory and subscription-gated, so
*where does the data live and what happens when it stops* is a first-order question, not a secondary one.

**Why not the other variants.** `PROMPT2-TECHNICAL-SYSTEMS.md` (used for `sleephq` and `nightscout`) assumes
an inspectable implementation surface — source, a published API, or a runnable binary. **Empatica has none of
the three**, so a systems-internals brief would generate speculation. `PROMPT2-USER-VALUE.md` is a poor fit
because the user value proposition is not in dispute: EpiMonitor does something few products do, and its user
is clearly served when it works. The open question is the evidence and the contract, not the value.

---

## Assignment

Produce a second-stage comparison of **Empatica EpiMonitor** against AnxietyWatch v1 (`../AnxietyWatch`) and
AnxietyWatch2 (`./`) driven by **traceability**: every performance claim either reaches a primary artifact or
is recorded as unreachable.

`empatica_CLAIMED_FEATURES_AND_PURPOSE.md` is complete and is the starting point, not a draft to redo. Close
the gaps named in `empatica_IMPLEMENTATION_INSPECTION.md` §8 in the order given, then answer the questions
below. **Closing gaps G1 and G2 is worth more than everything else in this prompt.**

**Do not present anything from the vendor's marketing pages as verified.** Nothing about this candidate's
runtime behaviour was observed. There are no runtime observations for `empatica` and you must not create the
impression that there are.

**Hard limits, unchanged from PROMPT1.** This is an authorized research brief, not permission to act.
**No DRM, authentication, licensing, encryption, certificate pinning, anti-debugging or platform protection
may be bypassed.** **No credentials or private data may be extracted.** **No vendor endpoint may be called
without an explicit authorization decision recorded in `PROCESS_STATE.md`.** Do not create an EpiMonitor
account, start a subscription, obtain a prescription, or enter a caregiver's contact details. Do not
intercept or decrypt traffic from the app.

---

## Candidate identity

- **Vendor:** Empatica Inc. (1 Broadway, 14th floor, Cambridge MA 02142) and **Empatica S.r.l.** (Via
  Stendhal 36, 20144 Milano) — the regulatory applicant and the App Store seller are the **S.r.l.**
- **Product under comparison:** **EpiMonitor** — EmbracePlus wearable **$399** plus a mandatory subscription
  (**$17.40 / $26.90 / $52.90** per month), prescription-gated in the US, clearance product code **POS**.
- **Platform under comparison:** the **Empatica Health Monitoring Platform**, sold to clinical-trial sponsors
  and CROs, with a **Cloud API** whose documentation is supplied only to clients.
- **Adjacent, not the subject:** EmbraceMini (K252981, sleep assessment), the Parkinson's monitoring line
  (K260374, tremor transducer), Actigraphy, eCOA, Digital Biomarkers, the **Empatica Care** app.
- **Legacy, withdrawn:** Embrace2 (no longer available for purchase); **E4 and its software suite** (sunset
  2025-02-14; documentation host emptied 2025-02-18).
- **Publisher of the primary artifact:** FDA — openFDA 510(k) records and product-code classification. The
  **K250515** 510(k) summary is the single most important document for this candidate.

---

## Read first

In this order, and only these. `docs/specs/AMENDMENTS.md` is mandatory before any claim about AnxietyWatch2.

1. `empatica_CLAIMED_FEATURES_AND_PURPOSE.md` — evidence ledger (E-1…E-83), conflict table, §12
2. `empatica_IMPLEMENTATION_INSPECTION.md` — §5 reconciliation points, §7 claim reconciliation, §8 gaps
3. `../../specs/AMENDMENTS.md` — all of it
4. `../PROCESS_STATE.md` — pipeline state, residual uncertainty, active-candidate line
5. `../RESEARCH_METHOD.md` — evidence classes, inspection modes, standing sub-checks
6. `../CANDIDATE_SHORTLIST.md` — why this candidate is ranked where it is
7. `../PROMPT2-EVIDENCE-FIRST.md` and `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md` — the base and supplementary
   variants
8. `../sleephq/sleephq_COMPARISON.md` and `../nightscout/nightscout_COMPARISON.md` — closest completed
   comparisons, for structure and tone

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

---

## Evidence rules

1. **Five evidence classes, never merged:** `[PUBLIC CLAIM]` · `[REGULATOR RECORD]` · `[THIRD PARTY]` ·
   `[INFERENCE]` · `[UNKNOWN]`. Plus `[runtime observation]` where it applies — of which this candidate has
   **none**. Do not manufacture one by describing a vendor screenshot as behaviour.
2. **A documented contract is not observed behaviour.** The Cloud API's Avro/CSV-in-S3 description is a
   *documented* property. Record it as documented, never as working.
3. **A regulator's document is a check, not a verdict on the vendor.** K250515 reporting PPA and FAR does not
   make the vendor's 98% claim false. It makes it untraceable from the page. Keep those apart.
4. **Quote the most conservative statement when vendor surfaces conflict**, and record the conflict. The
   eight conflicts are tabulated in the claimed-features deliverable — use them, do not re-litigate them.
5. **Absence of evidence is `[UNKNOWN]`.** The Android listing, the support article, the security.txt and the
   Cloud API contract are `[UNKNOWN]`, not "missing" and not "does not exist".
6. **A vendor's EULA is a quotable counter-surface, not a scandal.** Report it as two instruments by the same
   author; do not imply bad faith. Mark the class difference — a **contract** is not a marketing page — which
   matters more than the fact of the disagreement.
7. **Never present a vendor-authored study as independent validation.** The EpiAlgo analyses in K250515 are
   the vendor's own submission published by a regulator: the *venue* is independent, the *work* is the
   vendor's. Say it that way.
8. **Never upgrade a preprint to peer-reviewed**, and never present the seizure-forecasting study as a
   feature — it is a study the vendor is recruiting for, and the cleared indication excludes prediction.
9. **Do not convert correlation into causation.** No *because Empatica does X, Y follows* without a citable
   mechanism.
10. **Never state float equality.** Use tolerances.
11. **Do not reproduce third-party machine paths or personal identifiers** found in public artifacts; cite as
    "a tester's machine path (redacted here)".
12. **An inspection mode is not an observation.** "Performed — artifact and regulator record" means *read*,
    not *ran*.
13. **Standing sub-checks — apply each, and record the negative results too:** undocumented value domain ·
    write-only field · generated `example` blocks · vendor material that undercuts the headline · two vendor
    surfaces in conflict · intra-page conflict · honest boundary statement · documented default the code does
    not implement · an undefined term in a decisive table · a falsified hypothesis (do not record it as a
    finding) · independent regulator artifact · EULA as counter-surface · **an artifact the vendor offers as
    evidence that cannot bear the adjacent claim** (new for this candidate).
14. **Verify a failure-mode hypothesis against the evidence before recording it.** If you hypothesise what the
    app does under a failure condition and cannot observe the app, record it as an **open question**, not a
    finding. Falsify your own hypothesis and drop it silently if the evidence contradicts you.

---

## Candidate-specific unresolved questions

Answer in order. **Q1 and Q2 are the assignment**; the rest are lower value.

| # | Question | Why it matters | What would resolve it |
|---|---|---|---|
| **Q1** | What is the basis of the vendor's "98% detection accuracy" claim? | It is the headline of the product page and is not reconstructible from K250515, which reports PPA/FAR, states "No clinical studies have been performed on the subject device", and derives real-world figures from **Embrace2** hardware. Either a source exists, or a medical device page makes an untraceable performance claim. | K181861's clinical study; a peer-reviewed paper reporting the figure; the vendor's **Publications** page; a literature search. Report the search and its result either way. |
| **Q2** | Does anything the vendor publishes tell the wearer **not to rely on the alert alone**? | The cleared product code's own definition says the device is "not intended as a standalone seizure monitoring device". That statement is absent from the four marketing pages read, and the surface most likely to carry it — the User Manual — is **image-only with no text layer**. The candidate's top evidence gap and the most direct question the comparison has about the alert layer. | OCR of the manual, a text-format manual, or a quick-start/insert/app screen reproduced as text. **If OCR is attempted, record the tool, its version and the pages read, and mark extracted text as OCR output, not as the vendor's verbatim text.** |
| Q3 | What does "corrected PPA" mean in K250515 Tables 1 and 3? | An undefined term in the table carrying the performance claim, and the corrected figures are materially lower than the raw ones (1.000 → 0.905; 0.87 → 0.86; 0.80 → 0.77). An undefined term in a decisive table is a recordable finding. | The submission's statistical methods section, or a published methods paper. |
| Q4 | What is the empirical false-alarm burden, and how does it compare with what AnxietyWatch2 specifies? | FAR is the number the vendor's page omits. Paediatric real-world FAR of 0.34/24 h implies roughly one false alert every three days, and the optional low-sensitivity mode exists to reduce it. AnxietyWatch2's supervised-alarm design has **not stated an acceptable false-alarm rate for any alert type** — this comparison is where that gap becomes visible. | The FAR tables (already extracted); a V2 statement of an acceptable false-alarm rate, searched in `openspec/` and the tree, and recorded as **not present** or **unknown** if absent. |
| Q5 | Is EpiMonitor available on Android, and how stale is the app? | The vendor claims Android; no Play listing was located; the iOS EpiMonitor app is ~6.5 months old at the research date while the companion Empatica Care app is current. For a safety-relevant alerting app, update tempo is a user-facing property. | The Play listing, or a vendor statement. Record `[UNKNOWN]` if not found, and state that the 404s are not evidence of absence. |
| Q6 | What does the vendor say happens when the phone is out of range, Bluetooth drops, or the subscription lapses? | The watch records on-board and syncs later, so a phone absence may be a delay rather than data loss — but the alert path is described as requiring the phone, Bluetooth **and** internet. Whether the user is told, and how, is a failure-mode question the comparison must answer for the alert layer. | The manual (image-only), the app's own screens, or a support article. **Open question** unless a text artifact is found. |
| Q7 | How does AnxietyWatch2's alarm-layer specification compare with a cleared Class II device on **false-alarm handling, cancellation and user-facing sensitivity control**? | EpiMonitor ships three mechanisms AnxietyWatch2's specification does not visibly have: a user cancellation window before caregivers are contacted, adjustable sensing precision, and an optional low-sensitivity mode. V2's specified alarm layer must be read against these rather than assumed adequate. | Read `openspec/` alarm/CNS specs and the registry; cite file and line; label anything not found as **not present** with the search that establishes it. |

---

## Comparison taxonomy for this candidate

Use these dimensions, in this order, so the report lines up with the other candidates' comparisons.

1. **Alarm-path topology** — device → app → cloud → caregiver, versus AnxietyWatch2's intended local path.
2. **Evidence quality per claim** — for every performance figure, which artifact carries it and whether it
   is traceable.
3. **False-alarm economics** — FAR per 24 h, cancellation, sensitivity modes, and what an acceptable rate is
   (or whether V2 has stated one).
4. **Missing-data and absent-signal posture** — wearing detection as a first-class signal; the three-state
   rule (`measured-good` / `measured-bad` / **not measured**) applied to each side.
5. **Regulatory posture and what it costs** — prescription, subscription, plan-gated caregiver count, cleared
   indication limits, the standalone caveat.
6. **Contract versus promise** — what the EULA commits to versus what the page says.
7. **Data location and operating mode** — cloud-mandatory, account-mandatory, no offline mode; privacy notice
   scope and its ambiguities.
8. **Longevity and upstream dependency** — the E4 withdrawal and documentation deletion as the concrete
   instance of *your platform's contract can vanish*.
9. **Derived measures** — 300+/200+ "digital measures" unenumerated; no published units, ranges or windows;
   contrast with `vitals/registry.toml` as a single source of truth.
10. **What transfers and what does not** — physiology is different; architecture and evidence discipline are
    the transferable part. Be explicit about which is which.

---

## Technical-systems supplementary block

This candidate has no inspectable implementation, so this block is **evidence-oriented by necessity**. Do
not fill it with speculation; mark unknowns as unknown.

- **No runtime observation is available and none may be implied.**
- **Static findings are limited to documents:** the EULA, the US privacy notice, `/legal/`, `/cloud-api/`,
  `/manuals/`, the marketing pages, and the FDA record.
- **The one measurable artifact finding** is the manual's text-layer test (`pdffonts` empty, `pdftotext`
  returns 178 bytes across 178 pages). If you re-run it, report the tool versions.
- **A vendor-published API reference does not exist.** If you find one, that changes this candidate's
  inspection mode and must be recorded as a mode change in `RESEARCH_METHOD.md`.
- **Three licensed instruments, not one:** the EULA, the ISO 13485 certificate, and the ISO/IEC 27001:2022
  statement. Keep their claim classes separate.

---

## Failure-mode scenarios to walk through

For each: state what the **documented** behaviour is, then state whether that documentation exists at all.

| Scenario | What to establish |
|---|---|
| Phone out of range (> 15 ft) | Documented requirement exists (15 ft); what the user is told does not. **Open question.** |
| Bluetooth disconnect | Same as above. |
| No internet on the phone | The EULA names "loss of Internet connectivity" as a failure it does not warrant. Whether the app signals it is **open**. |
| Subscription lapses | Store page: the device requires an active subscription. Whether alerts stop, and whether the user is warned, is **not published** here. |
| Detection false positive | Documented cancellation window before caregiver contact; **length not published**. |
| Seizure the algorithm does not detect | Documented scope limit (non-motor seizures). Whether the user is told the limit at the point of use is **open** (manual unreadable). |
| Vendor infrastructure outage or withdrawal | The E4 sunset is the worked example: software "rendering services inoperative". For EpiMonitor, no continuity commitment was found. |
| Alert delivered but caregiver does not respond | The EULA: the vendor "cannot verify or influence and shall not be liable in respect of any conduct of the Caregiver(s)" and the user indemnifies the vendor. |
| Watch not worn | Wearing detection is published as a platform feature; whether it gates alerts is not published. **Open.** |
| Claim has no traceable source | Q1. The failure mode of the *evidence*, and the one this candidate exists in the shortlist to illustrate. |

---

## Required report structure

Match `../nightscout/nightscout_COMPARISON.md` and `../sleephq/sleephq_COMPARISON.md` — fourteen sections:

1. Executive finding · 2. Product and availability · 3. System boundary and architecture (ASCII) ·
4. Data path and source comparison · 5. Claim-versus-documented-contract reconciliation · 6. Feature matrix ·
7. Derived values versus stored signals · 8. Real-time / overnight / failure-mode · 9. Provenance,
missingness, reproducibility · 10. Local/cloud/privacy and operational cost · 11. Alerting and the alarm
path · 12. User-facing consequences · 13. Limitations, unverified claims and evidence gaps ·
14. Balanced conclusion. Plus a "Related files" footer.

---

## Tone and claim discipline

- No medical advice; no claim that any product is unsafe; no claim that a clearance is improper.
- No catastrophising. A bench-equivalence clearance is a **lawful and common** route. The finding is about
  what a marketing page does and does not disclose about it.
- No unsupported superlatives, in either direction. *The only candidate in the package with an independent
  alert path* is checkable and fine; "the most trustworthy" is not.
- Distinguish **observed** from **documented** from **inferred** in the prose, not only in the tables.
- Where the evidence does not support a conclusion, write that. A recorded gap is a valid result.
- Do not overstate the comparison in AnxietyWatch2's favour. If V2 lacks something this candidate has — the
  cancellation window, the sensitivity modes, the wearing-detection signal, the standards list — say so
  plainly in §6 and §11.

---

## Sources to use

**Primary, already retrieved (re-verify the hashes before quoting):**

- openFDA `device/510k.json` — all nine Empatica records
- openFDA `device/classification.json` — POS, DQA, MWI, LEL, GYD definitions
- FDA **K250515** 510(k) summary — 13 pages; the extracted text is 602 lines and contains Tables 1–4
- EULA at `s3.amazonaws.com/box.empatica.com/embrace/alert/agreements/eula/0.2-black.html`
- US privacy notice; `/legal/`; `/cloud-api/`; `/manuals/`; `/embracemini/`; `/embraceplus/`; `/epimonitor/`;
  `/store/epimonitor/`; `/epimonitor/seizure-forecasting/`; `/research/e4-sunset/`
- GitHub API: `empatica/empatica.github.io` commits and tree; `orgs/empatica/repos`
- Apple lookup API: `com.empatica.healthmonitor.alert`, `com.empatica.healthmonitor`

**To seek, in priority order:** the vendor's **Publications** page; the **K181861** summary and its clinical
study; a peer-reviewed paper reporting the 98% figure; a text-format or OCR-able manual; the EHMP **Data
Processing Agreement** (listed on `/legal/`, not yet fetched); the Cloudflare-blocked subscription-plans
article; a Play listing.

**Do not use:** aggregator or review-site summaries of the FDA record; secondary pages that restate the
vendor's 98%; anything paywalled; anything that would require bypassing a control to reach.

---

## Output

`empatica_COMPARISON.md` in this directory, following the fourteen-section structure above.

When you finish: update `../PROCESS_STATE.md` (candidate 7 dossier, residual uncertainty turned into numbered
items, active-candidate line) and `../RESEARCH_METHOD.md` if the inspection mode or a standing sub-check
changed. Then run `scripts/check-public-info.py --self-test` and the gate, `git add` the files **by name**,
and commit with the `Co-Authored-By: OpenAI <noreply@openai.com>` trailer. Verify `git show --check` is clean.

**Acceptance is the exit code of the gate, not your own assessment.** Do not mark candidate 7 complete on the
strength of a written report; record the commands you ran and what they returned.
