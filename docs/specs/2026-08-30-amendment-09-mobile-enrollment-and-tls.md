# Amendment 09 — Production mobile enrollment, request authentication, and TLS topology

**Date:** 2026-08-30

**Status:** Accepted

**Operator approval:** 2026-08-30

**Amends:** Amendment 02 key custody for Apple clients; Amendment 07 client-facing HTTP API;
`SPEC-admin-console.md` §9c bind/auth decision; `SPEC-ios-client.md` §6.1

**Affects:** T-I1, T-I2, T-I4, T-I5, T-I7, T-I17, T-I18, T-M2a, T-M2h, and every `/api/v1/client/*`
operation

This amendment resolves `G-IOS-AUTH`. It defines how an iPhone that may be away from home reaches the
Pi without making the admin console public, how a device is enrolled once, and how every later request
is authenticated and made replay-resistant. It does not implement the protocol.

## 0. Decisions in one place

**Enrollment decision:** An authenticated operator creates a ten-minute, single-use enrollment
artifact in the private admin console. The iPhone confirms the HTTPS origin and server identity,
generates a non-exportable Secure Enclave P-256 signing key, and exchanges the artifact for a scoped
per-device public-key credential. Existing `/api/envelope/register` plaintext secret registration is
not a production enrollment flow.

**Replay protection:** Every authenticated request uses RFC 9421 HTTP Message Signatures and RFC 9530
Content-Digest over the method, authority, path, content digest, API version, client/key identifiers,
a UUID request id, and bounded creation/expiry times. PostgreSQL durably rejects a repeated
`(client_id, request_id)`; item-level UUID idempotency remains the retry mechanism.

**Credential rotation:** Device credentials expire after 180 days and rotate when 30 days remain. A
new Secure Enclave public key is authorized by a request signed with the current key. The old key has
a maximum 24-hour overlap and is then revoked. Outbox payloads contain no credential-bound ciphertext
or signature, so rotation and recovery rebind pending writes at send time without rewriting or losing
them.

**TLS trust:** The public client origin uses a publicly trusted ACME certificate, ATS system trust,
and hostname validation on port 443. Certificate/public-key pinning is deliberately not used because
ACME rotation and single-appliance recovery make safe backup-pin operation less reliable than system
trust; invalid, expired, wrong-host, and user-installed untrusted certificates remain rejected.

## 1. Threat model

| Threat | In scope | Control |
|---|---|---|
| Passive or active network interception | Yes | TLS 1.2+ with ATS system trust and hostname validation; no redirects during enrollment/auth |
| A copied, expired, or reused enrollment QR/token | Yes | 256-bit random token, hash-only server storage, ten-minute expiry, atomic single use, operator reauthentication, rate limits |
| Replay of a captured authenticated request | Yes | RFC 9421 covered fields, 120-second signature lifetime, ±5-minute clock bound, durable request-id replay table |
| A retry creating a duplicate journal/sample | Yes | A fresh signed request id plus stable item UUIDs and item-level idempotency outcomes |
| Database disclosure revealing a reusable phone credential | Yes | Only public keys and enrollment-token hashes are stored; private keys remain in Secure Enclave |
| A lost or sold phone retaining access | Yes | Per-device revocation, fixed credential expiry, `ThisDeviceOnly` key storage, explicit device/last-used evidence |
| Credential rotation or expiry stranding queued health/journal data | Yes | Payloads are credential-neutral and signed only at send time; queue survives revocation and re-enrollment |
| Public endpoint body/connection exhaustion | Yes | Reverse-proxy and axum body limits, bounded timeouts, per-source throttling, narrow route exposure |
| Browser CSRF against enrollment/revocation | Yes | Private admin origin, Secure/HttpOnly/SameSite session, CSRF token, recent operator reauthentication |
| A compromised app process after first unlock using the signing key | Partly | The key is non-exportable, but a compromised authorized process can use its signing oracle until revocation |
| Root on the running Pi accepting fabricated data or changing authorization | No | Amendment 02's audit/integrity controls provide evidence; this protocol cannot protect a server from its own root |
| Availability of the home connection, DNS, public CA, or Pi | No | Writes remain queued and unknown/stale is shown; no failed network path may imply success |

The protocol authenticates a device, not a human medical identity. This remains a single-owner
appliance. It does not add social login, OAuth federation, advertising identity, or a cloud account.

## 2. Production topology and bind decision

The topology has two deliberately separate origins:

