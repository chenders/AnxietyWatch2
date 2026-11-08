# Tasks — CPAP ez Share ingestion

The conductor ticks a box only after running that task's Verify command and seeing exit 0.

- [x] T-CP1 — Safe, resumable ez Share HTTP mirror · implement-complex · openai/gpt-5.6-sol
- [x] T-CP2 — Clean-room EDF semantics, registry contract, identity and clock metadata · implement-complex
- [ ] T-CP3 — Transactional append-only CPAP importer + attempt ledger · implement-complex
- [ ] T-CP4 — Dedicated Wi-Fi route and hardened systemd supervision · implement-standard · **physical gate**
- [ ] T-CP5 — Authenticated ez Share Wi-Fi setup/rotation + console evidence · implement-complex
- [ ] T-CP6 — Real-card end-to-end acceptance · implement-complex · **physical gate**

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

**Verify:** `test "$(basename "$PWD")" = aw2-cpap-t-cp1 && python3 -m unittest discover -s batch/cpap_ezshare/tests -v && python3 -m compileall -q batch/cpap_ezshare && git diff --check origin/main...HEAD`

**Expect:** exit 0; hostile-server tests pass; no network or hardware is required.

---

## T-CP2 — EDF semantics, registry, identity, and clock contract

**Depends:** T-CP1 (landed) · **Review required:** yes

**Amends:** 07 (Python batch-parser carve-out; the Rust HTTP/API decision remains binding).

**Reads:** `docs/specs/AMENDMENTS.md` · `docs/specs/2026-08-05-amendment-07-api-in-rust.md` §"What
Python keeps" · `docs/integration/hms-cpap-ezshare-integration.md` ·
`docs/integration/resmed-air11-ble-stream.md` (the channel set and cadence rules already settled for
the BLE path) · `openspec/changes/cpap-ezshare-ingest/{proposal,design}.md` ·
`vitals/registry.toml` · `vitals/tools/gen_registry.py` (the `--check` contract) ·
`batch/cpap_ezshare/{mirror.py,README.md,NOTICE}` (the manifest shape this task consumes, and the
clean-room NOTICE pattern) · the published EDF/EDF+ specification at `edfplus.info`.

**Context budget:** 14 files / 24k tokens.

### Frozen decision: a clean-room stdlib EDF reader, no third-party parser

The brief allowed "the selected permissive EDF library". None is selected; the reader is written
from the published format instead. Four reasons, in order of weight:

1. **The bytes are hostile.** These EDF files arrive from the ez Share card, which T-CP1 treats as
   an adversary throughout. `pyedflib` is a compiled C extension, so handing card bytes to it
   reverses the one property T-CP1 was built to establish — that untrusted input is parsed by code
   that cannot corrupt memory. A pure-Python reader with explicit bounds checks keeps it.
2. **No third-party runtime Python dependency exists in this repository.** `pyproject.toml` declares
   only `pytest` and `ruff`, both dev-only. Adding the first runtime dependency to a Pi that also
   runs the alarm is a decision that needs justifying, not a convenience.
3. **EDF is small and fully documented.** A 256-byte fixed header, `ns × 256` bytes of signal
   headers, then data records of little-endian `int16`. Scaling comes from each signal's own
   physical/digital min and max. This is a few hundred lines, and every field is in a public
   specification, so the clean-room NOTICE pattern T-CP1 established applies unchanged.
4. **`cpap-py` is unverified** on licence and maintenance, and the proposal requires attribution for
   every runtime library. OSCAR remains formats-only; no GPL implementation is read or copied.

If EDF+ annotation parsing (`EVE`/`CSL` TAL records) turns out to need materially more than the
signal reader, that is a finding to report rather than a reason to reach for a dependency silently.

**Do:** Create `batch/cpap_edf/`, stdlib only. Record synthetic, de-identified golden EDF fixtures
generated by a committed script — never a real patient recording. Map the supported ResMed channels
to the canonical registry types that already exist for `resmed_air11` (`patient_flow`,
`mask_pressure`, `inspiratory_pressure`, `expiratory_pressure`, `leak`, `tidal_volume`,
`minute_ventilation`, `respiratory_rate`, `ie_ratio`, `snore_index`, `flow_limitation`), and add one
new registry **source** for the SD/EDF transport — the types are already declared, so this task adds
a source rather than inventing physiology. Regenerate `vitals/generated/` with
`vitals/tools/gen_registry.py`; never hand-edit it.

