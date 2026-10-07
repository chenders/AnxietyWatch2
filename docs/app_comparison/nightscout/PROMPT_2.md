# Prompt 2 — Nightscout versus AnxietyWatch v1 and AnxietyWatch2

**Candidate:** `nightscout` — Nightscout ("CGM in the Cloud"), `nightscout/cgm-remote-monitor` v15.0.8
**Base variant:** `../PROMPT2-TECHNICAL-SYSTEMS.md`
**Supplementary block:** operating-mode and privacy questions, drawn from
`../PROMPT2-PRIVACY-AND-OPERATING-MODE.md`
**Chosen because:** Nightscout is the only candidate whose interest is almost entirely in the **platform
layer** rather than the physiology — self-hosting, wireless ingest, alerting, retention, deletion, access
control and derived statistics. It is also one of only two candidates (with OSCAR) whose actual
implementation has been read, so the technical-systems framing can be applied to verified code rather than
to marketing copy. Its physiology (glucose and insulin) barely overlaps AnxietyWatch2's, so a
feature-by-feature comparison would be shallow and misleading; the useful comparison is architectural, and
that is what this prompt demands.

---

## Assignment

Assess Nightscout against AnxietyWatch v1 and AnxietyWatch2, using the completed evidence base in this
directory. Nightscout is **not** a competing consumer product for either version, and the report must not
pretend it is. It is three other things, and they must be kept separate:

1. **A reference implementation of the platform layer AnxietyWatch2 is building.** Self-hosted service,
   continuous wireless physiological ingest, user-owned store, browser and third-party clients, alerting,
   scheduled retention. Seventeen years of deployment in real homes, with real battery and connectivity
   failures, is the part of the record that AnxietyWatch2 does not have and cannot specify its way to.
2. **A published, dated, and in places wrong account of itself.** The project ships its own audits,
   requirements, test specifications and OpenAPI description. Three of those documents disagree with the
   code at the release commit. That is both a warning about self-audits as evidence and an unusually good
   opportunity to study a real pipeline's failure modes.
3. **A near-miss on the safety posture.** Nightscout and AnxietyWatch2 state the *same* rules in prose —
   do not rely on this alone, it is not a regulated device, always have another way to check — and then
   diverge sharply in the code: unit conversion, missing-data semantics, invalid-value handling, alarm
   behaviour with absent data, and threshold configurability. The report should establish how large that
   divergence is, and where it is narrower than expected.

The deliverable should let the owner decide four things: whether Nightscout's architectural choices
validate or challenge AnxietyWatch2's; whether any of its specific mechanisms (role/permission strings,
`history` delta sync, tombstone-plus-permanent-delete, write-path-triggered pruning, the plugin/sandbox
seam) are worth adopting or deliberately rejecting; whether AnxietyWatch2's *private LAN, no account, no
cloud dependency* position is justified by the *operational* record rather than only by principle; and
whether the three-state rule and the registry-as-single-source-of-truth rule are worth their cost, given
a widely deployed system that violates both and still functions.

---

## Candidate identity

| Field | Value |
|---|---|
| Product | Nightscout / "CGM in the Cloud" |
| Repository | `nightscout/cgm-remote-monitor` |
| Version inspected | **15.0.8** ("Chocolate Chip Muffin"), released 2026-09-04 |
| Licence | AGPL-3.0 |
| Steward | Volunteer project; the **Nightscout Foundation**, a US 501(c)(3) (EIN 47-2130306), funds the ecosystem and states it takes no industry funding |
| Runtime | Node `>=20.x`, Express, MongoDB; Docker/Pi/NAS/PaaS |
| Physiology | glucose (CGM), insulin, carbohydrate, pump/loop device status |
| Mobile apps | none first-party; third-party uploaders and followers |
| Cost | free; ten advertised commercial hosting providers |
| Inspection outcome | **Performed** — full source read at pinned commit `92d0834219aa771b5837dbcbf1baeb839a200cf6`; **nothing executed** |

---

## Read first

1. `nightscout_CLAIMED_FEATURES_AND_PURPOSE.md` — public claims, governance, pricing, the security page's
   own disclosures.
2. `nightscout_IMPLEMENTATION_INSPECTION.md` — the pinned-commit findings, §5.1–§5.20, including the
   hypotheses checked and refuted (§5.20).
3. `../PROCESS_STATE.md` — candidate 6's queue position and its residual uncertainty.
4. `../RESEARCH_METHOD.md` §6.1 — the inspection-mode taxonomy, including what "Performed" does and does
   not mean.
