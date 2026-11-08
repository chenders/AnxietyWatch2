"""Read units and ranges from `vitals/registry.toml`.

Kept apart from `decode.py` on purpose: the decoder takes ranges as an
argument and does no I/O, so a caller cannot be unaware that the bounds came
from the registry rather than from the decoder's own opinion. This module is
the one place that opens the file.

`tomllib` is standard library from Python 3.11, so this adds no dependency.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

SD_SOURCE = "resmed_air11_sd"


class RegistryError(Exception):
    """The registry could not be read, or does not say what this package needs."""


def _load(path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except FileNotFoundError as exc:
        raise RegistryError(f"registry rejected: {path} is absent") from exc
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise RegistryError(f"registry rejected: {path} is unreadable or malformed ({exc})") from exc


def load_units(path: Path) -> dict[str, str]:
    """type name -> canonical unit."""
    data = _load(path)
    units: dict[str, str] = {}
    for name, body in (data.get("type") or {}).items():
        unit = body.get("unit")
        if not isinstance(unit, str):
            raise RegistryError(f"registry rejected: type {name!r} has no string unit")
        units[name] = unit
    if not units:
        raise RegistryError(f"registry rejected: {path} declares no types")
    return units


def load_ranges(path: Path) -> dict[str, tuple[float, float]]:
    """type name -> (low, high), the valid range the decoder enforces."""
    data = _load(path)
    ranges: dict[str, tuple[float, float]] = {}
    for name, body in (data.get("type") or {}).items():
        bounds = body.get("range")
        if not isinstance(bounds, list) or len(bounds) != 2:
            raise RegistryError(f"registry rejected: type {name!r} has no two-element range")
        low, high = bounds
        if not isinstance(low, (int, float)) or not isinstance(high, (int, float)):
            raise RegistryError(f"registry rejected: type {name!r} range is not numeric")
        if low >= high:
            raise RegistryError(
                f"registry rejected: type {name!r} range [{low}, {high}] is empty or inverted"
            )
        ranges[name] = (float(low), float(high))
    if not ranges:
        raise RegistryError(f"registry rejected: {path} declares no types")
    return ranges


def sd_source_provides(path: Path) -> tuple[int, frozenset[str]]:
    """The SD source's id and the types it declares it provides.

    Used to assert that every type this package can emit is declared on the
    source. A decoder emitting a type the registry does not list for its source
    would produce rows no downstream consumer expects.
    """
    data = _load(path)
    source = (data.get("source") or {}).get(SD_SOURCE)
    if source is None:
        raise RegistryError(f"registry rejected: no [source.{SD_SOURCE}]")
    source_id = source.get("id")
    if not isinstance(source_id, int) or isinstance(source_id, bool):
        raise RegistryError(f"registry rejected: [source.{SD_SOURCE}] has no integer id")
    provides = source.get("provides")
    if not isinstance(provides, list) or not all(isinstance(p, str) for p in provides):
        raise RegistryError(f"registry rejected: [source.{SD_SOURCE}] provides is not a string list")
    return source_id, frozenset(provides)
