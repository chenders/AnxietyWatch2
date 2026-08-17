#!/usr/bin/env python3
"""Tests for ear-driven candidate identification.

The load-bearing case is `test_a_constant_flyer_never_wins`: an intersection of
"what was overhead when I heard it" ranks the busiest aircraft first, which is
the confident wrong answer this module exists to avoid.
"""

import multiprocessing
import pathlib
import tempfile
import unittest

import panel_candidate_id as cid


def obs(at, hex_code, distance_km=1.0):
    return {"at": at, "hex": hex_code, "distance_km": distance_km}


def concurrent_mark(path, gate):
    gate.wait()
    cid.mark_heard(at=1000.0, event_id="hue-concurrent", path=path)


class Windowing(unittest.TestCase):
    def test_window_reaches_backwards_further_than_forwards(self):
        # The owner marks the sound after noticing it, so the past matters more.
        self.assertGreater(cid.WINDOW_BEFORE_S, cid.WINDOW_AFTER_S)

    def test_aircraft_inside_the_window_is_counted(self):
        self.assertEqual(cid.overhead_at([obs(1000.0, "AAAAAA")], 1000.0, 6.0),
                         {"AAAAAA"})

    def test_aircraft_outside_the_window_is_not(self):
        self.assertEqual(cid.overhead_at([obs(0.0, "AAAAAA")], 10_000.0, 6.0), set())

    def test_aircraft_beyond_the_radius_is_not(self):
        self.assertEqual(
            cid.overhead_at([obs(1000.0, "AAAAAA", distance_km=50.0)], 1000.0, 6.0),
            set())

    def test_rows_without_usable_fields_are_skipped(self):
        rows = [{"at": 1000.0}, {"hex": "AAAAAA"}, {"at": 1000.0, "hex": "B", "distance_km": None}]
        self.assertEqual(cid.overhead_at(rows, 1000.0, 6.0), set())

    def test_addresses_are_compared_uppercase(self):
        self.assertEqual(cid.overhead_at([obs(1000.0, "a4c3eb")], 1000.0, 6.0),
                         {"A4C3EB"})


