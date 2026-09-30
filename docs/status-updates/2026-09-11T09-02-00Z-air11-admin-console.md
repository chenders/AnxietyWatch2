# Status update — 2026-09-11T09:02:00Z

## Goal

Add a thorough, safe admin-console flow for opting into and operating the read-only AirSense 11 integration, while documenting and preserving its credential boundary.

## Since the last update

- Added the bounded Pi acceptance harness and the AirSense integration runbook.
- Documented the admin-page status contract, including separate link and measurement states, unavailable/null handling, durability-unknown handling, and safe diagnostics.
- Confirmed that the existing console currently has general read-only dashboard routes and three visual directions, but no AirSense-specific page, configuration model, or lifecycle controls.
- Began reviewing the console's Rust/Axum routes, static assets, service-state seam, and admin-console requirements.

## What was learned and changed because of it

- The existing console has no safe AirSense control boundary yet; adding buttons alone would not be sufficient.
- The AirSense master key must remain outside the database and outside browser/API payloads. The database is downstream and may be unavailable when the collector authenticates; storing the key there would also widen access and backup exposure. The planned design is to keep the operator-provisioned file plus systemd credential loading, and store only non-secret opt-in/configuration metadata and audit facts in the database if a schema is added.
- The admin page will therefore report credential presence/validity and provide operator instructions, but will never display, generate, pair with, rotate, or retrieve the master key.

## Completed since the last update

- Acceptance harness implementation and documentation are complete locally.
- Admin-console architecture review is in progress; no AirSense admin page code has been completed yet.

## Working on now

Designing and implementing the AirSense admin page and its backend boundary: safe status projection, opt-in state, explicit lifecycle actions, authorization/confirmation, auditability, and credential-file validation without secret handling in the browser or database.

## Next interval

Implement the backend routes and tests first, then add the page to each existing visual direction, wire the status/counters and safe configuration flow, update the integration documentation, and run the Rust/JavaScript/static verification gates. Hardware activation will remain a separate explicitly authorized gate.

## Estimate

- Current completion for the admin-page/configuration goal: **20%**.
- Estimated remaining local implementation and verification: **approximately 2–4 hours**, subject to the existing console authorization and database-schema boundaries.
- Pi/live acceptance ETA: **not estimable until explicit authorization and Pi access are available**.
