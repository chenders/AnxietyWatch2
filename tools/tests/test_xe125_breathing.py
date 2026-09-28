"""Tests for the vendor-independent part of the XE125 bench runner."""

from __future__ import annotations

import json
from types import SimpleNamespace

import pytest

from tools.xe125_breathing import observation_dict, parse_args


def result(*, state: str, rate: object, presence_detected: bool = True) -> SimpleNamespace:
    """Build the small result shape consumed by ``observation_dict``."""
    return SimpleNamespace(
        app_state=SimpleNamespace(name=state),
        breathing_result=SimpleNamespace(breathing_rate=rate),
        distances_being_analyzed=(4, 7),
        presence_result=SimpleNamespace(
            presence_detected=presence_detected,
            presence_distance=0.82,
            intra_presence_score=1.5,
            inter_presence_score=6.0,
        ),
    )


def test_duration_defaults_to_ten_minutes_and_rejects_longer_runs() -> None:
    assert parse_args(["--serial-port", "/dev/ttyUSB0"]).duration_seconds == 600.0
    with pytest.raises(SystemExit):
        parse_args(["--serial-port", "/dev/ttyUSB0", "--duration-seconds", "0"])
    with pytest.raises(SystemExit):
        parse_args(["--serial-port", "/dev/ttyUSB0", "--duration-seconds", "601"])


def test_unknown_state_is_safely_coerced() -> None:
    observation = observation_dict(
        result(state="SOME_BOGUS_STATE", rate=15.0),
        observed_at_unix_micros=1_700_000_000_000_000,
    )

    assert observation["app_state"] == "UNKNOWN_STATE"
    assert observation["rate_available"] is False
    assert observation["breathing_rate_breaths_per_min"] is None


def test_estimate_contains_rate_and_presence_context() -> None:
    observation = observation_dict(
        result(state="ESTIMATE_BREATHING_RATE", rate=13.5),
        observed_at_unix_micros=1_700_000_000_000_000,
    )

    assert observation["schema"] == 1
    assert observation["observed_at_unix_micros"] == 1_700_000_000_000_000
    assert observation["rate_available"] is True
    assert observation["breathing_rate_breaths_per_min"] == 13.5
    assert observation["presence_distance_m"] == 0.82
    assert observation["range_point_indices"] == [4, 7]


def test_motion_state_suppresses_a_stale_rate() -> None:
    observation = observation_dict(
        result(state="INTRA_PRESENCE_DETECTED", rate=13.5),
        observed_at_unix_micros=1_700_000_000_000_000,
    )

    assert observation["app_state"] == "INTRA_PRESENCE_DETECTED"
    assert observation["rate_available"] is False
    assert observation["breathing_rate_breaths_per_min"] is None


def test_no_presence_clears_distance_and_rate() -> None:
    observation = observation_dict(
        result(
            state="NO_PRESENCE_DETECTED",
            rate=None,
            presence_detected=False,
        ),
        observed_at_unix_micros=1_700_000_000_000_000,
    )

    assert observation["presence_detected"] is False
    assert observation["presence_distance_m"] is None
    assert observation["breathing_rate_breaths_per_min"] is None
    assert observation["rate_available"] is False


def test_non_finite_rate_is_absent_and_json_safe() -> None:
    observation = observation_dict(
        result(state="ESTIMATE_BREATHING_RATE", rate=float("nan")),
        observed_at_unix_micros=1_700_000_000_000_000,
    )

    assert observation["rate_available"] is False
    assert observation["breathing_rate_breaths_per_min"] is None
    encoded = json.dumps(observation, allow_nan=False)
    assert '"breathing_rate_breaths_per_min": null' in encoded
