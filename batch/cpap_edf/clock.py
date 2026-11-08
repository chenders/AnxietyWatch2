"""Versioned device-clock interpretation for CPAP samples.

The design invariant this implements:

> **Clock correction is metadata.** Store raw device time and a versioned
> transform plus derivation evidence. Apply the transform at read time;
> changing it appends a version and never updates samples.

The point is not that the offset is known -- it is not, and the identity
transform below says so explicitly rather than pretending otherwise. The point
is that the *shape* is fixed now, before any sample is stored, so correcting
the clock later is an append rather than a migration.

Why it cannot be applied at write time: the AirSense has no network time
source, so its clock drifts and is occasionally reset by hand. If a correction
were baked into stored timestamps, then discovering six months later that the
device was 94 seconds fast would require rewriting every historical row -- and
`samples` is append-only, so that rewrite is not available. Worse, there would
be no way to tell a corrected row from an uncorrected one, which is the
unknown-rendered-as-known failure the project forbids.

Storing the raw reading plus a transform id keeps both answers available: the
raw device time is evidence of what the device said, and any version of the
correction can be applied over it at read time.
"""

from __future__ import annotations

from dataclasses import dataclass

# Bumped whenever a transform's *meaning* changes. Stored alongside every
# sample so a reader knows which rule produced a corrected timestamp.
TRANSFORM_SCHEMA_VERSION = 1


@dataclass(frozen=True)
class ClockTransform:
    """One version of the device-clock correction.

    `offset_s` is a constant shift and `drift_ppm` a linear rate relative to
    `anchor_epoch_s`, so a correction derived from two observations can be
    expressed without storing a curve. `derived_from` is prose evidence -- how
    the numbers were obtained -- because a transform whose derivation is not
    recorded cannot be audited or superseded with confidence.
    """

    version: int
    offset_s: float
    drift_ppm: float
    anchor_epoch_s: int
    derived_from: str

    def apply(self, raw_epoch_s: float) -> float:
        """Corrected UTC epoch seconds for a raw device reading.

        Pure and total: the same raw input always gives the same corrected
        output for a given transform version, which is what makes the correction
        reproducible at read time rather than a one-way edit.
        """
        elapsed = raw_epoch_s - self.anchor_epoch_s
        return raw_epoch_s + self.offset_s + elapsed * (self.drift_ppm / 1_000_000.0)

    def describe(self) -> str:
        return (
            f"v{self.version}: offset {self.offset_s:+g}s, drift {self.drift_ppm:+g}ppm from "
            f"anchor {self.anchor_epoch_s} ({self.derived_from})"
        )


# The transform in force until a measurement replaces it. It is the identity,
# and it says so: no correction has been derived, because doing that needs the
# device and a trusted reference side by side, which is T-CP6's physical gate.
#
# This is deliberately not an absent transform. A stored sample must always name
# the version that interpreted it, so "uncorrected" is version 1 rather than
# null -- otherwise a later reader cannot distinguish "no correction was needed"
# from "no correction was recorded".
IDENTITY_TRANSFORM = ClockTransform(
    version=1,
    offset_s=0.0,
    drift_ppm=0.0,
    anchor_epoch_s=0,
    derived_from=(
        "no correction derived; the AirSense has no network time source and no "
        "device-versus-reference observation has been taken. Raw device time is stored as read."
    ),
)


def transform_metadata(transform: ClockTransform) -> dict[str, object]:
    """The transform as a JSON-serialisable record to store beside samples.

    Flat and explicit so it can go into a provenance column without a schema
    migration when a field is added.
    """
    return {
        "schema_version": TRANSFORM_SCHEMA_VERSION,
        "transform_version": transform.version,
        "offset_s": transform.offset_s,
        "drift_ppm": transform.drift_ppm,
        "anchor_epoch_s": transform.anchor_epoch_s,
        "derived_from": transform.derived_from,
    }
