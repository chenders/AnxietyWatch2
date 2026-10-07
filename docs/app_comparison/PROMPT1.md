# PROMPT1 — master brief for the AnxietyWatch app-comparison research

**Version:** 2.0 (2026-10-07 rewrite).
**Supersedes:** the v1 brief, whose Phase 2A was gated on an authorization decision and whose default
outcome for a closed-source candidate was "inspection not performed". That posture is gone. See
§"Investigation policy".

**Read first, every run:** this file, `RESEARCH_METHOD.md`, `CANDIDATE_SHORTLIST.md`, `PROCESS_STATE.md`.
**Read before inspecting:** `FREE-ANDROID-TRAFFIC-INSPECTION.md`.

---

## 1. Objective

Produce an evidence-based comparison of **one** competing or adjacent product at a time against:

- **AnxietyWatch v1** — the shipped Swift/iOS app (journal + HealthKit + Polar H10 + CPAP/pulse-ox import,
  correlation engine, PDF report, self-hosted sync server), and
- **AnxietyWatch2** — the Rust/Pi platform under construction in this repository: `no_std` decoders,
  provenance-preserving append-only store, always-on collectors, and the specified — not yet built —
  CNS early-warning and independent-alarm subsystems.

The comparison answers a product question, not a survey question: **what does this product do that we do
not, what do we do that it does not, and what does its shipped implementation say about how well either
one works?** Every candidate is examined on its public claims *and* on its code, because the gap between
the two is usually the most valuable finding.

### Comparability tiers

A candidate competes only if a person with AnxietyWatch's use case could **adopt it instead of**
AnxietyWatch. Record the tier with every candidate:

| Tier | Meaning | Examples |
|---|---|---|
| **T1 — product competitor** | A product a user chooses instead of AnxietyWatch (app, wearable ecosystem, service) | Bearable, Oura, Visible, Empatica |
| **T2 — platform comparator** | A platform a user would deploy or self-host instead of the AnxietyWatch2 platform | Nightscout, Home Assistant |
| **T3 — tool, format or data source** | Something AnxietyWatch *consumes* or interoperates with. **Not a comparison candidate.** | OSCAR (it reads the same SD-card data we read), EDF, the FHIR standard, an OpenAPI surface with no product behind it |

The T3 test exists because the previous run ranked a desktop CPAP-analysis tool inside its top ten and
then spent an entire candidate's budget discovering that it is a dependency, not a competitor. If a
candidate only makes sense as an input to us, it belongs in `docs/` as an integration note, not here.

---

## 2. Investigation policy

**Inspect everything. Decompile by default.** The artifact is obtained and taken apart for every
candidate. This is step 2 of the pipeline, it is required, and it is not contingent on anything.

There is no authorization gate in this package, no "lawful artifact" precondition, and no acceptable
outcome of the form "inspection not performed". If you find yourself writing that sentence, you have
stopped one step early — write what you *did* read and name the mechanism that stopped you at that point.

**What is read, in order of value:**

1. **The decompiled artifact.** Android: pull the installed package (`adb shell pm path <pkg>`, then
   `adb pull`) or fetch the published APK from a public mirror, then read it with `jadx` (Java/Kotlin),
   `apktool` (resources, manifest, smali), `baksmali`, `apkanalyzer`. iOS: unpack the `.ipa`, read
   `Info.plist`, entitlements, resources, bundled configuration, localization tables, framework
   metadata, and the readable parts of the Mach-O.
2. **The class map.** What the app can do: screens, data models, permissions, background modes, Bluetooth
   and HealthKit identifiers, storage keys, export paths, notification channels, analytics and
   advertising SDKs, subscription gates, feature flags, endpoints, error handling.
3. **The network contract.** Every host, path, auth scheme, payload shape and third party that the
   decompiled client talks to — taken from the code, which is more complete than any captured session.
4. **Runtime observation**, where a device path exists: a disposable emulator or device, synthetic data,
   mitmproxy on `10.0.2.2:8080` configured *before* the app launches, packet capture as a fallback.
   Runtime confirms what static analysis predicts; it does not replace it.
