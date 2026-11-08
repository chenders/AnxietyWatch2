"""A clean-room EDF/EDF+ reader for mirrored ResMed SD-card files.

Written from the published European Data Format specification, not from any
existing implementation. No OSCAR or hms-cpap code was read or copied; see
NOTICE.

The input is bytes mirrored from the ez Share card, which `batch/cpap_ezshare`
treats as an adversary from end to end. That property has to survive into this
module or it is worthless: a reader that trusts a declared signal count, a
declared record count, or a declared sample-per-record count will happily be
told to allocate a terabyte or read past the end of the buffer.

So every field that sizes a later read is bounded before it is used, and every
read is checked against the actual buffer length rather than against the
header's claim about it. There is no C extension in this path on purpose —
handing hostile bytes to one would reverse the guarantee the mirror exists to
provide.

The layout, which is fixed and fully specified:

    offset  size            field
    0       8               version, "0" followed by spaces
    8       80              patient identification
    88      80              recording identification
    168     8               start date, dd.mm.yy
    176     8               start time, hh.mm.ss
    184     8               bytes in the header record
    192     44              reserved
    236     8               number of data records, -1 if unknown
    244     8               duration of one data record, seconds
    252     4               number of signals, ns

then, for ns signals, each field repeated ns times in this order: label (16),
transducer type (80), physical dimension (8), physical minimum (8), physical
maximum (8), digital minimum (8), digital maximum (8), prefiltering (80),
samples per data record (8), reserved (32). The header is therefore exactly
256 + ns * 256 bytes, and the specification requires the declared header size
to say so.

Data records follow: for each record, for each signal, `samples_per_record`
little-endian signed 16-bit integers. A digital sample is scaled to its
physical value by

    physical = (digital - digital_min) * (physical_max - physical_min)
               / (digital_max - digital_min) + physical_min
"""

from __future__ import annotations

import re
from dataclasses import dataclass

HEADER_BYTES = 256
SIGNAL_HEADER_BYTES = 256
BYTES_PER_SAMPLE = 2

# Bounds on declared sizes. These are not EDF limits; they are this project's
# limits, chosen so a hostile header cannot turn into an allocation. A ResMed
# BRP/PLD file carries single digits of signals, and a night at 25 Hz is on the
# order of 10^6 samples per signal.
MAX_SIGNALS = 256
MAX_SAMPLES_PER_RECORD = 1 << 16
MAX_RECORDS = 1 << 22
MAX_TOTAL_SAMPLES = 1 << 26

_DATE = re.compile(r"\A(\d{2})\.(\d{2})\.(\d{2})\Z")
_TIME = re.compile(r"\A(\d{2})\.(\d{2})\.(\d{2})\Z")


class EdfError(Exception):
    """A rejected EDF file or field. The message names the value and the reason."""


@dataclass(frozen=True)
class Signal:
    index: int
    label: str
    transducer: str
    dimension: str
    physical_min: float
    physical_max: float
    digital_min: int
    digital_max: int
    prefiltering: str
    samples_per_record: int

    def scale(self, digital: int) -> float:
        """Convert one digital sample to its physical value.

        Pre-validated by `_signal`, so the denominator cannot be zero here.
        """
        span_physical = self.physical_max - self.physical_min
        span_digital = self.digital_max - self.digital_min
        return (digital - self.digital_min) * span_physical / span_digital + self.physical_min


@dataclass(frozen=True)
class EdfHeader:
    version: str
    patient: str
    recording: str
    start_date: str
    start_time: str
    header_bytes: int
    reserved: str
    record_count: int
    record_duration_s: float
    signals: tuple[Signal, ...]

    @property
    def total_samples_per_record(self) -> int:
        return sum(s.samples_per_record for s in self.signals)


def _field(data: bytes, offset: int, size: int, name: str) -> str:
    end = offset + size
    if end > len(data):
        raise EdfError(
            f"{name} rejected: needs bytes {offset}..{end} but the file is {len(data)} bytes"
        )
    raw = data[offset:end]
    try:
        # EDF is defined as printable US-ASCII. Decoding strictly means a file
        # with high bytes in its text fields is rejected rather than silently
        # mangled into replacement characters.
        return raw.decode("ascii", "strict").strip()
    except UnicodeDecodeError as exc:
        raise EdfError(f"{name} rejected: not printable ASCII at byte {offset + exc.start}") from exc


def _int_field(data: bytes, offset: int, size: int, name: str) -> int:
    text = _field(data, offset, size, name)
    if not text:
        raise EdfError(f"{name} rejected: empty")
    try:
        return int(text)
    except ValueError as exc:
        raise EdfError(f"{name} rejected: {text!r} is not an integer") from exc


