# Amendment 10 — First-draft private-LAN transport

**Date:** 2026-09-15

**Status:** Accepted

**Operator approval:** 2026-09-15

**Amends:** Amendment 09 §§2, 3.2, and 7 for the first-draft iPhone release; `SPEC-ios-client.md`
§§6.1 and 7.2

**Affects:** first-draft `T-I1`, `T-I2`, `T-I4`, `T-I5`, `T-I7`, and `T-M2a` implementation

This is a deliberate first-draft scope decision for the owner-controlled private LAN. It does not
claim that cleartext transport is suitable for the eventual away-from-home or production topology.
The eventual production target remains the HTTPS/Caddy design in Amendment 09 unless a later accepted
amendment changes it.

## Decisions

1. The first-draft iPhone connects only to a manually configured Pi address on the private LAN. The
   app rejects non-private addresses and does not expose or use an Internet client endpoint.
2. The first draft permits `http://` to that configured private address. It has no away-from-home
   operation; writes remain queued until the phone returns to the LAN.
3. The owner accepts that HTTP exposes request contents to anyone who can observe or impersonate the
   private LAN. The app must label the connection as first-draft LAN-only HTTP and must not imply TLS
   protection.
4. Device authentication remains mandatory. Enrollment still requires a recent-authenticated
   operator-created, ten-minute, single-use artifact, and the iPhone still uses a non-exportable
   Secure Enclave P-256 key. Authenticated requests still use RFC 9421 signatures, content digests,
   bounded lifetimes, request IDs, and durable replay rejection.
5. No bearer token, shared LAN secret, unsigned request, public bind, or legacy plaintext envelope
   registration is introduced.
6. This mode is not a substitute for production TLS. A later production release must restore the
   Amendment 09 public HTTPS origin, ATS system trust, hostname validation, and no-cleartext gate
   before enabling away-from-home use.

The first draft also defers HealthKit, watchOS, reminders/notifications, and other non-journal domain
features; these are product-scope decisions, not transport security controls.
