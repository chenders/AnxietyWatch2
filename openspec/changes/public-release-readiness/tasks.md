# Tasks: Public-release readiness

Ordered; each task depends on the one before it unless it says otherwise. `$PURGE` means
`$HOME/Source/aw2-history-purge`, the out-of-repo directory `history-phi-purge` uses. **Nothing
under `$PURGE` is ever copied into the repository.**

**The tip is frozen from T-PR-7 to T-PR-8**: any commit to `main` makes the rewritten mirror
stale and breaks the push leases. Decisions and results recorded during the freeze go to
`$PURGE/records.md` (out of the repository, never quoting a removed value); T-PR-15 copies them
here (the T-PR-16 results heading, never its private section), and ticks the boxes, in one PR after the push. **The
freeze applies again to T-PR-16's re-run**, from its re-run of T-PR-7 until T-PR-16's Verify exits
0: T-PR-16's tip PR and the PR that adds T-PR-16 merged with their head branches deleted before `final.sh`
runs, no other PR open or branch pushed, Dependabot paused (step 1), and any other session's work
pushed before the re-run starts.

Four steps are **owner actions** (T-PR-16's re-push included) and one is an **owner
decision**; every other step is reversible
or local and proceeds without asking. Every Verify that calls GitHub runs under `set -o pipefail`,
so an unreachable API fails the check instead of passing it.

- [ ] T-PR-1 — Land or close every open pull request · integrate · host
- [ ] T-PR-2 — Remove the medication names from the design mock-ups · implement-simple · host
- [ ] T-PR-3 — Reject self-hosted runners on default-branch fork-triggerable workflows · implement · host
- [ ] T-PR-4 — Add SECURITY.md · implement-simple · host
- [ ] T-PR-5 — Turn the personal-term check on locally · ops · host
- [ ] T-PR-6 — Decide signatures and the runner's exposure · **owner decision**
- [ ] T-PR-7 — Re-run the purge on the final tip and prove it · implement · host
- [ ] T-PR-8 — Force-push the rewritten history · ops · host · **owner action**
- [ ] T-PR-9 — Move every clone onto the new history · ops · host, pi
- [ ] T-PR-10 — Delete every Actions run, artifact and cache · ops · host
- [ ] T-PR-11 — Scrub GitHub-side text: PR bodies, comments, review diff hunks · ops · host
- [ ] T-PR-16 — Remove what the fixture history lets a reader infer, then re-push · implement ·
  host · **owner action** (re-push)
- [ ] T-PR-12 — Ask GitHub to purge what only it can reach · ops · **owner action**
- [ ] T-PR-13 — Prove what GitHub serves is clean · investigate · host
- [ ] T-PR-14 — Publish with no exposure window · ops · **owner action**
- [ ] T-PR-15 — Record the results and gate every commit a PR introduces · implement · host

---

### T-PR-1 — Land or close every open pull request

**Depends:** none · **Review required:** per PR

**Amends:** none.

**Reads:** `gh pr list --state open`.

**Do:** A force-push rewrites the base of every open PR under it. Merge each open PR that has
passed review and CI; close the rest with a comment saying why (Dependabot re-opens its own after
the push). The only PRs opened before T-PR-7 are this change's own.

**Verify:**
```sh
set -o pipefail; n=$(gh pr list --state open --json number --jq length) && test "$n" = 0
```

**Context budget:** the open PRs.

---

### T-PR-2 — Remove the medication names from the design mock-ups

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `docs/design/admin-console/claude-design/{a,b,c}/records.html`,
`$PURGE/manifest/replace-health.txt`.

**Do:** Replace every medication **name** in the three mock-ups with the neutral placeholders the
purge already uses for the iOS client, so tip and rewritten history agree. Doses stay: without a
name a dose identifies nothing. Add matching pairs for every historical form to
`$PURGE/manifest/replace-health.txt`, the terms to `$PURGE/manifest/medication-terms.txt`, and a
pattern to `$PURGE/residuals.sh`. Name no medication in a commit message, a PR or this file.

**Verify:**
```sh
F="$HOME/Source/aw2-history-purge/manifest/medication-terms.txt"; test -s "$F" && git fetch -q origin && ! git grep -qiE -f "$F" origin/main
```

**Context budget:** 3 files, 3 out-of-repo files.

---

### T-PR-3 — Reject self-hosted runners on default-branch fork-triggerable workflows

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `.github/workflows/*.yml`, `.github/workflows/ci.yml` (`public-info` job).

**Do:** `scripts/check-workflow-runners.py`, run in `public-info`: a workflow whose triggers
leave {`push`, `workflow_dispatch`, `schedule`} must use GitHub-hosted labels only and must not
call a remote reusable workflow; violations name file, job and trigger; malformed input fails
with a named reason. **Scope, stated plainly:** this guards workflows that reach the default
branch (`pull_request_target`, `issue_comment`, `workflow_run`, …). It does **not** stop a fork
PR from running its *own* workflow files on `pull_request`, merged or not — that is T-PR-6's
runner decision and T-PR-14's approval policy.

**Verify:**
```sh
uv run python3 scripts/check-workflow-runners.py --self-test && uv run python3 scripts/check-workflow-runners.py && grep -q 'check-workflow-runners.py' .github/workflows/ci.yml
```

**Context budget:** 3 files.

---

### T-PR-4 — Add SECURITY.md

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `README.md`, `scripts/check-tracked-paths.py` (root-file allowlist).

**Do:** Private vulnerability reporting (enabled in T-PR-14), scope, the deliberately deferred
gaps in `prototype-hardening-backlog`, and an accurate description of how pull requests are kept
off the self-hosted runner. Allowlist it in `check-tracked-paths.py` in the same commit.

**Verify:**
```sh
test -s SECURITY.md && uv run python3 scripts/check-tracked-paths.py
```

**Context budget:** 2 files.

---

### T-PR-5 — Turn the personal-term check on locally

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `.githooks/pre-push` (denylist section), `.githooks/pii-denylist.local.txt.example`,
`$PURGE/manifest/replace-identifiers.txt`.

**Do:** Create `.githooks/pii-denylist.local.txt` (git-ignored) from the personal terms the purge
manifest replaces, excluding terms the repository uses legitimately (the public GitHub handle, the
LICENSE name) and the LAN bind address (a non-goal). The hook reads it from the root of the
worktree it runs in, so every worktree needs its own copy.

**Verify:**
```sh
test -s .githooks/pii-denylist.local.txt && git check-ignore -q .githooks/pii-denylist.local.txt && ! git ls-files --error-unmatch .githooks/pii-denylist.local.txt 2>/dev/null
```

**Context budget:** 1 local file.

---

### T-PR-6 — Decide signatures and the runner's exposure

**Depends:** T-PR-1…T-PR-5 · **Owner decision** (one decision point, two questions)

**Amends:** history-phi-purge T-HP-5 (it assumed a `non_fast_forward` rule that does not exist
and did not account for signatures).

**Reads:** `gh api repos/chenders/AnxietyWatch2/rulesets/20287061`,
`gh api repos/chenders/AnxietyWatch2/actions/runners`.

**Do:** Present, with costs and a recommendation, and record the answers in `$PURGE/records.md`:
1. **Signatures** — `required_signatures` (active, no bypass actor) rejects the unsigned commits
   `git filter-repo` produces. *Re-sign:* sign every rewritten commit with the owner's key, setting
   the owner as committer on commits GitHub, Dependabot or Copilot committed (a signature verifies
   only against the committer). *Lift:* disable the rule for the one push, re-enable after; history
   stays unsigned.
2. **Runner** — on a public repository a fork PR can run its own workflow on `pull_request`, and
   GitHub advises against self-hosted runners on public repositories. *Move:* re-register
   `vitals-pi` to a private repository that owns deployment, and remove it from this one.
   *Approval-gated:* keep it, require approval for every external contributor's run, and accept
   that one mistaken approval runs a stranger's code on the Pi.

Decided before T-PR-7 so the mirror is built once, for the chosen push.

**Verify:**
```sh
R="$HOME/Source/aw2-history-purge/records.md"; grep -qE '^Signatures: (re-sign|lifted)$' "$R" && grep -qE '^Runner: (moved|approval-gated)$' "$R"
```

**Context budget:** none.

---

### T-PR-7 — Re-run the purge on the final tip and prove it

**Depends:** T-PR-6 · **Review required:** yes

**Amends:** history-phi-purge T-HP-3, T-HP-4 (re-run, unchanged in substance).

**Reads:** `$PURGE/final.sh`, `$PURGE/verify.py`, `$PURGE/residuals.sh`.

**Do:** The tip is frozen from here to T-PR-8. Run `$PURGE/final.sh` (fresh `--all` backup,
manifest regenerated against the current `origin/main`, full-mirror rewrite, `verify.sh`,
`residuals.sh`); if T-PR-6 chose re-signing, re-sign the mirror and re-run the checks; scan every
ref with gitleaks; regenerate `$PURGE/push.sh` with leases pinned to GitHub's current branch SHAs.
Write `$PURGE/old-shas.txt`: a sample of pre-rewrite commit SHAs (every commit that touched a
removed path, plus the tips of every pushed branch) for T-PR-9 and T-PR-13.

**Verify:**
```sh
P="$HOME/Source/aw2-history-purge"; M="$P/mirror.git"; test -s "$P/records.md" && bash "$P/verify.sh" && bash "$P/residuals.sh" "$M" && gitleaks git --no-banner --redact --log-opts=--all "$M" && test -s "$P/old-shas.txt" && { ! grep -qx 'Signatures: re-sign' "$P/records.md" || { refs=$(git --git-dir="$M" for-each-ref --format='%(refname)' refs/aw2-remotes/origin/) && test -n "$refs" && test -z "$(git --git-dir="$M" log --format=%G? $refs | grep -vx G)"; }; }
```

**Context budget:** the mirror.

---

### T-PR-8 — Force-push the rewritten history

**Depends:** T-PR-7 · **Review required:** yes · **Owner action**

**Amends:** history-phi-purge T-HP-5.

**Reads:** `$PURGE/push.sh`.

**Do:** With no open PR, run `$PURGE/push.sh` (atomic, `--force-with-lease` per branch). If the
rule was lifted, the owner restores it immediately after.

**Verify:**
```sh
set -o pipefail; M="$HOME/Source/aw2-history-purge/mirror.git"; heads=$(git ls-remote --heads origin) && test -n "$heads" && while read -r sha ref; do test "$(git --git-dir="$M" rev-parse "refs/aw2-remotes/origin/${ref#refs/heads/}")" = "$sha" || { echo "mismatch: $ref"; exit 1; }; done <<<"$heads" && gh api repos/chenders/AnxietyWatch2/rulesets/20287061 --jq '.enforcement + " " + ([.rules[].type] | join(","))' | grep -qE '^active .*required_signatures'
```

**Context budget:** none.

---

### T-PR-9 — Move every clone onto the new history

**Depends:** T-PR-8 · **Review required:** no

**Amends:** history-phi-purge T-HP-6, T-HP-7.

**Reads:** `$PURGE/backup/` (stash and worktree lists), `scripts/sync-to-pi.sh`.

**Do:** As T-HP-6 for this checkout and its worktrees (reflog expired, `gc --prune=now`). The
Pi has **no git clone** of the repository: `$PI_PATH` is an rsync'd tree, so it has no history,
but rsync can carry untracked or ignored files — remove any removed path found there. The
self-hosted runner's checkout under `/opt/actions-runner-vitals/_work/` *is* a clone (shallow);
delete it, and expect `pi-deploy` to re-create it from the rewritten history after T-PR-8. This
amends T-HP-7, which assumed a clone. The Pi half of the check is `$PURGE/pi-residue-check.sh`:
it applies every `manifest/paths.txt` entry to the Pi tree with filter-repo's rules (literal =
that path or anything under it; `glob:`; `regex:`), requires the runner checkout, if present, to
hold none of `old-shas.txt`, and fails closed — naming entry numbers, never values — on an unset
`PI_HOST`/`PI_PATH`, an unreachable host, a missing tree or a refused `sudo -n`. If the Pi is
powered down this task waits; T-PR-10 and T-PR-11 do not depend on it (T-PR-12 does, through
T-PR-16, whose re-run repeats this task).

