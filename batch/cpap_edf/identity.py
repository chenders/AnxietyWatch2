"""Deterministic session and sequence identity for CPAP samples.

The design invariant this implements, from `design.md`:

> **Stable identity.** Deterministic session identity from immutable
> file/session evidence and stable per-channel sample sequence. Arrival time is
> provenance, never identity. Replays deduplicate under
> `(source, type, session_id, seq)`.

Both halves below are *contracts*, not conveniences: once a sample is stored
under a `(source, type, session_id, seq)` key, re-importing the same bytes must
produce the same key or the append-only store gains a duplicate it cannot
detect. So each derivation uses only evidence that cannot change for a given
recording, and the reasoning is written down because a later reader cannot
re-derive intent from a hash.

## session_id

Derived from the **card directory holding the file** plus the **recording start
date**, not from the file's own hash.

A ResMed night is several files -- `BRP`, `PLD`, `SAD`, `CSL` -- written into
one `DATALOG/<date>` directory and describing one therapy session. Keying on
the file hash would give each of them a different session, splitting one night
into four and making a cross-channel query impossible. Keying on the directory
and date groups them, and both are immutable for a given recording.

**The assumption, stated so it can be falsified:** one directory and date holds
one session. If the device ever writes two distinct sessions into one date
directory, they would merge under one id. `session_conflicts` exists to detect
exactly that -- it reports files sharing a session key whose start times are
further apart than a therapy night plausibly spans -- so the assumption fails
loudly against real data in T-CP3 rather than silently producing a merged
night.

Following Amendment 13's convention, the id is the first eight bytes of a
SHA-256 read as a signed two's-complement integer, so it fits a PostgreSQL
`bigint` without a second representation.

## seq

Milliseconds from the session start to the sample's **raw device time**.

The obvious alternative -- a running sample ordinal -- breaks as soon as two
files in one session carry the same type: `MaskPres` appears in both `BRP` and
`PLD` at different rates, and two independent ordinals would collide at
`seq = 0, 1, 2 …` while describing different instants. A time-derived ordinal
collides only for samples of the same type at the same instant, which *are* the
same measurement and *should* deduplicate.

It is also monotonic, so a range query over a session is a range query over
`seq`, and it survives a parser upgrade that changes how many samples a file
yields.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass

SOURCE_NAME = "resmed_air11_sd"

# A therapy session is one night. Two files in one date directory whose starts
# are further apart than this are evidence the one-directory-one-session
# assumption does not hold for this device.
MAX_SESSION_SPAN_S = 20 * 3600

SIGNED_64_MIN = -(1 << 63)
SIGNED_64_MAX = (1 << 63) - 1


def _signed_64(digest: bytes) -> int:
    return int.from_bytes(digest[:8], "big", signed=True)


def session_id(card_directory: str, start_date: str) -> int:
    """The session id for a file, from its directory and recording start date.

    `card_directory` is the POSIX-relative directory the mirror recorded, e.g.
    `DATALOG/20260101`. `start_date` is the EDF header's own `dd.mm.yy` field.
    Neither changes for a given recording.
    """
    if not isinstance(card_directory, str) or not isinstance(start_date, str):
        raise TypeError("card_directory and start_date must be text")
    material = f"{SOURCE_NAME}/session/{card_directory}/{start_date}".encode()
    return _signed_64(hashlib.sha256(material).digest())


def sample_seq(session_start_epoch_s: int, raw_epoch_s: int, offset_within_second: float) -> int:
    """Milliseconds from the session start to this sample's raw device time.

    `offset_within_second` is the sample's position inside its data record, in
    seconds -- for a 25 Hz signal, sample 7 of a one-second record is 0.28.
    Rounding is to the nearest millisecond, which is finer than any ResMed
    channel and coarse enough that float accumulation cannot reorder samples.
    """
    if offset_within_second < 0:
        raise ValueError(f"offset within second rejected: {offset_within_second}; must be >= 0")
    delta_s = (raw_epoch_s - session_start_epoch_s) + offset_within_second
    seq = round(delta_s * 1000)
    if not SIGNED_64_MIN <= seq <= SIGNED_64_MAX:
        raise ValueError(f"seq rejected: {seq} does not fit a signed 64-bit integer")
    return seq


@dataclass(frozen=True)
class SessionFile:
    """One mirrored EDF file's contribution to a session."""

    card_path: str
    card_directory: str
    start_date: str
    start_epoch_s: int

    @property
    def session_id(self) -> int:
        return session_id(self.card_directory, self.start_date)


def session_conflicts(files: list[SessionFile]) -> list[str]:
    """Detect files that share a session key but cannot be one session.

    Returns a description per conflicting group; empty means the
    one-directory-one-session assumption holds for this input. This is the
    falsification check for the assumption documented at the top of this module,
    and T-CP3 is expected to run it against real card data before trusting the
    grouping.
    """
    grouped: dict[int, list[SessionFile]] = {}
    for item in files:
        grouped.setdefault(item.session_id, []).append(item)
    problems: list[str] = []
    for key, group in sorted(grouped.items()):
        if len(group) < 2:
            continue
        starts = sorted(f.start_epoch_s for f in group)
        span = starts[-1] - starts[0]
        if span > MAX_SESSION_SPAN_S:
            names = ", ".join(sorted(f.card_path for f in group))
            problems.append(
                f"session {key} spans {span}s across {len(group)} files, exceeding "
                f"{MAX_SESSION_SPAN_S}s: {names}. One date directory appears to hold more than "
                "one therapy session, so the session grouping is wrong for this device."
            )
    return problems
