"""Tests for the T-B1 acceptance gate.

This script decides whether a bounded hardware capture counts. Every test here is a
way it could pass when it should not — which is the only failure that matters
for a gate, because a false pass costs another night.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from gap_histogram import DRIFT_LIMIT_PCT, Analysis, main, parse, report  # noqa: E402


def write(tmp_path: Path, lines: list[str]) -> Path:
    p = tmp_path / "capture.log"
    p.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return p


def steady(
    start: float = 1_700_000_000.0,
    seconds: float = 3600.0,
    rr_ms: float = 1000.0,
    gap_at: float | None = None,
    gap_len: float = 0.0,
) -> list[str]:
    """A capture with one RR interval per notification, optionally with a gap."""
    lines: list[str] = []
    t = start
    end = start + seconds
    while t < end:
        lines.append(f"{t:.3f} 60 {rr_ms:.3f}")
        step = rr_ms / 1000.0
        if gap_at is not None and t - start >= gap_at and gap_len > 0:
            step = gap_len
            gap_len = 0.0
        t += step
    return lines


# ── parsing ──────────────────────────────────────────────────────────────────


def test_parses_a_well_formed_capture(tmp_path: Path) -> None:
    log = write(tmp_path, ["1700000000.0 60 1000.0,900.0", "1700000001.9 61 950.0"])
    a = parse(log)
    assert a.count == 2
    assert a.rr_count == 3
    assert not a.malformed


def test_a_notification_without_rr_intervals_still_counts(tmp_path: Path) -> None:
    """The link was up. That is the thing being measured."""
    log = write(tmp_path, ["1700000000.0 60", "1700000001.0 61"])
    a = parse(log)
    assert a.count == 2
    assert a.rr_count == 0
    assert not a.malformed


def test_malformed_lines_are_collected_not_skipped(tmp_path: Path) -> None:
    """Silently skipping would let a corrupt capture look shorter but clean."""
    log = write(
        tmp_path,
        [
            "1700000000.0 60 1000.0",
            "1700000001.0 61 1000.0 TRAILING",
            "garbage",
            "1700000002.0 x 1000.0",
        ],
    )
    a = parse(log)
    assert a.count == 1
    assert len(a.malformed) == 3


# ── the gate ─────────────────────────────────────────────────────────────────


def test_a_short_capture_fails_the_duration_requirement(tmp_path: Path, capsys) -> None:
    log = write(tmp_path, steady(seconds=8 * 60.0))
    rc = main([str(log), "--require-duration-minutes", "9"])
    assert rc == 1
    assert "under the required 9" in capsys.readouterr().err


def test_a_ten_minute_capture_passes_the_bounded_hardware_gate(tmp_path: Path) -> None:
    log = write(tmp_path, steady(seconds=10 * 60 + 1))
    assert main([str(log), "--require-duration-minutes", "10", "--max-gap-seconds", "300"]) == 0


def test_a_ten_minute_capture_can_fail_the_bounded_gate(tmp_path: Path, capsys) -> None:
    log = write(tmp_path, steady(seconds=9 * 60 + 59))
    rc = main([str(log), "--require-duration-minutes", "10"])
    assert rc == 1
    assert "under the required 10" in capsys.readouterr().err


def test_capture_over_ten_minutes_fails_the_maximum(tmp_path: Path, capsys) -> None:
    log = write(tmp_path, steady(seconds=10 * 60 + 2))
    rc = main([str(log), "--max-duration-minutes", "10"])
    assert rc == 1
    assert "exceeds the maximum 10" in capsys.readouterr().err


def test_a_full_length_capture_passes(tmp_path: Path) -> None:
    log = write(tmp_path, steady(seconds=9 * 60.0 + 1))
    assert main([str(log), "--require-duration-minutes", "9", "--max-gap-seconds", "300"]) == 0


def test_a_long_gap_fails_and_names_its_length(tmp_path: Path, capsys) -> None:
    log = write(tmp_path, steady(seconds=9 * 60.0 + 1, gap_at=120.0, gap_len=400.0))
    rc = main([str(log), "--require-duration-minutes", "9", "--max-gap-seconds", "300"])
    assert rc == 1
    err = capsys.readouterr().err
    assert "longest gap" in err
    assert "400" in err, "the gate must say how long the gap was"


def test_malformed_lines_fail_the_gate(tmp_path: Path, capsys) -> None:
    lines = steady(seconds=9 * 60.0 + 1)
    lines.insert(100, "this is not a notification")
    log = write(tmp_path, lines)
    rc = main([str(log), "--require-duration-minutes", "9"])
    assert rc == 1
    assert "malformed" in capsys.readouterr().err


def test_a_missing_file_is_an_error_not_a_pass(tmp_path: Path) -> None:
    assert main([str(tmp_path / "nope.log")]) == 2


def test_non_monotonic_timestamps_fail_the_gate(tmp_path: Path, capsys) -> None:
    log = write(tmp_path, ["1700000001.0 60", "1700000000.0 60"])
    rc = main([str(log)])
    assert rc == 1
    assert "non-monotonic" in capsys.readouterr().err


# ── the failure that would waste a hardware check ────────────────────────────


def test_a_one_line_capture_fails_and_explains_systemd_run(tmp_path: Path, capsys) -> None:
    """`systemd-run` without --pipe sends the child's stdout to the journal, so
    the shell redirect captures only "Running as unit: hrspike.service". A
    bounded capture becomes a one-line file, and a gate that accepted it
    would cost another hardware check."""
    log = write(tmp_path, ["Running as unit: hrspike.service"])
    rc = main([str(log)])
    assert rc == 1
    err = capsys.readouterr().err
    assert "--pipe" in err, "the failure must name the actual cause"


def test_an_empty_capture_fails(tmp_path: Path) -> None:
    log = write(tmp_path, [])
    assert main([str(log)]) == 1


# ── drift ────────────────────────────────────────────────────────────────────


def test_drift_is_near_zero_when_the_clocks_agree(tmp_path: Path) -> None:
    log = write(tmp_path, steady(seconds=3600.0, rr_ms=1000.0))
    a = parse(log)
    drift = a.drift_pct()
    assert drift is not None
    assert abs(drift) < 0.1, f"expected agreement, got {drift}%"


def test_a_fast_sensor_clock_is_detected(tmp_path: Path) -> None:
    """RR intervals summing to more than the elapsed host time means the sensor
    thinks more time passed than the host did."""
    lines: list[str] = []
    t = 1_700_000_000.0
    for _ in range(3600):
        # Host advances 1.0s; the sensor claims 1.02s.
        lines.append(f"{t:.3f} 60 1020.000")
        t += 1.0
    log = write(tmp_path, lines)
    a = parse(log)
    drift = a.drift_pct()
    assert drift is not None
    assert drift > DRIFT_LIMIT_PCT, f"a 2% fast clock must exceed the limit, got {drift}%"


def test_require_drift_report_fails_when_drift_cannot_be_computed(
    tmp_path: Path, capsys
) -> None:
    """A flag a script may accept and ignore is a check that cannot fail."""
    # Two notifications an hour apart: long duration, no unbroken run.
    log = write(tmp_path, ["1700000000.0 60 1000.0", "1700003600.0 60 1000.0"])
    rc = main([str(log), "--require-drift-report"])
    assert rc == 1
    assert "drift could not be computed" in capsys.readouterr().err


def test_out_of_tolerance_drift_fails_the_gate(tmp_path: Path, capsys) -> None:
    lines: list[str] = []
    t = 1_700_000_000.0
    for _ in range(3600):
        lines.append(f"{t:.3f} 60 1020.000")
        t += 1.0
    log = write(tmp_path, lines)
    rc = main([str(log), "--require-drift-report"])
    assert rc == 1
    assert "exceeds" in capsys.readouterr().err


def test_drift_ignores_dropped_connections(tmp_path: Path) -> None:
    """A dropped connection loses beats, which would read as a slow sensor
    clock if the whole capture were used instead of the longest unbroken run."""
    lines = steady(seconds=1800.0)
    lines += steady(start=1_700_000_000.0 + 1800.0 + 600.0, seconds=1800.0)
    log = write(tmp_path, lines)
    a = parse(log)
    drift = a.drift_pct()
    assert drift is not None
    assert abs(drift) < 0.1, f"a 600s outage must not read as {drift}% drift"


# ── the estimate, honestly labelled ──────────────────────────────────────────


def test_dropped_is_reported_as_an_estimate_with_its_method(tmp_path: Path) -> None:
    log = write(tmp_path, steady(seconds=3600.0))
    a = parse(log)
    _, how = a.dropped_estimate()
    assert how, "the method must always be stated"


def test_dropped_estimate_is_zero_when_nothing_is_missing(tmp_path: Path) -> None:
    log = write(tmp_path, steady(seconds=3600.0, rr_ms=1000.0))
    a = parse(log)
    dropped, _ = a.dropped_estimate()
    assert dropped == 0


def test_dropped_estimate_notices_unaccounted_time(tmp_path: Path) -> None:
    """Notifications arriving on time but carrying fewer intervals than the
    elapsed period implies means beats went missing."""
    lines: list[str] = []
    t = 1_700_000_000.0
    for i in range(3600):
        # Every tenth notification carries no RR interval at all.
        rr = "" if i % 10 == 0 else "1000.000"
        lines.append(f"{t:.3f} 60 {rr}".rstrip())
        t += 1.0
    log = write(tmp_path, lines)
    a = parse(log)
    dropped, how = a.dropped_estimate()
    assert dropped > 0, f"expected a nonzero estimate, got {dropped} ({how})"


# ── the report ───────────────────────────────────────────────────────────────


def test_report_states_everything_the_gate_checks(tmp_path: Path) -> None:
    """The artifact you read and the check that passes are one thing, so the
    picture can never drift from the truth."""
    log = write(tmp_path, steady(seconds=3600.0))
    text = report(parse(log))
    for expected in ("notifications", "RR intervals", "duration", "longest gap",
                     "gap histogram", "clock drift", "dropped (est.)"):
        assert expected in text, f"report omits {expected!r}"


def test_report_shows_every_bucket_even_when_empty(tmp_path: Path) -> None:
    """A missing bucket and a zero bucket look the same in a table that omits
    empties, and they mean different things."""
    log = write(tmp_path, steady(seconds=600.0))
    text = report(parse(log))
    for name in ("<2s", "2-10s", "10-60s", "60s-5m", ">5m"):
        assert name in text


def test_an_empty_analysis_does_not_crash_the_report() -> None:
    assert "notifications" in report(Analysis())


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