**Verify:**
```sh
P="$HOME/Source/aw2-history-purge"; test -s "$P/old-shas.txt" && ! while read -r s; do git cat-file -e "$s^{commit}" 2>/dev/null && echo "still local: $s"; done < "$P/old-shas.txt" | grep -q . && bash "$P/pi-residue-check.sh"
```

**Context budget:** none.

---

### T-PR-10 — Delete every Actions run, artifact and cache

**Depends:** T-PR-8 · **Review required:** no

**Amends:** none.

**Reads:** `gh api repos/chenders/AnxietyWatch2/actions/{runs,artifacts,caches}`.

**Do:** On a public repository run logs are readable by anyone and artifacts by any signed-in
user; they also quote pre-rewrite SHAs and messages, and at least one deploy log contains the
Pi's hostname. A rewrite touches none of it. Delete every workflow run (which removes its logs
and artifacts), any remaining artifact, and every cache. CI runs created after this point run on
the rewritten history only, until T-PR-16's re-push, whose re-run repeats this task.

**Verify:**
```sh
set -o pipefail; for e in runs artifacts caches; do n=$(gh api "repos/chenders/AnxietyWatch2/actions/$e" --jq .total_count) && test "$n" = 0 || { echo "$e remaining: ${n:-api error}"; exit 1; }; done
```