5. In the AnxietyWatch2 repository: `docs/specs/AMENDMENTS.md` (mandatory), `vitals/registry.toml`, and
   `openspec/changes/cpap-ezshare-ingest/`.
6. In the AnxietyWatch v1 repository: `REQUIREMENTS.md` and the sync/service layer.

**Do not treat the inspection document as a behaviour report.** Every finding in it is a reading of source
code. Nothing in this package has run Nightscout, and no deployed instance has been contacted (§6 of the
inspection).

---

## AnxietyWatch status labels (use these exactly)

| Label | Meaning |
|---|---|
| **v1 shipped** | present in AnxietyWatch v1 as it exists |
| **V2 current** | implemented in AnxietyWatch2 at the time of writing |
| **V2 target** | specified in `openspec/` but not implemented |
| **V2 not planned** | no evidence of intent; say so rather than implying a gap |
| **unknown** | no evidence either way; do not fill the gap with reasoning that reads as fact |

**Prohibited.** Do not present a V2 target as shipped. Specifically: the CNS engine, the supervised alarm,
the hardware dead-man switch, `xe125-respiratory-rate` (2/5 tasks), `cpap-ezshare-ingest` (0/6), the
admin console roadmap (17/29) and the iOS client stage 7 (15/9) are **V2 target** unless verified as
merged. Do not present v1's CPAP defect (§9 of `PROCESS_STATE.md`) as a failure of diligence — the CPAP
path rests on community reverse engineering because there is no other lawful route.

---

## Evidence rules

1. **Separate the classes.** Public claim / documented contract / vendor-published internal document /
   static-only finding / runtime observation / inference / unknown. Never merge them, and never present a
   source reading as a runtime observation.
2. **A vendor's self-audit is not independent validation.** Nightscout's `docs/audits/` and
   `docs/requirements/` are quoted as the project's own statements, with dates. Where they disagree with
   each other, say so; do not pick the convenient one.
3. **Absence of evidence is `unknown`, not proof of absence.**
4. **A falsified hypothesis is not a finding.** The inspection documents five hypotheses that were checked
   and refuted (§5.20). Do not resurrect them. If a new hypothesis is raised, mark it as a hypothesis until
   it is checked against the tree.
5. **Do not count "the vendor disclosed it" as mitigation unless it changes what a user experiences.**
   The readable-by-world warning and the A1c note are both disclosures. Say what each does and does not
   change.
