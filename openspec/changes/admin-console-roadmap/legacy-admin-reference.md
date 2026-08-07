# Generation-1 admin behaviour reference

This is a behaviour inventory of the generation-1 Flask admin under `server/admin.py` and
`server/templates/`. It is a reference for later task dispatch, not a porting mandate. The verdicts
in `docs/design/GEN1-REASSESSMENT.md` govern: `ADOPT`, `REPLACE`, `EXTRACT`, and `DROP` still mean
what that document says, and Amendment 07 requires the generation-2 HTTP/backend implementation to
remain Rust. Where this inventory and an accepted amendment disagree, the amendment wins.

## Behaviour worth preserving, in the generation-2 shape

| Gen-1 surface | Useful operator behaviour | Generation-2 expectation / owner |
|---|---|---|
| Dashboard | At-a-glance record counts, database size, active credential count, and latest sync metadata | Overview keeps concise build/schema/storage and promoted-exception evidence. It must not use success-only “last sync” as health. M1/M2 own the replacement. |
| API keys | Create a labelled key, reveal plaintext once, list prefix/created/last-used/request count/status, revoke an active key | Preserve the lifecycle and one-time reveal. Add expiry/rotation, immutable audit, CSRF, proportionate confirmation, and never return stored plaintext. T-M2h. |
| ResMed settings | Show credential presence without echoing values, configure schedule, run an immediate sync, and show last result | Preserve write-only credential semantics in T-M2h and the operator-triggered myAir import in T-M3e’s import-specific supervised lifecycle. Use encrypted storage, validated configuration, attempt-before-work, sanitised errors, and durable history. T-M4a may reuse the lifecycle contract later; T-M3e does not depend on M4. Recurring pollers remain collector-owned. |
| CPAP EDF upload | Multi-file upload, file-type validation, parse sessions, report per-file accepted outcomes, and clean temporary files | Preserve parser capability, not the request-thread execution. Cap before disk, isolate the Python parser, model replay/reject/zero-accepted outcomes, support cancel, retain upload history, and remove uploaded PHI/scratch files after every terminal outcome. T-M3e. |
| Data browser | Quickly inspect operational/domain data without a shell | Preserve the high-value inspection capability as bounded targeted views, shared date windows, keyset paging, provenance/completeness, and scoped export. Do not inherit arbitrary-table browsing, `OFFSET`, hypertable `COUNT(*)`, or a prompt-enforced read-only connection. T-M3c/T-M3d/T-M3i. |
| Patient profile | Edit demographics, other medications, raw/structured medical history, see tracked medications, generate/refine an editable prompt summary | Preserve profile and prompt-input curation. AI output is a candidate beside the prior version until explicit acceptance; all billed calls use the common ledger. T-M4c/T-M4d/T-M5b/T-M5c. |
| Psychiatrist profile | Store name/location and an editable prompt summary | Preserve operator-authored profile context and version history. Do **not** preserve web research for adverse information about an identified psychiatrist; `GEN1-REASSESSMENT.md` explicitly drops it. T-M5b/T-M5c. |
| Conflicts | List active before resolved; capture objective description plus both sides’ perspectives, assumptions, desired resolutions, requests, and context; resolve/reopen deliberately | Preserve the two-sided prompt context, explicit status transition, history, and active selection. Do not silently overwrite prior text. T-M5b/T-M5c. |
| Analysis launch/history | Choose date range and allowed model, quick ranges, mark incomplete medication tracking, request detailed output, optionally include active conflict, and inspect prior runs | Preserve explicit parameters and history, but preflight egress/model/data/cost, enforce budget, and run through the supervised job lifecycle. T-M4a/T-M4c/T-M4f. |
| Analysis result | Distinct pending/running/completed/failed states; elapsed refresh; summary/trend; severity-grouped insights; confidence explanation/supporting data; request/response visibility; token/model/cost readout; print/PDF; conflict evidence and synthesis | Preserve the information architecture and inspectability. Add cancel/retry/recovery, durable parse failures, sanitised bounded payload views, unified accounting, and Pi-generated clinical PDF. T-M4a–T-M4f. |
| Therapy schedule | Add weekly/monthly sessions with day/time/type/commute/notes and remove from the active list | Preserve schedule context, validation, and active/inactive semantics as versioned domain records with history. T-M5b/T-M5c. |
| Songs | List title/artist/album, occurrence count, last occurrence, and lyrics presence; edit metadata/lyrics; inspect occurrences and associated anxiety; optionally fetch lyrics | Preserve songs, tags/occurrences, and manual lyrics correction as versioned records in T-M5b/T-M5c. Drop the unowned Genius scraper; a future external lyrics fetch requires its own reviewed egress contract/task. |
| App settings | Configure the timezone that controls analysis day bucketing | Preserve the setting, but validate against a real IANA zone and show effective value; never accept a typo and silently fall back. T-M2h. |

## Analysis and job semantics to use as fixtures

- Generation 1 assembles anxiety entries, medication doses, CPAP sessions, health data, barometric
  readings, correlations, and song context over an explicit local-day window. Generation 2 must use
  canonical sample/domain sources and preserve absence/provenance rather than porting the old tables.
- Correlations use a minimum of 12 paired days, Pearson correlation with a p-value, constant-array
  rejection, and an abnormal-versus-normal anxiety comparison. These are fixture inputs for T-M4b,
  not permission to copy the old daily-snapshot plumbing.
- The analysis UI exposes incomplete-dose and detailed-output choices. If these remain meaningful in
  the new data model, they stay explicit run parameters; if superseded, the implementation task must
  record the reason and update the interface documentation.
- The conflict-analysis DAG shape is useful: health analysis; four evidence jobs supporting and
  challenging each side; then synthesis after all four. The in-process `ThreadPoolExecutor`, page
  load stale sweeper, and lack of cancellation/retry are explicitly replaced by T-M4a.
- Raw request/response, model, token usage, estimated/actual cost, parse status, and source range are
  operator evidence. They require sanitisation, size bounds, and the unified billed-call ledger.

## Generation-1 behaviours that are specifically not precedents

- A global shared-password lockout, unbounded session, missing CSRF, and cookie-only transport
  assumptions are defects, not compatibility requirements.
- A success-only sync row cannot establish ingest health. Attempts are recorded before work and
  silence/never-observed remain visible.
- Uploading a whole body before checking its size, running parsers in the HTTP process, transient
  flash-only outcomes, and rendering raw exception/subprocess text are prohibited.
- Generic arbitrary SQL/table browsing, request-thread analysis, hardcoded hidden models, and
  unmetered AI calls are prohibited.
- Unversioned profile/conflict/song edits and soft-delete actions without audit are prohibited.
- Psychiatrist adverse-information web research is intentionally dropped.
- Five-second whole-page refresh is not the generation-2 job-monitoring contract.

## Dispatch rule

Tasks that cite this file should read only the relevant row(s) and the named generation-1 source
files explicitly listed in that task’s `Reads:` field. The old implementation supplies fixtures and
operator expectations; it does not override current amendments, crate boundaries, security rules,
or acceptance commands.
