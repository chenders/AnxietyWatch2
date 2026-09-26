# Amendment 11 — Optional direct lyrics retrieval on the iPhone

**Date:** 2026-09-16

**Status:** Accepted

**Operator approval:** 2026-09-16

**Amends:** the first-draft lyrics decision in `2026-09-16-song-metadata-and-lyrics.md` and the
first-draft iOS records/transport behavior

**Affects:** first-draft `T-I4`, `T-I7`, `T-I11`, `T-I13`, and Pi lyrics enrichment/admin work

The owner accepts an opt-in path for seeing lyrics while the iPhone is outside the private LAN. This
is an explicit exception to the earlier Pi-only Genius credential decision; the Pi remains the
credential authority and the domain-record authority.

## Decisions

1. The Genius.com API key is entered only through the authenticated Pi admin page and is stored in
   the Pi secret/configuration store. The admin page never echoes it after entry and shows presence
   plus last-four only.
2. The Pi admin page exposes a capability state. With no configured key, direct lyrics retrieval is
   unavailable, forced off, and disabled with a named explanation and guided key-setup flow.
3. After a key is present, the iPhone may explicitly opt in to **“Download lyrics immediately once a
   journal entry is submitted.”** This is a per-device setting. It is off until the owner enables it.
   The iPhone displays that the setting sends song metadata and lyrics requests to Genius.com and that
   the Pi-provisioned key will be held on this device.
4. The Genius key is never sent over first-draft LAN HTTP in plaintext. The enrolled iPhone creates a
   separate non-exportable Secure Enclave P-256 key-agreement key. The Pi provisions the key only in a
   signed, replay-protected response encrypted to that device public key using the approved P-256
   ECDH/HKDF/AES-GCM implementation. The key is stored `ThisDeviceOnly`, excluded from backups, and
   never logged or placed in diagnostics. No handwritten cryptography is permitted.
5. If the iPhone is outside the LAN but has Internet access, an opted-in submission may call Genius
   directly. If either Internet or Genius is unavailable, the journal entry still saves normally and
   the lyrics request remains queued with an explicit pending state; it never blocks or fabricates
   lyrics.
6. A successful direct lyrics result is stored in the protected local journal mirror and marked with
   provider, lookup identity, retrieval time, and attribution/licensing metadata. When the Pi becomes
   reachable, the phone uploads the enrichment result so the Pi can retain the authoritative synced
   copy and audit evidence. The key itself is not uploaded back from the phone.
7. The Pi may revoke lyrics capability or rotate the configured key. The iPhone deletes its copied key
   when it receives a revocation/disable result or when the owner turns the feature off. Because an
   erased/offline phone cannot be remotely wiped, the opt-in screen names that residual risk.
8. The Pi global automatic-enrichment setting remains separate. The Pi-side automatic setting controls
   Pi fetching after sync; the iPhone per-device setting controls direct off-LAN fetching. Both require
   a configured Pi Genius key. The first-draft Pi default remains automatic; direct iPhone fetching is
   opt-in.
9. Before any public release, licensing, Genius credential ownership, per-user key onboarding,
   external disclosure/consent, and optional AI analysis require a new release gate. The public flow
   must not silently reuse the private first-draft owner's key across users.
10. The iPhone has both a Journal section and a dedicated Songs section. Journal song links open the
    shared song detail surface, which shows all available metadata, artwork, provider/retrieval
    evidence, and lyrics when present. Missing, pending, or unavailable lyrics remain explicit states;
    they are never rendered as empty or fabricated content.