6. **The most conservative of two conflicting vendor surfaces governs**, and the conflict is recorded
   rather than resolved (see the ten advertised hosts versus the security page's "The system has not been
   designed for scenarios where a third party operates a Nightscout installation on behalf of others").
7. **Do not upgrade a comment into behaviour.** `//assume current` and `// bless absence with known type`
   are evidence of *intent*; the behaviour claim comes from the surrounding code, and it must be stated as
   a static finding.
8. **Never state float equality without a tolerance**, and never convert correlation into causation.
9. **A documented default the code does not implement is a finding**, independent of which is safer
   (the `DEVICESTATUS`=60 case).
10. **Do not treat the 73,382 forks as a community-size signal** — the project's own deployment
    instructions tell users to deploy from a fork, so many forks are installations (inference; state it as
    such).
11. **Cite `file:line` only against the pinned tree**, and verify each citation before publishing. Do not
    carry line numbers over from the inspection document without re-reading them.

---

## Candidate-specific unresolved questions

Answer each, or record it as unresolved with the reason. For each, state what would resolve it and what
that would cost. **Q1–Q4 are the decisive four.**

| # | Question | Why it matters | What would resolve it |
|---|---|---|---|
| **Q1** | In a default deployment of 15.0.8, is the `readable` default actually in effect for an anonymous caller, and does the "readable by world" admin notice appear? | This is the single most consequential security claim in the whole package, and it is currently a code reading. It decides whether Nightscout's default posture is a documentation-level warning or a live exposure. | Authorised local instance on disposable infrastructure + one unauthenticated HTTP request + one admin-page load. Cost: a MongoDB and a Node app on a throwaway host. **Requires an explicit authorisation decision; not performed.** |
| **Q2** | Does the Daily Stats A1c render as ~95.8% in mg/dL display and ~6.9% in mmol/L display at this commit, and is the fix present in a later release? | Establishes the difference between *documentation says the number was wrong* and *the shipped code produces a wrong clinical number*. Also tests whether the vendor's disclosure is accurate in magnitude. | A unit test of the two functions with a fixed input (no server needed), or a running instance. Also: check the default branch and release notes for the fix. Cost: low. |
| **Q3** | With data flowing and then stopping, does the stale-data alarm fire? With data never having arrived, does anything fire at all? | The code distinguishes these two cases in a way that is easy to misread (`timeago.js:26` returns early only when there is no entry *or* the entry is in the future, so old-but-present data proceeds to `checkStatus`). This is the exact failure mode AnxietyWatch2's three-state rule exists to address. | A runtime instance with a synthetic feed, or a careful test of `timeago.checkNotifications`/`checkStatus` in isolation. Cost: low for the test, moderate for the runtime. |
| **Q4** | Are the audit-vs-code discrepancies known to the maintainers, tracked, or fixed on the default branch or in 15.0.9? | Determines whether the three drift findings are *the project knows and is fixing it* or *the project's security documentation is currently misleading*. Changes the finding's weight, not its existence. | Search the repository's issues, pull requests and the default branch for the token/JWT, delay-list, role-table and swagger-default changes. Public and read-only. Cost: low. |
| **Q5** | Do alarms actually cross group boundaries over the socket, and can any token actually silence an alarm in practice? | Code says both; the project's README says the second in prose. A runtime check would move this from a code reading to a demonstrated behaviour. | Two authorised WebSocket clients with distinct tokens against a local instance. Cost: moderate; needs Q1's instance. |
| **Q6** | What do the ten advertised hosting providers commit to in writing — data location, encryption, retention, breach notification, sub-processors, and under which law do they act? | The project publishes no privacy policy because it holds no data. For any user who pays a host, the privacy question is entirely delegated. Nothing in the package currently answers it. | Read each provider's published terms and privacy page. Cost: low (public pages); note that claims remain provider claims. |
| **Q7** | What does the storage purifier actually strip, and does it treat `activity` identically to `entries`? | The documentation's sanitisation claim was verified only at the import level. What is removed determines whether a hostile or malformed upload can store script-bearing content for later rendering. | Read `lib/server/storage-purifier.js` and `lib/api3/shared/writePurifier.js` bodies. Cost: low. |
| **Q8** | Can one Nightscout instance serve more than one person, and what happens if it is used that way? | The documented model is one site per person. A multi-person deployment changes every security and retention conclusion at once. Relevant because the advertised hosting market implies shared infrastructure. | Read the profile/subject/role code for multi-subject support; check the hosting providers' terms. Cost: low. |
| **Q9** | End to end, what does the `39` value do — in the alarm engine, in the reports, and in the UI? | The code has 39 special-cased differently in at least four places, including reports that *include* it and alarms that *exclude* it. The patient-safety consequence cannot be stated from source alone. | Trace the callers of `displayBg` and the report filter with a synthetic `39` entry, ideally on a running instance. Cost: low for the trace, moderate for runtime. |
| **Q10** | Does the default retention (no application-level pruning) match what the API description promises, and what actually removes data in practice? | Three surfaces give two answers. The practical question for a user is *how long does my data live if I do nothing?* | Read the remaining `lib/storage/*` index definitions for TTL indexes; check any MongoDB migration scripts. Cost: low. |
| **Q11** | Does anything in the surrounding ecosystem (uploaders, followers, analytics, upstream projects like Loop/Trio/AAPS) overlap AnxietyWatch2's sleep, SpO₂, respiratory or context surfaces? | The shortlist treats Nightscout as the platform analogue, but the ecosystem is large and was not surveyed. If a companion app does sleep or SpO₂, the competitive picture changes. | Survey the documented uploader/follower list and the ecosystem projects' feature pages. Cost: moderate. |
| **Q12** | What is the practical effect of the security page's warning that users must verify that third-party apps "have data validation processes in place"? | The project states it does not validate consumers; the ecosystem is where data actually leaves the system. For AnxietyWatch2's "third-party read" target this is directly relevant. | Read a representative follower app's source where available; otherwise record as unresolved. Cost: moderate. |
| **Q13** | Who governs the repository and the trademark, and what is the bus factor? | Nightscout is the reference implementation for the platform layer. Its long-term maintainability is relevant if any part of its design is adopted. | Read `CONTRIBUTING.md`, the recent commit history's authorship, and the Foundation's leadership page. Cost: low. |
| **Q14** | Could AnxietyWatch2's amendment-03 multi-modal alerting target be met on local hardware without any third-party cloud account, and what does Nightscout's IFTTT/Pushover path cost in accounts, quotas and failure modes? | The comparison's most directly actionable question: if Nightscout — a mature, widely deployed alerting system — needs four cloud services to alert a phone, that is evidence about the difficulty, not just about its design choices. | Enumerate the delivery path's dependencies from the code and the docs; assess against amendment 03's stated channels. Cost: low. |

---

## Comparison taxonomy for this candidate

Structure the comparison around these seven axes, in this order. Each finding must carry an evidence
class.

### 1. Operating model (the centrepiece)

Self-hosted service versus local daemon; account versus none; browser versus app; cloud dependency versus
LAN-only; free-with-a-hosting-market versus owner-maintained. Include what each choice costs the user:
Nightscout's costs are a server, a database, updates, backups (which it does not provide) and possibly a
hosting bill; AnxietyWatch2's costs are a device, an install, and whatever the BLE path's fragility turns
out to be. Note the project's own framing — a working internet connection and third-party cloud services
are required — and the Security-Page contradiction about third-party operators.

### 2. The store: append-only, tombstones, retention, deletion

Nightscout's two delete semantics in one process (§5.9), the tombstone-plus-history model of API3, the
hard-deleting admin tools, the opt-in prune with a three-way `$or` that can reach recently created
documents (§5.10), the unimplemented documented default (§5.11), and no built-in backup. Compare with
AnxietyWatch2's append-only `samples`, tombstone rows, `time_bucket_gapfill`, 7-day compression, 1-minute
aggregate, and 2-year raw retention. State plainly which rules Nightscout enforces, which it leaves to the
operator, and where the "never use `locf()`/`interpolate()`" amendment maps onto Nightscout's behaviour.

### 3. Units and derived values

The mg/dL canonical store, scattered conversions, the string-returning `mgdlToMMOL` (§5.13), and the
verified unit-dependent A1c (§5.3). Then V2's one-unit-per-type, converted-once-at-the-decoder rule, and
the registry as the only place a range or threshold appears. **This is the axis on which the evidence is
strongest; give it the most space, and show the arithmetic.**

### 4. Absent and invalid data

`//assume current`, `// bless absence with known type`, early returns in the alarm path, the four-of-six
severity collapse into `current`, and the complete absence of physiological validation at the store
boundary (§§5.4–5.5, 5.12). Then the three-state rule, reject-with-reason, quarantine, and the
`assert!(b.rejected.is_empty(), "absent is not rejected")` boundary. Address the counter-argument
honestly: Nightscout is a display and alerting system fed by hardware that already decides what is
measurable, and an API that rejects a glucose value could drop a real low reading. Weigh that against
*a wrong number is worse than no number downstream*.

### 5. Alarms: thresholds, silencing, and failure

Thresholds as code literals with settings that only rename events (§5.15); the clamp that makes a forecast
below 36 display as 36; alarms missing when data stops (§5.6); any-token silencing with a namespace-wide
broadcast (§5.7); in-memory ack state that a restart clears; a failing plugin that logs and vanishes
(§5.8). Then V2's registry thresholds, supervised alarm target, and reject-don't-clamp rule. Note the
one place Nightscout is clearly more conservative: its explicit non-dependence disclaimers to users.

### 6. Alert delivery and multi-modal reach

The IFTTT + Google + Pushover path, the vendor's own documented alarm-fatigue drawback, the quota limits,
and the absence of any first-party mobile app or push channel. Against amendment 03's target. This is
where the comparison should produce an actionable conclusion for V2 rather than a score.

### 7. Documentation as an artifact

The audit-vs-requirements-vs-code three-way disagreement (§5.2), the role table (§5.16), the delay list
(§5.17), and the swagger default (§5.11) — set against the project's published audit-first methodology and
traceability programme. Then the *reverse* case: the sanitisation claim that was checked and found
accurate, and the `TODO` that was checked and found stale. Generalise carefully to AnxietyWatch2's own
OpenSpec/`AMENDMENTS.md` workflow: what does this package's evidence suggest about self-audit as a
control?

---

## Supplementary block — operating mode and privacy

Answer these in addition, drawn from `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md`:

1. **What is the data controller for a Nightscout site?** There is no privacy policy. Work through who
   holds the data in each deployment mode (self-hosted, PaaS, managed host) and what follows.
2. **What actually leaves the box in a default deployment?** Any third-party requests in the code paths
   read — telemetry, update checks, IFTTT, Pushover, CDN assets in the client. Separate what the code does
   from what the user must configure.
3. **What does the security page warn about, and what does it not?** Quote "In a default setup, anyone
   with your Nightscout URL can view your site", the third-party-operator paragraph, and the frame-embedding
   default, and state what each means for a user who never reads the page.
4. **How does Nightscout's stated safety posture compare with AnxietyWatch2's?** Both say do not rely on
   this alone. Locate the difference in the code rather than in the prose.
5. **Does the operating model assume a competent operator?** The DIY framing, the "not supported by any
   company" statement, the volunteer support model, and the existence of a paid hosting market are four
   answers to "who runs this". Say what each implies about the audience, and what AnxietyWatch2's audience
   is by comparison.

---

## Failure-mode scenarios to walk through

For each, state what Nightscout does *per the inspected code*, what it says it does, what AnxietyWatch v1
does, and what AnxietyWatch2 currently does or targets. Mark each answer with its evidence class.

1. **The phone uploader dies at 02:00 and nothing arrives all night.** (Which alarms fire? Which render as
   normal? What does the history show?)
2. **The CGM reports the bottom-of-scale value (39).** (Which engines alarm, which exclude it, what appears
   in reports?)
3. **A user changes the display unit from mmol/L to mg/dL.** (Which derived numbers change value?)
4. **An operator enables auto-pruning after a year of data.** (Which documents are deleted, and does the
   `$or` filter behave as the operator expects?)
5. **A shared alarm token leaks to someone who should not be able to silence alarms.** (What can they do,
   and what evidence would exist afterwards?)
6. **An alarm plugin starts throwing on every tick.** (What does the user see? What does the operator see?
   What is persisted?)
7. **An uploader sends a glucose value of `0` and then a valid value.** (Where is it accepted, where is it
   filtered, and what does a report compute?)
8. **The server is redeployed with a fresh `node_modules`.** (What happens to issued JWTs, and what would
   the operator see?)

---

## Required report structure

1. Executive summary — five findings maximum, each with its evidence class.
2. What Nightscout is, in one paragraph, and what it is not.
3. The operating model, cost of ownership, and the hosting market (including the security-page conflict).
4. The store: retention, deletion, tombstones, backup.
5. Units and derived values — **including the A1c arithmetic, shown**.
6. Absent and invalid data.
7. Alarms: thresholds, silencing, failure.
8. Alert delivery and multi-modal reach, against amendment 03.
9. Documentation as an artifact, and what it implies about self-audit.
10. Feature and capability matrix: `v1 shipped` / `V2 current` / `V2 target` / Nightscout / evidence class.
11. Failure-mode walk-throughs (the eight above).
12. Evidence quality: what is verified, what is read, what is claimed, what is unknown.
13. What AnxietyWatch2 should adopt, what it should deliberately reject, and why.
14. Risks, residual uncertainty, and the runtime questions that remain open.

---

## Tone and claim discipline

- **No scoreboard.** *Nightscout 6 / AnxietyWatch2 8* is not a finding. State mechanisms and consequences.
- **No victory laps on the A1c defect.** It is one wrong number in one report, in a project with seventeen
  years of operation; it is useful because it is *verifiable*, not because it is damning. The same
  discipline applies in both directions: do not soften it into nothing, and do not present it as a verdict
  on Nightscout.
- **Do not present a vendor's disclosure as a mitigation.** Say what a user experiences either way.
- **Do not imply that not being regulated is a defect.** Nightscout says it plainly; so does much of the
  ecosystem. Report it as a stated boundary.
- **Do not treat the Foundation's claims as verified.** They are statements, not records.
- **Attribute every quotation** to its document, its date and its status.
- **Where a question cannot be answered, say so and say why.** An honest "unknown" is the deliverable in
  those cases.

---

## Sources to use

- `nightscout_CLAIMED_FEATURES_AND_PURPOSE.md` and `nightscout_IMPLEMENTATION_INSPECTION.md` in this
  directory — the primary evidence base, with its own evidence classes.
- For the AnxietyWatch side, the repository docs listed in **Read first**; do not re-derive v1's behaviour
  from memory.
- Public Nightscout documentation only where the claimed-features file already cites it; if a new page is
  needed, fetch it, hash it, and label it as a public claim.
- The pinned clone at `/tmp/aw2research/nightscout/repo` @ `92d0834219aa771b5837dbcbf1baeb839a200cf6` for
  any `file:line` verification. **Do not modify the clone.** If it is gone, re-clone and check out the
  same commit; findings tied to a different commit must be labelled as such.

---

## Output

`nightscout_COMPARISON.md` in this directory, following the structure above. Length should be driven by
evidence: the sections with verified code findings (units/derived values, absent data, alarms, store) will
be longer than the sections that are documentation-only. Include the A1c arithmetic explicitly, and
include the eight failure-mode walk-throughs even where the answer is *unknown, because it was not run*.
