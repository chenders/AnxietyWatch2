# Song metadata and lyrics decision

**Status:** Accepted by the owner for the private first draft.

The iPhone may search basic song metadata directly through a provider that requires no user login or
API secret. The first choice is the Apple/iTunes Search API. Search may return title, artist, album,
artwork reference, and provider ID; the journal stores a metadata snapshot so the entry remains useful
outside the LAN. The mirror stores the artwork URL and a small local thumbnail so the cover remains
visible offline, subject to provider licensing and attribution review. A manual free-form song/album
entry remains available when the iPhone has no network.
Search requests never include journal text.

Lyrics retrieval is a Pi-side enrichment step. The private first draft may keep the existing
Genius.com API key on the Pi only; it must never be copied into the iPhone, repository, fixtures, logs,
or diagnostics. The Pi downloads lyrics only after the record is synced. A global lyrics-enrichment setting controls
whether this happens automatically, and its first-draft default is automatic.

Before any public release, the product must revisit Genius licensing/credentials and AI analysis. The
public flow will require transparent user disclosure and consent, explain that lyrics and any optional
AI analysis leave the device/Pi boundary as applicable, and guide each user through registering and
supplying their own Genius.com API key. That public-release work is not part of the first draft.
