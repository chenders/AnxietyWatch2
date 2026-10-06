# Prompt 2 — OSCAR versus AnxietyWatch v1 and AnxietyWatch2

**Candidate:** `oscar` — OSCAR, the Open Source CPAP Analysis Reporter
**Base variant:** `../PROMPT2-EVIDENCE-FIRST.md`
**Supplementary block:** technical-systems questions, drawn from `../PROMPT2-TECHNICAL-SYSTEMS.md`
**Chosen because:** OSCAR's comparison value is almost entirely a question of *evidence quality* — what a
device-reported number actually means, which invalidity policies a pipeline applies, and how far a
conclusion can travel from a decoded sample. It is also the only candidate whose implementation is fully
readable, so the evidence-first framing can be pushed further here than anywhere else on the shortlist.

---

## Assignment

Assess OSCAR against AnxietyWatch v1 and AnxietyWatch2, using the completed evidence base in this
directory. OSCAR is **not** a competing consumer product and the report should not pretend otherwise.
It is two other things, and the report must keep them separate:

1. **An upstream dependency of AnxietyWatch v1.** v1 imports OSCAR's CSV output, adopts OSCAR's
   noon-to-noon day boundary, and documents its own CPAP parser as based on OSCAR's reverse engineering.
   Defects in OSCAR's leak and AHI arithmetic propagate into v1's CPAP conclusions.
2. **A reference implementation, in GPLv3 source, of a device-data pipeline.** It shows, in one
   codebase, both the invalid-data anti-pattern this project forbids and the correct pattern — and the
   correct one is the newer code.

The deliverable should let the owner decide three things: whether OSCAR belongs in the comparison at all
given it is closer to a dependency than a rival; whether v1's OSCAR-derived CPAP numbers need a
qualification added where leak or AHI is used; and whether any of OSCAR's design decisions are worth
adopting in AnxietyWatch2.

---

## Candidate identity

| Field | Value |
|---|---|
| Name | OSCAR — Open Source CPAP Analysis Reporter |
| Slug | `oscar` |
| Vendor | The OSCAR Team (voluntary); donations to Apnea Board |
| Licence | GNU GPL v3 |
| Version assessed | 2.0.1, released 2026-06-23 |
| Source inspected | `gitlab.com/CrimsonNape/oscar-sql` @ `28df2e46e35e6260e186fced997edadcf001cc24` (2026-10-04) |
| Platforms | Windows 10/11, macOS 13+/15.7+, Debian 12/13, Ubuntu 22.04/24.04/26.04, Chromebook, RasPiOS 12 |
| Price | Free; no account, no tier, no upsell |
| Inspection mode | **Performed** — full source read; no runtime observation |
| Shortlist rank | 4 of 10 |

---

## Read first

In this order, and completely:

1. `oscar_CLAIMED_FEATURES_AND_PURPOSE.md` — especially §13, the v1 dependency section, and §14, the
   limitations OSCAR states about itself.
2. `oscar_IMPLEMENTATION_INSPECTION.md` — especially §5.1 (the CSV correspondence), §5.2 (leak
   percentile), §5.3 (the two opposed invalidity policies), §5.5 (refusing to print unmeasurable
   numbers), and §7 (claim reconciliation).
3. `../PROCESS_STATE.md` — the sequencing rules and the residual-uncertainty convention.
4. The v1 evidence, in the v1 tree: `REQUIREMENTS.md` (the CPAP row and the parser checklist item),
   and the tests `CPAPBySessionImportTests.swift`, `CSVImportRouterTests.swift`,
   `ModelEnumTests.swift`.
5. The v2 evidence: `docs/specs/AMENDMENTS.md` (mandatory), then the AirSense 11 changes under
   `openspec/changes/` — `resmed-air11-ble-stream`, `cpap-ezshare-ingest`, `xe125-respiratory-rate`,
   `pi-prototype-storage-degraded-mode`.

