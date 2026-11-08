"""Deterministic synthetic EDF builders for tests.

Committed because the fixtures are golden: the same inputs must produce the
same bytes on every machine and in CI, so a decoding change shows up as a test
failure rather than as a regenerated blob nobody diffed.

**Nothing here is a recording of a person.** Every value is chosen by the
caller, and the defaults are round numbers picked so expected physical values
can be worked out by hand from the EDF scaling formula rather than from this
project's own reader — which is the only way a parser test proves anything. A
real patient recording must never be committed, and `make_edf` deliberately
offers no way to read one in.

The scaling choice worth knowing: with `digital_min=-32768`,
`digital_max=32767`, `physical_min=-P` and `physical_max=+P`, a digital sample
of `-32768` is exactly `-P`, `32767` is exactly `+P`, and `0` is
`P / 65535`, which is not zero. Tests that want an exact midpoint should use
asymmetric digital bounds instead; the helpers below take all four explicitly
so no test has to guess.
"""

from __future__ import annotations

from dataclasses import dataclass, field


def _ascii(value: str, width: int, name: str) -> bytes:
    text = str(value)
    if len(text) > width:
        raise ValueError(f"{name} does not fit {width} bytes: {text!r}")
    if not text.isascii():
        raise ValueError(f"{name} must be ASCII: {text!r}")
    return text.ljust(width).encode("ascii")


@dataclass
class SignalSpec:
    label: str
    dimension: str
    physical_min: float
    physical_max: float
    digital_min: int = -32768
    digital_max: int = 32767
    samples_per_record: int = 1
    transducer: str = ""
    prefiltering: str = ""
    # Digital samples, flattened across records. Shorter than
    # record_count * samples_per_record means the remainder is filled with
    # `fill`, which lets a test build a long file without listing every value.
    samples: list[int] = field(default_factory=list)
    fill: int = 0


def make_edf(
    signals: list[SignalSpec],
    *,
    record_count: int = 1,
    record_duration_s: float = 1.0,
    start_date: str = "01.01.26",
    start_time: str = "22.30.00",
    patient: str = "X X X X",
    recording: str = "Startdate 01-JAN-2026 X X X",
    reserved: str = "",
    version: str = "0",
    declared_record_count: int | None = None,
    declared_header_bytes: int | None = None,
    truncate_bytes: int = 0,
) -> bytes:
    """Build one EDF file.

    `declared_record_count`, `declared_header_bytes` and `truncate_bytes` exist
    so a test can produce a file whose header lies about its own contents —
    which is the interesting case for a reader handling untrusted bytes, and
    cannot be produced by a well-behaved writer.
    """
    ns = len(signals)
    header_bytes = 256 + ns * 256
    out = bytearray()
    out += _ascii(version, 8, "version")
    out += _ascii(patient, 80, "patient")
    out += _ascii(recording, 80, "recording")
    out += _ascii(start_date, 8, "start date")
    out += _ascii(start_time, 8, "start time")
    out += _ascii(
        str(header_bytes if declared_header_bytes is None else declared_header_bytes),
        8,
        "header bytes",
    )
    out += _ascii(reserved, 44, "reserved")
    out += _ascii(
        str(record_count if declared_record_count is None else declared_record_count),
        8,
        "record count",
    )
    out += _ascii(_number(record_duration_s), 8, "record duration")
    out += _ascii(str(ns), 4, "number of signals")

    for width, name, extract in (
        (16, "label", lambda s: s.label),
        (80, "transducer", lambda s: s.transducer),
        (8, "dimension", lambda s: s.dimension),
        (8, "physical min", lambda s: _number(s.physical_min)),
        (8, "physical max", lambda s: _number(s.physical_max)),
        (8, "digital min", lambda s: str(s.digital_min)),
        (8, "digital max", lambda s: str(s.digital_max)),
        (80, "prefiltering", lambda s: s.prefiltering),
        (8, "samples per record", lambda s: str(s.samples_per_record)),
        (32, "reserved", lambda s: ""),
    ):
        for spec in signals:
            out += _ascii(extract(spec), width, f"{name} for {spec.label!r}")

    if len(out) != header_bytes:
        raise AssertionError(f"built a {len(out)}-byte header, expected {header_bytes}")

    for record in range(record_count):
        for spec in signals:
            for position in range(spec.samples_per_record):
                flat = record * spec.samples_per_record + position
                value = spec.samples[flat] if flat < len(spec.samples) else spec.fill
                if not -32768 <= value <= 32767:
                    raise ValueError(f"sample {value} for {spec.label!r} is not a signed 16-bit int")
                out += int(value).to_bytes(2, "little", signed=True)

    if truncate_bytes:
        if truncate_bytes >= len(out):
            raise ValueError(f"cannot truncate {truncate_bytes} bytes from a {len(out)}-byte file")
        del out[len(out) - truncate_bytes:]
    return bytes(out)


def _number(value: float) -> str:
    """EDF numbers are ASCII in 8 characters, so they are rendered compactly.

    An integral float becomes "1" rather than "1.0", because the field is only
    eight bytes wide and "-32768.0" already spends all of them.
    """
    if value == int(value):
        return str(int(value))
    text = repr(float(value))
    if len(text) <= 8:
        return text
    trimmed = f"{value:.6f}".rstrip("0").rstrip(".")
    if len(trimmed) > 8:
        raise ValueError(f"{value!r} cannot be written in 8 ASCII bytes")
    return trimmed
