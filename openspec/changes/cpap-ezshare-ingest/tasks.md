# Tasks — CPAP ez Share ingestion

The conductor ticks a box only after running that task's Verify command and seeing exit 0.

- [ ] T-CP1 — Safe, resumable ez Share HTTP mirror · implement-complex · openai/gpt-5.6-sol
- [ ] T-CP2 — Clean-room EDF semantics, registry contract, identity and clock metadata · implement-complex
- [ ] T-CP3 — Transactional append-only CPAP importer + attempt ledger · implement-complex
- [ ] T-CP4 — Dedicated Wi-Fi route and hardened systemd supervision · implement-standard · **physical gate**
- [ ] T-CP5 — Console evidence and real-card end-to-end acceptance · implement-complex · **physical gate**

---

## T-CP1 — Safe, resumable ez Share HTTP mirror

**Context budget:** 12k tokens.

**Reads:** `docs/integration/hms-cpap-ezshare-integration.md` ·
`docs/integration/resmed_bluetooth_research.md` ·
`docs/specs/2026-08-05-amendment-07-api-in-rust.md` ·
`openspec/changes/cpap-ezshare-ingest/{proposal,design,tasks}.md`

**Amends:** 07 (Python batch-parser carve-out; Rust HTTP/API decision remains binding).

**Why:** Parsing and ingestion cannot be made replayable until card bytes are mirrored safely. The ez
Share interface is unauthenticated plain HTTP serving attacker-controlled HTML and filenames, so a
naive recursive downloader is a path-traversal, disk-exhaustion, and hung-service risk.

**Do:** Create a self-contained Python batch package under `batch/cpap_ezshare/` using the standard
library unless a dependency is justified. Implement only discovery and mirroring—no EDF parsing, SQL,
registry edits, deployment, or network configuration in this task.

Required behavior:

1. Configuration accepts one fixed `http://` origin and mirror directory. Reject userinfo, query,
   fragment, non-HTTP schemes, and origins outside the configured host/port. Never follow a redirect to
   another origin.
2. Query the documented `/dir?dir=...` interface and parse listings without executing HTML. Normalize
   card paths as POSIX relative paths; reject absolute paths, `..`, NUL/control bytes, backslashes, and
   names that escape the mirror root. URL-encode request parameters rather than concatenating them.
3. Enforce configurable finite connect/read timeout, maximum listing bytes, maximum file bytes, maximum
   files per run, and maximum traversal depth. Defaults must be conservative and named in `--help`.
4. Download to a same-directory temporary file with exclusive creation, stream while computing SHA-256,
   fsync the file, atomically rename, and fsync the parent directory. Clean partial files on handled
   failure. Never overwrite bytes at an existing content path.
5. Maintain an atomic versioned JSON manifest containing card path, local immutable content path, byte
   length, SHA-256, and first-seen UTC epoch seconds. A replay of unchanged name+size must make no HTTP
   file download; changed bytes or size create new immutable content rather than replacing old evidence.
   Do not trust name+size as the ultimate identity once bytes are fetched.
6. Return structured run effects (`discovered`, `downloaded`, `replayed`, `rejected`, `bytes`) and a
   nonzero process exit for listing/download/validation failure. Error text names the rejected value and
   limit/reason, but never credentials.
7. Unit/integration tests run an in-process hostile fixture server and cover valid nested discovery,
   spaces/URL encoding, replay without redownload, changed content, traversal names, cross-origin
   redirect, oversized listing/file, timeout, depth/file-count limits, truncated response cleanup, and
   manifest recovery after interrupted replacement. Fixed timestamps only.
8. Add a README documenting the trust boundary, CLI, manifest format, and that this task neither parses
   EDF nor writes physiological data. Add a clean-room `NOTICE` naming protocol/documentation sources;
   do not copy hms-cpap or OSCAR implementation code.

**Verify:** `cd /Users/chris/Source/aw2-cpap-t-cp1 && python3 -m unittest discover -s batch/cpap_ezshare/tests -v && python3 -m compileall -q batch/cpap_ezshare && git diff --check origin/main...HEAD`

**Expect:** exit 0; hostile-server tests pass; no network or hardware is required.

---

## T-CP2 — EDF semantics, registry, identity, and clock contract

**Reads:** to be frozen after T-CP1; must include the integration/design documents, registry, generated
registry workflow, sample schema, and only the selected permissive EDF library documentation.

**Do:** Record synthetic/de-identified golden EDF fixtures; map supported ResMed channels to canonical
registry types and ranges; add generated source/type definitions; specify deterministic session/sequence
identity and versioned raw-time transform; reject sentinels and out-of-range data. No database writes.
Every parser path gets malformed/truncated/property tests. Never assert float equality.

**Verify:** must be replaced with an exact command before dispatch; task is not dispatchable until then.

---

## T-CP3 — Transactional importer and attempts

**Reads/Do/Verify:** freeze after T-CP2. Must create attempt before work, insert through a private staging
transaction, preserve append-only identity/provenance, report created/replayed/rejected effects, and prove
byte-for-byte replay creates zero new samples. No `UPDATE` or `DELETE` against `samples`.

---

## T-CP4 — Network and service deployment

**Gate:** stop and ask the operator to verify the card API, interface names, and route. Never reconfigure
networking speculatively. Keep ez Share plain HTTP isolated to its dedicated interface/subnet; no LAN
forwarding. Harden the service with bounded runtime/resources and explicit restart behavior.

**Verify:** must include observable real-Pi route, origin confinement, systemd sandbox, timeout, restart,
and card-offline behavior; freeze only after the physical gate is recorded.

---

## T-CP5 — Evidence and end-to-end acceptance

Expose ez Share and parser attempts, last success, row effects, unique ownership, raw clock transform and
version in the existing shared console model. Run one real card import twice: first run appends expected
validated rows; second run appends zero; raw mirror hashes stay unchanged; card-offline state names its
blocker and does not fabricate physiological data.

**Verify:** must be replaced with an exact command after T-CP4; task is not dispatchable before then.