5. **The public record**: store listings and their data-safety panels, the privacy policy and terms, the
   help centre, pricing, the vendor's repositories, regulator filings, registered trials, peer-reviewed
   publications and their corrigenda.

**Technical limits are the only limits, and each one is recorded with its mechanism named.**

| Limit | How it is recorded |
|---|---|
| FairPlay-encrypted iOS code sections, with no decryption device available | "Code sections are FairPlay-encrypted; read instead: `Info.plist`, entitlements, 312 bundled resources, 6 framework plists, and the served client bundle." |
| Certificate pinning prevents interception | "The client pins; the endpoint contract is therefore taken from the decompiled code, which lists 41 hosts — recorded as `[STATIC]`, not `[OBSERVED]`." |
| Hardware-backed attestation / Play Integrity blocks a modified client | "Attestation rejects the emulator; the flows behind it are read statically and recorded as `[STATIC]`." |
| A server path that exists only behind a paid account | "Recorded `[UNKNOWN]`; the client-side request shape is known, the response is not." |
| Obfuscated native code with no readable symbols | "Native libraries are stripped; JNI entry points and the Java/Kotlin side are read; the native layer is named as unread." |

**What this package does not do — harm limits, not permission limits.** These protect third parties
rather than constrain the investigation, and they are absolute:

- **No other person's credentials or accounts.** Use your own throwaway accounts or none; never a real
  user's session, and never a token lifted from a leaked dump.
- **No real user data submitted to a vendor's service.** Synthetic data only, in any live request.
- **No destructive actions against a vendor's servers.** Reads, not writes; no account deletion, no
  abuse of an administrative surface, no traffic that degrades the service.
- **A secret found in code is a finding, and it is never published.** Record that it exists, where, and
  what it grants. Redact the value. Do not use it to reach anything.
- **Do not redistribute a vendor's binary or its decompiled source** in this repository.

These four are the whole list. Anything else that looks like a rule is either a technical limit (record
it as one) or is not a rule.

---

## 3. Sequencing

**Exactly one candidate is active at a time.** Start it with the runner, which refuses to start a second:

```sh
scripts/compare-one-app.sh start "Visible" visible      # creates docs/app_comparison/visible/
scripts/compare-one-app.sh lab visible                  # the removable inspection lab, when wanted
scripts/compare-one-app.sh finish visible               # closes it, updates FINDINGS.md
```

No cross-candidate parallelism, no prefetching the next candidate's public pages while the current one is
open, no shared scratch directory. `PROCESS_STATE.md` holds the queue and the in-progress row; the runner
reads and writes it.

---

## 4. Deliverables — four files, same names every time

| File | Contents |
|---|---|
| `{slug}_CLAIMED_FEATURES_AND_PURPOSE.md` | What the vendor says the product is and does: every claim, dated, quoted, sourced; pricing; positioning; audience; and a claim ledger with stable ids (`V-1`, `V-2`, …) |
| `{slug}_IMPLEMENTATION_INSPECTION.md` | What the artifact actually is: build, hashes, tools used, permissions, SDK inventory, data model, endpoints, storage, background behaviour, plus the **claim-versus-implementation reconciliation table** |
| `PROMPT_2.md` | The candidate-specific deep-dive prompt, derived from `PROMPT2.md` with the lenses that fit |
| `{slug}_COMPARISON.md` | The comparison itself: the 14 sections listed in `PROMPT2.md` §6 |

A candidate is complete when all four exist, the reconciliation table has no unexplained rows, and the
quotations verify (§5 of `RESEARCH_METHOD.md`).

---

## 5. Evidence classes

Every substantive statement in every deliverable carries one of these. They are not decoration; the
reconciliation table is only meaningful because the classes are kept apart.

