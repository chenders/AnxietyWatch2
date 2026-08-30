#!/usr/bin/env python3
"""Verify the generation-one Apple-client parity ledger without third-party packages.

GEN1-PARITY.yaml intentionally uses JSON syntax, which is a valid YAML 1.2 subset. This keeps the
repository verification command runnable with the Python standard library while allowing later YAML
tooling to consume the same file.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import NoReturn


EXPECTED_TYPES = {
    "HKCategoryTypeIdentifierSleepAnalysis",
    "HKCharacteristicTypeIdentifierBiologicalSex",
    "HKCharacteristicTypeIdentifierDateOfBirth",
    "HKClinicalTypeIdentifierLabResultRecord",
    "HKDataTypeIdentifierHeartbeatSeries",
    "HKQuantityTypeIdentifierActiveEnergyBurned",
    "HKQuantityTypeIdentifierAppleExerciseTime",
    "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
    "HKQuantityTypeIdentifierAppleWalkingSteadiness",
    "HKQuantityTypeIdentifierAtrialFibrillationBurden",
    "HKQuantityTypeIdentifierBloodGlucose",
    "HKQuantityTypeIdentifierBloodPressureDiastolic",
    "HKQuantityTypeIdentifierBloodPressureSystolic",
    "HKQuantityTypeIdentifierBodyMass",
    "HKQuantityTypeIdentifierBodyTemperature",
    "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
    "HKQuantityTypeIdentifierHeadphoneAudioExposure",
    "HKQuantityTypeIdentifierHeartRate",
    "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
    "HKQuantityTypeIdentifierOxygenSaturation",
    "HKQuantityTypeIdentifierPhysicalEffort",
    "HKQuantityTypeIdentifierRespiratoryRate",
    "HKQuantityTypeIdentifierRestingHeartRate",
    "HKQuantityTypeIdentifierStepCount",
    "HKQuantityTypeIdentifierTimeInDaylight",
    "HKQuantityTypeIdentifierVO2Max",
    "HKQuantityTypeIdentifierWalkingAsymmetryPercentage",
    "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage",
    "HKQuantityTypeIdentifierWalkingHeartRateAverage",
    "HKQuantityTypeIdentifierWalkingSpeed",
    "HKQuantityTypeIdentifierWalkingStepLength",
    "HKWorkoutTypeIdentifier",
}

EXPECTED_SOURCE_SETS = {
    "authorization": {
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
        "HKQuantityTypeIdentifierHeartRate",
        "HKQuantityTypeIdentifierRestingHeartRate",
        "HKQuantityTypeIdentifierRespiratoryRate",
        "HKQuantityTypeIdentifierOxygenSaturation",
        "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
        "HKQuantityTypeIdentifierStepCount",
        "HKQuantityTypeIdentifierActiveEnergyBurned",
        "HKQuantityTypeIdentifierAppleExerciseTime",
        "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
        "HKQuantityTypeIdentifierBloodPressureSystolic",
        "HKQuantityTypeIdentifierBloodPressureDiastolic",
        "HKQuantityTypeIdentifierBloodGlucose",
        "HKQuantityTypeIdentifierVO2Max",
        "HKQuantityTypeIdentifierWalkingHeartRateAverage",
        "HKQuantityTypeIdentifierHeadphoneAudioExposure",
        "HKQuantityTypeIdentifierAppleWalkingSteadiness",
        "HKQuantityTypeIdentifierAtrialFibrillationBurden",
        "HKQuantityTypeIdentifierWalkingSpeed",
        "HKQuantityTypeIdentifierWalkingStepLength",
        "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage",
        "HKQuantityTypeIdentifierWalkingAsymmetryPercentage",
        "HKQuantityTypeIdentifierTimeInDaylight",
        "HKQuantityTypeIdentifierPhysicalEffort",
        "HKCategoryTypeIdentifierSleepAnalysis",
        "HKWorkoutTypeIdentifier",
        "HKCharacteristicTypeIdentifierDateOfBirth",
        "HKCharacteristicTypeIdentifierBiologicalSex",
        "HKClinicalTypeIdentifierLabResultRecord",
    },
    "observer_or_anchor": {
        "HKQuantityTypeIdentifierHeartRate",
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
        "HKQuantityTypeIdentifierOxygenSaturation",
        "HKQuantityTypeIdentifierRespiratoryRate",
        "HKQuantityTypeIdentifierRestingHeartRate",
        "HKQuantityTypeIdentifierVO2Max",
        "HKQuantityTypeIdentifierWalkingHeartRateAverage",
        "HKQuantityTypeIdentifierAppleWalkingSteadiness",
        "HKQuantityTypeIdentifierBloodPressureSystolic",
        "HKQuantityTypeIdentifierBloodPressureDiastolic",
        "HKQuantityTypeIdentifierBloodGlucose",
        "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
        "HKQuantityTypeIdentifierHeadphoneAudioExposure",
        "HKCategoryTypeIdentifierSleepAnalysis",
    },
    "local_mirror": {
        "HKQuantityTypeIdentifierHeartRate",
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
        "HKQuantityTypeIdentifierRestingHeartRate",
        "HKQuantityTypeIdentifierRespiratoryRate",
        "HKQuantityTypeIdentifierOxygenSaturation",
        "HKQuantityTypeIdentifierBloodGlucose",
        "HKQuantityTypeIdentifierBodyTemperature",
        "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
        "HKQuantityTypeIdentifierBloodPressureSystolic",
        "HKQuantityTypeIdentifierBloodPressureDiastolic",
        "HKQuantityTypeIdentifierBodyMass",
        "HKCategoryTypeIdentifierSleepAnalysis",
    },
    "snapshot_input": {
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
        "HKQuantityTypeIdentifierRestingHeartRate",
        "HKCategoryTypeIdentifierSleepAnalysis",
        "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
        "HKQuantityTypeIdentifierRespiratoryRate",
        "HKQuantityTypeIdentifierOxygenSaturation",
        "HKQuantityTypeIdentifierStepCount",
        "HKQuantityTypeIdentifierActiveEnergyBurned",
        "HKQuantityTypeIdentifierAppleExerciseTime",
        "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
        "HKQuantityTypeIdentifierBloodPressureSystolic",
        "HKQuantityTypeIdentifierBloodPressureDiastolic",
        "HKQuantityTypeIdentifierBloodGlucose",
        "HKQuantityTypeIdentifierVO2Max",
        "HKQuantityTypeIdentifierWalkingHeartRateAverage",
        "HKQuantityTypeIdentifierAppleWalkingSteadiness",
        "HKQuantityTypeIdentifierAtrialFibrillationBurden",
        "HKQuantityTypeIdentifierHeadphoneAudioExposure",
        "HKQuantityTypeIdentifierWalkingSpeed",
        "HKQuantityTypeIdentifierWalkingStepLength",
        "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage",
        "HKQuantityTypeIdentifierWalkingAsymmetryPercentage",
        "HKQuantityTypeIdentifierTimeInDaylight",
        "HKQuantityTypeIdentifierPhysicalEffort",
    },
    "ui_or_feature_documentation": {
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
        "HKQuantityTypeIdentifierHeartRate",
        "HKQuantityTypeIdentifierRestingHeartRate",
        "HKQuantityTypeIdentifierRespiratoryRate",
        "HKQuantityTypeIdentifierOxygenSaturation",
        "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
        "HKQuantityTypeIdentifierStepCount",
        "HKQuantityTypeIdentifierActiveEnergyBurned",
        "HKQuantityTypeIdentifierAppleExerciseTime",
        "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
        "HKQuantityTypeIdentifierBloodPressureSystolic",
        "HKQuantityTypeIdentifierBloodPressureDiastolic",
        "HKQuantityTypeIdentifierVO2Max",
        "HKQuantityTypeIdentifierWalkingHeartRateAverage",
        "HKQuantityTypeIdentifierAppleWalkingSteadiness",
        "HKQuantityTypeIdentifierAtrialFibrillationBurden",
        "HKQuantityTypeIdentifierWalkingSpeed",
        "HKQuantityTypeIdentifierWalkingStepLength",
        "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage",
        "HKQuantityTypeIdentifierWalkingAsymmetryPercentage",
        "HKCategoryTypeIdentifierSleepAnalysis",
        "HKClinicalTypeIdentifierLabResultRecord",
    },
    "queried_elsewhere": {"HKDataTypeIdentifierHeartbeatSeries"},
}

EXPECTED_CAPABILITIES = {
    "journal.anxiety_rating",
    "journal.notes",
    "journal.tags",
    "journal.song",
    "journal.express_mode",
    "medication.dose",
    "checkin.dose_followup",
    "checkin.random",
    "watch.quick_log",
    "viewer.today_dashboard",
    "viewer.journal_history",
    "viewer.medications",
    "viewer.trends_history",
    "viewer.clinical_reports",
    "viewer.health_access_sync",
    "viewer.settings_diagnostics",
    "healthkit.source_sample_proxy",
    "ble.polar_h10",
    "ble.emay_oura_cuff",
    "analysis.baselines_trends_cns_correlations",
    "storage.health_snapshots",
    "repair.rebuild_refresh_snapshots",
    "restore.store_replacement",
    "sync.bidirectional_crdt_wall_clock_cursor",
    "reports.local_pdf_generation",
    "imports.device_csv_and_local_clinical_records",
}

TYPE_CLASSIFICATIONS = {
    "proxy_sample",
    "proxy_category",
    "context_only",
    "blocked_registry",
    "excluded",
}
CAPABILITY_DECISIONS = {
    "retain_phone",
    "retain_watch",
    "replace_pi_projection",
    "replace_thin_proxy",
    "remove",
}
FIXED_FIXTURE_DATE = re.compile(r"^2025-01-15T\d{2}:\d{2}:\d{2}Z$")


def fail(message: str) -> NoReturn:
    print(f"ios-parity: FAIL — {message}", file=sys.stderr)
    raise SystemExit(1)


def load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(f"missing {path}")
    except json.JSONDecodeError as error:
        fail(f"{path} is not valid JSON-compatible YAML: {error}")


def require_record_fields(record: dict[str, object], fields: set[str], label: str) -> None:
    missing = fields - record.keys()
    if missing:
        fail(f"{label} is missing fields: {', '.join(sorted(missing))}")
    for field in fields:
        value = record[field]
        if value is None or value == "" or value == []:
            fail(f"{label}.{field} must be non-empty")


def verify_source_sets(data: dict[str, object]) -> None:
    source_sets = data.get("source_sets")
    if not isinstance(source_sets, dict):
        fail("source_sets must be an object")
    if set(source_sets) != set(EXPECTED_SOURCE_SETS):
        fail("source_sets names differ from the reviewed generation-one sets")
    for name, expected in EXPECTED_SOURCE_SETS.items():
        values = source_sets[name]
        if not isinstance(values, list) or not all(isinstance(value, str) for value in values):
            fail(f"source_sets.{name} must be a string list")
        if len(values) != len(set(values)):
            fail(f"source_sets.{name} contains duplicate types")
        actual = set(values)
        if actual != expected:
            fail(
                f"source_sets.{name} mismatch; missing={sorted(expected - actual)}, "
                f"unexpected={sorted(actual - expected)}"
            )


def verify_healthkit_types(data: dict[str, object]) -> None:
    records = data.get("healthkit_types")
    if not isinstance(records, list):
        fail("healthkit_types must be a list")
    ids: list[str] = []
    for index, value in enumerate(records):
        if not isinstance(value, dict):
            fail(f"healthkit_types[{index}] must be an object")
        require_record_fields(
            value,
            {"id", "name", "kind", "classification", "owner", "reason", "evidence", "gate"},
            f"healthkit_types[{index}]",
        )
        identifier = value["id"]
        if not isinstance(identifier, str):
            fail(f"healthkit_types[{index}].id must be a string")
        ids.append(identifier)
        if value["classification"] not in TYPE_CLASSIFICATIONS:
            fail(f"{identifier} has unknown classification {value['classification']!r}")
        evidence = value["evidence"]
        if not isinstance(evidence, list) or not all(
            isinstance(item, str) and ":" in item and not item.startswith("/") for item in evidence
        ):
            fail(f"{identifier} evidence must contain relative path:line entries")
    if len(ids) != len(set(ids)):
        fail("healthkit_types contains duplicate IDs")
    actual = set(ids)
    if actual != EXPECTED_TYPES:
        fail(f"HealthKit parity mismatch; missing={sorted(EXPECTED_TYPES - actual)}, unexpected={sorted(actual - EXPECTED_TYPES)}")
    classifications = {record["classification"] for record in records}
    if classifications != TYPE_CLASSIFICATIONS:
        fail(f"ledger must exercise every classification; absent={sorted(TYPE_CLASSIFICATIONS - classifications)}")

    source_sets = data["source_sets"]
    discovered = set().union(*(set(values) for values in source_sets.values()))
    if discovered != actual:
        fail(f"source-set union differs from type ledger; missing entries={sorted(discovered - actual)}")


def verify_capabilities(data: dict[str, object]) -> None:
    records = data.get("capabilities")
    if not isinstance(records, list):
        fail("capabilities must be a list")
    ids: list[str] = []
    for index, value in enumerate(records):
        if not isinstance(value, dict):
            fail(f"capabilities[{index}] must be an object")
        require_record_fields(value, {"id", "decision", "owner", "reason", "evidence"}, f"capabilities[{index}]")
        identifier = value["id"]
        if not isinstance(identifier, str):
            fail(f"capabilities[{index}].id must be a string")
        ids.append(identifier)
        if value["decision"] not in CAPABILITY_DECISIONS:
            fail(f"{identifier} has unknown decision {value['decision']!r}")
        evidence = value["evidence"]
        if not isinstance(evidence, list) or not all(
            isinstance(item, str) and ":" in item and not item.startswith("/") for item in evidence
        ):
            fail(f"{identifier} evidence must contain relative path:line entries")
    if len(ids) != len(set(ids)):
        fail("capabilities contains duplicate IDs")
    actual = set(ids)
    if actual != EXPECTED_CAPABILITIES:
        fail(
            f"capability parity mismatch; missing={sorted(EXPECTED_CAPABILITIES - actual)}, "
            f"unexpected={sorted(actual - EXPECTED_CAPABILITIES)}"
        )


def verify_aggregates(data: dict[str, object]) -> None:
    aggregates = data.get("generation_one_aggregates")
    if not isinstance(aggregates, dict):
        fail("generation_one_aggregates must be an object")
    require_record_fields(aggregates, {"decision", "owner", "reason", "evidence", "fields"}, "generation_one_aggregates")
    fields = aggregates["fields"]
    if not isinstance(fields, list) or len(fields) < 40 or len(fields) != len(set(fields)):
        fail("generation_one_aggregates.fields must retain the reviewed unique snapshot field inventory")
    if aggregates["decision"] != "exclude_from_phone_transport_and_replace_with_pi_projections":
        fail("generation-one aggregates must remain separate from source-sample transport")


def verify_fixtures(root: Path, data: dict[str, object]) -> None:
    fixtures = data.get("fixtures")
    if not isinstance(fixtures, list) or len(fixtures) < 3:
        fail("at least three sanitized fixture paths are required")
    for relative in fixtures:
        if not isinstance(relative, str) or relative.startswith("/") or ".." in Path(relative).parts:
            fail(f"invalid fixture path {relative!r}")
        path = root / relative
        fixture = load_json(path)
        if not isinstance(fixture, dict) or fixture.get("fixture") is not True:
            fail(f"{relative} must be marked fixture=true")
        serialized = json.dumps(fixture, sort_keys=True)
        if "com.example.fixture" not in serialized and fixture.get("record_type") != "journal_entry":
            fail(f"{relative} must use com.example.fixture provenance")
        timestamps = [value for key, value in fixture.items() if key.endswith("_at")]
        if not timestamps or not all(isinstance(value, str) and FIXED_FIXTURE_DATE.match(value) for value in timestamps):
            fail(f"{relative} must use fixed synthetic UTC timestamps on 2025-01-15")
        lowered = serialized.lower()
        for forbidden in ("password", "credential", "bearer ", "https://", "private/var/mobile"):
            if forbidden in lowered:
                fail(f"{relative} contains forbidden non-fixture material: {forbidden}")


def verify_report(root: Path) -> None:
    report = root / "clients/apple/GEN1-PARITY.md"
    try:
        text = report.read_text(encoding="utf-8")
    except FileNotFoundError:
        fail(f"missing {report}")
    required = (
        "32 distinct HealthKit types",
        "Source samples are not generation-one aggregates",
        "Body mass appears in the mirror",
        "Heartbeat series",
        "watchOS Quick Log",
        "Removed or replaced",
        "Sanitized fixtures",
    )
    absent = [phrase for phrase in required if phrase not in text]
    if absent:
        fail(f"human-readable report is missing required findings: {absent}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: verify-ios-parity.py clients/apple/GEN1-PARITY.yaml")
    root = Path(__file__).resolve().parent.parent
    ledger_path = Path(sys.argv[1])
    if not ledger_path.is_absolute():
        ledger_path = root / ledger_path
    loaded = load_json(ledger_path)
    if not isinstance(loaded, dict):
        fail("ledger root must be an object")
    if loaded.get("schema_version") != 1:
        fail("schema_version must be 1")
    verify_source_sets(loaded)
    verify_healthkit_types(loaded)
    verify_capabilities(loaded)
    verify_aggregates(loaded)
    verify_fixtures(root, loaded)
    verify_report(root)
    print("ios-parity: PASS — 32 HealthKit types, 26 capabilities, 49 aggregates, and 3 sanitized fixtures verified")


if __name__ == "__main__":
    main()