**Context budget:** none.

---

### T-PR-11 — Scrub GitHub-side text: PR bodies, comments, review diff hunks

**Depends:** T-PR-8 · **Review required:** yes

**Amends:** none.

**Reads:** the purge manifest; `gh api` for pulls, issues, issue comments and review comments.

**Do:** None of this is in git. Write `$PURGE/scan-github-text.py`: page through every PR and
issue (title, body), issue comment, PR review **summary** (`pulls/{n}/reviews`, body), review
comment (body **and** `diff_hunk`) and commit comment, match the manifest's sources, residual
patterns and medication terms, and print each hit as kind + id + count (never the value); exit 1
on any hit **or any API failure**. Edit or delete each hit's text through the API. Review-comment
diff hunks cannot be edited: delete the comment, or list its id in
`$PURGE/support-request-hunks.txt` for T-PR-12. The allowlist option accepts **only**
review-comment `diff_hunk` hits; any other kind listed there is itself a failure.

**Verify:**
```sh
uv run python3 -I "$HOME/Source/aw2-history-purge/scan-github-text.py" --allow-listed-for-support "$HOME/Source/aw2-history-purge/support-request-hunks.txt"
```

**Context budget:** the GitHub API.

---

### T-PR-16 — Remove what the fixture history lets a reader infer, then re-push

