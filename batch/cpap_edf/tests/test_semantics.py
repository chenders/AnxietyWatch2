"""Tests for channel mapping, units, identity, the clock contract and decoding.

The unit tests here matter more than they look. A wrong conversion factor does
not crash and does not fail a schema check -- it produces a plausible number in
the wrong scale, and `AGENTS.md` is explicit that a mis-scaled physiological
reading is indistinguishable from a real one downstream. So the 60x cases are
asserted by hand, both directions.
"""

from __future__ import annotations

import unittest
from pathlib import Path

from batch.cpap_edf import channels, clock, decode, identity, registry
from batch.cpap_edf.fixtures import SignalSpec, make_edf

TOLERANCE = 1e-9
REGISTRY = Path(__file__).resolve().parents[3] / "vitals" / "registry.toml"


class RegistryAgreementTests(unittest.TestCase):
    """The registry is the source of truth; this package must not drift from it."""

    def test_registry_file_is_where_this_package_expects(self) -> None:
        self.assertTrue(REGISTRY.is_file(), f"{REGISTRY} is absent")

    def test_canonical_units_match_the_registry(self) -> None:
        problems = channels.check_units(registry.load_units(REGISTRY))
        self.assertEqual(problems, [], "\n".join(problems))

    def test_every_emitted_type_is_declared_on_the_sd_source(self) -> None:
        source_id, provides = registry.sd_source_provides(REGISTRY)
        self.assertEqual(source_id, 8)
        emitted = set(channels.LABEL_TO_TYPE.values())
        undeclared = sorted(emitted - provides)
        self.assertEqual(
            undeclared, [],
            f"these types can be emitted but are not in [source.resmed_air11_sd].provides: "
            f"{undeclared}",
        )

    def test_ranges_load_and_are_ordered(self) -> None:
        ranges = registry.load_ranges(REGISTRY)
        for type_name in channels.CANONICAL_UNIT:
            with self.subTest(type=type_name):
                low, high = ranges[type_name]
                self.assertLess(low, high)

    def test_a_malformed_registry_names_its_blocker(self) -> None:
        with self.assertRaisesRegex(registry.RegistryError, "is absent"):
            registry.load_units(Path("/nonexistent/registry.toml"))


class ChannelTests(unittest.TestCase):
    def test_label_spellings_fold_to_one_type(self) -> None:
        for label in ("Mask Pres", "MaskPres", "mask pres", "MASK  PRES"):
            with self.subTest(label=label):
                self.assertEqual(channels.map_signal(label), "mask_pressure")

    def test_colon_label_folds(self) -> None:
        self.assertEqual(channels.map_signal("I:E"), "ie_ratio")

    def test_unmapped_label_returns_none_rather_than_guessing(self) -> None:
        for label in ("Ti", "Te", "TgMV", "Resp Event", "Something New"):
            with self.subTest(label=label):
                self.assertIsNone(channels.map_signal(label))

    def test_leak_conversion_is_60x_in_the_right_direction(self) -> None:
        """L/s -> L/min multiplies by 60. Getting this backwards turns a 24
        L/min leak into 0.4 and a 0.4 L/s leak into 0.4."""
        self.assertAlmostEqual(channels.conversion_factor("leak", "L/s"), 60.0, delta=TOLERANCE)
        self.assertAlmostEqual(channels.conversion_factor("leak", "L/min"), 1.0, delta=TOLERANCE)

    def test_tidal_volume_conversion_is_1000x(self) -> None:
        self.assertAlmostEqual(channels.conversion_factor("tidal_volume", "L"), 1000.0, delta=TOLERANCE)
        self.assertAlmostEqual(channels.conversion_factor("tidal_volume", "ml"), 1.0, delta=TOLERANCE)

    def test_dimension_folding_keeps_the_characters_that_matter(self) -> None:
        """'l/s' and 'l/min' differ by exactly the 60x factor, so the slash and
        the word must survive normalisation even though case does not."""
        self.assertEqual(channels.normalise_dimension(" L/Min "), "l/min")
        self.assertNotEqual(
            channels.normalise_dimension("L/s"), channels.normalise_dimension("L/min")
        )

    def test_unrecognised_dimension_is_rejected_not_defaulted_to_one(self) -> None:
        with self.assertRaisesRegex(ValueError, "dimension 'hPa' rejected for mask_pressure"):
            channels.conversion_factor("mask_pressure", "hPa")

    def test_rejection_names_the_accepted_set_and_the_canonical_unit(self) -> None:
        with self.assertRaises(ValueError) as caught:
            channels.conversion_factor("leak", "furlongs")
        message = str(caught.exception)
        self.assertIn("l/min", message)
        self.assertIn("L/min", message)

    def test_unit_drift_is_detected(self) -> None:
        problems = channels.check_units({**channels.CANONICAL_UNIT, "leak": "L/s"})
        self.assertEqual(len(problems), 1)
        self.assertIn("registry says 'L/s'", problems[0])


