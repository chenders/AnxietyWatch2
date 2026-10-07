# Prompt 2 — Home Assistant versus AnxietyWatch v1 and AnxietyWatch2

Base variant: **`PROMPT2-TECHNICAL-SYSTEMS.md`**
Supplementary block: **`PROMPT2-PRIVACY-AND-OPERATING-MODE.md`** (§ Operating-model audit)
Candidate: **Home Assistant** (shortlist rank 9 of 10)
Prepared: 2026-10-06

## Assignment

Write a technical-systems comparison of **Home Assistant** against AnxietyWatch v1 and AnxietyWatch2.

Home Assistant competes with **none** of AnxietyWatch's product surface. It does not measure a body, does
not claim a physiological result, is not regulated, and has no clinical pretence. It is on this shortlist
for one reason, and the report is wrong if it drifts from it:

> **AnxietyWatch2 is building a Pi-hosted, local-first, always-on collector with a retention policy, an
> alarm path and an operator console. Home Assistant has run exactly that shape, at a scale of millions of
> instances, for over a decade, as a non-profit. What did it get right, what did it get wrong, and which
> of AnxietyWatch2's own rules does its code vindicate?**

The governing question, restated for the writer:

> **Home Assistant has a first-class `unknown`/`unavailable` model in its state machine — and its alert
> integration still answers "the sensor is gone" by ending the alert and sending the user's own resolution
> message. How does a mature platform lose a correct three-state rule downstream, and what does that say
> about where AnxietyWatch2 must enforce its own "a missing signal never reassures" rule?**

Three selection reasons, which the comparison should be organised around:

1. **Architectural, not clinical, overlap** — always-on edge collection, local storage, local alerting,
   operator console, Raspberry Pi target. No other candidate overlaps V2's *platform layer* this closely.
2. **A counter-witness to a shortlist hypothesis.** The shortlist speculated that a general-purpose hub
   would have "no three-state honesty model." It does; the shortlist note is wrong, and the correction is
   part of the deliverable (`homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` §14.4).
3. **Published source.** This is one of the two candidates in the package whose implementation could be
   read lawfully and fully — so the comparison can cite code, not marketing.

A fourth reason is negative: **it measures nothing on its own.** Its health surface reports what other
Android apps already wrote to Health Connect. It must not be credited for those measurements.

## Candidate identity

