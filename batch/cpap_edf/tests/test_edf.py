"""Tests for the clean-room EDF reader.

Expected physical values are derived by hand from the EDF scaling formula, never
from this package's own output. That is the only way a parser test proves
anything: a golden file produced by the parser under test would agree with any
bug the parser has.

The fixtures use asymmetric digital bounds -- digital 0..100 onto physical
0..50 -- so every expectation is exactly `digital * 0.5` and can be checked
mentally. Float comparisons use a tolerance, never equality.
"""

from __future__ import annotations

import unittest

from batch.cpap_edf import edf
from batch.cpap_edf.fixtures import SignalSpec, make_edf

TOLERANCE = 1e-9


def one_signal(**changes: object) -> SignalSpec:
    base: dict[str, object] = {
        "label": "Mask Pres",
        "dimension": "cmH2O",
        "physical_min": 0.0,
        "physical_max": 50.0,
        "digital_min": 0,
        "digital_max": 100,
        "samples_per_record": 4,
        "samples": [0, 20, 50, 100],
    }
    base.update(changes)
    return SignalSpec(**base)  # type: ignore[arg-type]


class HeaderTests(unittest.TestCase):
    def test_minimal_file_parses_with_the_documented_layout(self) -> None:
        data = make_edf([one_signal()])
        # 256 header + 1 signal * 256 + 4 samples * 2 bytes.
        self.assertEqual(len(data), 256 + 256 + 8)
        header = edf.read_header(data)
        self.assertEqual(header.version, "0")
        self.assertEqual(len(header.signals), 1)
        self.assertEqual(header.record_count, 1)
        self.assertEqual(header.header_bytes, 512)
        self.assertAlmostEqual(header.record_duration_s, 1.0, delta=TOLERANCE)
        self.assertEqual(header.total_samples_per_record, 4)

    def test_signal_fields_are_read_from_their_interleaved_blocks(self) -> None:
        """Each EDF field is stored as ns consecutive copies, so a two-signal
        file is the case where a wrong block offset still parses."""
        data = make_edf([
            one_signal(label="Flow", dimension="L/s", physical_min=-10.0, physical_max=10.0),
            one_signal(label="Leak", dimension="L/min", physical_min=0.0, physical_max=120.0),
        ])
        header = edf.read_header(data)
        self.assertEqual([s.label for s in header.signals], ["Flow", "Leak"])
        self.assertEqual([s.dimension for s in header.signals], ["L/s", "L/min"])
        self.assertAlmostEqual(header.signals[0].physical_min, -10.0, delta=TOLERANCE)
        self.assertAlmostEqual(header.signals[1].physical_max, 120.0, delta=TOLERANCE)

    def test_scaling_matches_the_specification_by_hand(self) -> None:
        data = make_edf([one_signal()])
        header = edf.read_header(data)
        signal = header.signals[0]
        raw = edf.read_signal_samples(data, header, 0)
        self.assertEqual(raw, [0, 20, 50, 100])
        for digital, expected in ((0, 0.0), (20, 10.0), (50, 25.0), (100, 50.0)):
            with self.subTest(digital=digital):
                self.assertAlmostEqual(signal.scale(digital), expected, delta=TOLERANCE)

    def test_negative_digital_range_scales_correctly(self) -> None:
        """Flow is signed, so the negative half of the range must not be lost."""
        signal_spec = one_signal(
            label="Flow", dimension="L/s",
            physical_min=-10.0, physical_max=10.0,
            digital_min=-100, digital_max=100,
            samples=[-100, -50, 0, 100],
        )
        data = make_edf([signal_spec])
        header = edf.read_header(data)
        signal = header.signals[0]
        for digital, expected in ((-100, -10.0), (-50, -5.0), (0, 0.0), (100, 10.0)):
            with self.subTest(digital=digital):
                self.assertAlmostEqual(signal.scale(digital), expected, delta=TOLERANCE)

    def test_start_epoch_is_utc_from_the_header_fields(self) -> None:
        data = make_edf([one_signal()], start_date="01.01.26", start_time="22.30.00")
        # 2026-01-01T22:30:00Z. Independently: 2025-01-01 is 1735689600, plus
        # 365 days is 1767225600, plus 22h30m is 81000.
        self.assertEqual(edf.start_epoch_seconds(edf.read_header(data)), 1767225600 + 81000)

    def test_two_digit_year_follows_the_1985_2084_rule(self) -> None:
        """The specification pins the century; guessing moves a night by 100 years."""
        late = edf.start_epoch_seconds(
            edf.read_header(make_edf([one_signal()], start_date="31.12.99", start_time="00.00.00"))
        )
        early = edf.start_epoch_seconds(
            edf.read_header(make_edf([one_signal()], start_date="01.01.00", start_time="00.00.00"))
        )
        self.assertEqual(late, 946598400)   # 1999-12-31T00:00:00Z
        self.assertEqual(early, 946684800)  # 2000-01-01T00:00:00Z
        self.assertLess(late, early)

    def test_leap_day_and_leap_year_boundaries(self) -> None:
        self.assertEqual(
            edf.start_epoch_seconds(
                edf.read_header(make_edf([one_signal()], start_date="29.02.24", start_time="12.00.00"))
            ),
            1709208000,  # 2024-02-29T12:00:00Z
        )


