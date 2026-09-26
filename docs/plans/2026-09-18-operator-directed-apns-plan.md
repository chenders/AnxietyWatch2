# Operator-directed APNs messages — implementation plan

**Status:** Planned; Amendment 16 accepted on 2026-09-18
**Scope:** Pi-to-iPhone operator messages, targeted delivery, and fanout to all active phones
registered with one Pi/server instance.
**Owning tasks:** admin roadmap `T-M6e`; Apple client `T-I22`; physical/device acceptance `T-I18`;
end-to-end acceptance `T-I20`.

## Outcome

The operator will be able to send a minimal, audited APNs notification to:

1. one selected active device;
2. all active devices belonging to one selected user; or
3. every active, non-revoked phone registered with the current Pi/server instance.

“Every phone” means every eligible registration known to this Pi at recipient-resolution time. It
does not mean every phone ever enrolled, a revoked/stale token, or a device registered to another
server instance. APNs acceptance is never reported as proof of delivery, display, or acknowledgement.

This feature is separate from the CNS/local klaxon path and does not alter Amendment 14: phone-silence
escalation must continue through the independent email path, never APNs to the silent phone.

## Current starting point

- Mobile enrollment and authenticated phone-to-Pi transport are working on the private LAN.
- The Pi has a protected configuration path and active `vitals-console` service.
- The authenticated device-registration endpoint, encrypted registration persistence, and the
  JWT/HTTP2 APNs provider transport are now implemented in the worktree; send/fanout API and
  delivery ledger are not available yet.
- No APNs secrets, device tokens, or fabricated delivery evidence may be committed.
- The song-artwork work currently in progress is independent of this plan.

## Milestones

### M0 — Contract and safety lock

**Owners:** T-M6e/T-I22 planning

- Keep Amendment 16, Amendment 14, the iOS specification, admin specification, traceability tables,
  and this plan consistent.
- Freeze the state vocabulary: `NOT_CONFIGURED`, `UNKNOWN`, `READY`, `ATTEMPTED`, `ACCEPTED`,
  `DELIVERED`, `DISPLAYED`, `ACKNOWLEDGED`, `REJECTED`, `REVOKED`, and `EXPIRED` as applicable.
- Define the immutable message ID, idempotency key, target snapshot, audit actor, payload class,
  provider response, token invalidation, retry state, and terminal outcome.
- Define the redaction rule: no HealthKit values, journal text, medication details, credentials,
  enrollment material, or sensitive lock-screen content.

**Exit gate:** Rust/Swift contract fixtures describe targeted, user, all-device, zero-recipient,
invalid-token, retry, and duplicate-message cases without claiming delivery from APNs acceptance.

### M1 — Registration identity and persistence

**Owners:** Pi/API first, then iPhone client

- Add a server-side registration model bound to the server instance and owning user/device identity.
- Store APNs environment, token fingerprint/secured token representation, app/build identity,
  authorization facts, first/last seen, revocation, and last provider reason.
- Define whether the current deployment has one operator identity or multiple user identities; do not
  silently treat a device identifier as a user identifier.
- Add migration deploy/revert/verify scripts and grants. Preserve append-only audit and delivery
  records; do not use destructive updates against unrelated sample tables.

**Exit gate:** migration verification passes; one registration can be created, refreshed, revoked,
and queried without returning the token or a provider private key.

### M2 — iPhone authorization and token registration

**Owner:** T-I22

- Request notification authorization with a minimal rationale and record the system outcome.
- Register the APNs token after `didRegisterForRemoteNotificationsWithDeviceToken` and refresh it
  whenever Apple supplies a new token.
- Send registration updates only through the existing authenticated mobile API and keep them
  idempotent.
- Handle denied authorization, missing token, logout/revocation, app reinstall, and stale build
  states explicitly.
- Add app-visible diagnostics that distinguish authorization, registration, Pi acknowledgement, and
  message outcomes.

**Exit gate:** Swift tests cover token rotation, denial, duplicate registration, revoked credential,
offline retry, and no-token states; physical-device registration evidence is captured without secrets.

### M3 — Pi APNs provider and single-recipient send

**Owner:** T-M6e

- Configure APNs token authentication on the Pi using protected provider configuration: Team ID,
  Key ID, bundle ID, environment, and private key material. Values are write-only and never logged or
  returned by the console.
