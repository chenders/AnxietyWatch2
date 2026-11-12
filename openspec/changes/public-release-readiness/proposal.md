# Proposal: Make the repository safe to publish on GitHub

Making a repository public publishes **every object reachable from any ref**, not the current
tree. As of 2026-11-10 the tree is clean (`scripts/check-public-info.py` passes on `main`), but
history is not, and several things outside git also change meaning when the repository becomes
public. This change is the complete list of what has to be true first, in the order it has to
happen. It reuses `history-phi-purge` for the rewrite itself and adds what that plan does not
cover.

## What blocks publication

1. **Health data and identifiers in history.** Everything `history-phi-purge` enumerates:
   screenshots including Health/HealthKit screens, a screen recording, a snapshot ZIP, real
   readings in fixtures, clinical detail, device identifiers, home paths, the Pi's hostname and
   address. Only that plan's tip clean-up (#105, #107–#114) has landed; its rewrite has not.
2. **Medication names still on the tip.** The purge manifest already replaces two medication
   names in the iOS client's history, but the same names remain in the three admin-console design
   mock-ups under `docs/design/admin-console/claude-design/`. If the iOS values were the owner's
   regimen, these are too. The names are replaced on the tip first, so the rewrite's "main's final
   tree is unchanged" proof still holds; doses stay, because without a name they identify nothing.
3. **Withheld code in history.** `firmware/` is in history from PR #22 and pending the owner's
   legal review. It is already in the purge's path list, so the rewrite removes it; publication
   no longer depends on the review's outcome.
4. **The `main` ruleset requires signed commits.** `required_signatures` is active with no
   bypass actor. `git filter-repo` drops signatures, so a force-push of the rewritten history is
   rejected as it stands. `history-phi-purge` T-HP-5 assumed a `non_fast_forward` rule, which the
   ruleset does not have, and did not account for signatures. Either the rewritten commits are
   re-signed — which requires rewriting the committer of commits that GitHub, Dependabot or
   Copilot committed, because a signature is verified only against the committer — or the owner
   lifts the rule for the one push and the rewritten history stays unsigned. **That is an owner
   decision**, and the only one this plan needs before the push.
5. **Pull-request refs.** GitHub keeps `refs/pull/*/head` for every PR. A force-push cannot
   change them, so publishing after a perfect rewrite would still expose the old commits through
   PR pages. Only GitHub Support can purge them, and that must finish **before** publication.
6. **Self-hosted runner.** `pi-deploy.yml` and `pi-runner-smoke.yml` run on a self-hosted runner
   on the Pi, on the owner's LAN. On a public repository a fork PR runs **its own** workflow files
   on `pull_request` — merged or not — so it can target that runner directly; GitHub advises
   against self-hosted runners on public repositories for this reason. A CI gate cannot stop that
   run. The controls are to move the runner to a private repository, or to require approval for
   every external contributor's run (an owner decision, T-PR-6), and to keep the runner offline
   while visibility changes, because the approval policy can only be set once the repository is
   public. The CI gate (T-PR-3) is defence in depth for workflows on the default branch.
7. **GitHub-side data a rewrite does not touch.** 901 Actions runs, 57 artifacts and 34 caches
   (logs readable by anyone once public; at least one deploy log names the Pi's host), and the
   text of 117 PRs, their review comments — whose `diff_hunk` quotes old file content — and issue
   comments, several of which carry the hostname, home paths or device-identifier-shaped strings.
   GitHub also serves orphaned commits by SHA until it garbage-collects them.
8. **The personal-term check is off on this machine.** The pre-push gate warns
   `no .githooks/pii-denylist.local.txt — personal-term checking is off` on every push.
9. **Publication settings.** Fork-PR workflow approval cannot be set while the repository is
   private; secret scanning, push protection and private vulnerability reporting should be on from
   the first public minute; the wiki (editable by any user once public) goes off; the `production`
   environment gets required reviewers. A `SECURITY.md` says where to report.

## Non-goals

- The open hardening backlog (#103, `prototype-hardening-backlog`). This is a LAN-only
  prototype; publishing the code does not expose the LAN. Those gaps stay tracked there.
- The Apple Team ID in `clients/apple/Config/Shared.xcconfig`. It is embedded in every signed
  build Apple distributes and is not a secret; it stays.
- The console's private LAN bind address in `deploy/vitals-console.service`. An RFC 1918 address
  identifies nothing outside the LAN; moving it out of the tracked unit is already
  `repository-hygiene-followups` T-RH-7. It is not on the personal-term denylist, which would
  otherwise block every legitimate edit to that unit.

## Order

Land or close every open PR → fix the tip → enable the local gates → **owner: signatures and
runner decision** → re-run the purge on the frozen tip and prove it → **owner: push** → move every
clone → delete Actions runs, artifacts and caches → scrub PR and comment text → remove what
the fixture history lets a reader infer, re-run the purge and **owner: re-push** (T-PR-16) →
**owner: GitHub Support purge** (PR refs, cached views, orphaned commits, uneditable diff hunks) → prove what
GitHub serves is clean, including old SHAs answering 404 → **owner: publish**, with the runner
offline until the approval policy is set. Publication is last and is the only step that cannot be
taken back.