```text
Internet iPhone
  -> https://client.<owner-domain>:443
  -> Caddy TLS + route/body/rate limits
  -> 127.0.0.1 vitals-console /api/v1/client/* only

Operator browser
  -> WireGuard tunnel + split DNS
  -> https://admin.<owner-domain>:443 on the WireGuard address
  -> Caddy TLS
  -> 127.0.0.1 vitals-console admin routes
```

1. `vitals-console` binds to `127.0.0.1` in production. It is never directly Internet-facing.
2. Caddy is the TLS boundary and uses automatic ACME renewal. Only TCP 443 is public. The admin DNS
   name resolves only inside the tunnel; its publicly trusted certificate uses DNS-01 with a
   least-privilege, systemd-loaded DNS credential, so issuance does not expose an admin listener.
3. The public virtual host allowlists `/api/v1/client/*`. It returns a named 404/405 for admin,
   test, legacy envelope-registration, and service-operation routes.
4. The admin virtual host is reachable only through WireGuard. It is not made public to simplify
   phone enrollment.
5. Proxy-supplied identity headers are deleted from public requests and regenerated only after local
   verification. The loopback listener refuses non-loopback peers, so a caller cannot spoof proxy
   authentication headers.
6. Caddy forwards the configured external `Host` unchanged and performs no client-route path rewrite.
   Axum reconstructs RFC 9421 `@authority` and `@path` only from the configured origin and validated
   loopback proxy metadata, never from arbitrary `Forwarded`/`X-Forwarded-*` input.
7. Request and response limits are enforced at both Caddy and axum. The smaller limit wins. Uploads,
   enrollment, and ordinary API JSON receive separately named bounds.
8. Production has no HTTP, IP-literal origin, non-443 origin, ATS exception, trust-all delegate, or
   downgrade redirect. A development listener may use localhost plaintext only in explicit test mode
   and must not share production credentials or data.

### 2.1 Operator authentication required by T-M2a

The private admin console uses one operator account with an Argon2id password verifier, per-source
throttling, and a Secure/HttpOnly/SameSite=Strict session cookie. Sessions have a 30-minute idle and
8-hour absolute lifetime. Every state-changing browser request requires CSRF validation. Creating an
enrollment artifact, approving recovery, revoking a device, rotating the operator credential, or
issuing a future export/deletion capability requires password reauthentication within the preceding
five minutes. A local-root recovery command may replace the operator verifier but never reveal it;
the event is audited.

The client API does not use browser cookies and is therefore not made dependent on CSRF tokens. It
uses the signed-device request protocol below.

## 3. One-time enrollment

### 3.1 Artifact creation

After recent operator reauthentication, the admin console creates an artifact containing:

- schema and API major version;
- normalized `https://` client origin, DNS hostname, and implicit port 443;
- stable server-instance UUID and operator-chosen display label;
- random enrollment UUID and 256-bit opaque token;
- allowed initial scopes and expiry, at most ten minutes;
- a human-readable instruction that the artifact grants one new device.

The artifact may be rendered as QR and copyable text. It is sensitive until expiry and is never
logged, included in diagnostics, or shown again after leaving the creation screen. The database
stores the enrollment UUID, SHA-256 token hash, expiry, scopes, creator/audit evidence, and consumed
state—not the token. A 256-bit random token does not need password stretching; verification uses
constant-time comparison. Creating artifacts is rate-limited and at most three may remain live.

### 3.2 Phone confirmation and claim

Before sending any health or journal data, the app shows:

- the exact DNS origin;
- the server display label and instance identifier carried by the physically transferred artifact;
- that ATS system TLS and hostname validation will be used;
- the requested device scopes and credential expiry policy.

The user explicitly confirms. The app rejects HTTP, IP literals, ports other than 443, embedded user
info, fragments, and enrollment redirects.

The app then creates a P-256 signing key in Secure Enclave using Security/CryptoKit. The private key
is non-exportable and available after first unlock only on that device, without a user-presence flag
that would break background delivery. There is no production software-key fallback; simulator keys
are test-only and cannot enroll against production.

The claim sends the one-time token, enrollment UUID, server-instance UUID, public key, app/API
versions, and a user-visible device label over TLS. The server atomically verifies and consumes the
artifact, stores the public key, and returns `client_id`, `key_id`, accepted scopes, issue/expiry
instants, and server time. A consumed artifact never issues a second credential, including when the
first response is lost: the console shows the pending/issued device and the phone uses the recovery
path rather than replaying enrollment.

## 4. Device credential and authorization scope

A credential is the tuple `(client_id, key_id, P-256 public key, scopes, issued_at, expires_at,
revoked_at)`. The server stores no bearer access token, refresh token, symmetric envelope key, or
private key.

