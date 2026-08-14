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
