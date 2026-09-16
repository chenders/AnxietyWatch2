---
applyTo: "clients/apple/**/*.swift"
---

# Swift rules for the thin Apple client

This is an iPhone-first thin client. The Pi owns authoritative durable records and analysis; the
phone keeps a durable outbox, bounded physiological projection caches, and a protected permanent
journal/anxiety read mirror for offline history.

## Data and sync invariants

- A successful save means durably queued on the iPhone, not accepted by the Pi. Show queued,
  acknowledged, retryable, rejected, and conflict as distinct states.
- Never discard an unacknowledged journal/anxiety entry during logout, reset, re-enrollment, or server
  replacement. Acknowledged deletion is an append-only tombstone.
- Same record ID with different content is a conflict, never an overwrite. Edits use a new ID and
  `supersedes` the prior version.
- The first-draft entry time is the iPhone capture instant. The Pi receipt time is transport evidence
  and must not replace it.
- Physiological Pi projections may be cached only with an expiry and visible stale/unknown state.
  Journal/anxiety history is a protected local mirror reconciled from the Pi. No local analysis,
  fabricated zero/healthy state, or polling loop.
- Missing, absent, denied, or unknown data contributes nothing and is never coerced to zero.

## Security and privacy

- Keep credentials in the approved Keychain/Secure Enclave path. Never log credential values or put
  health/journal content in URLs, analytics, crash breadcrumbs, or diagnostics.
- First-draft LAN HTTP is permitted only by Amendment 10 to a configured private address; device
  signatures, enrollment approval, scoped credentials, replay IDs, and private-address validation
  remain mandatory. Do not add unsigned or shared-secret fallback.
- Do not add HealthKit, watchOS, reminders, camera, Bluetooth, location, or tracking permissions
  without an accepted feature decision and matching tests.
- Do not add a third-party dependency without recording its license, privacy, maintenance, and need.

## Swift implementation

- Use Swift 6 strict concurrency and structured concurrency. Keep transport/state ownership explicit;
  do not introduce detached work or parallel drain owners.
- Use `Result`/typed errors or explicit state models instead of Boolean-plus-out-parameter APIs.
- Avoid force unwraps, `fatalError`, `precondition`, and unchecked casts in production paths.
- Never assert floating-point equality; use a tolerance.
- Use fixed reference timestamps in tests, never wall-clock assertions.
- Every behavior change gets a Swift Testing test or a deliberate physical-device acceptance test.
- Keep pure packages free of SwiftUI, HealthKit, WatchConnectivity, Core Data, SwiftData, Core
  Bluetooth, filesystem I/O, and network access.
- Do not hand-edit generated registry output. Numeric source/type identifiers, units, ranges, and
  thresholds come from the registry/API contract.

## Review and verification

Run the applicable `scripts/verify-ios-*.sh` gates, package tests, and `scripts/scan-public-info.sh`.
A task's literal `Verify:` command is the acceptance contract and must not be weakened to fit the
implementation.
