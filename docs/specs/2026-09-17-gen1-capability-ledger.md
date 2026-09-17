# Gen-1 capability ledger

**Status:** Review evidence recorded 2026-09-17. Generation 1 is not decommissioned.

This ledger assigns every capability in the admin-console migration-risk register an owner,
disposition, and evidence. A planned or deferred row remains open until its owner completes the
named task and its Verify command exits zero.

## Migration-risk capabilities

| Capability | Disposition | Owner | Evidence / remaining gate |
|---|---|---|---|
| Redundant push alert channel | DEFERRED | T-M6c | Remote push evidence and independent test operation; blocked on the alarm daemon/core contracts. |
| Clinical PDF report generation | DEFERRED | T-M4e | Pi-generated report preserving nil-AHI exclusion, unfiltered baselines, and completeness; task Verify remains outstanding. |
| JSON/CSV export | DEFERRED | T-M3d | Scoped export with a real read-only role and completeness metadata; blocked on T-M3a/T-M3c. |
| Correlation recompute | DEFERRED | T-M4b | Port minimum paired-day, p-value, abnormal-vs-normal, nil-AHI, and baseline rules; task Verify remains outstanding. |
| Timezone setting | REPLACED | T-M2h | Validated configuration lifecycle and operator-visible setting; `6de4902` and T-M2h verification. |
| Schema revision + pending migrations | REPLACED | T-M1c / deployment | Version surface exposes schema revision and pending count; M1 regression suite in `4db01cf`. |
| Deployed build identity | REPLACED | T-M1c / deployment | Version surface exposes build commit and build time; M1 regression suite in `4db01cf`. |
| Log sanitisation | REPLACED | T-M2c | Central sanitised service/log evidence; `8a8acf8` and T-M2c verification. |
| Phone has gone dark | REPLACED | T-X1 and T-M2e/T-M2f | Pi-owned email policy in Amendment 14; `121dee9`; detector and email owners remain responsible for implementation verification. |
| Phone-side health-access state | REPLACED | T-X2 and T-M1/M2 ingest | Authenticated status beacon, separate storage, and non-healthy mapping for missing/denied/no-sample/failure states; `6588f10`, T-X2 verification. |
| Device↔server reconciliation | DEFERRED | T-M5e / Stage 7 | Projection rebuild replaces the empty-store guard and per-table importers; task remains outstanding until the cursor/order contract lands. |

## Undocumented correctness rules

| Rule | Disposition | Owner | Evidence / remaining gate |
|---|---|---|---|
| Replay window uses server receipt time, not device timestamp | REPLACED | T-X4 / ingest contracts | HealthKit receiver and replay fixtures preserve source/client receipt evidence; `444bfdf`, T-X4 verification. |
| Non-finite values are rejected explicitly | REPLACED | T-X4 / decoder boundary | Pure mapper rejects non-finite input rather than allowing a floor comparison to pass; `444bfdf`, T-X4 verification. |
| Nil-AHI nights are excluded, never averaged as zero | DEFERRED | T-M4b | Correlation and report ports must carry the rule into fixed fixtures; task Verify remains outstanding. |
| Baseline windows are unfiltered and never mislabelled | DEFERRED | T-M4b | Correlation port must preserve the unfiltered-baseline rule; task Verify remains outstanding. |

## Security and operational corrections

| Capability / correction | Disposition | Owner | Evidence / remaining gate |
|---|---|---|---|
| Third-party token revocation | DROPPED WITH REASON | Future external-integration task owner | Gen-2 carries no legacy third-party OAuth integration; no token is imported or presented as revocable. Any future OAuth integration must define revoke/expiry before acceptance. |
| Mobile credential token/key revocation | REPLACED | T-M2h | Credential lifecycle has expiry, revocation, audit, and recent-auth gates; `6de4902`, T-M2h verification. |
| Mobile key rotation | REPLACED | T-M2h | Device-bound credential lifecycle owns overlap/rotation and audit evidence; `6de4902`, T-M2h verification. |
| Database read-only enforcement | DEFERRED | T-M3d | A real least-privilege Postgres role with statement timeout replaces prompt-enforced SQL read-only access; task remains outstanding. |

## Decommission gate

Generation 1 remains in service. Decommissioning is permitted only after every `DEFERRED` row has
completed its owner task and Verify command, or an accepted amendment changes the disposition. The
`DROPPED WITH REASON` row is intentionally not a hidden omission: the corresponding legacy OAuth
integration is not part of the generation-2 system.
