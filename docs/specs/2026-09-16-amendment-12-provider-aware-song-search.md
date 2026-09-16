# Amendment 12 — Provider-aware song search and provider-independent identity

**Date:** 2026-09-16

**Status:** Accepted

**Operator approval:** 2026-09-16

**Amends:** the first-draft song-search provider choice and song-catalog identity behavior

**Affects:** first-draft `T-I11`, `T-I13`, and song metadata/Pi lyrics work

## Decisions

1. Song-search provider priority is capability-aware:
   - If a Genius.com key is configured on the Pi and the Pi is reachable, use the Pi Genius proxy.
   - If the owner has explicitly provisioned the Pi-held key to this iPhone under Amendment 11, use
     Genius directly even when the iPhone is outside the LAN.
   - If no Genius capability is available, use the no-login/no-secret Apple/iTunes Search API directly.
   - If Genius has a technical failure or returns no usable result, fall back to iTunes and show which
     provider supplied the result. Manual free-form entry remains available when neither search works.
2. Genius credentials remain governed by Amendment 11: Pi-admin entry, no-key disabled state, explicit
   iPhone opt-in for key provisioning, encrypted device-bound transfer, `ThisDeviceOnly` storage, and
   no key in source, logs, backups, or diagnostics.
3. Provider identifiers such as Genius IDs and iTunes track IDs are external lookup metadata only. They
   are never the domain record ID, primary key, or sole deduplication key.
4. Each logical song receives a provider-independent `song_key` derived from a versioned canonical
   tuple: case-independent Unicode-normalized artist, title, and album, with collapsed whitespace and
   preserved punctuation. The displayed metadata remains unchanged. The canonical tuple is retained
   for collision review; its SHA-256 digest is the compact lookup key.
5. Provider IDs, URLs, artwork, duration/version labels when available, and search-provider evidence
   are stored as metadata attached to the logical song. A missing album is an explicit empty component,
   not a guessed value. A later provider result does not silently merge a conflicting canonical tuple.
6. A canonical-key collision or ambiguous live/remaster/edition match is surfaced for explicit choice;
   it is never resolved by provider-ID precedence or last-write-wins behavior.
7. The iPhone preserves the previous app's Journal/Songs interaction and relationship model: a
   deduplicated Song catalog, one SongOccurrence-style journal link with timestamp/source, recent-use
   ordering, and a detail screen with metadata, artwork, lyrics, and occurrence history. These are
   implemented as versioned domain records and tombstones rather than mutable local SwiftData rows.
