# Song metadata and lyrics decision

**Status:** Accepted by the owner for the private first draft.

Song search is capability-aware. If a Genius.com key is configured on the Pi and the Pi is reachable,
the iPhone uses the Pi Genius proxy. If the owner has explicitly provisioned that key to the iPhone,
the iPhone uses Genius directly even outside the LAN. Without Genius capability, it uses the no-login/
no-secret Apple/iTunes Search API directly. A Genius technical failure or unusable result falls back
to iTunes with the provider shown; manual free-form song/album entry remains available without search.
Search may return title, artist, album, artwork reference, and provider ID; the journal stores a
metadata snapshot so the entry remains useful outside the LAN. The mirror stores the artwork URL and a
small local thumbnail so the cover remains visible offline, subject to provider licensing and
attribution review.
Search requests never include journal text.

Lyrics retrieval is a Pi-side enrichment step. The private first draft may keep the existing
Genius.com API key on the Pi only; it must never be copied into the iPhone, repository, fixtures, logs,
or diagnostics. The Pi downloads lyrics only after the record is synced. The Pi admin page owns the global
lyrics-enrichment setting; it defaults to automatic. If no Genius.com key is configured, enrichment is
forced off and the disabled control explains that a key is required. The page offers a guided setup
for obtaining a Genius.com key and placing it into the Pi's secret/configuration store; the key is
never displayed after entry. Once a key is present, the setting can be enabled.

Provider IDs are lookup metadata only. Each logical song uses a provider-independent versioned
canonical key derived from case-independent Unicode-normalized artist, title, and album components;
the key is not a Genius or iTunes ID. Ambiguous editions and canonical-key collisions require explicit
selection.

The iPhone preserves the previous app's Journal/Songs relationship model: a deduplicated logical Song
record is linked to each journal entry through a SongOccurrence containing the entry timestamp and
`journal` source. The Songs section sorts by most recent activity and then title. A song detail surface
shows metadata, artwork, lyrics when available, and occurrence history. The first draft permits one
linked song per journal entry; changing the link creates the corresponding immutable domain edit rather
than mutating history in place.

Before any public release, the product must revisit Genius licensing/credentials and AI analysis. The
public flow will require transparent user disclosure and consent, explain that lyrics and any optional
AI analysis leave the device/Pi boundary as applicable, and guide each user through registering and
supplying their own Genius.com API key. That public-release work is not part of the first draft.