Initial scopes are allowlisted and least-privilege:

- submit enabled platform-health batches and status beacons;
- submit domain-record classes explicitly enabled for Stage 7;
- read projections/reports belonging to this single-owner appliance;
- read registry/API compatibility and the device's own credential status;
- rotate its own key.

A phone credential cannot access admin pages, operate systemd services, alter alarms, mint/revoke
other devices, change registry policy, issue enrollment artifacts, or bypass item validation.
Authorization is enforced server-side per route and per item class. The watch has no Pi credential;
its eventual Quick Log is a phone-relayed domain item.

The Keychain stores only the Secure Enclave key reference and non-secret client/key metadata using a
`ThisDeviceOnly` accessibility class compatible with background work after first unlock. An optional
Face ID/Touch ID screen lock is presentation privacy, not remote authentication and not a boolean
substitute for Keychain access control.

## 5. Authenticated request profile

Every protected request follows RFC 9421 and RFC 9530 using `ecdsa-p256-sha256`. Implementations use
Security/CryptoKit on Apple platforms and maintained Rust HTTP-signature, P-256, and constant-time
verification libraries. No handwritten canonicalization or cryptographic primitive is permitted.

The signature base covers at least:

- `@method`, `@authority`, and `@path`;
- `content-digest` for every request with a body;
- `content-type` and API major-version header;
- client id and key id headers;
- a cryptographically random UUID request-id header;
- RFC 9421 `created` and `expires` parameters.

`expires - created` is at most 120 seconds. The server allows no more than five minutes of clock skew,
uses its own clock for expiry decisions, validates scope before decoding expensive payloads, and
inserts `(client_id, request_id, request_digest, expires_at)` in PostgreSQL in the same admission
transaction. A duplicate request id is rejected as `request_replayed`, even if its body differs;
the response names the client id, request id, and blocking condition without echoing health content.
Replay rows remain at least 24 hours. A request outside the clock bound fails as
`client_clock_out_of_range`; it does not fall back to unsigned or bearer authentication.

A transport retry creates a new request id and signature while preserving item UUIDs. Item-level
created/replayed/rejected/conflict outcomes determine outbox deletion. HTTP success alone does not.

## 6. Expiry, revocation, rotation, and recovery

### 6.1 Rotation

Credentials expire 180 days after issuance. On any permitted foreground or background drain with 30
or fewer days remaining, the app generates a new Secure Enclave key and submits its public key in a
rotation request signed by the current key. The server activates the new key and permits both key ids
for at most 24 hours so an interrupted response can recover. The app confirms one request with the
new key before deleting the old key reference. The server then revokes the old key; the overlap may
be shortened but never extended beyond 24 hours.

Outbox records store item identity and immutable payload, not an encrypted copy tied to the device
credential and not a precomputed request signature. Rotation changes the signer selected at send
time. No queued payload is decrypted/re-encrypted, rewritten, dropped, or assigned a new item id.

### 6.2 Revocation and lost devices

The operator can revoke a device or individual key immediately after recent reauthentication. The
server denies it before body processing and records last-used, revocation actor, time, and reason.
The phone treats `credential_revoked` and `credential_expired` as blocked authentication states,
preserves every unacknowledged outbox item, stops automatic retry loops, and directs the owner to
recovery. Revocation does not delete server records or local queued content.

### 6.3 Recovery

Recovery always creates a fresh single-use enrollment artifact and a fresh Secure Enclave key. It
never reveals, downloads, or copies an old private key or enrollment token. After operator approval,
the server links the new key to the intended device record (or creates a replacement device) and the
phone signs pending credential-neutral items with the new key. Same item IDs make replay a no-op.

If the app was deleted or the phone erased, `ThisDeviceOnly` material and the outbox are
unrecoverable; the UI and operator console must state that fact rather than implying server recovery
can restore unacknowledged phone-only writes. Server-held records are unaffected.

## 7. TLS trust and pinning decision

Certificate pinning is **not justified for the initial single-owner release**:

- the endpoint uses a publicly trusted certificate and ATS hostname validation;
- ACME certificates rotate frequently, while the Pi may be unavailable during an emergency repair;
- safe pinning requires tested backup keys, overlap, expiry, and an out-of-band update path;
- a stale pin would silently strand HealthKit and subjective writes, creating the exact long-lived
  data freeze this project is designed to expose;
- per-device signed requests prevent a TLS-terminating impostor without the device key from creating
  accepted writes, even though such an impostor could still observe plaintext at its endpoint.

