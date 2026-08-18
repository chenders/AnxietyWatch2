#!/usr/bin/env python3
"""Golden and simulator tests for review-only aircraft model proposals."""
import importlib.machinery
import json
import pathlib
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name("panel-aircraft-learn")
learn = importlib.machinery.SourceFileLoader("panel_aircraft_learn", str(SCRIPT)).load_module()


def label(at, outcome="human_heard", event=None, eligible=True, provenance="hue_button"):
    return {"observed_at": at, "event_id": event or f"e-{at}", "label": outcome,
            "eligible_for_training": eligible, "provenance": provenance}


def flight(at, score, hex_code="ABC123"):
    return {"at": at, "noise_index": score, "identity": {"hex": hex_code}}


def simulated(outcome="human", positives=18, controls=60, positive_score=65, control_score=35):
    name = "cat_scared_top_priority" if outcome == "cat" else "human_heard"
    labels, passes = [], []
    # Interleave chronology so each 60/20/20 partition contains both classes.
    positive_slots = {int(i * controls / positives) for i in range(positives)}
    positive_number = 0
    for i in range(controls):
        base = i * 2000.0
        if i in positive_slots:
            labels.append(label(base + 100.0, name, f"{outcome}-{positive_number}"))
            passes.append(flight(base + 100.0, positive_score, f"P{positive_number:05d}"))
            positive_number += 1
        passes.append(flight(base + 1000.0, control_score, f"N{i:05d}"))
    return labels, passes


class Eligibility(unittest.TestCase):
    def test_synthetic_and_ineligible_rows_never_enter_training(self):
        rows = [label(1), label(2, provenance="synthetic"), label(3, eligible=False)]
        self.assertEqual([r["event_id"] for r in learn.eligible_labels(rows, "human")], ["e-1"])

    def test_cat_evidence_stays_separate_but_implies_human_heard(self):
        """Owner-stated semantics: a cat event encompasses a human-heard
        episode ("the two presses also encompass an episode where i heard it
        first"). So the cat outcome trains ONLY on cat labels, while the
        human outcome includes both -- separate gates, never collapsed the
        other way."""
        rows = [label(1), label(2, "cat_scared_top_priority")]
        self.assertEqual(len(learn.eligible_labels(rows, "human")), 2)
        self.assertEqual([r["event_id"] for r in learn.eligible_labels(rows, "cat")],
                         ["e-2"])

    def test_new_double_press_schema_contributes_to_both_outcomes(self):
        row = label(2, "cat_scared_top_priority")
        row["outcomes"] = ["human_heard", "cat_scared_top_priority"]
        self.assertEqual(len(learn.eligible_labels([row], "human")), 1)
        self.assertEqual(len(learn.eligible_labels([row], "cat")), 1)

    def test_warning_unheard_never_trains_a_heard_model(self):
        # The hold gesture says "nothing was heard". It must not appear as a
        # positive for either outcome -- silence scored as noise would be the
        # exact inversion of what the button reported.
        row = label(1)
        row["label"] = "warning_unheard"
        row["outcomes"] = ["warning_unheard"]
        self.assertEqual(learn.eligible_labels([row], "human"), [])
        self.assertEqual(learn.eligible_labels([row], "cat"), [])

    def test_duplicate_event_id_counts_once(self):
        rows = [label(1, event="same"), label(2, event="same")]
        self.assertEqual(len(learn.eligible_labels(rows, "human")), 1)

    def test_operator_reports_are_real_training_evidence(self):
        rows = [label(1, provenance="operator_report")]
        self.assertEqual(len(learn.eligible_labels(rows, "human")), 1)

    def test_legacy_double_press_counts_for_the_human_outcome_too(self):
        # A double press has always meant BOTH outcomes; legacy rows lack the
        # explicit outcomes list but carry press_count=2.
        row = label(2, "cat_scared_top_priority")
        row["press_count"] = 2
        self.assertEqual(len(learn.eligible_labels([row], "human")), 1)
        self.assertEqual(len(learn.eligible_labels([row], "cat")), 1)

    def test_corroborations_never_train(self):
        row = label(1)
        row["corroborates"] = "hue-123-1"
        self.assertEqual(learn.eligible_labels([row], "human"), [])


class WarningReviewExamples(unittest.TestCase):
    def test_exact_prediction_score_becomes_human_example(self):
        warnings = [{"warning_id": "w1", "at": 1000.0, "hex": "A11B00",
                     "prediction": {"noise_index": 58.4}}]
        reviews = [{"warning_id": "w1", "reviewed_at": 1100.0,
                    "verdict": "accurate"}]
        rows = learn.reviewed_warning_examples(warnings, reviews)
        self.assertEqual(len(rows), 1)
        self.assertTrue(rows[0]["positive"])
        self.assertAlmostEqual(rows[0]["score"], 58.4)

    def test_inaccurate_is_explicit_negative(self):
        rows = learn.reviewed_warning_examples(
            [{"warning_id": "w1", "at": 1000.0,
              "prediction": {"noise_index": 58.4}}],
            [{"warning_id": "w1", "reviewed_at": 1100.0,
              "verdict": "inaccurate"}])
        self.assertFalse(rows[0]["positive"])

    def test_recovered_warning_without_original_score_is_not_invented(self):
        warnings = [{"warning_id": "w1", "at": 1000.0, "prediction": None,
                     "matched_pass": {"noise_index": 48.9}}]
        reviews = [{"warning_id": "w1", "reviewed_at": 1100.0,
                    "verdict": "accurate"}]
        self.assertEqual(learn.reviewed_warning_examples(warnings, reviews), [])

    def test_unknown_review_contributes_nothing(self):
        rows = learn.reviewed_warning_examples(
            [{"warning_id": "w1", "at": 1000.0,
              "prediction": {"noise_index": 58.4}}],
            [{"warning_id": "w1", "reviewed_at": 1100.0,
              "verdict": "unknown"}])
        self.assertEqual(rows, [])

    def test_latest_review_wins(self):
        rows = learn.reviewed_warning_examples(
            [{"warning_id": "w1", "at": 1000.0,
              "prediction": {"noise_index": 58.4}}],
            [{"warning_id": "w1", "reviewed_at": 1100.0, "verdict": "accurate"},
             {"warning_id": "w1", "reviewed_at": 1200.0, "verdict": "inaccurate"}])
        self.assertFalse(rows[0]["positive"])


