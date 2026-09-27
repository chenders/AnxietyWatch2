# Proposal — 99-last-specs consolidation

## Status

Active consolidation change. This folder is the final OpenSpec destination for tasks that are
recovered from older or newly added checkouts and are ready to continue, but do not yet belong to
their final implementation change.

The `99-` prefix deliberately sorts this change after the named product changes. It is a holding
change, not permission to combine unrelated implementation work without preserving each task's
provenance and acceptance contract.

## Initial migration

`T-B1 — Eight-hour Polar H10 capture` moves here from
`openspec/changes/pi5-vitals-core/tasks.md`. Its task text and Verify command are retained; the
vitals-core change keeps only a redirect and its historical implementation notes.

## Goals

1. Keep one authoritative task entry while a task is being moved between checkouts.
2. Preserve the original task id, owner, host, dependencies, amendment references, Verify command,
   and evidence requirements.
3. Provide a place for subsequent recovered tasks to be added with an explicit source and status.
4. Prevent a task from being silently counted twice in two OpenSpec changes.

## Non-goals

- No implementation, deployment, pairing, or hardware operation.
- No change to a task's acceptance bar merely because it moved.
- No blanket merge of unrelated files or artifacts.
- No claim that a task is complete until its own Verify command exits 0.