| Field | Value |
|---|---|
| Product | Home Assistant |
| Project owner | **Open Home Foundation** (non-profit) |
| Companion-app publisher | **Nabu Casa, Inc** |
| Slug | `homeassistant` |
| iOS | `io.robbie.HomeAssistant`, v2026.9.3, free, min OS 16.4 |
| Platforms | Self-hosted hub (OS / Container / Core / Supervised) + iOS + Android + web |
| Model | Software free; optional paid **Home Assistant Cloud** (out of scope of the project privacy policy) |
| Licence | **Apache-2.0** |
| Scale (third-party metadata) | repo 91,266 stars · 38,804 forks; app description claims "over 1 million users" |
| Inspected release | tag **2026.9.4**, commit `9212531f40a0b7b23229a90d688dd79d9dfccff4` (2026-09-27) |
| Inspection | **Performed — published source.** Source read at a pinned commit; docs read. **No binary, no running instance, no runtime observation** |
| Closest overlap | V2 platform layer: local store + retention, local alarm path, operator console, Pi/BLE |

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/homeassistant/homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/homeassistant/homeassistant_IMPLEMENTATION_INSPECTION.md
./docs/app_comparison/CANDIDATE_SHORTLIST.md
./docs/app_comparison/RESEARCH_METHOD.md
```

**Do not re-run implementation inspection.** The source inspection is complete and its findings are static
evidence — use them as such. **There are no runtime observations for this candidate**; do not introduce
any, and do not upgrade a source reading into a behaviour. Cite `file:line` against the pinned tree when
a finding matters to the verdict.

## AnxietyWatch status labels (use these exactly)

- **V1 documented/implemented** — exists in `../AnxietyWatch` today
- **V2 current** — exists in `./` today
- **V2 specified target** — fully specified in `openspec/` but not yet shipped
- **Unresolved** — no evidence either way

Never present a V2 specified target (supervised alarm, hardware dead-man switch, CNS pipeline stages) as
shipped. Never present a V1 capability as present in V2. In particular: **V2's supervised alarm path is
specified and not built.** Do not score it as a Home Assistant defeat, and do not score Home Assistant's
built-in confirmation as a defeat of a system that has not finished building its own.

## Evidence rules

Apply these to every claim in the report.

1. **Label every finding** as: public claim · static finding (authorized published source) · runtime
   observation · experimental-or-unreachable · inference · unknown. For Home Assistant there are **no
   runtime observations**.
2. **Never present a source reading as a public claim**, and never present a public claim as an
   implementation fact. `[FOUND IN SOURCE]` means read; it does not mean observed.
3. **Absence of evidence is unknown, not proof of absence.** That no `STATE_UNAVAILABLE` reference exists in
   `components/recorder/` is a finding; that the recorder cannot represent absence is not.
4. **A reading of a code path is a static finding about that path.** The alert false-all-clear is read
   from `entity.py`; it is not an observation of a live false alarm. Do not describe it as users being
   told the leak is over without marking that as inference from the code.
5. **Do not upgrade a documented default into implemented behaviour**, and do not upgrade documented
   behaviour into a verified runtime property. The nightly 04:12 purge is documented; the purge *code*
   was read; the *schedule* running at 04:12 was not verified.
6. **A file that does not parse is a static fact about a file.** Its consequence for a running instance is
   inference. State both, separately, every time.
7. **Require a mechanism before crediting a capability.** A claim that the platform adds local control and
   privacy is only a claim; the mechanism (self-hosted state machine, local recorder, opt-in telemetry) is
   what earns credit.
8. **Vendor-published and third-party figures are labelled as such.** The star counts are third-party
   platform metadata; the user count is the vendor's claim; the `averageUserRating = 0` field is an
   **unresolved ambiguity** and must not appear in any tally or verdict.
9. **Never present the vendor's own statement about its limits as independent validation** — and do not
   treat the Foundation's candour about the cloud carve-out as a substitute for the fact that the paid
   surface is governed by a separate policy.
10. **Documented ≠ implemented ≠ validated.** A state table documents three states; the code implements
    three states; neither is evidence that a live alert behaves correctly under an absent sensor.
11. **Never state float equality.** Use tolerances.
12. **No medical, clinical, diagnostic or emergency claims.** Home Assistant makes none; the report says
    so plainly rather than implying a hidden one — and should note that a local notification is not an
    independent alarm.

## Candidate-specific unresolved questions

Answer these explicitly. If the evidence does not exist, say so and label the consequence.

- **H1** Is the alert false-all-clear on `unknown`/`unavailable` an intended semantic or an unhandled
  branch? Present both readings against `entity.py:91-99` and the frozen-development docstring
  (`:3,33`), and say which the evidence supports. Do **not** assert a maintainer's intent.
- **H2** Does the `sensor/recorder.py` syntax error block a running instance, and on which import path?
  The inspection deliberately stopped at "this file does not parse." If the report claims a user-visible
  consequence, mark it as inference and cite what would resolve it.
- **H3** Across a restart, what happens to `_firing`, `_ack` and `_next_delay`? The entity holds them
  in memory and schedules with `cancel_on_shutdown=True`. The component's state-reproduction path was
  **not read** — record that limit rather than closing the question.
- **H4** What are the actual `habluetooth` watchdog values (`SCANNER_WATCHDOG_TIMEOUT`,
  `SCANNER_WATCHDOG_INTERVAL`)? They are outside the inspected tree. Do V2's own BLE liveness expectations
  resemble the five-minute device-unavailable default (`bluetooth/const.py:34`)?
- **H5** What is the real long-term-statistics retention? Only two code paths delete from `statistics`
  (duplicate cleanup, user service); the scheduled purge does not. Is the description of hourly aggregates
  as kept forever by default the accurate statement, and what are the consequences for a decade-old
  instance?
- **H6** What exactly does `mean_weight` mean, and how does a partial hour survive into `mean`? Contrast
  with V2's `time_bucket_gapfill` and its rule that missing data contributes nothing.
- **H7** Should V2 adopt a circular mean for any registry type? `statistics.py:144-162` implements it.
  If no current V2 type is circular, say so and do not invent a requirement.
- **H8** For the health sensors, is staleness distinguished from absence? Most descriptions say "the last
  recorded …", while `last_reboot` explicitly says it "will be unavailable if the timestamp cannot be
  determined." Is the absent-state rule applied uniformly, or only to some sensors?
- **H9** Are the 26 Health Connect types mapped one-to-one to a registry style, and **where are units
  converted**? The documented units are a mixed SI/non-SI set (mmHg, mg/dL, kcal, "steps", "floors"), and
  one is misspelled ("celsus"). State the architectural contrast with V2's one-canonical-unit-per-type
  rule and do not treat the typo as a data-integrity finding.
- **H10** Who is the controller for the paid surface, and what does the policy actually cover? The policy
  names its own scope and carves the cloud out. Push and crash-reporting third parties are **not named**.
- **H11** The project is a non-profit, the apps are published by a for-profit company, and the cloud is a
  separate legal entity. Map the three and state which claim attaches to which surface. Do not infer
  intent from the arrangement.
- **H12** What do Home Assistant's REST and WebSocket APIs actually contract? **They were not read.**
  Record that as a gap and do not compare the external interface.
- **H13** V2's shortlist entry names Home Assistant as the benchmark for "deployment, backup, upgrade and
  onboarding." Those components were **not inspected** (only their documented story was read). Record the
  limit; if the report praises the story, it must cite the documentation and mark it as documented.
- **H14** `validate_state` raises while the internal setter substitutes `STATE_UNKNOWN`
  (`core.py:202-208` vs `:2500-2507`). Which path does V2 resemble, and does V2's decoder boundary have
  the same split between a public reject and an internal coerce?
- **H15** Does the five-minute `UNAVAILABLE_TRACK_SECONDS` staleness rule for BLE devices support or
  challenge V2's own staleness design? Compare the *numbers and the policy*, not the products.

## Comparison taxonomy for this candidate

Organise the technical comparison around these axes, which is where Home Assistant actually instructs.

### 1. The alarm path (the centrepiece)

- Home Assistant: an equality test on the watched state (`entity.py:98`), three documented states
  (`idle`/`on`/`off`), no absent-state branch, a `done_message` that fires on resolution, an interval
  that saturates, `ServiceNotFound`-only error handling, and in-memory firing state.
- AnxietyWatch v2: "a missing signal holds state; it never reassures and never escalates on its own," a
  supervised alarm path, a hardware dead-man switch — **all V2 specified target, not shipped.** Label
  every one.
- The verdict to argue: not that V2's alarm is better (it is not built) and not that Home Assistant's is
  broken (that is a static reading), but **the platform's most mature alarm primitive has the exact failure
  mode V2's rule forbids, in a codebase that already has the missing state available to test for.** That is
  the strongest available external evidence that the rule must be enforced at the alarm layer and tested,
  not assumed from a correct state machine.

### 2. The three-state model — where it exists and where it is lost

- Exists: `STATE_UNKNOWN`/`STATE_UNAVAILABLE` (`const.py:303,313`), a documented absent state in the
  companion reference (`last_reboot`), two divergent invalid-state policies in `core.py`.
- Lost at the alarm layer (no branch) and at storage (both absences and `""` in one `String(255)` column).
- V2: three states everywhere, sentinel rejection (`0x07FF`/`0x0800`), no clamping, quarantine.
- The transferable lesson, stated as a design rule: **a correct core is defeated by a single downstream
  equality test.** The deliverable should name the V2 layers where the same risk exists and what test would
  catch it.

### 3. Storage, retention and the aggregate layer (the sharpest technical contrast)

- Home Assistant: append-only-ish `states` rows **hard-deleted** after `purge_keep_days=10` with no
  tombstone; hourly `statistics` rows **upserted** on a unique `(metadata_id, start_ts)` index; a
  `mean_weight` coverage column; long-term stats effectively kept forever; circular means available.
- V2: append-only `samples` with tombstones, no `UPDATE`/`DELETE` against `samples`, `time_bucket_gapfill`,
  and a rule that invalid data contributes nothing.
- Argue honestly in both directions: Home Assistant's **`mean_weight` and circular-mean helpers are
  positives V2 has no equivalent of**, and its aggregation model is a legitimate different design. Do not
  present "append-only" as automatically superior; present the *consequence* difference (can you tell,
  years later, what was removed and why?).
- Distinguish clearly for the reader: **HA deletes raw history by policy; V2 deletes nothing by policy.**
  Neither is a defect in the abstract; the comparison is about which questions a user can still ask of the
  data afterwards.

### 4. Delivery confirmation and escalation

- Home Assistant: local push requires confirmation within `PUSH_CONFIRM_TIMEOUT = 10` s
  (`push_notification.py:10`); on timeout the channel is torn down and one `fallback_send` runs; receipts
  are not persisted; no escalation to a second recipient; `async_confirm_notification` returns `False`
  for an unknown `confirm_id`.
- V2: supervised alarm and hardware dead-man switch — **specified, not built**.
- The fair framing: Home Assistant has *built* a delivery-confirmation mechanism and stopped after one
  hop; V2 has *specified* a fuller one. Credit the built mechanism, credit the honest `False` on an
  unmatched receipt, and state plainly that neither answers *what if the fallback also fails, and nobody
  notices?*

### 5. Health-signal ingestion versus V2's typed registry

- Home Assistant: 26 Health Connect sensors, **last-value entities**, "only the last 30 days," per-sensor
  units including non-SI, one misspelled unit, and no analysis layer.
- V2: `vitals/registry.toml` as the single source of truth, generated enums, one canonical unit converted
  once at the decoder, ranges from the registry, never hardcoded.
- The contrast to argue: Home Assistant's model is **entities and events**, not **typed measurement
  series**. That is not a criticism of Home Assistant — it is the correct model for a home hub and the
  wrong one for a physiological store. Say why, precisely: no per-sample device identity beyond the
  entity ID, no unit on the sample row, no validity decision at ingest, no quarantine.

### 6. Operating model and privacy boundary

- Non-profit project; for-profit app publisher; separately governed cloud. Map the three (§H11).
- Opt-in analytics with published retention; disabled Cloudflare analytics with its own stated
  limitation; self-hosted Plausible; companion data not logged by the Foundation; push/crash third parties
  unnamed.
- V2's own privacy and network-trust posture (`09`–`11` amendments) — **V2 current or V2 specified target**,
  labelled.

### 7. Operational maturity (documented only)

- The shortlist names Home Assistant the benchmark for deployment, backup, upgrade and onboarding — and
  those components were not inspected. Discuss only what the **documentation** claims, mark it documented,
  and put the gap in the limitations section rather than converting the documentation into a score.

## Failure-mode scenarios to walk through

Walk each of these through both systems, and label the evidence class in each answer.

1. **A sensor stream drops during an overnight monitoring window.** Home Assistant: the BLE device goes
   `unavailable` after five minutes (`bluetooth/const.py:34`) — and the alert layer, if it watches that
   entity, ends the alert and may announce resolution. V2: the rule is that the missing signal holds state.
   This is the scenario the report exists to answer.
2. **The phone is unavailable.** Home Assistant: the hub keeps running locally; the companion app is only
   a transport for sensors and notifications. V2: compare against the phone-as-collector assumption in
   v1 and the Pi-hosted design in v2.
3. **Internet or cloud is unavailable.** Home Assistant: local operation continues; local push is
   attempted first with a cloud fallback that is simply unavailable. V2: same class of question, different
   plumbing.
4. **A reading is stale, malformed, out of range, or absent.** Home Assistant: absent states are
   first-class; malformed values are handled by parseability filtering (`sensor/recorder.py:226-249`),
   not by a named rejection; over-long states are replaced by `unknown` (`core.py:2500-2507`). V2: reject,
   name the reason, quarantine; never clamp; sentinel rejection.
5. **A clinician or operator needs an export with source and quality context.** Home Assistant: history
   and statistics services, no per-sample provenance, no quality flag, and raw history likely already
   purged. V2: append-only samples with provenance. Compare the *questions each export can answer*.
6. **An alert path has a technical failure.** Home Assistant: `ServiceNotFound` is logged and retried at
   the next interval (`entity.py:183-193`); the fallback is one hop; nothing records the failure; and a
   restart returns the alert to `idle`. V2: the specified supervised path, labelled as a target.
7. **The hub restarts while an alert is firing.** Read `entity.py`'s in-memory fields and
   `cancel_on_shutdown=True`, and state the limit of that evidence (§H3). This is the dead-man-switch
   scenario, and it is the second-strongest finding in the package after the false all-clear.

## Required report structure

Write to `./docs/app_comparison/homeassistant/homeassistant_COMPARISON.md` with these 14 sections:

1. Executive finding
2. Product and availability
3. System boundary and architecture (ASCII)
4. Data path and source comparison
5. Claim-versus-documented-contract reconciliation
6. Feature matrix (columns: **v1 · v2 current · v2 target · Home Assistant · evidence/maturity**)
7. Derived values versus stored signals
8. Real-time, overnight and failure-mode comparison
9. Provenance, missingness and reproducibility
10. Local/cloud/privacy and operational cost
11. Alerting and the alarm path
12. User-facing consequences
13. Limitations, unverified claims and evidence gaps
14. Balanced conclusion — whole-product competition versus subsystem competition

End with a `## Related files` footer naming the other three deliverables in this directory.