Specify deterministic session and per-channel sequence identity derived only from immutable file and
session evidence — arrival time is provenance, never identity — and a versioned raw-time transform
stored as metadata, applied at read time. Reject sentinels and out-of-range values against the
registry range; a rejected value contributes no sample and names its reason. No database writes, no
SQL, no systemd, no network code. Every parser path gets malformed, truncated and property tests.
Never assert float equality; compare with a tolerance.

**Verify:** `test "$(basename "$PWD")" = aw2-cpap-t-cp2 && python3 -m unittest discover -s batch/cpap_edf/tests && test "$(python3 -c 'import unittest;print(unittest.defaultTestLoader.discover("batch/cpap_edf/tests").countTestCases())')" -ge 30 && python3 -m compileall -q batch/cpap_edf && python3 vitals/tools/gen_registry.py --check && rg -q '^\[source\.resmed_air11_sd\]' vitals/registry.toml && ! rg -qE 'psycopg|sqlalchemy|INSERT INTO|DELETE FROM' batch/cpap_edf && git diff --check origin/main...HEAD`

**Note on this Verify line.** It deliberately does **not** pipe the suite into `tee` or `grep` to
count tests. `python3 -m unittest … | grep -qE '^Ran …'` returns *grep's* status, so a suite that
fails every test still exits 0 — the same fail-open shape as the `for e in $(ls -A "$dir")` guard
named in `review-gate-integrity`. The exit code and the count are therefore asserted separately: the
suite runs once for its status, and `countTestCases()` is asked for the number. The count exists
because `unittest discover` exits 0 on a *partial* discovery — a file renamed out of the `test*.py`
pattern takes its cases with it silently, measured at 1 of 3 ran, exit 0 — while an empty discovery
(exit 5) and a broken import (exit 1) already fail on their own.

`gen_registry.py --check` is in the line because `vitals/generated/` is never hand-edited and CI
regenerates and diffs it; a registry change that forgets to regenerate would otherwise land green
here and fail there.

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

## T-CP5 — Authenticated Wi-Fi setup, credential rotation, and evidence

Add an ez Share setup/configuration screen to all three admin-console renderings through one shared view
model. It accepts SSID and password, supports explicit password replacement, and provides a bounded
connection test. It must use the console's protected write/authentication and CSRF contract; if that
contract has not shipped, this task is blocked rather than exposing an unauthenticated secret endpoint.

The console backend passes the secret only to a narrow root-owned helper over a private local channel.
The helper validates SSID/password bounds, writes a root-only NetworkManager profile/key store atomically,
and invokes NetworkManager without placing the password in argv or process environment. Neither current
nor prior passwords may enter PostgreSQL, attempts/detail, logs, browser storage, HTML, API responses,
source control, or a general service `EnvironmentFile`. Read APIs expose only configured/not-configured,
SSID, last test time/outcome, and a blocker naming interface/profile/status values. Audit rotation and
connection-test attempts without secret material. Add redaction tests using distinctive canary secrets
across logs, API payloads, rendered assets, process invocation, and attempt rows.

Also expose ez Share/parser last attempt, last success, row effects, unique ownership, and versioned raw
clock-transform evidence in the shared console model. Unknown/card-offline must not become success merely
because old CPAP samples exist.

**Verify:** must be replaced with an exact command after the console authentication/write contract and
T-CP4 helper interface are frozen; task is not dispatchable before then.

---

## T-CP6 — Real-card end-to-end acceptance

**Gate:** the operator confirms the default card password has been rotated and supplies the resulting
credential only through the admin setup flow—never through chat, a task prompt, source control, or shell
history. Stop and ask before changing Pi networking.

Run one real card import twice: first run appends expected validated rows; second run appends zero; raw
mirror hashes stay unchanged; card-offline state names its blocker and does not fabricate physiological
data. Prove the credential canary is absent from repo, service environment, journal, database attempt
details, API responses, and browser storage.

**Verify:** must be replaced with an exact command after T-CP5; task is not dispatchable before then.
