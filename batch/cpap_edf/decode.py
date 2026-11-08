"""Decode one mirrored EDF file into canonical, range-checked samples.

No database, no SQL, no network. The output is plain data for T-CP3 to insert
transactionally; this module's contract is that everything it emits is already
in the registry's canonical unit, inside the registry's range, and carries the
identity and clock metadata frozen by `identity.py` and `clock.py`.

The three-state rule from `AGENTS.md` governs the return shape:
measured-and-good, measured-and-bad, and **not measured**. So a decode returns
samples *and* rejections, and a rejection is never an absent sample -- it is a
named record of something the file contained that could not be trusted. A
caller that reports only the sample count would be reporting
unknown-as-nothing.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from . import channels, edf
from .clock import IDENTITY_TRANSFORM, ClockTransform
from .identity import SOURCE_NAME, sample_seq, session_id


@dataclass(frozen=True)
class Sample:
    """One canonical reading, ready for append-only insertion."""

    source: str
    type_name: str
    session_id: int
    seq: int
    # Raw device time, never corrected in place. The transform travels beside it.
    raw_epoch_s: int
    offset_within_second: float
    value: float
    unit: str

    def corrected_epoch_s(self, transform: ClockTransform) -> float:
        """Apply a clock transform at read time, as the invariant requires."""
        return transform.apply(self.raw_epoch_s + self.offset_within_second)


@dataclass
class DecodeResult:
    samples: list[Sample] = field(default_factory=list)
    rejections: list[str] = field(default_factory=list)
    # Labels present in the file that this project does not ingest. Reported
    # rather than dropped, so a renamed channel is visible instead of simply
    # yielding a shorter night.
    unmapped_labels: list[str] = field(default_factory=list)
    session_id: int | None = None
    start_epoch_s: int | None = None
    transform_version: int = IDENTITY_TRANSFORM.version

    def reject(self, reason: str) -> None:
        self.rejections.append(reason)

    @property
    def counts(self) -> dict[str, int]:
        by_type: dict[str, int] = {}
        for sample in self.samples:
            by_type[sample.type_name] = by_type.get(sample.type_name, 0) + 1
        return by_type


def _range_for(registry_ranges: dict[str, tuple[float, float]], type_name: str) -> tuple[float, float]:
    bounds = registry_ranges.get(type_name)
    if bounds is None:
        # Refusing is the only safe option: without the registry's range there
        # is nothing to check a value against, and emitting it unchecked would
        # put an unvalidated physiological number into an append-only store.
        raise ValueError(
            f"type {type_name!r} has no registry range; refusing to emit unvalidated samples"
        )
    return bounds


def decode_file(
    data: bytes,
    *,
    card_path: str,
    card_directory: str,
    registry_ranges: dict[str, tuple[float, float]],
    max_samples_per_type: int | None = None,
) -> DecodeResult:
    """Decode one EDF file's bytes.

    `registry_ranges` comes from `vitals/registry.toml` -- passed in rather than
    read here, so this module does no I/O and the caller cannot be unaware that
    the ranges are the registry's and not this file's invention.
    """
    result = DecodeResult()
    try:
        header = edf.read_header(data)
    except edf.EdfError as exc:
        result.reject(f"{card_path}: {exc}")
        return result

    try:
        start_epoch = edf.start_epoch_seconds(header)
    except edf.EdfError as exc:
        result.reject(f"{card_path}: {exc}")
        return result

    result.start_epoch_s = start_epoch
    result.session_id = session_id(card_directory, header.start_date)

    complete = edf.complete_record_count(data, header)
    if complete < header.record_count:
        # The earlier records are real evidence and are kept. The shortfall is
        # named rather than silently losing the tail, because a night that
        # decodes 7 hours out of 8 and says nothing is indistinguishable from a
        # 7-hour night.
        missing = header.record_count - complete
        result.reject(
            f"{card_path}: truncated after {complete} of {header.record_count} data records; "
            f"{missing} record(s) are missing from the file and contribute nothing"
        )

    for index, signal in enumerate(header.signals):
        type_name = channels.map_signal(signal.label)
        if type_name is None:
            result.unmapped_labels.append(signal.label)
            continue
        try:
            factor = channels.conversion_factor(type_name, signal.dimension)
        except ValueError as exc:
            result.reject(f"{card_path} signal {index} ({signal.label!r}): {exc}")
            continue
        try:
            low, high = _range_for(registry_ranges, type_name)
        except ValueError as exc:
            result.reject(f"{card_path} signal {index} ({signal.label!r}): {exc}")
            continue

        try:
            digital = edf.read_signal_samples(data, header, index)
        except edf.EdfError as exc:
            result.reject(f"{card_path} signal {index} ({signal.label!r}): {exc}")
            continue

        period = header.record_duration_s / signal.samples_per_record
        out_of_range = 0
        first_bad: str | None = None
        emitted = 0
        for ordinal, raw in enumerate(digital):
            if max_samples_per_type is not None and emitted >= max_samples_per_type:
                result.reject(
                    f"{card_path} signal {index} ({signal.label!r}): stopped after "
                    f"{emitted} samples at the caller's limit; the remainder was not decoded"
                )
                break
            physical = signal.scale(raw) * factor
            if physical != physical or physical in (float("inf"), float("-inf")):
                out_of_range += 1
                if first_bad is None:
                    first_bad = f"digital {raw} scaled to a non-finite value"
                continue
            if not low <= physical <= high:
                # Never clamped. A clamped reading is indistinguishable from a
                # real one downstream, so it contributes nothing and says why.
                out_of_range += 1
                if first_bad is None:
                    first_bad = (
                        f"digital {raw} -> {physical:g} {channels.CANONICAL_UNIT[type_name]} "
                        f"outside the registry range [{low:g}, {high:g}]"
                    )
                continue
            record = ordinal // signal.samples_per_record
            position = ordinal % signal.samples_per_record
            within = record * header.record_duration_s + position * period
            result.samples.append(
                Sample(
                    source=SOURCE_NAME,
                    type_name=type_name,
                    session_id=result.session_id,
                    seq=sample_seq(start_epoch, start_epoch, within),
                    raw_epoch_s=start_epoch,
                    offset_within_second=within,
                    value=physical,
                    unit=channels.CANONICAL_UNIT[type_name],
                )
            )
            emitted += 1
        if out_of_range:
            result.reject(
                f"{card_path} signal {index} ({signal.label!r} -> {type_name}): "
                f"{out_of_range} of {len(digital)} samples contributed nothing; first was "
                f"{first_bad}"
            )
    return result