## Tone and claim discipline

- Home Assistant is not a competitor in the product sense, and the report must not manufacture a contest.
  The whole-product verdict is that they do not compete; the subsystem verdict is where the value is.
- Do not praise Home Assistant for scale alone, and do not fault it for not being a medical device. Judge
  it on the layer it shares with V2.
- Do not use the syntax error as a rhetorical weapon. Record it as a static finding with its verification
  chain, note that the integration it lives in is a statistics sensor path, and leave the runtime
  consequence where the evidence leaves it.
- Do not treat the frozen alert integration as negligence. It is a lifecycle fact the project publishes
  about itself.
- Where the two systems agree — substituting an explicit absent state instead of clamping; an explicit
  failure when an alert cannot be acknowledged; coverage weighting in aggregates — say so. A comparison
  that only lists differences is not a comparison.
- Every number quoted from the pinned tree gets a `file:line`. Every number from a website gets a URL and
  the retrieval date. Every store figure is third-party. Never state float equality.

## Sources to use

- `homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` and `homeassistant_IMPLEMENTATION_INSPECTION.md` in this
  directory, with their `file:line` citations against commit `9212531f…`.
- The v2 tree: `vitals/registry.toml`, `docs/specs/AMENDMENTS.md`, and the `openspec/changes/` proposals
  already read for this package (`pi5-vitals-core`, `linux-bluez-broker`, `admin-console-roadmap`,
  `pi-prototype-storage-degraded-mode`).
- The v1 tree where a v1 comparison is claimed.
- Do **not** query any live Home Assistant instance, public or private, and do not re-fetch the source.

## Output

One file: `./docs/app_comparison/homeassistant/homeassistant_COMPARISON.md`. 14 sections. Every claim
carries its evidence class. Every source citation is `file:line` against the pinned commit or a URL with a
retrieval date. No runtime observation is invented. No V2 target is presented as shipped.
