# App comparison research package

This directory is the working package for the AnxietyWatch / AnxietyWatch2 competitive research programme.
It compares other people's products — on their public claims, on their shipped implementation, and on what
their users actually experience — against what AnxietyWatch v1 does today and what AnxietyWatch2 specifies.

**Reset:** the previous run of this package (11 candidates, 44 deliverables, an Oura findings PDF and a
180 KB process log) was removed on **2026-10-07**. The method was rebuilt around one change of posture:
**implementation inspection is a required step, not a permission.** The old run is still in this branch's
ancestry, so it is recoverable **while that history is kept** — its last complete state is local commit
`0d6cab0`, which is not on `main` and has not been pushed.

---

## The one rule that governs sequencing

**One candidate at a time.** Not two, not a pipelined one, not a "prefetch" of the next one's public
pages while the current one is still being inspected. Everything about this package is designed for
depth per candidate, and depth does not survive concurrency — candidates share terminology, and a fact
gathered for candidate B while writing candidate A reliably ends up in the wrong section.

That rule is enforced by the runner, not by good intentions:

```sh
scripts/compare-one-app.sh start  "<app name or URL>" [slug]   # refuses if one is already in progress
scripts/compare-one-app.sh status                             # who is in progress, if anyone
scripts/app-decompile.sh android <slug> --package <pkg.id>    # obtain + decompile; writes an inventory
scripts/app-decompile.sh ios     <slug> --bundle-id <id>      # unpack + read everything readable
scripts/compare-one-app.sh lab    <slug>                      # proxy + emulator for runtime observation
scripts/compare-one-app.sh finish <slug>                      # closes it after verifying the deliverables
```

If you use the pi harness, the same thing is one command:

```
/compare-app Visible
```

`/compare-app` expands the full single-candidate pipeline prompt and calls the runner, so it cannot start
a second candidate by accident.

---

## The pipeline, per candidate

| Step | What happens | Output |
|---|---|---|
| 0 | Candidate admitted to the shortlist, artifact identified (store id, publisher, version) | `PROCESS_STATE.md` row |
| 1 | **Public claims**: site, store listings, help centre, privacy policy, terms, pricing, support, research | `{slug}_CLAIMED_FEATURES_AND_PURPOSE.md` |
| 2 | **Implementation inspection** — the artifact is obtained and taken apart (`scripts/app-decompile.sh`). Required, not optional. | `{slug}_IMPLEMENTATION_INSPECTION.md` |
| 3 | **Runtime observation** where a device path exists: disposable emulator/device + mitmproxy, launched via `scripts/compare-one-app.sh lab` | same file, `[OBSERVED]` rows |
| 4 | **Claim-versus-implementation reconciliation**: every claim from step 1 marked corroborated, contradicted, unmatched or unmeasurable | `{slug}_IMPLEMENTATION_INSPECTION.md` §reconciliation |
| 5 | **Candidate-specific deep dive** from `PROMPT2.md`, choosing the lenses that fit this candidate | `{slug}/PROMPT_2.md` |
| 6 | **Comparison write-up** against v1 and v2, 14 sections | `{slug}_COMPARISON.md` |
| 7 | **Quote verification** (both modes, see `RESEARCH_METHOD.md` §5), then commit, then close | `FINDINGS.md` row |

Nothing is marked complete because it looks complete. A step is done when its file exists, its quotes
verify against the source bytes, and its evidence classes are labelled.

---

## Files

**Process documents** (change rarely, read before a run):

| File | Purpose |
|---|---|
| `PROMPT1.md` | The master brief: scope, comparability test, pipeline, deliverables, evidence classes |
| `PROMPT2.md` | The per-candidate deep-dive prompt, with four selectable lenses |
| `RESEARCH_METHOD.md` | How evidence is gathered, labelled, verified and quoted |
| `FREE-ANDROID-TRAFFIC-INSPECTION.md` | The Android toolchain: decompile + intercept + the fallbacks |
| `PROJECT-DESCRIPTIONS-AND-CATEGORIES.md` | AnxietyWatch's own descriptions, categories and standout features |

**Live state** (change every run):

| File | Purpose |
|---|---|
| `CANDIDATE_SHORTLIST.md` | The ten candidates, the comparability tiers, and the reserves |
| `PROCESS_STATE.md` | Who is in progress, what is done, what is blocked and why |
| `FINDINGS.md` | The cross-candidate summary — the file a reader should read first |

**Per-candidate output:** `{slug}/` holds the four deliverables listed above.

---

## The posture this package takes

Short version, expanded in `PROMPT1.md` §"Investigation policy":

- **Inspect everything.** Every candidate's artifact gets obtained and taken apart. Android APKs are
  pulled from the device or fetched from a public mirror and read with `jadx`/`apktool`. iOS packages are
  unpacked and everything readable is read. No candidate is closed with "inspection not performed".
- **The only limits are technical, and they are named.** FairPlay encryption, certificate pinning,
  hardware attestation, a paid-account-only server path — each is recorded with the mechanism named,
  in the same breath as what was still read. "Encrypted, and here is what was read instead" is a result.
- **Absence of evidence is `[UNKNOWN]`, not absence.** A feature that could not be reached is not a
  feature that does not exist.
- **A vendor's own material can refute the vendor's own headline**, and when it does, both are quoted.
- **No harm to third parties**: no other person's accounts or credentials, no real user data submitted to
  a vendor service, no destructive action against a vendor's servers, no publication of a secret found
  in code.