class RejectionTests(unittest.TestCase):
    def assert_rejects(self, data: bytes, pattern: str) -> None:
        with self.assertRaisesRegex(edf.EdfError, pattern):
            edf.read_header(data)

    def test_file_shorter_than_the_fixed_header(self) -> None:
        self.assert_rejects(b"0" * 100, "shorter than the 256-byte header")

    def test_unknown_version(self) -> None:
        self.assert_rejects(make_edf([one_signal()], version="1"), "version rejected")

    def test_signal_count_of_zero(self) -> None:
        data = bytearray(make_edf([one_signal()]))
        data[252:256] = b"0   "
        self.assert_rejects(bytes(data), "number of signals rejected: 0")

    def test_absurd_signal_count_is_bounded_before_allocation(self) -> None:
        """A header claiming 9999 signals must be refused on the number, not by
        trying to read 9999 * 256 bytes of signal headers."""
        data = bytearray(make_edf([one_signal()]))
        data[252:256] = b"9999"
        self.assert_rejects(bytes(data), r"number of signals rejected: 9999 outside 1\.\.256")

    def test_header_bytes_must_match_the_signal_count(self) -> None:
        self.assert_rejects(
            make_edf([one_signal()], declared_header_bytes=4096), "header bytes rejected"
        )

    def test_non_numeric_header_field(self) -> None:
        data = bytearray(make_edf([one_signal()]))
        data[236:244] = b"not-int "
        self.assert_rejects(bytes(data), "number of data records rejected")

    def test_non_ascii_text_field_is_rejected_not_mangled(self) -> None:
        data = bytearray(make_edf([one_signal()]))
        data[8] = 0xFF
        self.assert_rejects(bytes(data), "not printable ASCII")

    def test_zero_record_duration(self) -> None:
        self.assert_rejects(
            make_edf([one_signal()], record_duration_s=0.0), "record duration rejected"
        )

    def test_equal_digital_bounds_would_divide_by_zero(self) -> None:
        self.assert_rejects(
            make_edf([one_signal(digital_min=50, digital_max=50)]), "divide by zero"
        )

    def test_inverted_digital_bounds(self) -> None:
        self.assert_rejects(
            make_edf([one_signal(digital_min=100, digital_max=0)]),
            "digital minimum 100 exceeds maximum 0",
        )

    def test_equal_physical_bounds_would_decode_a_constant(self) -> None:
        self.assert_rejects(
            make_edf([one_signal(physical_min=7.0, physical_max=7.0)]), "decode to a constant"
        )

    def test_digital_bounds_outside_int16(self) -> None:
        self.assert_rejects(
            make_edf([one_signal(digital_min=-40000, digital_max=100)]),
            "does not fit a signed 16-bit sample",
        )

    def test_zero_samples_per_record(self) -> None:
        self.assert_rejects(
            make_edf([one_signal(samples_per_record=0, samples=[])]), "samples per record 0"
        )

    def test_absurd_samples_per_record_is_bounded(self) -> None:
        data = bytearray(make_edf([one_signal()]))
        # The samples-per-record block sits at 256 + 216*ns for ns=1.
        offset = 256 + 216
        data[offset:offset + 8] = b"99999999"
        self.assert_rejects(bytes(data), r"samples per record 99999999 outside")

    def test_absurd_record_count_is_bounded(self) -> None:
        self.assert_rejects(
            make_edf([one_signal()], declared_record_count=99999999),
            "number of data records rejected: 99999999 exceeds",
        )

    def test_non_finite_physical_bound(self) -> None:
        data = bytearray(make_edf([one_signal()]))
        offset = 256 + 104  # physical minimum block, ns=1
        data[offset:offset + 8] = b"nan     "
        self.assert_rejects(bytes(data), "is not finite")

    def test_malformed_start_date_and_time(self) -> None:
        # All eight characters wide, because the field is eight bytes and the
        # fixture builder rightly refuses anything longer. The interesting
        # malformations are therefore wrong separators and out-of-range values,
        # not wrong lengths.
        for field_name, value, pattern in (
            ("start_date", "26-01-01", "start date rejected"),
            ("start_time", "22:30:00", "start time rejected"),
            ("start_date", "32.01.26", "day or month out of range"),
            ("start_time", "25.00.00", "out of range"),
            ("start_date", "aa.bb.cc", "start date rejected"),
        ):
            with self.subTest(field=field_name, value=value):
                data = make_edf([one_signal()], **{field_name: value})  # type: ignore[arg-type]
                with self.assertRaisesRegex(edf.EdfError, pattern):
                    edf.start_epoch_seconds(edf.read_header(data))