**Depends:** T-PR-11 · **Blocks:** T-PR-12 (and so T-PR-13, T-PR-14) · **Review required:** yes ·
**Owner action** (the re-push, as T-PR-8)

**Amends:** history-phi-purge T-HP-2 (manifest additions) and T-HP-4 (re-run); re-runs T-PR-7 to
T-PR-11 of this change.

**Reads:** `$PURGE/records.md` (all of it: earlier entries carry run conditions the re-run needs),
`$PURGE/t-pr-16-check.sh`, `$PURGE/final.sh`, `$PURGE/run-push.sh`,
`$PURGE/push-with-coverage-lifted.sh`, `$PURGE/ruleset-20287061-before.json`,
`$PURGE/migrate-local.sh`, `$PURGE/delete-actions-data.sh`, `$PURGE/fix-github-text.py`,
`$PURGE/rewrite.sh`, `$PURGE/assemble.sh`, `$PURGE/verify.py`, `$PURGE/residuals.sh`,
`$PURGE/scan-github-text.py`, `$PURGE/gen-push-and-old-shas.sh`, `$PURGE/push.sh`,
`$PURGE/old-shas.txt`, `$PURGE/resign.sh`, `$PURGE/verify.sh`, `$PURGE/pi-residue-check.sh`,
`$PURGE/.gitleaksignore`, `$PURGE/manifest/` (`replace-content-extra.txt`, `replace-health.txt`,
`replace-message-extra.txt`, `renames.txt`, `blob_callback.py`, `commit_callback.py`,
`location-commits.txt`, `stash-exclude.txt`, `readme-oids.txt`, `checksum-mask-map.new.png`),
`$PURGE/mirror.git/filter-repo/commit-map`, `.github/dependabot.yml`; on `main`,
the tip files listed in `$PURGE/records.md` (section T-PR-16).