def _float_field(data: bytes, offset: int, size: int, name: str) -> float:
    text = _field(data, offset, size, name)
    if not text:
        raise EdfError(f"{name} rejected: empty")
    try:
        value = float(text)
    except ValueError as exc:
        raise EdfError(f"{name} rejected: {text!r} is not a number") from exc
    # NaN and the infinities are parseable floats and would poison every sample
    # scaled through them.
    if value != value or value in (float("inf"), float("-inf")):
        raise EdfError(f"{name} rejected: {text!r} is not finite")
    return value


def _signal(data: bytes, ns: int, index: int) -> Signal:
    # Each field is stored as ns consecutive copies, so signal `index`'s copy of
    # a field sits at (end of the header) + (bytes of all earlier field blocks,
    # each ns wide) + index * (this field's width). `block` is that cumulative
    # offset within one signal's worth of fields.
    def at(block: int, size: int, name: str) -> tuple[bytes, int, int, str]:
        return data, HEADER_BYTES + block * ns + index * size, size, f"signal {index} {name}"

    label = _field(*at(0, 16, "label"))
    transducer = _field(*at(16, 80, "transducer"))
    dimension = _field(*at(96, 8, "physical dimension"))
    physical_min = _float_field(*at(104, 8, "physical minimum"))
    physical_max = _float_field(*at(112, 8, "physical maximum"))
    digital_min = _int_field(*at(120, 8, "digital minimum"))
    digital_max = _int_field(*at(128, 8, "digital maximum"))
    prefiltering = _field(*at(136, 80, "prefiltering"))
    samples_per_record = _int_field(*at(216, 8, "samples per record"))

    if digital_min == digital_max:
        raise EdfError(
            f"signal {index} ({label!r}) rejected: digital minimum and maximum are both "
            f"{digital_min}; scaling would divide by zero"
        )
    if digital_min > digital_max:
        raise EdfError(
            f"signal {index} ({label!r}) rejected: digital minimum {digital_min} exceeds maximum "
            f"{digital_max}"
        )
    # The specification requires these to differ. Equal bounds would decode every
    # sample to the same constant, which is a broken file rather than a flat
    # recording, and silently emitting a night of identical values would read as
    # measured data.
    if physical_min == physical_max:
        raise EdfError(
            f"signal {index} ({label!r}) rejected: physical minimum and maximum are both "
            f"{physical_min}; every sample would decode to a constant"
        )
    if not 0 < samples_per_record <= MAX_SAMPLES_PER_RECORD:
        raise EdfError(
            f"signal {index} ({label!r}) rejected: samples per record {samples_per_record} outside "
            f"1..{MAX_SAMPLES_PER_RECORD}"
        )
    # A 16-bit sample cannot represent a digital range wider than this, so a
    # header claiming one is describing a file that cannot exist.
    if digital_min < -32768 or digital_max > 32767:
        raise EdfError(
            f"signal {index} ({label!r}) rejected: digital range {digital_min}..{digital_max} does "
            "not fit a signed 16-bit sample"
        )
    return Signal(
        index=index,
        label=label,
        transducer=transducer,
        dimension=dimension,
        physical_min=physical_min,
        physical_max=physical_max,
        digital_min=digital_min,
        digital_max=digital_max,
        prefiltering=prefiltering,
        samples_per_record=samples_per_record,
    )


def read_header(data: bytes) -> EdfHeader:
    if len(data) < HEADER_BYTES:
        raise EdfError(f"file rejected: {len(data)} bytes is shorter than the {HEADER_BYTES}-byte header")

    version = _field(data, 0, 8, "version")
    # EDF's version field is "0"; EDF+ keeps it and marks itself in the reserved
    # field. Anything else is a format this reader has not been shown.
    if version != "0":
        raise EdfError(f"version rejected: {version!r}; only EDF version '0' is supported")

    ns = _int_field(data, 252, 4, "number of signals")
    if not 0 < ns <= MAX_SIGNALS:
        raise EdfError(f"number of signals rejected: {ns} outside 1..{MAX_SIGNALS}")

    declared_header = _int_field(data, 184, 8, "header bytes")
    expected_header = HEADER_BYTES + ns * SIGNAL_HEADER_BYTES
    if declared_header != expected_header:
        raise EdfError(
            f"header bytes rejected: declared {declared_header} but {ns} signals require "
            f"{expected_header}"
        )
    if len(data) < expected_header:
        raise EdfError(
            f"file rejected: header needs {expected_header} bytes but the file is {len(data)} bytes"
        )

    duration = _float_field(data, 244, 8, "record duration")
    if duration <= 0:
        raise EdfError(f"record duration rejected: {duration}; must be > 0 seconds")

    signals = tuple(_signal(data, ns, index) for index in range(ns))
    per_record = sum(s.samples_per_record for s in signals)
    if per_record <= 0:
        raise EdfError("record layout rejected: no samples in a data record")

    record_count = _int_field(data, 236, 8, "number of data records")
    available = len(data) - expected_header
    complete = available // (per_record * BYTES_PER_SAMPLE)
    if record_count == -1:
        # Permitted by the specification for a recording still in progress. The
        # file's own length is then the only evidence of how much is there.
        record_count = complete
    elif record_count < 0:
        raise EdfError(f"number of data records rejected: {record_count}")
    if record_count > MAX_RECORDS:
        raise EdfError(f"number of data records rejected: {record_count} exceeds {MAX_RECORDS}")
    if record_count * per_record > MAX_TOTAL_SAMPLES:
        raise EdfError(
            f"record layout rejected: {record_count} records x {per_record} samples exceeds "
            f"{MAX_TOTAL_SAMPLES}"
        )

    return EdfHeader(
        version=version,
        patient=_field(data, 8, 80, "patient identification"),
        recording=_field(data, 88, 80, "recording identification"),
        start_date=_field(data, 168, 8, "start date"),
        start_time=_field(data, 176, 8, "start time"),
        header_bytes=expected_header,
        reserved=_field(data, 192, 44, "reserved"),
        record_count=record_count,
        record_duration_s=duration,
        signals=signals,
    )