Do not re-run the inspection from scratch. The pinned commit and the file/line references are in the
inspection document; go to the source only to resolve a specific question the documents leave open.

---

## AnxietyWatch status labels (use these exactly)

Apply these labels to every AnxietyWatch feature mentioned. Do not blur them.

| Label | Meaning |
|---|---|
| **v1 documented** | Described in the v1 repository's own documents |
| **v1 implemented** | Present in v1 source or asserted by a passing v1 test |
| **v2 current** | Implemented in the AnxietyWatch2 tree today |
| **v2 specified target** | Fully specified in `openspec/` but **not shipped**. Never describe as shipped. |
| **v2 aspiration** | Directional only, not fully specified |

Known traps in this comparison specifically:

- The supervised alarm and the hardware dead-man switch are **v2 specified target**, not shipped.
- V2's AirSense 11 BLE streaming is a specified change; verify its current status in `AMENDMENTS.md`
  and the relevant `tasks.md` before describing it as working.
- V1's Claude-based AI analysis is **v1 documented** and opt-in. Do not describe it as a v2 feature,
  and do not describe v2 as having an AI feature.
- OSCAR's own claims are labelled **documented** unless the inspection verifies them. Only the pinned
  source is evidence of implementation.

---

## Evidence rules

The standard set from `PROMPT2-EVIDENCE-FIRST.md` applies, with these emphases for this candidate:

1. **Separate the three relationships.** Dependency (v1 reads OSCAR's output), equivalence (two
   implementations of AHI or leak), and comparison (what each product does for a user). Do not let a
   dependency finding leak into a feature comparison.
2. **A vendor's internal document is the vendor's statement.** The leak-percentile review, the schema
   design philosophy and the bug log are OSCAR's own findings, quoted with dates. They are not
   independent validation of anything, and their numbers were not re-measured here.
3. **Reported-then ≠ present-now.** The leak review is dated 2026-04-16. The *method* was confirmed in
   the current tree; the specific defects were not re-confirmed. Say which is which.
4. **Source inspection is static.** No OSCAR binary was executed. Never write "OSCAR does X at runtime"
   — write "the code at line N does X".
5. **Do not upgrade a comment into a behaviour.** OSCAR's code comments are unusually candid and are
   excellent evidence of *intent and awareness*; they are not evidence of what a build does.
6. **Absence of evidence is unknown.** OSCAR has no BLE stack in the pinned commit. That is a strong
   static finding about that commit. It is not proof about every past or future OSCAR build.
7. **Never present a v1 defect as a v1 failure of diligence.** V1's CPAP path rests on community
   reverse engineering because there is no other lawful route to the data. If the numbers carry an
   inherited uncertainty, say so as a property of the route, not as a criticism.
8. **Keep the leak finding proportionate.** The 2.0× case is one night in one tester's profile,
   reproduced by the vendor, explained by a method choice. It is a reason to qualify a leak-based claim,
   not a reason to declare OSCAR's leak data wrong.

---

## Candidate-specific unresolved questions

Answer each with a labelled finding and, where possible, a file/line reference or a named source. If a
question cannot be answered from the material, say so and stop — do not speculate.

**O1 — Where exactly does v1 consume OSCAR, and how much of v1's CPAP model is inherited?**
Enumerate every v1 touchpoint: the source enum, the importer, the format detector, the day boundary, the
unscored-hours rule, the report generators, the server round-trip. For each, state whether it is a
*vocabulary* dependency (a header string, a date convention) or a *semantic* one (a definition of AHI,
of a session, of leak). The distinction determines how much damage an OSCAR defect can do.

**O2 — Is the CSV correspondence exact, and where can it break?**
The inspector verified the Summary header correspondence. Check the **by-session** header
(`date,start,ahi,rdi,oa,...`) that v1 also detects, against the same exporter in the pinned commit. Does
the OSCAR exporter emit that column set in that order? What happens to v1's detector if OSCAR's column
list changes, and what happens on a localised build? Report the mechanism, not a test result.