**Why:** a tenth independent semantic audit of the history-phi-purge mirror, run before T-PR-8,
returned PUSH-BLOCKING with three MEDIUM findings, all present in what T-PR-8 pushed. None is a
value, a location or an identity: each is something the way the decoder fixtures were substituted,
or wording around them, lets a reader infer about the owner's real readings. The specifics, the
files involved and the patterns are recorded in `$PURGE/records.md` and `$PURGE/t-pr-16-check.sh`,
both outside the repository; this task deliberately does not restate them, because this file will
be public.

**Do:**
1. **Tip (a normal PR, merged before the re-run):** reword the tip files listed in
   `$PURGE/records.md` as recorded there, re-render the README chart PNG with `rsvg-convert` after
   first confirming that rendering the unchanged SVG reproduces the committed PNG with zero
   differing pixels, and confirm `t-pr-16-check.sh` reports no tip match. In the same PR, set
   `open-pull-requests-limit: 0` for every ecosystem in `.github/dependabot.yml` (the freeze's
   Dependabot pause; restored after this task's Verify exits 0). That PR's commit messages and
   body describe only a wording and fixture clean-up: they quote no removed phrase and state no
   inference, as T-PR-2 requires for its own PR.
2. **Manifest (`$PURGE`, never the repository):**
   a. design the fixture substitution pairs, every historical form of each value (encoded and
      decoded, and every table that repeats them), keeping each old commit's own tests passing;
      record them in `$PURGE/records.md` (section T-PR-16) before applying them, and run the
      decoder crate's tests at the first commit they touch in the rewritten mirror (not checked by
      the Verify; record the result);
   b. map every historical form of the wording step 1 changes to the new wording;
   c. add message pairs for every message `t-pr-16-check.sh`'s patterns match on any ref of the
      rewrite mirror (`git log --all`, which `residuals.sh` also scans), including messages
      wrapped across lines;
   d. make `manifest/checksum-mask-map.new.png` byte-identical to step 1's committed PNG, and key
      the blob callback to every historical chart PNG object except that one;
   e. add the check's two pattern literals, verbatim, to the `BLOB_RX` and `MSG_RX` literals of
      `residuals.sh` (which `scan-github-text.py` also reads), and make both match with whitespace
      collapsed per object, so T-PR-13 checks `refs/pull/*` and GitHub text the same way;
   f. never overwrite `old-shas.txt`: first copy it to `old-shas.gen1.txt` and make
      `gen-push-and-old-shas.sh` append and de-duplicate; then append the current tip of every
      GitHub branch (the lease SHAs of the regenerated `push.sh`), every commit the re-run's
      `filter-repo/commit-map` maps to a different ID, and every superseded PR head recorded in
      `$PURGE/records.md`;
   g. empty `location-commits.txt`, `stash-exclude.txt` and `readme-oids.txt`, recording the
      reason in `$PURGE/records.md`: they name first-generation IDs that no longer exist (the
      surviving commit map points at pre-signing IDs, and blob IDs cannot be translated), and the
      pushed generation already carries the replaced commit bodies and the dropped lines, as
      `verify.sh` and `residuals.sh` show. The location and usage blob lists are regenerated by
      `final.sh` on every run;
   h. point `resign.sh` at a current owner-signed commit on `main` for the owner's identity (the
      commit it names now is a first-generation SHA that no longer exists), recording which in
      `$PURGE/records.md`;
   i. after the rewrite and re-signing, refresh the fingerprints in `$PURGE/.gitleaksignore`: its two known
      fixture findings are pinned to commit SHAs the rewrite changes.