The app therefore uses the unmodified system trust store and does not install a custom
`URLSessionDelegate` trust override. It rejects certificate errors, hostname mismatch, expiry,
chains not accepted by system policy, and redirects on enrollment/auth requests. A root that the
owner explicitly installs and enables for full trust is part of iOS system trust and can intercept
traffic; that residual risk is accepted for the initial release. Request signatures prevent such an
interceptor from authoring accepted writes, but do not stop it observing requests or spoofing reads.
The settings/diagnostics surface shows the normalized origin, last TLS success/failure, certificate
subject/issuer and expiry as observations—not as a second trust algorithm.

This satisfies MASVS-NETWORK-2 as a documented risk-based decision, not as a claim that pinning is
universally harmful. T-I17 must revisit pinning if the endpoint moves behind a third-party TLS
terminator, the threat model changes, or reliable managed backup pins become available.

## 8. Existing envelope routes are not production mobile authentication

The current routes accept a caller-chosen symmetric secret at `/api/envelope/register`, store that
secret in `envelope_clients`, and decrypt XSalsa20-Poly1305 payloads at `/api/envelope`. This was a
localhost M0 test seam. It has no operator authorization, expiry, revocation, replay table, remote
server-identity ceremony, or safe rotation.

Accordingly:

1. `/api/envelope/register` and `/api/envelope/plaintext-test` are disabled in production and are
   never routed by the public TLS virtual host.
2. No Apple client calls them or stores their symmetric key.
3. T-I2 introduces versioned `/api/v1/client/*` placeholders and fixtures; only operations whose
   authentication and behavior are implemented advertise success.
4. Legacy secretbox code may remain behind explicit loopback test mode until its tests migrate. It
   is not a compatibility path and receives no production credentials or health data.
5. TLS plus request signatures replace ad hoc application-layer encryption. The server already must
   read accepted content, so a symmetric key stored beside that content does not add a meaningful
   server-compromise boundary and makes credential rotation destructive.

## 9. MASVS and MASTG mapping

| Control | Decision and acceptance evidence |
|---|---|
| MASVS-AUTH-1 | Single-use operator-authorized enrollment; non-exportable per-device key; RFC 9421 authenticated requests; server-side route/item scopes; fixed expiry/revocation/rotation |
| MASVS-AUTH-2 | Secure Enclave + `ThisDeviceOnly` Keychain custody; optional biometric UI lock is explicitly separate and cannot authorize network requests by boolean |
| MASVS-AUTH-3 | Recent operator password reauthentication is required to enroll/recover/revoke devices and issue future export/deletion capability; CSRF remains mandatory |
| MASVS-NETWORK-1 | HTTPS only, ATS system trust, hostname validation, no cleartext/redirect/trust override, narrow public routes, bounded traffic |
| MASVS-NETWORK-2 | Pinning assessed and rejected with a documented rotation/availability rationale; revisit trigger and negative TLS tests are required |

Implementation acceptance maps to MASTG-TEST-0064 (iOS local authentication), MASTG-TEST-0266
(biometry without Keychain), MASTG-TEST-0065 (network encryption), MASTG-TEST-0066 (ATS),
MASTG-TEST-0068 (endpoint identity), and MASTG-TEST-0233 (disabled certificate validation). Dynamic
physical-device tests must prove background signing after first unlock, locked-state behavior,
wrong-host/expired/untrusted certificate rejection, revocation, clock failure, replay rejection,
rotation overlap, and preservation of queued records.

## 10. Task consequences

- **T-M2a:** implement loopback axum bind, WireGuard-only admin origin, operator Argon2id session,
  per-source throttling, CSRF, limits, visible TLS state, and recent-auth evidence. Public routing is
  not enabled until these controls and the Caddy allowlist tests pass.
- **T-I2:** define versioned client routes, RFC 9421/OpenAPI headers and stable unavailable/auth error
  fixtures. It does not make placeholder operations succeed.
- **T-I4:** implement enrollment, Secure Enclave/Keychain lifecycle, identity confirmation,
  revocation/recovery UI, and negative trust tests.
- **T-I5:** keep outbox payloads credential-neutral so rotation and recovery cannot require payload
  rewriting.
- **T-I7:** implement fresh request ids/signatures per attempt, durable replay outcomes, rotation
  trigger, bounded retry, and clock diagnostics.
- **T-I17/T-I18:** run MASVS static gates and physical-device locked/background/TLS/revocation tests.
- **T-M2h:** expose write-only enrollment artifacts, device/key status, expiry, last used, rotation,
  revocation, and audit evidence; never reveal old key material.
