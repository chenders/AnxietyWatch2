"""ResMed EDF signal labels and units, mapped to the canonical registry.

Two rules from `AGENTS.md` shape this file:

* **One canonical unit per type, converted exactly once, at the decoder.** That
  conversion happens here and nowhere else. Nothing downstream rescales.
* **Never clamp an out-of-range value. Reject it, name the reason, quarantine
  it.** A clamped reading is indistinguishable from a real one downstream.

The unit is taken from the EDF signal header rather than assumed per source,
which is the opposite of the BLE path. `[source.resmed_air11]` in the registry
carries `conversion = { leak = 60.0, tidal_volume = 1000.0 }` because that wire
format states no units at all, so a fixed factor is the only option. EDF states
a physical dimension for every signal, so the file is the authority and an
unrecognised dimension is a rejection -- never a guess. The failure mode avoided
is specific: if ResMed ever writes leak in L/min where it used to write L/s, a
fixed 60x factor would turn a 24 L/min leak into 1440 and nothing would notice,
because a mis-scaled leak reading looks exactly like a real one.

Label and dimension spellings come from the file formats, which the proposal
permits OSCAR documentation to inform. No OSCAR implementation code was read.
"""

from __future__ import annotations

import re

# Canonical units, copied here only as a readable assertion of what the registry
# says. `vitals/registry.toml` remains the source of truth; `check_units`
# compares the two so this table can never drift away from it silently.
CANONICAL_UNIT = {
    "patient_flow": "L/s",
    "mask_pressure": "cmH2O",
    "inspiratory_pressure": "cmH2O",
    "expiratory_pressure": "cmH2O",
    "leak": "L/min",
    "tidal_volume": "mL",
    "minute_ventilation": "L/min",
    "respiratory_rate": "breaths/min",
    "ie_ratio": "ratio",
    "snore_index": "index",
    "flow_limitation": "ratio",
    "spo2": "percent",
    "hr": "bpm",
}

# EDF label (normalised) -> registry type name.
#
# Unmapped labels are not errors and not silently dropped: `map_signal` returns
# None and the caller reports the label, so a firmware change that renames a
# channel surfaces as a named observation instead of a quietly shorter night.
# Ti, Te, TgMV and Resp Event are deliberately absent -- no registry type for
# them exists yet, and inventing one here would put physiology in the wrong
# file.
LABEL_TO_TYPE = {
    "flow": "patient_flow",
    "patientflow": "patient_flow",
    "maskpres": "mask_pressure",
    "maskpressure": "mask_pressure",
    "press": "inspiratory_pressure",
    "ipap": "inspiratory_pressure",
    "epap": "expiratory_pressure",
    "leak": "leak",
    "rr": "respiratory_rate",
    "resprate": "respiratory_rate",
    "vt": "tidal_volume",
    "tidalvolume": "tidal_volume",
    "mv": "minute_ventilation",
    "minutevent": "minute_ventilation",
    "snore": "snore_index",
    "snoreindex": "snore_index",
    "ffl": "flow_limitation",
    "flowlimit": "flow_limitation",
    "ie": "ie_ratio",
    "spo2": "spo2",
    "pulse": "hr",
}

# registry type -> {normalised EDF dimension: multiplier onto the canonical unit}
#
# Only spellings ResMed is documented to write are listed. A pressure in hPa
# would be a legitimate physical quantity and is still rejected, because the
# cmH2O-per-hPa factor would be a number this project had not verified against
# a device -- and an unverified conversion on a therapy pressure is worse than a
# refusal that names itself.
DIMENSION_FACTOR: dict[str, dict[str, float]] = {
    "patient_flow": {"l/s": 1.0, "ml/s": 0.001},
    "mask_pressure": {"cmh2o": 1.0},
    "inspiratory_pressure": {"cmh2o": 1.0},
    "expiratory_pressure": {"cmh2o": 1.0},
    # L/s -> L/min is the one conversion the BLE source also needs, and it is
    # the one that silently scales by 60 if guessed.
    "leak": {"l/min": 1.0, "l/s": 60.0},
    "tidal_volume": {"ml": 1.0, "l": 1000.0},
    "minute_ventilation": {"l/min": 1.0, "l/s": 60.0},
    "respiratory_rate": {"breaths/min": 1.0, "bpm": 1.0, "1/min": 1.0, "min-1": 1.0, "/min": 1.0},
    # A dimensionless ratio or index: ResMed writes an empty dimension, and
    # ":1" appears for I:E in some files.
    "ie_ratio": {"": 1.0, "ratio": 1.0, ":1": 1.0},
    "snore_index": {"": 1.0, "index": 1.0},
    "flow_limitation": {"": 1.0, "ratio": 1.0},
    "spo2": {"%": 1.0, "percent": 1.0},
    "hr": {"bpm": 1.0, "1/min": 1.0, "min-1": 1.0, "/min": 1.0},
}

_NON_ALNUM = re.compile(r"[^a-z0-9]+")


def normalise_label(label: str) -> str:
    """Fold an EDF label to a lookup key.

    ResMed writes the same channel as "Mask Pres" in one file and "MaskPres" in
    another, so spacing, case and punctuation are all removed. "I:E" becomes
    "ie" for the same reason.
    """
    return _NON_ALNUM.sub("", label.lower())


def normalise_dimension(dimension: str) -> str:
    """Fold a physical dimension for lookup, keeping the characters that matter.

    Unlike labels, `/` and `-` are significant here -- "l/s" and "l/min" differ
    by exactly the 60x factor this table exists to get right -- so only case and
    whitespace are folded.
    """
    return "".join(dimension.lower().split())


def map_signal(label: str) -> str | None:
    """The registry type for an EDF label, or None if this label is not ingested."""
    return LABEL_TO_TYPE.get(normalise_label(label))


def conversion_factor(type_name: str, dimension: str) -> float:
    """The multiplier taking `dimension` onto the registry's canonical unit.

    Raises `ValueError` naming the dimension and the accepted set. The caller
    turns that into a rejection; it must never fall back to 1.0, because a
    silent identity conversion is how a 60x error ships.
    """
    accepted = DIMENSION_FACTOR.get(type_name)
    if accepted is None:
        raise ValueError(f"type {type_name!r} has no declared dimensions")
    key = normalise_dimension(dimension)
    if key not in accepted:
        raise ValueError(
            f"dimension {dimension!r} rejected for {type_name}: expected one of "
            f"{sorted(accepted)} to convert onto {CANONICAL_UNIT[type_name]}"
        )
    return accepted[key]


def check_units(registry_units: dict[str, str]) -> list[str]:
    """Compare `CANONICAL_UNIT` against the registry's own units.

    Returns a list of mismatch descriptions; empty means they agree. A test
    calls this with units parsed from `vitals/registry.toml`, so the table above
    cannot drift from the source of truth without something going red.
    """
    problems = []
    for type_name, unit in sorted(CANONICAL_UNIT.items()):
        actual = registry_units.get(type_name)
        if actual is None:
            problems.append(f"{type_name}: absent from the registry")
        elif actual != unit:
            problems.append(f"{type_name}: registry says {actual!r}, this table says {unit!r}")
    for type_name in sorted(set(CANONICAL_UNIT) - set(DIMENSION_FACTOR)):
        problems.append(f"{type_name}: no accepted EDF dimensions declared")
    for type_name in sorted(set(LABEL_TO_TYPE.values()) - set(CANONICAL_UNIT)):
        problems.append(f"{type_name}: mapped from a label but has no canonical unit")
    return problems