class TruncationTests(unittest.TestCase):
    def test_unknown_record_count_is_derived_from_the_file(self) -> None:
        """-1 is permitted for a recording still in progress; the file's own
        length is then the only evidence of how much is there."""
        data = make_edf([one_signal(samples=[0, 20, 50, 100] * 3)], record_count=3,
                        declared_record_count=-1)
        header = edf.read_header(data)
        self.assertEqual(header.record_count, 3)
        self.assertEqual(len(edf.read_signal_samples(data, header, 0)), 12)

    def test_truncated_tail_keeps_complete_records_and_is_measurable(self) -> None:
        full = make_edf([one_signal(samples=[0, 20, 50, 100] * 3)], record_count=3)
        # Drop six bytes: three samples, so the last record is incomplete.
        cut = make_edf([one_signal(samples=[0, 20, 50, 100] * 3)], record_count=3,
                       truncate_bytes=6)
        self.assertEqual(len(cut), len(full) - 6)
        header = edf.read_header(cut)
        self.assertEqual(header.record_count, 3)
        self.assertEqual(edf.complete_record_count(cut, header), 2)
        # Only whole records are returned; the partial tail contributes nothing.
        self.assertEqual(len(edf.read_signal_samples(cut, header, 0)), 8)

    def test_header_present_but_no_data_records(self) -> None:
        data = make_edf([one_signal(samples=[])], record_count=0)
        header = edf.read_header(data)
        self.assertEqual(header.record_count, 0)
        self.assertEqual(edf.read_signal_samples(data, header, 0), [])

    def test_signal_index_out_of_range(self) -> None:
        data = make_edf([one_signal()])
        header = edf.read_header(data)
        with self.assertRaisesRegex(edf.EdfError, "signal index rejected"):
            edf.read_signal_samples(data, header, 5)


class PropertyTests(unittest.TestCase):
    def test_scaling_is_monotonic_and_hits_both_endpoints(self) -> None:
        """A property rather than a point check: whatever the bounds, scaling is
        increasing and maps the digital extremes onto the physical extremes."""
        cases = [
            (0, 100, 0.0, 50.0),
            (-32768, 32767, -20.0, 20.0),
            (-1, 1, 0.0, 1.0),
            (0, 1023, -10.5, 10.5),
        ]
        for digital_min, digital_max, physical_min, physical_max in cases:
            with self.subTest(bounds=(digital_min, digital_max)):
                data = make_edf([one_signal(
                    digital_min=digital_min, digital_max=digital_max,
                    physical_min=physical_min, physical_max=physical_max,
                    samples_per_record=1, samples=[digital_min],
                )])
                signal = edf.read_header(data).signals[0]
                self.assertAlmostEqual(signal.scale(digital_min), physical_min, delta=1e-6)
                self.assertAlmostEqual(signal.scale(digital_max), physical_max, delta=1e-6)
                previous = signal.scale(digital_min)
                step = max(1, (digital_max - digital_min) // 17)
                for digital in range(digital_min + step, digital_max + 1, step):
                    current = signal.scale(digital)
                    self.assertGreater(current, previous)
                    previous = current

    def test_round_trip_through_every_sample_position(self) -> None:
        """Interleaving means a sample's byte offset depends on the signal index,
        the record and the position. Every combination must come back."""
        first = SignalSpec("Flow", "L/s", -10.0, 10.0, -100, 100, samples_per_record=3,
                           samples=[-100, 0, 100, -50, 50, 25])
        second = SignalSpec("Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=2,
                            samples=[0, 60, 120, 30])
        data = make_edf([first, second], record_count=2)
        header = edf.read_header(data)
        self.assertEqual(edf.read_signal_samples(data, header, 0), [-100, 0, 100, -50, 50, 25])
        self.assertEqual(edf.read_signal_samples(data, header, 1), [0, 60, 120, 30])


if __name__ == "__main__":
    unittest.main()
