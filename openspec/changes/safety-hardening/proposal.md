# Proposal — production safety hardening and monitoring evidence

## Problem

The current implementation can show a fresh EMAY live value while the PostgreSQL-backed history is
old, and it cannot account for every accepted spool row through database commit and display. The
existing tests are green but the workspace coverage is 84.16%, the production drainer is not in the
checked-out workspace, and the active segment can remain unavailable to the drainer for the one-hour
rotation interval.

This is unacceptable for a safety-relevant monitoring system. A running process, a rewritten status
file, or a green colour is not proof that data was received, durably buffered, committed, or
truthfully displayed.

## Goals

1. Preserve every source-identified EMAY stored record through the existing collector → `vitalsd`
   → COPY spool → PostgreSQL path.
2. Never assign a host-generated identity to the anonymous EMAY `EB` stream.
3. Make every important pipeline state observable: received, spooled, fsynced, drained, committed,
   and displayed.
4. Bound pending spool lag and represent pending data explicitly, without interpolation,
   duplication, smoothing, or fabricated timestamps.
5. Make infrastructure and display health fail closed when durability evidence is missing.
6. Reconcile the deployed drainer with source control and test the exact artifact.
7. Establish a continuously runnable validation framework with a hard 95% line-coverage gate for
   testable core packages, fuzzing for byte decoders, dependency policy checks, and mutation tests.
8. Add a read-only admin monitoring page showing current state, pipeline evidence, and bounded
   historical records/gaps.

## Non-goals

- No second history store.
- No host sequence for anonymous live EMAY frames.
- No interpolation, LOCF, smoothing, or gap hiding.
- No change to H10/BP identity or alarm semantics.
- No remote destructive operation as part of local implementation.

## Safety rule

If a requirement cannot be proven at the current stage, the system must say `UNKNOWN`, `PENDING`,
`UNAVAILABLE`, or `NOT COMMITTED` and name the blocker. It must not promote a process state into a
health assertion.
