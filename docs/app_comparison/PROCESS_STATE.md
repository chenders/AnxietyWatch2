# PROCESS_STATE — run state and queue

**Research date:** 2026-10-07 · **Method:** `RESEARCH_METHOD.md` · **Brief:** `PROMPT1.md`
**Shortlist:** `CANDIDATE_SHORTLIST.md` (v2, ten candidates) · **Summary:** `FINDINGS.md`

**Reset:** the previous run's record (11 candidates, 44 deliverables, an Oura findings PDF, a 180 KB
process log and a 2,000-file quote corpus) was removed on 2026-10-07. Its last complete state is local
commit `0d6cab0` in this branch's ancestry — not on `main`, and not pushed anywhere. This file starts from
empty, and the empty state below is accurate: **no candidate has been researched in this run.**

---

## 1. In progress

**None.** One candidate may be active at a time; the runner enforces it.

```sh
scripts/compare-one-app.sh status      # prints the in-progress candidate, or "none"
```

---

## 2. Queue

Status values: `not-started` · `in-progress` · `complete` · `held`.

| # | Slug | Tier | Status | Started | Finished | Files |
|---|---|---|---|---|---|---|
| 1 | bearable | T1 | not-started | - | - | 0/4 |
| 2 | visible | T1 | not-started | - | - | 0/4 |
| 3 | welltory | T1 | not-started | - | - | 0/4 |
| 4 | empatica | T1 | not-started | - | - | 0/4 |
| 5 | oura | T1 | not-started | - | - | 0/4 |
| 6 | nightscout | T2 | not-started | - | - | 0/4 |
| 7 | homeassistant | T2 | not-started | - | - | 0/4 |
| 8 | sleephq | T1 | not-started | - | - | 0/4 |
| 9 | mindshift | T1 | not-started | - | - | 0/4 |
| 10 | exist | T1 | not-started | - | - | 0/4 |

**Reserve (not queued):** `emoods`, `daylio`, `withings-sleep`, `medisafe`, `loop`. See
`CANDIDATE_SHORTLIST.md` §4.2. Admitting one of these is an owner decision, recorded here with its date.

---

## 3. Sequencing rule

Only one row may be `in-progress`. `scripts/compare-one-app.sh start` refuses when another row is open, and
`finish` is what closes a row — not the existence of a directory. Per `PROMPT1.md` §3, there is no
cross-candidate parallelism and no prefetching of the next candidate's material while one is open.

---

## 4. Coverage limits

Recorded per candidate as they occur: the mechanism, and what was read in its place
(`PROMPT1.md` §2). Nothing is recorded here yet in this run.

| Candidate | Limit | Mechanism | Read instead |
|---|---|---|---|
| — | — | — | — |

---

## 5. Decisions that changed the method

| Date | Decision | Reason |
|---|---|---|
| 2026-10-07 | OSCAR removed from the shortlist; a **T3 comparability gate** added | OSCAR is a tool and data source this project consumes, not a product a user adopts instead of AnxietyWatch. A candidate budget was spent establishing that in the previous run. |
| 2026-10-07 | **Inspection is mandatory and ungated.** The previous run's authorization step, "lawful artifact" precondition and "inspection not performed" outcome are removed from every process document | A comparison package that cannot see inside the products it compares produces claim-versus-claim marketing summaries. Technical limits are recorded and named; permission is not a category. |
| 2026-10-07 | Inspection capability removed from the ranking dimensions (F), leaving evidence quality | The old F mixed "is the evidence good" with "can we see inside it", which rewarded published APIs over shipped products. |
| 2026-10-07 | `visible` promoted from reserve into the ten; `oscar`'s slot | It is the only candidate combining a shipped continuous-monitoring consumer alert with a vendor-authored peer-reviewed paper. |
| 2026-10-07 | Summary file `FINDINGS.md` created; the per-candidate PDF practice dropped | One readable entry point beats a PDF regenerated per candidate with a WeasyPrint toolchain. |

---

## 6. Rules this file enforces

- A row is closed by `finish`, which verifies all four files exist and that `FINDINGS.md` was updated in
  the same commit.
- A candidate's status is never advanced on the strength of a claim. Four files exist, or the row is open.
- A coverage limit names its mechanism. "Not inspected" is not a limit, it is an unfinished task.
- Nothing is marked complete without the candidate's own quote verification having run in both modes.
- A count in this file is counted, never remembered. The `Files` column is produced by
  `scripts/compare-one-app.sh status`, not typed.