**O3 — What does the leak finding actually mean for v1's numbers?**
AnxietyWatch v1 supports an "EDF leak parsing" path as well as OSCAR CSV. Establish which of v1's leak
values come through OSCAR and which do not. If leak-based correlations can be computed from either path,
the qualification needed is narrower than if they cannot. Be precise; do not apply the caveat more widely
than the evidence supports.

**O4 — Does AnxietyWatch2 inherit any of the leak or AHI uncertainty?**
Trace V2's AirSense 11 path: does it read the device's own reported percentile, compute its own, or read
EzShare/cloud values? Cite the OpenSpec change and the registry where relevant. The expected answer is
that it does not inherit OSCAR's, but the *reason* matters and must be evidenced, not assumed.

**O5 — The two invalidity policies: which one does each AnxietyWatch version resemble?**
Place both versions on the same axis. V2's rule is stated in `AGENTS.md`; check whether V2's actual
decoders follow it, and whether any V2 path holds a value over in the way `resmed_loader.cpp` does. If a
V2 path does, that is a finding about V2 and should be reported with the same directness as the finding
about OSCAR. V1's handling is likely closer to OSCAR's — say so if the evidence supports it.

**O6 — Is "quarantine" a real requirement, or is stderr logging enough?**
OSCAR rejects correctly and logs transiently; the project's rule asks for a named, durable quarantine.
Check what V2 does with a rejected sample — is there a tombstone, a counters table, a quarantine
channel, or a log line? Then state plainly which of OSCAR's approach and V2's approach better satisfies
the rule, and whether the rule as written is actually implementable in V2's store.

**O7 — The `device_time_corrections` precedent: does V2 need it?**
V2 stores UTC epoch seconds and nothing else. OSCAR stores device-local time plus a typed, reasoned,
reversible correction ledger with a linear drift model. Identify the concrete failure V2 would
experience without such a ledger (an AirSense 11 with a drifting clock, a device that crosses a DST
boundary, a device whose clock was reset during a night) and say whether V2's UTC rule plus the
collector's own timestamping is sufficient. Recommend nothing; identify the gap or confirm its absence.

**O8 — Which OSCAR decisions are worth adopting, and which are actively wrong for V2?**
Separate them into two lists with reasons. Likely adoptions: capability-gated channels, refusing to print
an unmeasurable `0.00`, `summary_only` as an explicit fidelity marker, per-sample source attribution,
deterministic source selection that aborts on unresolved ambiguity. Likely rejections: destructive purge,
no-migration upgrades, unencrypted profile storage, hold-last-value substitution, localised output
headers. Argue each from V2's own stated rules, not from general preference.

**O9 — Is there a case for OSCAR being a V2 *output* target rather than an input?**
V2 decodes the AirSense 11 itself. OSCAR exports CSV and its database is publicly documented. Could V2
emit something OSCAR can read, so a user can review V2's data in OSCAR's graphs? Assess feasibility and
whether the GPL creates any obligation. Note that emitting a CSV is not the same as linking GPL code.
State the assessment; do not recommend an implementation.

**O10 — What is OSCAR's honest position on the shortlist?**
Given that it is a free desktop review tool, not a consumer health product: is it a competitor, a
complement, a dependency, or a reference? Defend the classification, and state what would change it.

**O11 — What does OSCAR's process record tell us about V2's own?**
OSCAR has 473 test assertions, `-Werror`, AddressSanitizer on Linux/macOS, a published schema philosophy,
a 6,748-line dated bug log with root causes — and no CI. Compare that with V2's CI, pre-push gates,
`Verify:` contract lines and amendment index. Which of the two processes would have caught the v7
weighted-average defect, and which would have caught OSCAR's silently-lost journal entries? Answer from
the mechanisms, not from preference.

**O12 — What would close the remaining gaps?**
Name the smallest set of additional evidence that would move the material findings from *reported* or
*static* to *observed*: for example compiling OSCAR at the pinned commit and running the Apple Health
tests, or importing a known-good Apple Health export and checking the summary against the accepted
sample count. List them; do not execute them.

