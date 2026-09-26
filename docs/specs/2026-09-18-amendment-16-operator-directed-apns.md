# Amendment 16 — Operator-directed APNs messages

**Date:** 2026-09-18

**Status:** Accepted by operator

**Supersedes:** The first-draft language in `SPEC-ios-client.md` §1.1 and §9 that broadly defers
remote notifications, and the Stage 7 traceability entry that leaves remote alarm delivery as a
later optional client task.

**Does not supersede:** Amendment 14. APNs remains prohibited for reporting phone silence to the
same phone whose silence is being detected. Local journal/dose/check-in reminders and direct
watch-to-Pi communication remain deferred unless separately accepted.

**Affects:** Admin M6 remote push work, Stage 7 client transport/notification work, and the mobile
registration schema/API.

## Decision

The Pi may send an operator-directed APNs notification to:

1. one explicitly selected active device;
2. all active devices belonging to one explicitly selected user; or
3. every active, non-revoked phone registered with this Pi/server instance.

“Every phone registered” means the current server-instance registry after revocation and liveness
filters. It is not a broadcast to devices belonging to another instance or tenant. A notification
is best-effort: APNs acceptance is not proof of delivery, display, or acknowledgement.

## Required behavior

- Each iPhone registers its APNs environment, device token, app/build identity, notification
  authorization facts, owning user/device identity, first-seen time, last-seen time, and revocation
  state through the authenticated mobile API.
- The Pi stores no APNs private key or token in browser-visible state. Provider credentials are
  write-only/configured through the protected operator configuration path and are shown thereafter
  only as presence and last-four metadata.
- The console previews the resolved recipient count and target identity before sending. “Every
  registered phone” requires explicit confirmation containing the measured count. Zero recipients is
  reported as zero, never as success.
- Every request receives an immutable message ID, target snapshot, audit actor, creation time,
  payload class, APNs response, token invalidation result, retry outcome, and terminal state.
  APNs accepted, delivered, displayed, and acknowledged remain distinct facts.
- Test sends are authenticated, separately confirmed, rate-limited, audited, visibly marked as
  tests, and cannot count as a real alarm delivery. A real operator message cannot count as a test.
- Notification content is minimal by default and must not contain HealthKit values, journal text,
  medication details, credentials, enrollment material, or other sensitive data. A category/deep
  link may identify a Pi surface, but the app must re-fetch protected content after unlock.
- Invalid, unregistered, expired, or revoked tokens are marked with the APNs reason and excluded
  from future fanout. A transient provider failure remains visible and is retried only under a
  bounded, idempotent policy.
- The feature is not the local klaxon path and does not suppress, acknowledge, or clear an alarm.
  It must not be used for Amendment 14 phone-silence escalation.
- Missing authorization, stale registration, provider misconfiguration, and never-tested state
  are explicit `UNKNOWN`/`NOT_CONFIGURED` states, never inferred as delivery success.

## Acceptance scope

The implementation must prove targeted send, user fanout, all-active-device fanout, no-recipient
handling, revoked-token exclusion, APNs invalid-token handling, retry/idempotence, audit records,
minimal payload redaction, independent test-send accounting, and the distinction between APNs
acceptance and device delivery. Physical-device evidence is required for actual display behavior;
server acceptance alone is insufficient.