def start_epoch_seconds(header: EdfHeader) -> int:
    """The recording start as UTC epoch seconds, from the header's own fields.

    EDF's two-digit year is defined by the specification as 1985..2084: 85..99
    mean 1985..1999 and 00..84 mean 2000..2084. Guessing differently would move
    a recording by a century.

    The header carries no timezone, and this returns epoch seconds as if the
    stamp were UTC. That is a *raw* device reading, not a corrected one — the
    device's offset and drift are the versioned transform in `clock.py`, applied
    at read time and never by rewriting this value.
    """
    date_match = _DATE.match(header.start_date)
    if date_match is None:
        raise EdfError(f"start date rejected: {header.start_date!r}; expected dd.mm.yy")
    time_match = _TIME.match(header.start_time)
    if time_match is None:
        raise EdfError(f"start time rejected: {header.start_time!r}; expected hh.mm.ss")
    day, month, two_digit_year = (int(part) for part in date_match.groups())
    hour, minute, second = (int(part) for part in time_match.groups())
    year = 1900 + two_digit_year if two_digit_year >= 85 else 2000 + two_digit_year
    if not 1 <= month <= 12 or not 1 <= day <= 31:
        raise EdfError(f"start date rejected: {header.start_date!r}; day or month out of range")
    if hour > 23 or minute > 59 or second > 60:
        raise EdfError(f"start time rejected: {header.start_time!r}; out of range")

    # calendar.timegm without importing calendar's locale machinery: days since
    # the epoch via the civil-from-days algorithm, which is exact for all inputs
    # and needs no library.
    shifted_year = year - (1 if month <= 2 else 0)
    era = (shifted_year if shifted_year >= 0 else shifted_year - 399) // 400
    year_of_era = shifted_year - era * 400
    day_of_year = (153 * (month + (-3 if month > 2 else 9)) + 2) // 5 + day - 1
    day_of_era = year_of_era * 365 + year_of_era // 4 - year_of_era // 100 + day_of_year
    days = era * 146097 + day_of_era - 719468
    if second == 60:
        # A leap second in the stamp. EDF has no way to express one and ResMed
        # does not write one, but mapping it onto :59 is a deliberate, named
        # choice rather than an exception from the arithmetic below.
        second = 59
    return days * 86400 + hour * 3600 + minute * 60 + second


def read_signal_samples(data: bytes, header: EdfHeader, index: int) -> list[int]:
    """Every digital sample for one signal, in record order.

    Returns raw 16-bit integers. Scaling is `Signal.scale`, and unit conversion
    happens exactly once, later, in `channels.py` — never twice and never here.
    """
    if not 0 <= index < len(header.signals):
        raise EdfError(f"signal index rejected: {index} outside 0..{len(header.signals) - 1}")
    signal = header.signals[index]
    per_record = header.total_samples_per_record
    offset_in_record = sum(s.samples_per_record for s in header.signals[:index])

    samples: list[int] = []
    for record in range(header.record_count):
        start = (
            header.header_bytes
            + (record * per_record + offset_in_record) * BYTES_PER_SAMPLE
        )
        end = start + signal.samples_per_record * BYTES_PER_SAMPLE
        if end > len(data):
            # A truncated tail. Stop at the last complete record rather than
            # raising: the earlier records are real evidence, and the caller
            # reports the shortfall as a named rejection.
            break
        chunk = data[start:end]
        samples.extend(
            int.from_bytes(chunk[position:position + 2], "little", signed=True)
            for position in range(0, len(chunk), 2)
        )
    return samples


def complete_record_count(data: bytes, header: EdfHeader) -> int:
    """How many whole data records the buffer actually holds."""
    available = max(0, len(data) - header.header_bytes)
    return available // (header.total_samples_per_record * BYTES_PER_SAMPLE)