---

## Comparison taxonomy for this candidate

Adapt the standard taxonomy rather than reusing it verbatim — OSCAR is a different kind of product.

### 1. The dependency boundary (the centrepiece)

Not a feature comparison. Establish what v1 inherits, what each inherited element means, and how far a
defect can propagate. Include the vocabulary-vs-semantic distinction from O1 and the break conditions
from O2.

### 2. Evidence quality of the decoded numbers

Leak percentile, AHI, unclassified apnoea, percentile method, gain handling, day aggregation, cache
invalidation. For each: who computes it, from what, with which known limitations, and which of the three
AnxietyWatch states it can be confused with.

### 3. Invalid-data and missing-data handling

The two-policy split in OSCAR; V2's rule and whether the code follows it; whether V1 has an equivalent
rule at all; how "not measured" is represented in each of the three systems' stores.

### 4. Time

OSCAR's device-local time plus correction ledger; V1's storage model; V2's UTC-epoch-seconds rule. The
AirSense 11 clock as the shared hazard. Holds whether this axis is already handled or is a live gap.

### 5. Collection, real-time and response

Where OSCAR is categorically absent, and what that absence proves: no alarm, no panic-time capture, no
overnight monitoring. Be careful to treat this as an architectural fact about OSCAR rather than a
deficiency — it is a review tool, and it claims nothing else.

### 6. Interoperability and exit

OSCAR's published schema, Python reader and CSV templates versus the closed candidates' absence of any
escape hatch. The one axis where OSCAR is the strongest product on the shortlist.

### 7. Operating model

Free, GPL, unfunded, no account, no telemetry, no cloud, community-governed, single-maintainer fork for
the 2.0 line with no CI. What that predicts about durability, and what it does not.

---

## Technical-systems supplementary block

Answer these in addition to the taxonomy, with file/line evidence from the pinned commit.

**T1 — Storage shape.** OSCAR stores dense waveforms as compressed, CRC-checked BLOBs plus materialised
daily caches; V2 stores sparse scalar samples append-only with tombstones. State what each shape makes
easy and what each makes impossible. Specifically: can OSCAR answer a question about a night without
loading the waveform? Can V2?

**T2 — Derived-value correctness.** OSCAR's schema v7 exists because a weighted average silently became
a simple average. V2 computes derived values too. Identify where V2's derived values are computed, whether
they are cached, and what invalidates the cache. Is there an equivalent silent-wrong-value exposure, and
what mechanism prevents or fails to prevent it?

**T3 — Capability gating.** OSCAR gates channels on what the device can measure, with the
`oahiChannels = ahiChannels − cahiChannels` identity enforced by construction, and hides statistics a
device cannot produce. V2 uses a generated registry (`vitals/registry.toml` → generated enums). Compare
the two mechanisms: what does the registry make impossible that OSCAR's approach leaves to convention,
and vice versa?

**T4 — Fidelity markers.** `summary_only`, `no_settings`, `events_loaded`, `enabled` on OSCAR sessions
versus V2's degraded-mode specification. Map each OSCAR flag to a V2 equivalent or record its absence.
Which store is better able to say "this session is incomplete, and here is in what way"?

**T5 — Import idempotence.** OSCAR skips an already-present night and counts it; V2 has dedup
requirements. Compare the two models of re-import — what makes a night "the same night" in each, and
which one can double-count under a scenario the other cannot?

---

## Failure-mode scenarios to walk through

For each, describe what OSCAR would do, what v1 would do, and what V2 would do, citing the mechanism.

1. The AirSense 11 records two sessions on one night, the second beginning within 60 seconds of the
   first ending. (OSCAR's release notes record a fix for this class; V1's importer has an unscored-hours
   rule.)
2. The CPAP's clock has drifted by 40 minutes over three months. (OSCAR has a typed `drift` correction
   with an intercept and rate. What does V2 have?)
