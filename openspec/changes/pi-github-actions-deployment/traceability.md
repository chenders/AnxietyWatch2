# Traceability — Pi GitHub Actions delivery

| Priority | Work | Acceptance evidence | Destructive authority |
|---|---|---|---|
| 1 | T-R1 native ARM64 runner | Manual smoke workflow, pinned toolchain, unprivileged account, no Docker socket | None |
| 2 | T-R2 trusted workflow boundary | Protected/manual workflow, `contents: read`, serialized concurrency, no PR route | None |
| 3 | T-R3 SHA artifact | Reproducible manifest, checksums, explicit unit-to-binary inventory | None |
| 4 | T-R4 first dry run | Disposable staging root, systemd validation, before/after zero-mutation proof | None |
| 5 | T-R5 repeated dry-run/fault rehearsal | Injected failures fail closed or restore prior release | None; test data only |
| 6 | T-R6 backup gate | Encrypted off-Pi backup plus non-vacuous restore evidence and spool boundary | None |
| 7 | T-R7 minor update | Operator-approved main deployment, health/socket/spool proof, retained rollback | Narrow allowlisted service activation |
| 8 | T-R8 observation and retirement | Repeated delivery evidence and manual rollback before sync retirement | Removal of old sync path only with approval |

## Explicit blockers

- The runner cannot be considered installed until a real ARM64 Pi run proves the labels, account,
  systemd service, and smoke workflow.
- The full service fleet cannot be considered deployable until every `ExecStart` maps to a built
  workspace artifact. H10 and BP are auto-discovered binaries in `vitals-collectors`; the checked-in
  `silence-detector.service` still has no corresponding workspace package. The inventory task must
  resolve that remaining mapping from the actual build graph.
- A backup file without successful restore evidence does not open T-R7.
- A healthy process without fresh data-path evidence does not open T-R7.
- Passing dry runs do not authorize a real deployment; T-R7 remains operator-gated.