class IdentityTests(unittest.TestCase):
    def test_session_id_is_deterministic(self) -> None:
        first = identity.session_id("DATALOG/20260101", "01.01.26")
        second = identity.session_id("DATALOG/20260101", "01.01.26")
        self.assertEqual(first, second)

    def test_session_id_fits_a_signed_bigint(self) -> None:
        value = identity.session_id("DATALOG/20260101", "01.01.26")
        self.assertGreaterEqual(value, identity.SIGNED_64_MIN)
        self.assertLessEqual(value, identity.SIGNED_64_MAX)

    def test_files_in_one_directory_share_a_session(self) -> None:
        """One night is several files -- BRP, PLD, SAD -- and they must not
        become three sessions."""
        brp = identity.SessionFile("DATALOG/20260101/BRP.edf", "DATALOG/20260101", "01.01.26", 100)
        pld = identity.SessionFile("DATALOG/20260101/PLD.edf", "DATALOG/20260101", "01.01.26", 160)
        self.assertEqual(brp.session_id, pld.session_id)

    def test_different_nights_are_different_sessions(self) -> None:
        a = identity.session_id("DATALOG/20260101", "01.01.26")
        b = identity.session_id("DATALOG/20260102", "02.01.26")
        self.assertNotEqual(a, b)

    def test_the_session_assumption_is_falsifiable(self) -> None:
        """The documented assumption is one directory, one session. This is the
        check that makes it fail loudly against real data instead of silently
        merging two nights."""
        same = "DATALOG/20260101"
        close = [
            identity.SessionFile("a.edf", same, "01.01.26", 0),
            identity.SessionFile("b.edf", same, "01.01.26", 3600),
        ]
        self.assertEqual(identity.session_conflicts(close), [])
        far = [
            identity.SessionFile("a.edf", same, "01.01.26", 0),
            identity.SessionFile("b.edf", same, "01.01.26", identity.MAX_SESSION_SPAN_S + 1),
        ]
        conflicts = identity.session_conflicts(far)
        self.assertEqual(len(conflicts), 1)
        self.assertIn("more than", conflicts[0])
        self.assertIn("a.edf", conflicts[0])

    def test_seq_is_milliseconds_from_the_session_start(self) -> None:
        self.assertEqual(identity.sample_seq(1000, 1000, 0.0), 0)
        self.assertEqual(identity.sample_seq(1000, 1000, 0.04), 40)
        self.assertEqual(identity.sample_seq(1000, 1060, 0.5), 60_500)

    def test_seq_is_monotonic_across_a_record(self) -> None:
        """A 25 Hz signal yields 40 ms steps; two samples must never share a seq."""
        seqs = [identity.sample_seq(0, 0, index / 25.0) for index in range(25)]
        self.assertEqual(seqs, sorted(seqs))
        self.assertEqual(len(set(seqs)), 25)

    def test_negative_offset_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "offset within second rejected"):
            identity.sample_seq(0, 0, -0.1)


