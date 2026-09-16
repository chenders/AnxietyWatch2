# Journal local-mirror decision

**Status:** Accepted by the owner for the first iPhone draft.

The Pi remains the authoritative owner, but the iPhone keeps a permanent local mirror of every
acknowledged journal/anxiety entry while the app remains installed. This mirror is for offline
reading and reconciliation; it is not an independent authority and performs no local analysis.

When the Pi is reachable on the private LAN, the client:

1. uploads queued creates, immutable edits, and tombstones;
2. pulls server changes through the dedicated records read contract; and
3. applies acknowledged server versions and tombstones to the local mirror.

The mirror retains entry content, version relationships, sync state, and deletion state. It must use
iOS Data Protection and never expose content in logs, URLs, analytics, or diagnostics. Backup
inclusion is a user setting: the app asks on first launch before journal use and exposes the same
setting in General Settings thereafter. A same-ID/different-content conflict preserves both facts for
explicit resolution. A phone erase or app deletion
may lose the local mirror, but Pi records remain authoritative and recoverable by a later read.

This supersedes the earlier bounded/expiring journal-cache decision. Physiological projection caches
remain bounded and expiring; this decision applies only to journal/anxiety records. The temporary
outbox remains excluded from backups; only the permanent mirror follows the user's backup choice.
