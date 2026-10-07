# PROMPT2 — the per-candidate deep dive

Given one candidate that has already been through `PROMPT1.md` steps 1–4, write
`{slug}/PROMPT_2.md` — the prompt that produces that candidate's comparison — and then execute it into
`{slug}/{slug}_COMPARISON.md`.

Fill `{APP_NAME}`, `{APP_SLUG}`, `{VENDOR}`, `{ARTIFACT}` (package id / bundle id / repo + commit) from
the candidate's own record. A prompt that leaves a placeholder unresolved is a defect.

---

## 1. Inputs, all required

| Input | Where it comes from |
|---|---|
| `{slug}_CLAIMED_FEATURES_AND_PURPOSE.md` | Step 1 — every claim, quoted and dated, claim ids `V-n` |
| `{slug}_IMPLEMENTATION_INSPECTION.md` | Steps 2–4 — build, hashes, tools, class map, endpoints, and the reconciliation table |
| AnxietyWatch v1 tree | `../AnxietyWatch` — the shipped iOS app |
| AnxietyWatch2 tree | this repository — `crates/`, `vitals/`, `openspec/changes/`, `docs/design/` |
| `docs/specs/AMENDMENTS.md` | **Mandatory.** Amendments supersede the original design; a task brief that contradicts one is wrong |

Do not write the comparison from the claim file alone. If the inspection file is missing, the candidate
is not ready: go back and inspect the artifact.

---

## 2. Choose the lenses

The comparison always covers the same 14 sections (§6). The lenses decide where the depth goes. Pick the
two or three that fit this candidate; a deep dive that gives every lens equal weight is shallow in all
of them.

### Lens A — Evidence-first

For candidates whose marketing is louder than their evidence: regulated devices, "clinically validated"
claims, a vendor-authored paper, a score with a percentage attached.

Ask: What exactly was measured, on whom, by whom, and when? Is the validation independent, and if a
vendor employee is an author, say so in the same sentence as the claim? Does the published artifact
validate the tier the marketing is selling, or a different one? Does an algorithm change postdate the
validation, and is the change versioned? Is there a regulator record, and what product code did the
regulator assign? Quote the most conservative of conflicting vendor surfaces.

### Lens B — Technical systems

For candidates with a real data path: sensors, sampling rates, storage, sync, derived scores, alerts.

Ask: What is sampled, at what rate, on which clock, and where does the raw value go? Is the raw signal
exportable by the user, or only the derived score? Is the "score" computed on-device or server-side, and
can it be recomputed? What happens to a day with no data — is there an explicit not-measured state, or
does the previous value carry forward silently? What is the failure mode of the alert path when the
phone, the network, or the sensor is absent?

### Lens C — Privacy and operating mode

For candidates that store health data, run ads/analytics, or sell a subscription.

Ask: What does the client actually send, to whom, under what identity, and with what consent default?
Separate the policy's promise from the client's behaviour from the storefront's declaration — three
surfaces, three possible answers. What is the retention period, stated or unstated? What does deletion
delete, and what does it explicitly not delete? What is the regional split in the consent
configuration, and which side of it is opt-out by default? What breaks without a network?

### Lens D — User value and category

For candidates whose main competitive claim is the experience: free positioning, cognition/anxiety
tools, a category the App Store puts us in.

Ask: What does the user actually do on a bad day with this product, minute by minute? What does it do
*during* an event versus after it? What is free and what is gated, and where exactly is the wall? What
would the user lose by switching to ours, and what would they miss?

---

## 3. Required comparison dimensions

Versus **AnxietyWatch v1** (shipped) and **v2** (current + specified, always labelled as such):

- logging burden and subjective signal quality
- sensor access, fidelity and provenance
- derived-value transparency (can the user see how a number was made?)
- correlation/analysis depth and what it may be computed over
- real-time, overnight and always-on capability
- alerting: channels, latency, failure behaviour, and the absent-signal case
- clinical communication: report, export, share, and what the clinician sees
- privacy: local-first vs cloud, export, deletion, third parties
- cost and the shape of the paywall
- evidence quality behind the vendor's central claim
- what we would have to build to match it, and whether we should

Every "AnxietyWatch v2 does not X" statement is checked against the tree before it is written. A
specified capability is described as specified, never as shipped, and never as a gap in the vendor's
favour or ours.

---

## 4. Output contract

Write `{slug}/{slug}_COMPARISON.md` with the 14 sections in §6, in that order, using those headings.
Every substantive sentence carries an evidence class from `PROMPT1.md` §5. Headline findings that rest on
a quotation reproduce the quotation and its source date.

**Never:** present a vendor-authored study as independent validation; present a runtime observation that
was inferred rather than observed; present a plan, spec or proposal of ours as shipped; present a v1
defect as a failure of the vendor's diligence; state float equality; convert a correlation into a cause;
publish a secret.

---

## 5. Self-check before "done"

1. Every `V-n` claim from the claimed-features file appears in a verdict row and has one of the five
   verdicts.
2. Every coverage limit names its mechanism and what was read instead.
3. No sentence reads "inspection not performed" or "no lawful artifact".
4. Every `[STATIC]` finding that is described as user-visible has been checked against the class map for a
   reachable path, or is labelled `[INFERRED]`.
5. Both quote-check modes ran clean (`RESEARCH_METHOD.md` §5).
6. `FINDINGS.md` and `PROCESS_STATE.md` were updated in the same commit.

---

## 6. The 14 sections

1. **Executive finding** — the one paragraph a decision-maker reads. Two-sided if the evidence is.
2. **Product and availability** — what it is, who makes it, where it ships, what it costs.
3. **System boundary and architecture** — device, app, cloud, third parties; where data crosses a boundary.
4. **Data path and source comparison** — every signal, its origin, rate, and where it lands.
5. **Claim-versus-contract reconciliation** — the verdict table from the inspection file, resolved.
6. **Feature matrix** — AnxietyWatch v1 / v2 / candidate, per capability, with evidence class.
7. **Derived values versus stored signals** — what is measured, what is computed, what is exportable.
8. **Real-time, overnight and failure-mode behaviour** — including the absent-signal case and the alarm path.
9. **Provenance, missingness and reproducibility** — can a third party recompute this?
10. **Local, cloud, and privacy posture** — the three surfaces (policy, client, storefront), compared.
11. **Alerting and the alarm path** — channels, thresholds, what happens when the phone is gone.
12. **User-facing consequences** — what the user gets, what they lose, what they are not told.
13. **Limitations, unverified claims and coverage gaps** — with mechanisms named.
14. **Balanced conclusion** — what we should copy, what we should avoid, what this candidate proves about
    the market. Then a **Related files** list.

---

## 7. Variant catalogue (lenses in combination)

| Variant | Lenses | Fits |
|---|---|---|
| `evidence-first` | A (+C if data leaves the device) | Regulated devices, "validated" claims, vendor papers |
| `technical-systems` | B (+A) | Sensor platforms, always-on collection, alerting |
| `privacy-operating-mode` | C (+D) | Ad-funded, cloud-first, subscription-gated products |
| `user-value-category` | D (+B) | Free apps, CBT/anxiety category, journaling apps |
| `full` | A+B+C+D | Our two closest whole-product competitors |

Record the chosen variant in `{slug}/PROMPT_2.md`'s first line, so the comparison's emphasis is
reproducible rather than a matter of taste.
