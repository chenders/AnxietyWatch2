---
description: Run the app-comparison pipeline on ONE candidate, start to finish
argument-hint: "<app name or URL> [slug]"
---
Run the AnxietyWatch app-comparison pipeline on exactly one candidate: **$1** (optional slug: $2).

Read first, in this order, and obey them:
1. `docs/app_comparison/PROMPT1.md` — the brief, the comparability gate, the investigation policy
2. `docs/app_comparison/RESEARCH_METHOD.md` — evidence classes, labelling rules, quote verification
3. `docs/app_comparison/CANDIDATE_SHORTLIST.md` — the queue and this candidate's hypothesis
4. `docs/app_comparison/PROCESS_STATE.md` — what is already open
5. `docs/app_comparison/FREE-ANDROID-TRAFFIC-INSPECTION.md` — the artifact toolchain

Then:

1. `scripts/compare-one-app.sh start "$1"` (add a second argument to force the slug) — it refuses if
   another candidate is in progress.
   If it refuses, stop and report which candidate is open. Do not finish or abandon that one on my behalf.
2. **Public claims.** Write `{slug}_CLAIMED_FEATURES_AND_PURPOSE.md`: every claim quoted, dated and
   sourced, with stable claim ids `V-n`, plus pricing and positioning. Store listing facts and website
   facts are kept apart.
3. **Inspect the artifact — mandatory.** `scripts/app-decompile.sh android <slug> --package <pkg>` (or
   `ios`, or `--apk`/`--ipa` for a fetched artifact). Read the inventory, then read the manifest,
   permissions, SDK inventory, data model, storage, endpoint list and gates directly. Then:
   `scripts/compare-one-app.sh lab <slug>` for runtime observation where a device path exists.
   Write `{slug}_IMPLEMENTATION_INSPECTION.md`, including the **claim-versus-implementation
   reconciliation table** — every `V-n` with a verdict, no row left unmeasured.
4. **Do not write "inspection not performed".** The policy in `PROMPT1.md` §2 is decompile-first and
   ungated. If something stops you, name the mechanism and record what you read instead.
5. **Build `PROMPT_2.md`** from `PROMPT2.md`, recording the chosen lens variant on its first line.
6. **Write `{slug}_COMPARISON.md`** with the 14 sections in order, every substantive sentence carrying an
   evidence class.
7. **Verify every quotation in both modes** (`RESEARCH_METHOD.md` §5): the full corpus and
   `AW2_VENDOR_ONLY=1`. Triage every miss; never auto-accept one. Fix real misquotations against the
   source bytes.
8. **Update `FINDINGS.md`** (its §1 row, the §2 matrix cells you can fill) and `PROCESS_STATE.md`.
9. `scripts/compare-one-app.sh finish <slug>` — it verifies all four deliverables exist and that
   `FINDINGS.md` names the candidate, then prints the commit command.
10. Commit **in the local-only repository**, never in the AnxietyWatch2 repository and never pushed to
    GitHub (`docs/app_comparison/LOCAL-ONLY.md`):
    `git -C docs/app_comparison add -A` then
    `git -C docs/app_comparison commit -m 'research(app-comparison): candidate <N> <slug> — <mode> inspection'`.
    Never `git add -f docs/app_comparison`, never `git add -A` in the parent repository, never to `main`.

Hard limits, which are harm limits and not investigation limits: no other person's credentials or
accounts; no real user data sent to a vendor service; no destructive or write actions against a vendor's
servers; never publish a secret you find — record that it exists, where, and what it grants.

Report at the end: the four file paths, the reconciliation verdict counts, the strongest finding, the
coverage limits with their mechanisms, and the commit hash.
