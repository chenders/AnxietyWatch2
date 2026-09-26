# Amendment 15 — HealthKit is a first-release phone capability

**Date:** 2026-09-17

**Status:** Accepted by operator

**Supersedes:** Amendment 10's first-draft sentence deferring HealthKit and the corresponding
first-draft deferral in `SPEC-ios-client.md` §1.1.

**Affects:** T-I8, T-I9, T-I11, T-I18, T-I19, T-I20, and the first-release scope of the Apple client.

## Decision

HealthKit is part of the first version of the iPhone app. The phone requests the retained read-only
HealthKit types from the generation-one parity ledger, queries them using per-type anchored queries,
preserves source and device provenance, and uploads bounded platform-sample envelopes to the Pi's
authenticated write API.

This is a source bridge, not a local health archive. The Pi remains authoritative and performs
canonical conversion, validation, persistence, projections, and analysis. The iPhone never writes to
HealthKit, computes snapshots or trends, or presents a successful authorization request as proof that
read access was granted.

## Required behavior

- Show a plain-language rationale for each enabled type before the system authorization sheet.
- Request only read access; the share set remains empty.
- Keep independent request, query, sample, outbox, acknowledgement, and Pi-receiving facts.
- Use one persisted `HKQueryAnchor` per enabled sample type.
- Commit every page's platform payloads and its next anchor atomically; a crash replays safely.
- Preserve HealthKit UUID, original type/unit/value or category, interval, source revision, device,
  allowlisted metadata, query batch identity, and phone record time.
- Keep the bounded outbox until terminal server acknowledgement; retry and replay are idempotent.
- Register observer/background delivery for enabled types, with callbacks limited to scheduling
  bounded work and completing promptly.
- Keep excluded parity entries explicitly excluded with their named reason. In particular, clinical
  lab documents, demographic context, heartbeat series, and any type without an accepted registry and
  receiver mapping are not silently flattened into scalar samples.

## Scope correction

The first-draft private-LAN HTTP and eventual HTTPS/Caddy decisions remain unchanged. HealthKit
payloads use the same authenticated phone-to-Pi transport and are never sent to a third party. Watch
relay behavior remains separate: the Watch holds no Pi credential and relays through the iPhone.
