# Proposal: Make the review gates verifiable

Two gates that guard this repository are themselves unverified, and both were found by
review rather than by a command.

**The pre-PR reviewer cannot fire.** `AGENTS.md` requires dispatching `c-pre-pr-reviewer`
on the diff before opening a PR. Commit `591c222` "migrate the review infrastructure to
Rust" renamed `c-pre-pr-reviewer.md` → `rust-pre-pr-reviewer.md` and
`c.instructions.md` → `rust.instructions.md` and changed **zero lines of content**. The
agent therefore still instructs its reader to open `.github/instructions/c.instructions.md`,
which no longer exists; its checklist asserts C invariants (`(const uint8_t *, size_t)`
signatures, `#ifdef __APPLE__`, "a C code fence") that the Rust tree cannot produce; and
its trigger names `vitals/src`, `vitals/tests`, `vitals/fuzz` and `vitals/collectors`,
none of which exist — the Rust tree is `crates/` and `fuzz/`. A reviewer whose scope can
never match any diff is not a gate, and the workflow that mandates it silently passes.

This is the exact defect class `AGENTS.md` already names: *"A plan doc that does not
reflect what shipped is a defect"* and *"A conditional that skips when its target is
missing does not report a gap -- it reports success."*

**The newest gates are shell, and nothing lints them.** `.githooks/pre-push`,
`scripts/*.sh` and `scripts/tests/*.sh` now carry the public-information gate, the
per-commit exposure scan, and the worktree-safety contract. No static analysis runs on
any of them in CI. Two fail-open defects shipped in shell guards on 2026-11-06 — one
derived a git worktree's admin directory from a directory basename and reported
"safe to remove" over a paused rebase; another used `for e in $(ls -A "$dir")`, whose
word-splitting silently exempted 15 space-named entries totalling ~3 GB from a size
threshold. Both failed open and reported success. `shellcheck` catches the second class
directly.

Adoption cost is measured, not estimated: of 42 tracked shell files, 37 are already
clean at `-S warning` and 5 carry 9 warnings between them.

## What this change does

Repoints the pre-PR reviewer at the Rust standard, rescopes its trigger to paths that
exist, rewrites its checklist against the Rust invariants in `AGENTS.md`, and adds
`shellcheck` to the `public-info` CI job so tracked shell is analysed on every PR.

## What it does not do

It does not weaken any existing gate, change the pre-push hook's behaviour, relax the
coverage thresholds, or alter the `physio-accuracy-reviewer` trigger. It does not rename
the reviewer again — the name `c-pre-pr-reviewer` is what `AGENTS.md` references, and
renaming it is a separate decision from making it correct.
