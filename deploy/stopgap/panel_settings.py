#!/usr/bin/env python3
"""Runtime settings the daemons re-read, so a threshold is not a redeploy.

WHY THIS EXISTS
---------------
Every tunable in this system is an environment variable in a systemd drop-in:
the ambient the loudness bands are derived from, which bands fire the Hue
light, how long it flashes, the cooldown, the FR24 poll interval. Changing any
of them means SSH, an editor, `daemon-reload`, and a service restart -- and
restarting the publisher drops the warning state mid-pass, which has already
cost a real alert.

Worse, most of them are not set at all. They run on the defaults compiled into
the scripts, invisible unless you read the source. There is no way to see what
the system is currently configured to do.

THE PRECEDENCE, AND WHY THIS WAY ROUND
--------------------------------------
    settings file  >  environment variable  >  code default

The environment stays the bootstrap: a daemon with no settings file behaves
exactly as it does today, so this cannot break a box where the file is missing
or unreadable. The file overrides, because it is the thing a person edits.

Read from a FILE, never from PostgreSQL. The publish loop rewrites the panel
once a second and must never block on a database -- the same rule the airframe
and route caches already follow.

UNKNOWN IS NOT A DEFAULT
------------------------
A malformed value is REJECTED and the previous value kept, with the reason
named. Silently falling back to a default would let a typo quietly disarm the
warning light, and a light that does not fire looks exactly like a quiet sky.
"""
import json
import os
from pathlib import Path

SETTINGS_PATH = Path(os.environ.get(
    "PANEL_SETTINGS_JSON", "/var/lib/panel-noise/settings.json"))

_cache = {}
_cache_mtime = None
_last_error = None

# Every tunable, its bounds, and what it does. ONE definition, used by the
# daemons to clamp-or-refuse and by the admin console both to validate a write
# and to render the form.
#
# Kept in one place deliberately. Bounds written twice drift, and this project
# has already paid for that pattern: the emitter category was interpreted in
# two places and ended up written into the wrong column in eleven rows.
#
# `label` and `note` exist so the console can explain a setting without the
# HTML restating anything, which is the other half of the same drift problem.
SCHEMA = {
    "hue_enabled": {
        "kind": "flag", "default": True,
        "label": "Light warnings armed",
        "note": "The master switch. Off means NOTHING flashes -- not a scored "
                "aircraft, not an unlocated air ambulance, not an unscored "
                "one overhead. One switch rather than three, because turning "
                "off only the obvious trigger leaves the light still firing "
                "while you believe it is disarmed.",
    },
    "ambient_dba": {
        "kind": "number", "default": 40.0, "min": 25.0, "max": 70.0,
        "label": "Assumed outdoor ambient (dBA)",
        "note": "Every loudness band is an offset from this, so it moves all "
                "of them at once. 25 is a quiet rural night, 40 low-density "
                "suburban, 60 urban. Outside 25-70 is refused as a typo.",
    },
    "hue_bands": {
        "kind": "words", "default": ["VERY LOUD", "LOUD"],
        "choices": ["VERY LOUD", "LOUD", "AUDIBLE", "FAINT", "UNKNOWN"],
        "label": "Bands that flash the light",
        "note": "An empty selection is honoured and means the light never "
                "fires on a scored aircraft.",
    },
    "hue_unlocated_kinds": {
        "kind": "words", "default": ["AIR_AMBULANCE"],
        "choices": ["AIR_AMBULANCE", "POLICE"],
        "label": "Flash for these, heard without a position",
        "note": "No geometry exists in this case, so there is no countdown. "
                "Police airframes loiter here for hours; including them means "
                "near-continuous flashing.",
    },
    "hue_lead_seconds": {
        "kind": "number", "default": 30.0, "min": 1.0, "max": 300.0,
        "label": "Warn this many seconds ahead",
        "note": "Measured to the PROJECTED HEARING TIME at the house (closest "
                "approach plus the sound's own travel time), 30 s by owner "
                "decision 2026-08-17. The entire warning budget for a Life "
                "Flight departure off the Providence pad is 34 seconds, set "
                "by geometry; a longer lead cannot be met for those.",
    },
    "display_metric": {
        "kind": "flag", "default": False,
        "label": "Show distances in metric",
        "note": "Off (the default) shows miles on the kiosk and panels; on "
                "shows kilometres. Display only -- storage and computation "
                "stay metric regardless. Aviation conventions are kept "
                "either way: altitudes in feet, speeds in knots.",
    },
    "hue_warn_unscored": {
        "kind": "flag", "default": True,
        "label": "Flash for an unscored aircraft directly overhead",
        "note": "Absence of a score is not evidence of quiet, and air "
                "ambulances are over-represented among aircraft that "
                "broadcast no category at all. Turning this off silences the "
                "case this system was built for.",
    },
    "hue_cooldown_seconds": {
        "kind": "number", "default": 30.0, "min": 0.0, "max": 3600.0,
        "label": "Minimum gap between warnings (s)",
        "note": "0 means never suppress. Tests never consume this.",
    },
}


