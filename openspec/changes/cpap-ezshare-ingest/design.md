# Design — CPAP ez Share ingestion

## Data path

`ez Share HTTP -> immutable local mirror -> Python EDF batch parser -> validated ingest batch -> PostgreSQL`

Python is retained only for the batch parser, as required by amendment 07. It does not implement the
HTTP/admin API or speak the vitalsd socket protocol. The importer may use PostgreSQL directly only
through a narrow transaction that records an attempt before work and atomically records its effect.

## Invariants

1. **The card is untrusted.** Listings and files are attacker-controlled bytes. Bound response size,
   request time, recursion, and file count; reject traversal and redirects away from the configured
   card origin. Never execute or render card content.
2. **Mirror before parse.** Download to a temporary file, fsync, hash, and atomically rename. A manifest
   records card path, byte length, SHA-256, first-seen time, and parse status. Retries are idempotent.
3. **Raw evidence is immutable.** Never modify a mirrored EDF. A parser upgrade can replay it.
4. **No invented physiology.** Unknown/sentinel/invalid channels contribute no numeric sample. Units and
   valid ranges come from `vitals/registry.toml` and conversion occurs once at decode.
5. **Stable identity.** Before ingest is implemented, its contract is frozen in a task: deterministic
   session identity from immutable file/session evidence and stable per-channel sample sequence. Arrival
   time is provenance, never identity. Replays deduplicate under `(source,type,session_id,seq)`.
6. **Clock correction is metadata.** Store raw device time and a versioned transform plus derivation
   evidence. Apply the transform at read time; changing it appends a version and never updates samples.
7. **Attempts precede work.** Every poll/import creates an attempt before I/O. Success, failure, timeout,
   no-op replay, and rejected rows are distinguishable; killed work remains unfinished and degrades.
8. **On-demand health.** ez Share SD is event-driven. Its age alone never changes it to failing; latest
   attempt outcome and explicit schedule/window policy determine the displayed state.
9. **Plain HTTP stays private.** The card has no TLS. Traffic is routed only over the dedicated card
   interface/subnet; the service must not expose the card to the LAN or accept arbitrary target URLs.
10. **OSCAR boundary.** OSCAR documentation/file formats may inform tests; no GPL implementation code is
    copied. Record attribution for every runtime library and follow hms-cpap's clean-room NOTICE pattern.

## Task boundaries

- T-CP1 builds only the safe mirror and fixtures. It cannot choose clinical mappings or write SQL.
- T-CP2 freezes EDF semantics, registry additions, identity, and clock metadata with golden fixtures.
- T-CP3 implements transactional append-only ingestion and attempts.
- T-CP4 installs routing and systemd supervision after the physical gate.
- T-CP5 exposes operator evidence and runs a real-card end-to-end acceptance test.