- Implement one-device sends with bounded timeout/retry, APNs reason capture, and idempotent message
  handling.
- Mark invalid/unregistered tokens revoked or excluded with the exact sanitized provider reason.
- Keep test sends separate from real operator messages and visibly mark them in the ledger.

**Exit gate:** a controlled test against a real registered device proves authenticated provider
communication, invalid-token handling, retry classification, and no secret leakage. A provider
acceptance is recorded only as `ACCEPTED`.

### M4 — Target resolution and operator controls

**Owner:** T-M6e

- Add protected console controls for device, user, and all-active-device targets.
- Resolve and persist the recipient snapshot before confirmation; show the measured recipient count
  and registration ages.
- Require explicit confirmation for all-device fanout with wording that includes the count.
- Make zero recipients a visible non-success outcome.
- Apply authentication, CSRF protection, authorization, rate limits, audit actor, and confirmation
  requirements to every state-changing operation.
- Keep the operator UI from exposing APNs tokens, private keys, or sensitive message content.

**Exit gate:** fixed tests prove device targeting, user targeting, all-device targeting, scope
isolation between server instances, revoked/stale exclusion, zero-recipient behavior, and duplicate
submission idempotence.

### M5 — Delivery ledger, observability, and protected content handling

**Owners:** T-M6e/T-I22

- Record request, provider acceptance/rejection, token invalidation, retry, terminal status, and
  audit evidence as separate facts.
- Support app acknowledgement only where the app can honestly report it; never synthesize display or
  acknowledgement from a provider response.
- Use minimal notification payloads. For protected categories/deep links, the app re-fetches content
  from the Pi after unlock rather than placing protected content in the APNs payload.
- Expose configured state, token count/age, last attempt, last accepted/rejected outcome, and stale/
  never-tested state in the console.
- Keep operational test notifications out of real alarm-delivery evidence.

**Exit gate:** ledger and console tests prove `UNKNOWN`/`NOT_CONFIGURED`, successful acceptance,
provider rejection, stale registration, retry, token revocation, and accepted-but-not-acknowledged
states.

### M6 — Physical-device and failure-mode acceptance

**Owner:** T-I18 with T-I22

On `theodore` and any additional actually registered device, measure and record only observed facts:

- authorization allowed and denied;
- token registration and rotation;
- single-device delivery/display behavior;
- all-active-device fanout with the measured recipient count;
- locked, foreground, background, force-quit, offline, and revoked-token behavior;
- notification privacy/redaction and protected-content re-fetch;
- duplicate message and retry behavior.

**Exit gate:** `artifacts/ios-stage7-apns-evidence.json` contains measured timestamps/build/device
observations, with unknown or unsupported behavior recorded as such. No evidence is filled from
expectation.

### M7 — End-to-end rollout and completion

**Owner:** T-I20 plus admin M6 acceptance

- Run the server APNs verification suite and Apple client tests.
- Run the physical-device evidence verifier.
- Confirm alarm independence with the iPhone powered off; an APNs failure must not affect local
  collection, detection, or klaxon acceptance.
- Confirm Amendment 14 phone-silence behavior still uses email and never APNs.
- Review secret scans, audit records, migrations, rate limits, payload redaction, and rollback.
- Update the acceptance report and mark T-M6e/T-I22/T-I18/T-I20 only after their written `Verify:`
  commands exit 0.

**Completion gate:** targeted send, user fanout, all-active-device fanout, no-recipient handling,
invalid/revoked token handling, bounded retry/idempotence, auditability, minimal payloads, and
separate APNs acceptance/display/acknowledgement evidence all pass.

## Dependencies and sequencing

```text
M0 contract
  -> M1 registration schema/API
      -> M2 iPhone token registration
          -> M3 single-device provider send
              -> M4 target resolution/fanout controls
                  -> M5 ledger/observability/content handling
                      -> M6 physical-device evidence
                          -> M7 end-to-end acceptance and rollout
```

The Pi contract and iPhone registration can be developed in parallel after M0, but no fanout is
considered complete until both sides and the physical-device evidence exist. Do not create a fake
APNs provider or mark delivery successful when Apple credentials, an eligible token, or a measured
device observation is unavailable.