| Class | Meaning |
|---|---|
| `[DOCUMENTED]` | The vendor states it, on a surface the vendor controls (site, listing, help article, contract, published spec) |
| `[STATIC]` | Read out of the artifact: code, resources, manifest, configuration. **Not** proof of reachability |
| `[OBSERVED]` | Produced by execution on a controlled device: UI seen, traffic captured, file written |
| `[PUBLISHED-CLIENT]` | Read out of a client the vendor serves to browsers (JS bundle, static config), not from the app package |
| `[THIRD-PARTY]` | An independent record: regulator, registry, peer-reviewed paper, another vendor's disclosure |
| `[INFERRED]` | The researcher's reasoning from the above, labelled as such |
| `[UNKNOWN]` | Not established. **Not** a synonym for "absent" or "no" |

Rules that follow from the classes:

- A string is not a feature. A bundled SDK is not a used SDK. An endpoint in the code is not an endpoint
  the app reaches. `[STATIC]` never upgrades itself to `[OBSERVED]`.
- A documented contract is not an observed behaviour. Label it `[DOCUMENTED]` and say what was not called.
- A vendor-authored study is `[THIRD-PARTY]` only in the sense that it is published; it is **not
  independent validation** of the vendor's product, and must be labelled as vendor-affiliated when it is.
- Missing data contributes nothing. Never render, log or score an absent signal as a good one, and never
  coerce a sentinel (SFLOAT `0x07FF`/`0x0800`) to a number.
- Never state float equality; use a tolerance. Never present a correlation as a cause.

---

## 6. Claim-versus-implementation reconciliation

The core instrument of this package. Every `V-n` claim from the claimed-features file appears here once:

| Claim id | Claim (quoted) | Class of evidence found | Verdict | Detail |
|---|---|---|---|---|
| `V-7` | "Get a phone notification when you are exerting…" | `[STATIC]` + help-centre `[DOCUMENTED]` | **Corroborated** | Two alert types in the client; thresholds server-set |
| `V-12` | "…all your data is encrypted" | `[DOCUMENTED]` conflicting surfaces | **Contradicted** | Policy permits ad-platform sharing; store panel lists a transfer the help centre omits |
| `V-19` | "clinically validated" | vendor-authored paper `[THIRD-PARTY]` | **Partially corroborated** | Paper validates the free tier's daily reading, not the paid continuous tier |
| `V-23` | "works offline" | `[STATIC]` | **Contradicted** | Measurement path requires a network round trip; the app blocks the reading without one |

Verdicts are exactly: **Corroborated**, **Contradicted**, **Partially corroborated**, **Unmatched**
(the claim has no implementation counterpart either way), **Unmeasurable** (cannot be established with
the available surfaces — say why). Never invent a sixth, and never leave a row without a verdict.

---

## 7. The summary files

- **`FINDINGS.md`** is the entry point for a reader who will not read eleven candidate directories. It
  accumulates: the cross-candidate feature matrix, the strongest finding per candidate, the recurring
  patterns across candidates (a gated morning score, a missing-data rule, a vocabulary of "not
  measured"), and the questions the run has not answered. It is updated at `finish`, never left stale.
- **`PROCESS_STATE.md`** is the operational record: queue, in-progress row, per-candidate status,
  coverage limits, and the decisions that changed the method mid-run, with dates.
- **`RESEARCH_METHOD.md`** is the rule register. A rule is added there when a run produces one worth
  keeping — and the rule is stated as a rule, not as a retelling of the incident that produced it.

---

## 8. What a good run leaves behind

1. Four deliverables per candidate, evidence-classed, with quotations that verify against the source
   bytes in both quote-check modes.
2. A reconciliation table in which every vendor claim has a verdict, and no verdict is "we did not look".
3. A named coverage limit wherever one exists — the mechanism, and what was read instead.
4. At least one finding per candidate that could not have been obtained from the vendor's marketing:
   usually a contradiction between two vendor surfaces, a documented default the code does not
   implement, or an algorithm change shipped without a version.
5. A `FINDINGS.md` that a product decision can be made from, and a `PROCESS_STATE.md` that a reader can
   reproduce the run from.
