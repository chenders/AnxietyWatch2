#!/usr/bin/env python3
"""Analyse a bounded Polar H10 hardware capture.

The project rests on the Pi holding a BLE connection and receiving valid RR
the script that says whether it did, and it is both the report you read and the
gate that passes — so the number you look at can never drift from the number
that decides.

Input is `hr-spike` output, one line per notification:

    <unix_epoch_seconds_float> <hr_bpm> <rr_ms,rr_ms,...>

The RR field may be empty; a notification without intervals is still evidence
the link was up.

Usage:
    gap_histogram.py night1.log
    gap_histogram.py capture.log --require-duration-minutes 10 --max-duration-minutes 10 \
                     --max-gap-seconds 300

Exits nonzero when a requirement is not met, and says which one. A gate that
will not name its blocker costs an hour of guessing.
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Gap buckets, in seconds. The boundaries are chosen around what they imply:
# under 2 s is a normal inter-notification interval, 2–10 s is a hiccup, beyond
# a minute the link genuinely dropped.
BUCKETS: list[tuple[str, float, float]] = [
    ("<2s", 0.0, 2.0),
    ("2-10s", 2.0, 10.0),
    ("10-60s", 10.0, 60.0),
    ("60s-5m", 60.0, 300.0),
    (">5m", 300.0, float("inf")),
]

# A sensor whose clock runs more than this fast or slow against the host is
# faulty, not merely imprecise (amendment 05 §1.4).
DRIFT_LIMIT_PCT = 0.5


@dataclass
class Notification:
    """One line of the capture."""

    t: float
    bpm: int
    rr_ms: list[float]
    line_no: int


@dataclass
class Analysis:
    """Everything the report and the gate both read."""

    notifications: list[Notification] = field(default_factory=list)
    malformed: list[tuple[int, str]] = field(default_factory=list)

    @property
    def count(self) -> int:
        return len(self.notifications)

    @property
    def rr_count(self) -> int:
        return sum(len(n.rr_ms) for n in self.notifications)

    @property
    def duration_s(self) -> float:
        if len(self.notifications) < 2:
            return 0.0
        return self.notifications[-1].t - self.notifications[0].t

    def gaps(self) -> list[float]:
        ts = [n.t for n in self.notifications]
        return [b - a for a, b in zip(ts, ts[1:], strict=False)]

    def histogram(self) -> dict[str, int]:
        h = {name: 0 for name, _, _ in BUCKETS}
        for g in self.gaps():
            for name, lo, hi in BUCKETS:
                if lo <= g < hi:
                    h[name] += 1
                    break
        return h

    def longest_gap(self) -> float:
        g = self.gaps()
        return max(g) if g else 0.0

    def drift_pct(self) -> float | None:
        """Sensor clock drift against the host, as a percentage.

        The H10 reports intervals in its own time base. Over eight hours the
        accumulated difference against the host clock is real and measurable,
        and **nothing else in this system measures it** — so a negative result
        is as valuable as a positive one.

        Compares summed RR duration against elapsed host time across the
        longest unbroken run, because a dropped connection loses beats and
        would otherwise read as the sensor running slow.
        """
        run = self._longest_unbroken_run()
        if run is None:
            return None
        start, end = run
        window = self.notifications[start : end + 1]
        # The first notification's intervals precede its arrival timestamp;
        # compare only intervals received after the window begins.
        measured = window[1:]
        rr_total_s = sum(sum(n.rr_ms) for n in measured) / 1000.0
        host_s = window[-1].t - window[0].t
        if host_s <= 0 or rr_total_s <= 0:
            return None
        return (rr_total_s - host_s) / host_s * 100.0

    def _longest_unbroken_run(self, max_gap_s: float = 2.0) -> tuple[int, int] | None:
        """Indices of the longest run with no gap over `max_gap_s`.

        Needs at least a few minutes to say anything about drift; a 30-second
        run would report noise as a clock fault.
        """
        if len(self.notifications) < 2:
            return None
        best: tuple[int, int] | None = None
        best_len = 0.0
        start = 0
        for i in range(1, len(self.notifications)):
            gap = self.notifications[i].t - self.notifications[i - 1].t
            if gap > max_gap_s:
                span = self.notifications[i - 1].t - self.notifications[start].t
                if span > best_len:
                    best_len, best = span, (start, i - 1)
                start = i
        span = self.notifications[-1].t - self.notifications[start].t
        if span > best_len:
            best_len, best = span, (start, len(self.notifications) - 1)
        return best if best_len >= 300.0 else None

    def dropped_estimate(self) -> tuple[int, str]:
        """Estimated dropped notifications, and how it was estimated.

        **Not a `seq` gap count.** Amendment 05 §1.3 offers
        `max(seq) - count(*)` as the lost-beat metric, but §1.1 defines RR `seq`
        as the count of intervals *received*, which is dense by construction —
        so that expression is identically zero and can never detect anything.
        The H10 transmits no beat counter, the same payload anonymity amendment
        06 §1 documents for the blood-pressure cuff.

        So this is an estimate from elapsed time versus summed RR duration, and
        it is labelled as one. An estimate presented as a measurement is how a
        number stops being questioned.
        """
        run = self._longest_unbroken_run()
        if run is None:
            return 0, "no unbroken run of 5 minutes or more; not estimable"
        start, end = run
        window = self.notifications[start : end + 1]
        measured = window[1:]
        rr_total_s = sum(sum(n.rr_ms) for n in measured) / 1000.0
        host_s = window[-1].t - window[0].t
        missing_s = host_s - rr_total_s
        if missing_s <= 0:
            return 0, "summed RR duration covers the elapsed window"
        mean_rr_s = rr_total_s / max(1, sum(len(n.rr_ms) for n in window))
        if mean_rr_s <= 0:
            return 0, "no RR intervals in the window"
        return round(missing_s / mean_rr_s), (
            f"estimated from {missing_s:.1f}s unaccounted across the longest "
            f"unbroken run, at a mean interval of {mean_rr_s * 1000:.1f}ms"
        )


def parse(path: Path) -> Analysis:
    """Reads the capture. Malformed lines are collected, never skipped silently."""
    a = Analysis()
    with path.open(encoding="utf-8") as fh:
        for line_no, raw in enumerate(fh, start=1):
            line = raw.strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) < 2:
                a.malformed.append((line_no, f"expected at least 2 fields, got {len(parts)}"))
                continue
            try:
                t = float(parts[0])
                bpm = int(parts[1])
            except ValueError as exc:
                a.malformed.append((line_no, str(exc)))
                continue
            if len(parts) > 3:
                a.malformed.append((line_no, f"expected at most 3 fields, got {len(parts)}"))
                continue
            rr: list[float] = []
            if len(parts) == 3 and parts[2]:
                try:
                    rr = [float(x) for x in parts[2].split(",") if x]
                except ValueError as exc:
                    a.malformed.append((line_no, f"RR field: {exc}"))
                    continue
            a.notifications.append(Notification(t=t, bpm=bpm, rr_ms=rr, line_no=line_no))
    return a


def report(a: Analysis) -> str:
    """The human-readable report. Same data the gate uses."""
    out: list[str] = []
    hrs = a.duration_s / 3600.0
    out.append("Polar H10 hardware capture")
    out.append("=" * 60)
    out.append(f"  notifications   {a.count}")
    out.append(f"  RR intervals    {a.rr_count}")
    out.append(f"  duration        {hrs:.2f} h ({a.duration_s:.0f}s)")
    out.append(f"  longest gap     {a.longest_gap():.1f}s")
    if a.count:
        bpms = [n.bpm for n in a.notifications]
        out.append(f"  HR              min {min(bpms)} · max {max(bpms)}")
    out.append("")
    out.append("  gap histogram")
    h = a.histogram()
    total = sum(h.values()) or 1
    for name, _, _ in BUCKETS:
        n = h[name]
        bar = "#" * round(40 * n / total)
        out.append(f"    {name:>8}  {n:>7}  {bar}")
    out.append("")

    drift = a.drift_pct()
    if drift is None:
        out.append("  clock drift     not estimable (no unbroken run of 5 minutes or more)")
    else:
        verdict = "within tolerance" if abs(drift) <= DRIFT_LIMIT_PCT else "OUT OF TOLERANCE"
        out.append(f"  clock drift     {drift:+.3f}% ({verdict}, limit ±{DRIFT_LIMIT_PCT}%)")

    dropped, how = a.dropped_estimate()
    out.append(f"  dropped (est.)  {dropped}")
    out.append(f"                  {how}")

    if a.malformed:
        out.append("")
        out.append(f"  MALFORMED LINES {len(a.malformed)}")
        for line_no, why in a.malformed[:10]:
            out.append(f"    line {line_no}: {why}")
        if len(a.malformed) > 10:
            out.append(f"    ... and {len(a.malformed) - 10} more")
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("log", type=Path)
    minimum_duration = p.add_mutually_exclusive_group()
    minimum_duration.add_argument("--require-duration-hours", type=float, default=None)
    minimum_duration.add_argument("--require-duration-minutes", type=float, default=None)
    p.add_argument("--max-duration-minutes", type=float, default=None)
    p.add_argument("--max-gap-seconds", type=float, default=None)
    p.add_argument(
        "--require-drift-report",
        action="store_true",
        help="fail if clock drift could not be computed. A flag a script may "
        "accept and ignore is a check that cannot fail.",
    )
    args = p.parse_args(argv)

    if not args.log.is_file():
        print(f"gap_histogram: {args.log}: no such file", file=sys.stderr)
        return 2

    a = parse(args.log)
    print(report(a))

    # A capture of one line is what `systemd-run` without --pipe produces: the
    # child's stdout goes to the journal and the shell redirect catches only
    # "Running as unit". Refusing it by name saves a wasted hardware check.
    if a.count < 2:
        print(
            f"\nFAIL: only {a.count} notification(s) parsed. If this was a systemd "
            "run, systemd-run needs --pipe — without it the child's stdout goes to "
            "the journal and the redirect captures only the unit banner.",
            file=sys.stderr,
        )
        return 1

    failures: list[str] = []
    if args.require_duration_hours is not None:
        hrs = a.duration_s / 3600.0
        if hrs < args.require_duration_hours:
            failures.append(
                f"duration {hrs:.2f}h is under the required {args.require_duration_hours}h"
            )
    if args.require_duration_minutes is not None:
        minutes = a.duration_s / 60.0
        if minutes < args.require_duration_minutes:
            failures.append(
                f"duration {minutes:.2f}m is under the required {args.require_duration_minutes}m"
            )
    if args.max_duration_minutes is not None:
        minutes = a.duration_s / 60.0
        if minutes > args.max_duration_minutes:
            failures.append(
                f"duration {minutes:.2f}m exceeds the maximum {args.max_duration_minutes}m"
            )
    if args.max_gap_seconds is not None:
        longest = a.longest_gap()
        if longest > args.max_gap_seconds:
            failures.append(
                f"longest gap {longest:.1f}s exceeds the {args.max_gap_seconds}s limit"
            )
    negative_gaps = [gap for gap in a.gaps() if gap < 0]
    if negative_gaps:
        failures.append(
            f"{len(negative_gaps)} non-monotonic timestamp gap(s); first is {negative_gaps[0]:.3f}s"
        )
    if args.require_drift_report:
        drift = a.drift_pct()
        if drift is None:
            failures.append(
                "clock drift could not be computed and --require-drift-report was given"
            )
        elif abs(drift) > DRIFT_LIMIT_PCT:
            failures.append(
                f"clock drift {drift:+.3f}% exceeds ±{DRIFT_LIMIT_PCT}% — a sensor or "
                "host clock fault, not normal variation"
            )
    if a.malformed:
        failures.append(f"{len(a.malformed)} malformed line(s) in the capture")

    if failures:
        print("", file=sys.stderr)
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