def describe():
    """The schema, for the console to render. Never the values."""
    return SCHEMA


def validate(name, value):
    """(ok, cleaned_or_reason). Used by the write API before anything lands.

    Rejects with a reason rather than silently dropping. A console that
    appears to save a value the daemons then ignore is worse than one that
    refuses, because the operator believes the light is armed.
    """
    spec = SCHEMA.get(name)
    if spec is None:
        return False, f"{name} is not a known setting"
    if value is None:
        return True, None            # explicit clear, falls back to env/default
    if spec["kind"] == "number":
        try:
            parsed = float(value)
        except (TypeError, ValueError):
            return False, f"{name} must be a number, got {value!r}"
        if parsed < spec["min"] or parsed > spec["max"]:
            return False, (f"{name} must be between {spec['min']:g} and "
                           f"{spec['max']:g}, got {parsed:g}")
        return True, parsed
    if spec["kind"] == "flag":
        if isinstance(value, bool):
            return True, value
        text_value = str(value).strip().lower()
        if text_value in ("1", "true", "yes", "on"):
            return True, True
        if text_value in ("0", "false", "no", "off"):
            return True, False
        return False, f"{name} must be true or false, got {value!r}"
    if spec["kind"] == "words":
        items = value if isinstance(value, (list, tuple)) else str(value).split(",")
        cleaned = [str(v).strip() for v in items if str(v).strip()]
        unknown = [v for v in cleaned if v not in spec["choices"]]
        if unknown:
            return False, f"{name} does not accept {', '.join(unknown)}"
        return True, cleaned
    return False, f"{name} has no validator"


def _load():
    """Re-read only when the file changes. Cheap enough for a 1 Hz loop."""
    global _cache, _cache_mtime, _last_error
    try:
        mtime = SETTINGS_PATH.stat().st_mtime
    except OSError:
        # No file is not an error: it means nothing has been overridden.
        return _cache
    if mtime == _cache_mtime:
        return _cache
    try:
        with SETTINGS_PATH.open() as stream:
            loaded = json.load(stream)
    except (OSError, ValueError) as exc:
        # Keep whatever we had. A half-written file during a save must not
        # blank the configuration of a running warning system.
        _last_error = f"{SETTINGS_PATH}: {exc}"
        return _cache
    if not isinstance(loaded, dict):
        _last_error = f"{SETTINGS_PATH}: top level is not an object"
        return _cache
    values = loaded.get("settings")
    if not isinstance(values, dict):
        _last_error = f"{SETTINGS_PATH}: no 'settings' object"
        return _cache
    _cache, _cache_mtime, _last_error = values, mtime, None
    return _cache


def last_error():
    """Why the settings file was ignored, or None. Guards must say why.

    Loads first. Reporting the stale global would answer "no error" to a
    caller that has not read anything yet, which is precisely the question
    this is asked to settle -- an admin console showing a green tick because
    it had not looked would be worse than no indicator at all.
    """
    _load()
    return _last_error


def _raw(name, env):
    """Settings file first, then the environment. None if neither has it."""
    values = _load()
    if name in values and values[name] is not None:
        return values[name]
    return os.environ.get(env)


def number(name, env, default, minimum=None, maximum=None):
    """A float setting, clamped to a sane range or rejected.

    Out of range is REJECTED, not clamped: a clamped value is
    indistinguishable downstream from one that was asked for, and this project
    does not clamp readings for exactly that reason. The same logic applies to
    a setting that would disable a warning.
    """
    value = _raw(name, env)
    if value is None:
        return default
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return default
    if minimum is not None and parsed < minimum:
        return default
    if maximum is not None and parsed > maximum:
        return default
    return parsed


def integer(name, env, default, minimum=None, maximum=None):
    value = number(name, env, None, minimum, maximum)
    return default if value is None else int(value)


def flag(name, env, default):
    value = _raw(name, env)
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    return str(value).strip().lower() not in ("0", "false", "no", "off", "")


def words(name, env, default):
    """A set of tokens, from a list or a comma-separated string.

    An empty result is returned as-is rather than replaced by the default:
    "fire on no bands at all" is a legitimate thing to ask for, and silently
    restoring the default would re-arm a light someone deliberately disarmed.
    """
    value = _raw(name, env)
    if value is None:
        return default
    if isinstance(value, (list, tuple)):
        items = [str(v).strip() for v in value]
    else:
        items = [v.strip() for v in str(value).split(",")]
    return frozenset(v for v in items if v)


def text(name, env, default):
    value = _raw(name, env)
    return default if value is None else str(value)


def snapshot():
    """Everything currently overridden, for the admin console to display."""
    return {
        "path": str(SETTINGS_PATH),
        "present": SETTINGS_PATH.exists(),
        "error": _last_error,
        "settings": dict(_load()),
    }