class ClockTests(unittest.TestCase):
    def test_the_default_transform_is_the_identity_and_says_so(self) -> None:
        transform = clock.IDENTITY_TRANSFORM
        self.assertAlmostEqual(transform.apply(1767225600), 1767225600, delta=TOLERANCE)
        self.assertIn("no correction derived", transform.derived_from)

    def test_uncorrected_is_a_version_not_an_absence(self) -> None:
        """A stored sample must always name the rule that interpreted it, or a
        reader cannot tell 'no correction needed' from 'none recorded'."""
        self.assertEqual(clock.IDENTITY_TRANSFORM.version, 1)
        metadata = clock.transform_metadata(clock.IDENTITY_TRANSFORM)
        self.assertEqual(metadata["transform_version"], 1)
        self.assertIsNotNone(metadata["derived_from"])

    def test_offset_and_drift_apply_at_read_time(self) -> None:
        transform = clock.ClockTransform(
            version=2, offset_s=-94.0, drift_ppm=10.0, anchor_epoch_s=1_000_000,
            derived_from="measured against a reference on 2026-01-01",
        )
        # At the anchor only the offset applies.
        self.assertAlmostEqual(transform.apply(1_000_000), 999_906.0, delta=1e-6)
        # 1e6 seconds later, 10 ppm is a further 10 seconds.
        self.assertAlmostEqual(transform.apply(2_000_000), 2_000_000 - 94.0 + 10.0, delta=1e-6)

    def test_applying_a_transform_is_pure(self) -> None:
        transform = clock.ClockTransform(3, 5.0, 0.0, 0, "x")
        self.assertAlmostEqual(transform.apply(42), transform.apply(42), delta=TOLERANCE)

    def test_a_new_version_does_not_change_an_older_one(self) -> None:
        """Changing the correction appends a version; it never rewrites history."""
        first = clock.ClockTransform(1, 0.0, 0.0, 0, "none")
        second = clock.ClockTransform(2, -94.0, 0.0, 0, "measured")
        self.assertAlmostEqual(first.apply(1000), 1000, delta=TOLERANCE)
        self.assertAlmostEqual(second.apply(1000), 906, delta=TOLERANCE)


class DecodeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.ranges = registry.load_ranges(REGISTRY)

    def decode(self, data: bytes, **kwargs: object) -> decode.DecodeResult:
        return decode.decode_file(
            data,
            card_path=kwargs.pop("card_path", "DATALOG/20260101/PLD.edf"),  # type: ignore[arg-type]
            card_directory=kwargs.pop("card_directory", "DATALOG/20260101"),  # type: ignore[arg-type]
            registry_ranges=self.ranges,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_a_clean_file_decodes_to_canonical_units(self) -> None:
        data = make_edf([SignalSpec(
            "Leak", "L/s", 0.0, 2.0, 0, 200, samples_per_record=3, samples=[0, 20, 100],
        )])
        result = self.decode(data)
        self.assertEqual(result.rejections, [])
        self.assertEqual(len(result.samples), 3)
        # digital 20 -> 0.2 L/s by hand, then x60 -> 12 L/min.
        self.assertAlmostEqual(result.samples[1].value, 12.0, delta=1e-6)
        self.assertEqual(result.samples[1].unit, "L/min")
        self.assertEqual(result.samples[1].type_name, "leak")
        self.assertEqual(result.samples[1].source, "resmed_air11_sd")

    def test_seq_and_session_are_populated(self) -> None:
        data = make_edf([SignalSpec(
            "Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=2, samples=[0, 60],
        )], record_duration_s=1.0)
        result = self.decode(data)
        self.assertEqual([s.seq for s in result.samples], [0, 500])
        self.assertIsNotNone(result.session_id)
        self.assertEqual(
            result.session_id, identity.session_id("DATALOG/20260101", "01.01.26")
        )

    def test_out_of_range_values_contribute_nothing_and_are_named(self) -> None:
        """spo2's registry range is 50..100, so 0 is out of range. It must not
        be clamped to 50 and must not be emitted."""
        data = make_edf([SignalSpec(
            "SpO2", "%", 0.0, 100.0, 0, 100, samples_per_record=3, samples=[0, 95, 100],
        )])
        result = self.decode(data)
        values = [s.value for s in result.samples]
        self.assertEqual(len(values), 2)
        for value in values:
            self.assertGreaterEqual(value, 50.0)
        self.assertEqual(len(result.rejections), 1)
        self.assertIn("contributed nothing", result.rejections[0])
        self.assertIn("outside the registry range", result.rejections[0])

    def test_unrecognised_dimension_rejects_the_whole_signal(self) -> None:
        data = make_edf([SignalSpec(
            "Mask Pres", "hPa", 0.0, 50.0, 0, 100, samples_per_record=1, samples=[50],
        )])
        result = self.decode(data)
        self.assertEqual(result.samples, [])
        self.assertEqual(len(result.rejections), 1)
        self.assertIn("hPa", result.rejections[0])

    def test_unmapped_labels_are_reported_not_dropped(self) -> None:
        data = make_edf([
            SignalSpec("Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=1, samples=[60]),
            SignalSpec("Resp Event", "", 0.0, 1.0, 0, 1, samples_per_record=1, samples=[0]),
        ])
        result = self.decode(data)
        self.assertEqual(result.unmapped_labels, ["Resp Event"])
        self.assertEqual(len(result.samples), 1)

    def test_a_truncated_file_keeps_what_it_has_and_names_the_shortfall(self) -> None:
        spec = SignalSpec("Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=2,
                          samples=[0, 60, 30, 90])
        data = make_edf([spec], record_count=2, truncate_bytes=2)
        result = self.decode(data)
        self.assertEqual(len(result.samples), 2)
        self.assertTrue(any("truncated after 1 of 2" in r for r in result.rejections))

    def test_a_malformed_header_is_one_named_rejection_not_a_traceback(self) -> None:
        result = self.decode(b"not an edf file at all")
        self.assertEqual(result.samples, [])
        self.assertEqual(len(result.rejections), 1)
        self.assertIn("shorter than", result.rejections[0])

    def test_counts_report_per_type(self) -> None:
        data = make_edf([
            SignalSpec("Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=2, samples=[0, 60]),
            SignalSpec("Pulse", "bpm", 0.0, 200.0, 0, 200, samples_per_record=1, samples=[60]),
        ])
        result = self.decode(data)
        self.assertEqual(result.counts, {"leak": 2, "hr": 1})

    def test_decoding_is_reproducible(self) -> None:
        """Replay safety: the same bytes must give the same identities, or the
        append-only store gains duplicates it cannot detect."""
        data = make_edf([SignalSpec(
            "Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=2, samples=[0, 60],
        )])
        first = self.decode(data)
        second = self.decode(data)
        self.assertEqual(
            [(s.session_id, s.type_name, s.seq) for s in first.samples],
            [(s.session_id, s.type_name, s.seq) for s in second.samples],
        )

    def test_caller_limit_stops_decoding_and_says_so(self) -> None:
        spec = SignalSpec("Leak", "L/min", 0.0, 120.0, 0, 120, samples_per_record=4,
                          samples=[0, 30, 60, 90])
        result = self.decode(make_edf([spec]), max_samples_per_type=2)
        self.assertEqual(len(result.samples), 2)
        self.assertTrue(any("at the caller's limit" in r for r in result.rejections))

    def test_no_sql_or_network_in_this_package(self) -> None:
        """Scope discipline, asserted rather than trusted."""
        root = Path(__file__).resolve().parents[1]
        forbidden = ("psycopg", "sqlalchemy", "INSERT INTO", "DELETE FROM", "socket", "urllib")
        for path in sorted(root.rglob("*.py")):
            if path.name == "test_semantics.py":
                continue
            text = path.read_text(encoding="utf-8")
            for needle in forbidden:
                with self.subTest(path=path.name, needle=needle):
                    self.assertNotIn(needle, text)


if __name__ == "__main__":
    unittest.main()
