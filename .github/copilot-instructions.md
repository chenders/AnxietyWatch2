# Copilot review instructions — AnxietyWatch2

Cross-cutting rules. Language-specific rules live in `.github/instructions/*.instructions.md` and
activate only for matching paths.

## What this system is

A Raspberry Pi 5 that collects physiological data over Bluetooth all night, detects CNS-depression
events, and sounds an alarm. Single user. The alarm is the point — a bug that silences it, or that
makes a missing signal look like a healthy one, is the most serious class of defect in this repo.

Review accordingly. Prefer flagging a real correctness hazard over a style nit.

## The invariants — flag any violation

**Unknown is not OK.** Three states exist everywhere: measured-and-good, measured-and-bad, and
**not measured**. Code that collapses the third into the first is a defect, even if it reads
naturally. A missing signal holds state; it never reassures and never escalates on its own.

**Invalid data contributes nothing.** Never coerce a sentinel, an error, or an absent reading to a
number. IEEE-11073 SFLOAT `0x07FF`/`0x0800` mean *absent*, not zero. An artifact sample is not a
measurement.

**Never clamp an out-of-range value.** Reject it, name the reason, quarantine it. A clamped reading
is indistinguishable from a real one downstream, which is worse than a gap.

**`samples` is append-only.** No `UPDATE`, no `DELETE`. Removal is a tombstone row. Flag any SQL
that mutates history in place.

**UTC epoch seconds, always.** The store holds no local time at any layer below presentation. Flag
any timezone-aware type or local-time conversion below the UI.

**One canonical unit per type, converted once, at the decoder.** Flag conversions at later layers,
and flag any hardcoded unit or range that should come from `vitals/registry.toml`.

**Cursors are derived from data, never from a clock.** A replication or sync cursor set to "now"
after I/O opens a hole: records created during the round trip are past the old cursor and before the
new one, so they are never sent. Flag any `= now()` cursor advance. The cursor advance and the write
that carried the rows must commit together.

**Guards must name their blocker.** A check that blocks a user-visible action states which condition
failed and with what value. `"restore blocked"` is a defect; `"restore blocked: HealthSnapshot=91"`
is correct. Failing closed is right; failing closed silently is not.

**Conditional-skip optimizations must preserve invariants the unconditional version held.** When a
change adds a branch that skips work, audit every downstream state mutation that assumed the work
happened — cursor advances, flag flips, counters, idempotency keys. Each must become conditional
too, or its invariant silently breaks.

## Verification

**A Verify command is a contract.** Tasks in `openspec/changes/*/tasks.md` carry literal `Verify:`
lines. If a PR modifies one to match its implementation rather than satisfying it as written, that
is a defect — flag it explicitly. This has already happened once in this repo.

**Every new or changed behavior needs a test.** No exceptions for "obvious" code.

**Warnings are failures.** The build runs `-Wall -Wextra -Werror -Wshadow -Wconversion`. A PR that
adds a suppression needs a comment justifying it.

## Sensitive data

This repo is private but assume it may not always be. Never approve:

- Real credentials, API keys, or tokens in any file, including tests and fixtures.
- Real names, addresses, phone numbers outside `555-0100`–`555-0199`, or medical provider details.
- Personalized device names. Use `Test iPhone`, not a real device nickname.
- Logging of credential *values*. Presence and length only (`key_present=true, len=84`).
- Committed capture artifacts: HAR files, network dumps, browser automation output.

Medication names and public drug names are fine — they are not personal data.

## Provenance

Agent-authored commits must carry a `Co-Authored-By:` trailer. The repo's git config signs every
commit as the owner, so without the trailer agent work is indistinguishable from human work in
`git log`. Flag agent-shaped commits missing it.

## Scope

Flag PRs that do more than their stated task. One task per PR. A drive-by refactor in an unrelated
file is a review finding, not a bonus — it makes the diff harder to reason about and couples
unrelated failures.

## What not to flag

- Formatting handled by `.clang-format` or `.editorconfig`.
- Generated files under `vitals/generated/` — review `registry.toml` and `gen_registry.py` instead.
  Those files are produced by codegen and CI verifies they are current.
- Missing coverage on BlueZ/D-Bus/systemd glue. It needs real radios and is measured, not gated.
