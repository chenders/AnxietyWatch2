# Tasks — Polar H10 physical acceptance

This change is intentionally deferred until the H10 sensor is physically available.

- [ ] H10-PHYS-1 — Verify approved identity, controller evidence, owner isolation, and Wellue-disabled state.
- [ ] H10-PHYS-2 — Run a bounded single-source H10 canary with `btmon` and journald evidence.
- [ ] H10-PHYS-3 — Verify fresh nonzero HR/RR notification, source/session identity, and durable ingest.
- [ ] H10-PHYS-4 — Run the separately authorized restart/reconnect and coexistence phases.

No task may be marked complete from a process start, stale `Connected` property, emulator, or
historical sample. Each task requires a written `Verify:` command and an evidence bundle before it
is started.