3. A ResMed EDF ends with padding that decodes to `-1`, and the padded region falls inside a session.
4. A HealthKit export contains an SpO2 sample of `0` because the watch could not get a reading.
5. A HealthKit export contains sleep stages from two sources, one of which is not the watch.
6. The same HealthKit export is imported twice.
7. A user upgrades an application and the stored schema no longer matches. (OSCAR: fresh database and
   reimport. V1 and V2: state the mechanisms.)
8. A leak percentile is computed from the same channel across two sessions in one day with different
   gains.
9. A night has CPAP data but no oximetry, and the report wants to print both.
10. A user asks for the last two years of their data in a portable form.

---

## Required report structure

Produce `oscar_COMPARISON.md` with:

1. **Executive finding** — the dependency classification, the two-policy finding, and the verdict on the
   shortlist position, in that order.
2. **Product and availability** — identity, licence, platforms, price, funding, inspection mode.
3. **The dependency boundary** — O1, O2, O3 and what they imply for v1's numbers.
4. **Evidence ledger** — every load-bearing claim with a source, an evidence class and a date.
5. **Feature matrix** — OSCAR vs v1 vs v2, with the status labels, and an explicit "not applicable"
   column for axes where a review tool has no analogue.
6. **Evidence-backed workflows** — the realistic user tasks both versions support, and where OSCAR sits
   in each.
7. **Data and analysis audit** — taxonomy 2–4, plus the technical-systems answers T1–T5.
8. **Local/cloud/privacy and operating cost** — taxonomy 6–7.
9. **What cannot be concluded** — the gaps from the inspection, plus everything O1–O12 left open.
10. **Balanced conclusion** — including the case *against* treating OSCAR as a competitor at all.

---

## Tone and claim discipline

- OSCAR is a volunteer project that gives people access to their own device data. Write about it as one.
  The negative findings are worth reporting precisely because the same tree also contains the strongest
  positive findings on the shortlist.
- Do not describe the leak divergence as a defect in OSCAR's users' data. It is a documented method
  difference with a bounded effect; keep it bounded.
- Do not present the GPL source availability as a privacy or security guarantee. It is an inspectability
  guarantee. State it as that.
- Do not claim the two-policy finding generalises to OSCAR's other loaders. Only the ResMed and Apple
  Health paths were read closely; the other seventeen loaders were not examined for this property. Say
  so.
- Do not imply AnxietyWatch2 has shipped anything it has not. The alarm, the dead-man switch and the CNS
  engine are specified targets.
- Where OSCAR is better than both AnxietyWatch versions on a given axis, say so without hedging. Exit
  and interoperability are the two clear cases.

---

## Sources to use

**Primary (in this package):**
- `oscar_CLAIMED_FEATURES_AND_PURPOSE.md` — vendor claims and the v1 dependency evidence
- `oscar_IMPLEMENTATION_INSPECTION.md` — pinned-commit findings with file/line references

**Primary (vendor, external):**
- The 2.0.1 download page at `sleepfiles.com/OSCAR/` — release notes and feature highlights
- The GPLv3 source at the pinned commit — the authority for every implementation claim

**Primary (AnxietyWatch, external to this package):**
- The v1 repository: `REQUIREMENTS.md`, its test suite, its CPAP importer sources
- The v2 repository: `docs/specs/AMENDMENTS.md`, `vitals/registry.toml`, `AGENTS.md`, and the AirSense
  11 changes under `openspec/changes/`

**Do not use:** forum posts, Reddit threads, or influencer reviews as evidence for any factual claim.
They may be cited only as evidence that a user perception exists, and labelled as such.

---

## Output

- One file: `oscar_COMPARISON.md`, in this directory.
- Length: substantial but not padded. A finding that cannot be evidenced does not belong.
- Every factual claim carries a source and an evidence class.
- Every open question is recorded in the "what cannot be concluded" section rather than dropped.
- Update `../PROCESS_STATE.md` with the candidate's completion state and its residual uncertainties.