class Ranking(unittest.TestCase):
    def test_a_constant_flyer_never_wins(self):
        # BUSY is overhead at every mark AND every quiet window. A set
        # intersection would rank it first; the ratio correctly rejects it,
        # while the genuinely correlated PPB address wins.
        marks = [{"at": t} for t in (1000.0, 2000.0, 3000.0, 4000.0, 5000.0)]
        quiet = [50_000.0, 60_000.0, 70_000.0, 80_000.0]
        observations = []
        for t in (1000.0, 2000.0, 3000.0, 4000.0, 5000.0):
            observations.append(obs(t, "BUSY01"))
            observations.append(obs(t, "PPB001"))
        for t in quiet:
            observations.append(obs(t, "BUSY01"))

        verdict, rows = cid.rank(marks, observations, quiet)
        self.assertEqual(verdict, cid.CANDIDATE)
        self.assertEqual(rows[0]["hex"], "PPB001")
        busy = next(r for r in rows if r["hex"] == "BUSY01")
        self.assertLess(busy["ratio"], rows[0]["ratio"])

    def test_one_mark_never_names_a_candidate(self):
        # A single coincidence must not create a roster entry that is then
        # trusted forever.
        marks = [{"at": 1000.0}]
        verdict, rows = cid.rank(marks, [obs(1000.0, "PPB001")], [50_000.0])
        self.assertEqual(verdict, cid.INSUFFICIENT)
        self.assertIn("only 1 marks", cid.explain(verdict, rows))

    def test_weak_correlation_is_reported_as_insufficient_with_the_numbers(self):
        marks = [{"at": float(t)} for t in range(1000, 6000, 1000)]
        quiet = [50_000.0, 60_000.0, 70_000.0, 80_000.0]
        observations = [obs(float(t), "MAYBE1") for t in range(1000, 6000, 1000)]
        observations += [obs(t, "MAYBE1") for t in quiet]
        verdict, rows = cid.rank(marks, observations, quiet)
        self.assertEqual(verdict, cid.INSUFFICIENT)
        reason = cid.explain(verdict, rows)
        self.assertIn("below the", reason)
        self.assertIn("quiet windows", reason)

    def test_no_quiet_windows_cannot_manufacture_a_candidate(self):
        # Without quiet windows there is no contrast, so presence alone must not
        # be promoted to evidence.
        marks = [{"at": float(t)} for t in range(1000, 6000, 1000)]
        observations = [obs(float(t), "PPB001") for t in range(1000, 6000, 1000)]
        verdict, _ = cid.rank(marks, observations, [])
        self.assertEqual(verdict, cid.INSUFFICIENT)

    def test_an_address_seen_once_does_not_divide_by_zero(self):
        marks = [{"at": float(t)} for t in range(1000, 6000, 1000)]
        quiet = [50_000.0, 60_000.0]
        observations = [obs(1000.0, "ONCE01")] + [obs(t, "OTHER1") for t in quiet]
        _, rows = cid.rank(marks, observations, quiet)
        for row in rows:
            self.assertIsInstance(row["ratio"], float)

    def test_empty_input_explains_itself(self):
        verdict, rows = cid.rank([], [], [])
        self.assertEqual(verdict, cid.INSUFFICIENT)
        self.assertIn("no aircraft", cid.explain(verdict, rows))

    def test_a_confirmed_candidate_is_told_to_be_verified_by_hand(self):
        # This module proposes; a human confirms against the FAA registry.
        marks = [{"at": float(t)} for t in range(1000, 6000, 1000)]
        quiet = [50_000.0, 60_000.0, 70_000.0, 80_000.0]
        observations = [obs(float(t), "PPB001") for t in range(1000, 6000, 1000)]
        verdict, rows = cid.rank(marks, observations, quiet)
        self.assertEqual(verdict, cid.CANDIDATE)
        self.assertIn("FAA registry", cid.explain(verdict, rows))


class Marks(unittest.TestCase):
    def test_a_mark_round_trips(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "marks.jsonl"
            cid.mark_heard(at=1000.0, note="loud orbit overhead",
                           suspected="POLICE", path=path)
            cid.mark_heard(at=2000.0, path=path)
            marks = cid.read_marks(path)
            self.assertEqual([m["at"] for m in marks], [1000.0, 2000.0])
            self.assertEqual(marks[0]["suspected"], "POLICE")

    def test_retry_with_same_event_id_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "marks.jsonl"
            cid.mark_heard(at=1000.0, event_id="hue-1", path=path)
            cid.mark_heard(at=1001.0, event_id="hue-1", path=path)
            marks = cid.read_marks(path)
            self.assertEqual(len(marks), 1)
            self.assertEqual(marks[0]["at"], 1000.0)

    def test_concurrent_retry_with_same_event_id_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "marks.jsonl"
            gate = multiprocessing.Event()
            workers = [multiprocessing.Process(target=concurrent_mark, args=(path, gate))
                       for _ in range(4)]
            for worker in workers:
                worker.start()
            gate.set()
            for worker in workers:
                worker.join(5)
                self.assertEqual(worker.exitcode, 0)
            self.assertEqual(len(cid.read_marks(path)), 1)

    def test_marks_come_back_in_time_order(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "marks.jsonl"
            for at in (3000.0, 1000.0, 2000.0):
                cid.mark_heard(at=at, path=path)
            self.assertEqual([m["at"] for m in cid.read_marks(path)],
                             [1000.0, 2000.0, 3000.0])

    def test_a_truncated_row_does_not_discard_the_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "marks.jsonl"
            cid.mark_heard(at=1000.0, path=path)
            with path.open("a") as stream:
                stream.write('{"at": 20')
            self.assertEqual(len(cid.read_marks(path)), 1)

    def test_missing_file_reads_as_empty(self):
        self.assertEqual(cid.read_marks(pathlib.Path("/nonexistent/marks.jsonl")), [])


if __name__ == "__main__":
    unittest.main()