class ControlsAndSplits(unittest.TestCase):
    def test_controls_are_deterministic_and_exclude_label_neighbourhood(self):
        labels = [label(1000)]
        passes = [flight(900, 70), flight(2000, 30), flight(2100, 31), flight(3000, 32)]
        first = learn.examples(labels, passes)
        second = learn.examples(labels, list(reversed(passes)))
        self.assertEqual(first, second)
        controls = [r for r in first if not r["positive"]]
        self.assertEqual([r["at"] for r in controls], [2000.0, 3000.0])

    def test_split_is_time_ordered_not_random(self):
        rows = [{"at": float(i)} for i in range(10)]
        train, validation, test = learn.split_time(rows)
        self.assertLess(train[-1]["at"], validation[0]["at"])
        self.assertLess(validation[-1]["at"], test[0]["at"])

    def test_quiet_controls_exclude_real_label_times_across_outcomes(self):
        """A moment the human marked is not quiet for the cat model: an
        aircraft was audibly there. Controls must clear EVERY eligible real
        label time, not only the outcome under analysis."""
        human_only = label(2000.0, "human_heard", "h-1")
        cat_label = label(50000.0, "cat_scared_top_priority", "c-1")
        passes = [flight(2000.0, 70, "NEARHUM"),   # inside the human exclusion
                  flight(50000.0, 71, "NEARCAT"),
                  flight(20000.0, 30, "QUIET1"),
                  flight(30000.0, 31, "QUIET2")]
        all_rows = [human_only, cat_label]
        cat_selected = learn.eligible_labels(all_rows, "cat")
        rows = learn.examples(cat_selected, passes,
                              excluded_times=learn.real_label_times(all_rows))
        controls = [r["aircraft"] for r in rows if not r["positive"]]
        self.assertNotIn("NEARHUM", controls)
        self.assertEqual(sorted(controls), ["QUIET1", "QUIET2"])

    def test_analyze_wires_the_cross_outcome_exclusion(self):
        human_only = label(2000.0, "human_heard", "h-1")
        cat_label = label(50000.0, "cat_scared_top_priority", "c-1")
        passes = [flight(2000.0, 70, "NEARHUM"), flight(50000.0, 71, "NEARCAT"),
                  flight(20000.0, 30, "QUIET1")]
        result = learn.analyze("cat", [human_only, cat_label], passes)
        self.assertEqual(result["counts"]["quiet_controls"], 1)


class ProposalGates(unittest.TestCase):
    def test_no_real_data_reports_insufficient_with_named_blockers(self):
        proposal = learn.build([], [], 1234.0)
        self.assertEqual([o["verdict"] for o in proposal["outcomes"]],
                         ["INSUFFICIENT", "INSUFFICIENT"])
        self.assertTrue(all(o["blockers"] for o in proposal["outcomes"]))
        self.assertFalse(proposal["automatic_promotion"])

    def test_simulated_separable_data_produces_review_only_human_proposal(self):
        labels, passes = simulated()
        proposal = learn.build(labels, passes, 1234.0)
        human = proposal["outcomes"][0]
        self.assertEqual(human["verdict"], "PROPOSAL")
        self.assertGreaterEqual(human["test"]["recall"], 0.7)
        self.assertLessEqual(human["test"]["false_positive_rate"], 0.35)
        self.assertTrue(proposal["promotion"]["requires_manual_review"])
        self.assertFalse(proposal["promotion"]["settings_changed"])

    def test_cat_requires_its_own_evidence(self):
        labels, passes = simulated(outcome="human")
        proposal = learn.build(labels, passes, 1234.0)
        self.assertEqual(proposal["outcomes"][0]["verdict"], "PROPOSAL")
        self.assertEqual(proposal["outcomes"][1]["verdict"], "INSUFFICIENT")

    def test_poor_validation_is_refused(self):
        labels, passes = simulated(positive_score=35, control_score=65)
        human = learn.build(labels, passes, 1234.0)["outcomes"][0]
        self.assertEqual(human["verdict"], "INSUFFICIENT")
        self.assertTrue(any("recall" in blocker or "false_positive" in blocker
                            for blocker in human["blockers"]))

    def test_output_is_append_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "proposals.jsonl"
            proposal = learn.build([], [], 1234.0)
            learn.append_proposal(path, proposal)
            learn.append_proposal(path, proposal)
            self.assertEqual(len(path.read_text().splitlines()), 2)
            self.assertEqual(json.loads(path.read_text().splitlines()[0])["proposal_id"],
                             proposal["proposal_id"])


if __name__ == "__main__":
    unittest.main()
