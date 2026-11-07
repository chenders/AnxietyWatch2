# Tasks: Make the review gates verifiable

- [ ] T-RGI-1 — Repoint, rescope and rewrite the pre-PR reviewer against the Rust tree · implement-simple · host
- [ ] T-RGI-2 — Gate tracked shell on shellcheck in CI · implement-simple · host

## T-RGI-1 — Repoint, rescope and rewrite the pre-PR reviewer against the Rust tree

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `docs/specs/AMENDMENTS.md`, `proposal.md`,
`specs/review-gate-integrity/spec.md`, `.claude/agents/rust-pre-pr-reviewer.md`,
`.github/instructions/rust.instructions.md`, `AGENTS.md` (the Workflow and Rust
sections), and `git show --stat 591c222`.

**Modifies:** `.claude/agents/rust-pre-pr-reviewer.md`.

**Do:** Replace the reviewer's C checklist with the Rust invariants this repository
states in `AGENTS.md`: `get()` rather than indexing because `indexing_slicing` is denied
and the input is attacker-controlled; `Option`/`Result` rather than a bool plus an
out-parameter; no `unwrap()`/`expect()`/`panic!` outside tests, since `panic = "abort"`
in release makes a panic a process death and a dead collector a silent alarm; the
`no_std` and `forbid(unsafe_code)` boundaries of `vitals-registry` and `vitals-decode`;
no BlueZ, SQL or systemd dependency crossing into those crates; a `cargo-fuzz` target in
`fuzz/fuzz_targets/` for every new byte-parsing function. Keep the existing
sentinel-handling, unknown-vs-OK, clamping and append-only-store sections — those are
language-independent and still correct. Repoint the reading instruction from
`.github/instructions/c.instructions.md` to `.github/instructions/rust.instructions.md`.
Rescope the `description:` trigger from `vitals/src`, `vitals/tests`, `vitals/fuzz`,
`vitals/collectors` to the paths that exist: `crates/`, `fuzz/`, and
`vitals/registry.toml`. Keep `name: c-pre-pr-reviewer`, because that is the identifier
`AGENTS.md` dispatches and renaming it is a separate decision. Do not weaken the
`physio-accuracy-reviewer` trigger.

**Verify:** `test -f .github/instructions/rust.instructions.md && ! grep -rqF 'c.instructions.md' .claude/agents/ && ! grep -rqE 'vitals/(src|tests|fuzz|collectors)' .claude/agents/ && grep -qF 'crates/' .claude/agents/rust-pre-pr-reviewer.md && grep -qF 'name: c-pre-pr-reviewer' .claude/agents/rust-pre-pr-reviewer.md && for d in $(grep -oE '(crates|fuzz)/[a-z0-9_-]*' .claude/agents/rust-pre-pr-reviewer.md | sort -u); do test -e "$d" || exit 1; done && openspec validate review-gate-integrity --type change --strict --no-interactive`

## T-RGI-2 — Gate tracked shell on shellcheck in CI

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `proposal.md`, `specs/review-gate-integrity/spec.md`,
`.github/workflows/ci.yml` (the `public-info` job), and the current `shellcheck -S
warning` output over `git ls-files '*.sh' '*.bash' .githooks/pre-push`.

**Modifies:** `.github/workflows/ci.yml`, plus the five tracked shell files that
currently carry warnings.

**Do:** Fix the 9 outstanding `shellcheck` warnings across the 5 files that have them —
fix the defects, do not add blanket `# shellcheck disable=` lines; a disable is
acceptable only with a comment stating why the pattern is correct here, in the style of
the existing `SC2086` disable. Add a step to the `public-info` job in
`.github/workflows/ci.yml` that installs `shellcheck` and runs it at `-S warning` over a
file list derived from `git ls-files`, so a newly added script is covered without
editing the workflow. Do not lower the severity to silence a finding, and do not
enumerate files literally.

**Verify:** `grep -qE '(^|\|)[[:space:]]*(xargs[[:space:]]+)?shellcheck[[:space:]]+[^|]*-S[[:space:]]+warning' .github/workflows/ci.yml && grep -qF 'git ls-files' .github/workflows/ci.yml && shellcheck -S warning $(git ls-files '*.sh' '*.bash' .githooks/pre-push) && openspec validate review-gate-integrity --type change --strict --no-interactive`

**Note on this Verify line.** The first grep requires an actual *invocation*, not the
word. The obvious form, `grep -qF 'shellcheck' .github/workflows/ci.yml`, already passes
today — satisfied by the `# shellcheck disable=SC2086` comment at `ci.yml:432`. A gate
whose acceptance test is satisfiable by a comment is not a gate, and that is the same
defect class as the two fail-open guards named in `proposal.md`: testing for the presence
of a string rather than the presence of the behaviour. Do not loosen it back.

It accepts either invocation style — `shellcheck -S warning $(git ls-files …)` at the
start of a line, or `git ls-files … | xargs shellcheck -S warning` after a pipe — because
the contract is "shell is analysed at this severity over a `git ls-files` list", not a
preferred shell idiom. An earlier draft of this line was anchored to `^` and silently
admitted only the first form.