3. **Re-run, under the freeze:** first pin every stash in the local checkout as
   `refs/aw2-stash/<n>` (`git update-ref refs/aw2-stash/<n> stash@{<n>}`), as the first run did by
   hand -- `rewrite.sh` carries stashes only through those refs -- and record
   `T-PR-16 stashes pinned: <count>`; T-PR-9's re-run must restore that many. Then T-PR-7 (fresh
   backup, `final.sh`, re-sign if T-PR-6 chose it,
   gitleaks, `push.sh` with fresh leases), recording `T-PR-16 re-run T-PR-7 Verify: exit 0`
   under the results heading of `$PURGE/records.md` (where every result line below also goes); an independent semantic audit of the new mirror, recorded as
   `T-PR-16 audit: clean` or `T-PR-16 audit: LOW` (the last such line counts), proceeding only if
   nothing above LOW remains; the owner's re-push (lifting `code_coverage` if needed, as at
   T-PR-8); T-PR-9 (every clone and the Pi; `PI_HOST` and `PI_PATH` exported); T-PR-10 (delete the
   Actions runs created since, recording the time in `$PURGE/records.md` as
   `T-PR-16 re-run T-PR-10: <UTC timestamp>`); T-PR-11 (scrub GitHub text, including this task's
   PR body), recording every item it edits as `T-PR-16 edited: <kind>:<id>` for T-PR-12, since
   each edit leaves its old revision in GitHub's edit history; the owner deletes those revisions in
   the UI, including the superseded revisions of the PR that adds T-PR-16 and the revisions left by the
   first run's T-PR-11 edits that `$PURGE/records.md` lists, before T-PR-13 runs
   (T-PR-13 fails on any matching revision still visible). Each re-run task's own Verify must exit 0; the check below runs
   them. Keep `$PURGE/mirror.git` until this task's Verify exits 0, then restore the Dependabot
   limit.

**Verify:** the check's sha256 is fixed here, so changing the check is a deliberate, reviewed edit
of this line. The check fails closed on any git error and requires: the tip, every reachable blob
and every commit message on GitHub's branches free of the recorded patterns (matched with
whitespace collapsed); only the tip's chart PNG in history; `old-shas.txt` containing the first
generation, every recorded superseded PR head, and every commit the re-run rewrote; leases that
name exactly GitHub's branches at this generation's SHAs; the patterns in `residuals.sh`;
the Verify of T-PR-8 to T-PR-11 exiting 0 (so `PI_HOST` and `PI_PATH` exported and the Pi up);
the recorded re-run of T-PR-7; a re-rendered tip chart PNG; a last audit verdict of clean or LOW;
and as many stashes restored as were pinned.
```sh
P="$HOME/Source/aw2-history-purge"; test "$(shasum -a 256 "$P/t-pr-16-check.sh" | cut -d' ' -f1)" = 2080ede13be39f6262af25845db29fc9da847f083fab892a16ab001f5fe1b280 && bash "$P/t-pr-16-check.sh"
```

**Context budget:** the `$PURGE` manifest and scripts, and the tip files recorded there.

---

### T-PR-12 — Ask GitHub to purge what only it can reach

**Depends:** T-PR-10, T-PR-11, T-PR-16 · **Owner action**

**Amends:** history-phi-purge T-HP-8.

**Reads:** GitHub's documentation on removing sensitive data from a repository.

**Do:** A request is drafted for the owner to send: the repository; the cached `refs/pull/*` of
every PR; cached views; **unreachable and orphaned commits** (GitHub serves `/commit/<sha>` until
it garbage-collects them — `$PURGE/old-shas.txt` and filter-repo's first-changed-commit list name
them); the PR body edit histories, superseded PR heads and `T-PR-16 edited:` items listed in
`$PURGE/records.md`; and the
review comments from `support-request-hunks.txt`. Record the ticket number, then
GitHub's confirmation date, in `$PURGE/records.md`.

**Verify:**
```sh
grep -qE '^GitHub support ticket: [0-9]+ \(purge confirmed [0-9]{4}-[0-9]{2}-[0-9]{2}\)$' "$HOME/Source/aw2-history-purge/records.md"
```

**Context budget:** none.

---

### T-PR-13 — Prove what GitHub serves is clean

**Depends:** T-PR-12 · **Review required:** yes

**Amends:** none.

**Reads:** `$PURGE/verify.py`, `$PURGE/residuals.sh`, `$PURGE/old-shas.txt`.

**Do:** Write `$PURGE/verify-github-clone.sh`, `set -euo pipefail`, which must fail on any of:
1. a fresh `git clone --mirror` from GitHub (fetching `refs/pull/*`) fails `verify.py`'s content
   and removed-path rules, `residuals.sh`, or `gitleaks --log-opts=--all`;
2. any SHA in `old-shas.txt` answers anything but 404 from
   `gh api repos/chenders/AnxietyWatch2/commits/<sha>`;
3. `scan-github-text.py` reports a hit in any kind it scans, review summaries and commit
   comments included (no allowlist this time);
4. any Actions run, artifact or cache exists from before the last run of T-PR-10 (T-PR-16's
   re-run, at the time recorded in `$PURGE/records.md`, once it has happened);
5. any revision in a PR, issue or comment's GitHub edit history (GraphQL `userContentEdits`)
   that is not deleted (`deletedAt` null) matches the purge's patterns -- no exemption: a
   revision still visible after T-PR-12 is still public.

**Verify:**
```sh
bash "$HOME/Source/aw2-history-purge/verify-github-clone.sh"
```

**Context budget:** a fresh mirror clone.

---

### T-PR-14 — Publish with no exposure window

**Depends:** T-PR-13 · **Owner action**

**Amends:** none.

**Reads:** none.

**Do:** Fork-PR approval cannot be set while the repository is private, so the runner must not be
reachable during the flip:
1. take `vitals-pi` offline (stop the runner service), or confirm T-PR-6's move removed it;
2. the owner changes visibility to public;
3. immediately: fork-PR workflow approval → all external contributors; private vulnerability
   reporting, secret scanning and push protection on; wiki off; required reviewers on the
   `production` environment;
4. only then bring the runner back, if T-PR-6 kept it.

**Verify:**
```sh
set -o pipefail; A=repos/chenders/AnxietyWatch2; gh repo view chenders/AnxietyWatch2 --json visibility --jq .visibility | grep -qx PUBLIC && gh api $A --jq '[.security_and_analysis.secret_scanning.status, .security_and_analysis.secret_scanning_push_protection.status, (.has_wiki|tostring)] | join(" ")' | grep -qx 'enabled enabled false' && gh api $A/private-vulnerability-reporting --jq .enabled | grep -qx true && gh api $A/actions/permissions/fork-pr-contributor-approval --jq .approval_policy | grep -qx all_external_contributors && gh api $A/environments/production --jq '[.protection_rules[].type] | index("required_reviewers") != null' | grep -qx true && R="$HOME/Source/aw2-history-purge/records.md" && test -s "$R" && { ! grep -qx 'Runner: moved' "$R" || test "$(gh api $A/actions/runners --jq .total_count)" = 0; }
```

**Context budget:** none.

---

### T-PR-15 — Record the results and gate every commit a PR introduces

**Depends:** T-PR-12, T-PR-16 (built on the final history) · **Review required:** yes

**Amends:** history-phi-purge T-HP-9 (scheduled here, unchanged).

**Reads:** `$PURGE/records.md`, and as T-HP-9.

**Do:** One PR: copy the T-PR-6 decisions, the lines under the T-PR-16 results heading and the
T-PR-12 ticket line from `$PURGE/records.md` into this file (no removed value), tick every box whose Verify was run and exited 0, and do
T-HP-9. Not a publication blocker; it keeps the result from regressing.

**Verify:** as T-HP-9, and
```sh
T=openspec/changes/public-release-readiness/tasks.md; grep -qE '^Signatures: (re-sign|lifted)$' "$T" && grep -qE '^Runner: (moved|approval-gated)$' "$T" && grep -qE '^GitHub support ticket: [0-9]+' "$T"
```

**Context budget:** 3 files.
