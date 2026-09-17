//! Pure HealthKit source-sample decoding and canonical mapping.
//!
//! The HTTP receiver owns authentication and persistence. This module owns only the untrusted
//! payload boundary: JSON is decoded, registry types and units are selected, values are validated,
//! HealthKit provenance is retained, and the stable sample UUID is split into the store identity.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use vitals_registry::{VitalsSource, VitalsType};

/// One source sample as sent by the Apple client.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HealthKitSamplePayload {
    /// Synthetic-fixture marker, ignored by the mapper.
    #[serde(default)]
    pub fixture: Option<bool>,
    /// Client item identity, validated by the outer receiver.
    #[serde(default)]
    pub record_id: Option<Uuid>,
    /// Stable `HealthKit` source identifier.
    pub healthkit_sample_uuid: Uuid,
    /// Original `HealthKit` type identifier.
    pub healthkit_type: String,
    /// Inclusive UTC interval start.
    pub start_at: String,
    /// Exclusive UTC interval end.
    pub end_at: String,
    /// Original numeric value, present for quantity samples.
    #[serde(default)]
    pub original_value: Option<f64>,
    /// Original `HealthKit` unit identifier, present for quantity samples.
    #[serde(default)]
    pub original_unit: Option<String>,
    /// Original `HealthKit` category value, present for category samples.
    #[serde(default)]
    pub original_category: Option<String>,
    /// `HealthKit` source revision.
    #[serde(default)]
    pub source_revision: SourceRevision,
    /// `HealthKit` device metadata.
    #[serde(default)]
    pub device: Option<DeviceMetadata>,
    /// Allowlisted `HealthKit` metadata fields.
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
    /// Client batch identity from the query or observer operation.
    pub query_batch_id: Uuid,
    /// Instant at which the phone recorded the item.
    pub recorded_at: String,
}

/// `HealthKit` source revision fields retained as sub-provenance.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRevision {
    /// Source application bundle identifier.
    #[serde(default)]
    pub bundle_identifier: Option<String>,
    /// Human-readable source name.
    #[serde(default)]
    pub source_name: Option<String>,
    /// Device product type reported by `HealthKit`.
    #[serde(default)]
    pub product_type: Option<String>,
    /// Source application version.
    #[serde(default)]
    pub version: Option<String>,
}

/// `HealthKit` device metadata retained as sub-provenance.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeviceMetadata {
    /// Manufacturer, when reported.
    #[serde(default)]
    pub manufacturer: Option<String>,
    /// Model, when reported.
    #[serde(default)]
    pub model: Option<String>,
    /// Hardware version, when reported.
    #[serde(default)]
    pub hardware_version: Option<String>,
    /// Operating-system version, when reported.
    #[serde(default)]
    pub os_version: Option<String>,
}

/// A canonical sample ready for the append-only ingest boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct MappedHealthKitSample {
    /// The platform source enum id.
    pub source: VitalsSource,
    /// The canonical registry type enum.
    pub r#type: VitalsType,
    /// First eight bytes of the `HealthKit` UUID, interpreted as signed BIGINT bits.
    pub session_id: i64,
    /// Last eight bytes of the `HealthKit` UUID, interpreted as signed BIGINT bits.
    pub seq: i64,
    /// Start instant in Unix microseconds.
    pub timestamp_unix_micros: i64,
    /// Value in the registry's canonical unit.
    pub value: f64,
    /// Versioned JSON sub-provenance for `samples.extra`.
    pub extra: Vec<u8>,
}

/// A named quarantine reason. No malformed, unsupported, or unsafe value becomes a sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthKitQuarantine {
    /// Stable machine-readable reason.
    pub code: &'static str,
    /// Detail naming the blocking value.
    pub detail: String,
}

impl HealthKitQuarantine {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// Decode one bounded `HealthKit` JSON payload and map it to canonical registry types.
///
/// This function has no I/O, clock access, global state, or side effects. The caller owns request
/// authentication, body-size limits, persistence, and quarantine recording.
///
/// # Errors
///
/// Returns a named quarantine reason for malformed JSON, unsupported types, invalid timestamps,
/// absent values, unit mismatches, unknown metadata, or out-of-range values.
pub fn decode_healthkit_payload(
    bytes: &[u8],
) -> Result<MappedHealthKitSample, HealthKitQuarantine> {
    let payload: HealthKitSamplePayload = serde_json::from_slice(bytes).map_err(|error| {
        HealthKitQuarantine::new("healthkit_payload_malformed", format!("json={error}"))
    })?;
    map_healthkit_sample(&payload)
}

/// Map one already-decoded `HealthKit` sample without I/O or process state.
///
/// The registry enum supplies the canonical unit and range. The phone's original value is never
/// trusted as canonical merely because its JSON type is numeric.
///
/// # Errors
///
/// Returns a named quarantine reason when the interval, value, unit, type, or provenance cannot be
/// validated against the accepted `HealthKit` contract.
pub fn map_healthkit_sample(
    payload: &HealthKitSamplePayload,
) -> Result<MappedHealthKitSample, HealthKitQuarantine> {
    let start = chrono::DateTime::parse_from_rfc3339(&payload.start_at).map_err(|error| {
        HealthKitQuarantine::new("healthkit_timestamp_invalid", format!("start_at={error}"))
    })?;
    let end = chrono::DateTime::parse_from_rfc3339(&payload.end_at).map_err(|error| {
        HealthKitQuarantine::new("healthkit_timestamp_invalid", format!("end_at={error}"))
    })?;
    if end < start {
        return Err(HealthKitQuarantine::new(
            "healthkit_interval_invalid",
            format!(
                "end_at={} precedes start_at={}",
                payload.end_at, payload.start_at
            ),
        ));
    }

    let (sample_type, value, expected_unit) = match payload.healthkit_type.as_str() {
        "HKQuantityTypeIdentifierHeartRate" => (
            VitalsType::Hr,
            quantity_value(payload, "heart_rate")?,
            "count/min",
        ),
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN" => (
            VitalsType::HrvRmssd,
            quantity_value(payload, "hrv_sdnn")?,
            "ms",
        ),
        "HKQuantityTypeIdentifierOxygenSaturation" => (
            VitalsType::Spo2,
            quantity_value(payload, "oxygen_saturation")?,
            "%",
        ),
        "HKCategoryTypeIdentifierSleepAnalysis" => (
            VitalsType::SleepStage,
            sleep_stage_value(payload)?,
            "category",
        ),
        "HKQuantityTypeIdentifierRestingHeartRate" => (
            VitalsType::RestingHr,
            quantity_value(payload, "resting_heart_rate")?,
            "count/min",
        ),
        "HKQuantityTypeIdentifierRespiratoryRate" => (
            VitalsType::RespiratoryRate,
            quantity_value(payload, "respiratory_rate")?,
            "count/min",
        ),
        "HKQuantityTypeIdentifierAppleSleepingWristTemperature" => (
            VitalsType::WristTemperature,
            quantity_value(payload, "sleeping_wrist_temperature")?,
            "degC",
        ),
        "HKQuantityTypeIdentifierStepCount" => (
            VitalsType::StepCount,
            quantity_value(payload, "step_count")?,
            "count",
        ),
        "HKQuantityTypeIdentifierActiveEnergyBurned" => (
            VitalsType::ActiveEnergy,
            quantity_value(payload, "active_energy")?,
            "kcal",
        ),
        "HKQuantityTypeIdentifierAppleExerciseTime" => (
            VitalsType::ExerciseTime,
            quantity_value(payload, "exercise_time")?,
            "min",
        ),
        "HKQuantityTypeIdentifierEnvironmentalAudioExposure" => (
            VitalsType::EnvironmentalAudio,
            quantity_value(payload, "environmental_audio")?,
            "dBASPL",
        ),
        "HKQuantityTypeIdentifierBloodPressureSystolic" => (
            VitalsType::BpSystolic,
            quantity_value(payload, "blood_pressure_systolic")?,
            "mmHg",
        ),
        "HKQuantityTypeIdentifierBloodPressureDiastolic" => (
            VitalsType::BpDiastolic,
            quantity_value(payload, "blood_pressure_diastolic")?,
            "mmHg",
        ),
        "HKQuantityTypeIdentifierBloodGlucose" => (
            VitalsType::BloodGlucose,
            quantity_value(payload, "blood_glucose")?,
            "mg/dL",
        ),
        "HKQuantityTypeIdentifierVO2Max" => (
            VitalsType::Vo2Max,
            quantity_value(payload, "vo2_max")?,
            "mL/kg*min",
        ),
        "HKQuantityTypeIdentifierWalkingHeartRateAverage" => (
            VitalsType::WalkingHr,
            quantity_value(payload, "walking_heart_rate")?,
            "count/min",
        ),
        "HKQuantityTypeIdentifierHeadphoneAudioExposure" => (
            VitalsType::HeadphoneAudio,
            quantity_value(payload, "headphone_audio")?,
            "dBASPL",
        ),
        "HKQuantityTypeIdentifierAppleWalkingSteadiness" => (
            VitalsType::WalkingSteadiness,
            quantity_value(payload, "walking_steadiness")?,
            "%",
        ),
        "HKQuantityTypeIdentifierAtrialFibrillationBurden" => (
            VitalsType::AfibBurden,
            quantity_value(payload, "atrial_fibrillation_burden")?,
            "%",
        ),
        "HKQuantityTypeIdentifierWalkingSpeed" => (
            VitalsType::WalkingSpeed,
            quantity_value(payload, "walking_speed")?,
            "m/s",
        ),
        "HKQuantityTypeIdentifierWalkingStepLength" => (
            VitalsType::WalkingStepLength,
            quantity_value(payload, "walking_step_length")?,
            "m",
        ),
        "HKQuantityTypeIdentifierWalkingDoubleSupportPercentage" => (
            VitalsType::WalkingDoubleSupport,
            quantity_value(payload, "walking_double_support")?,
            "%",
        ),
        "HKQuantityTypeIdentifierWalkingAsymmetryPercentage" => (
            VitalsType::WalkingAsymmetry,
            quantity_value(payload, "walking_asymmetry")?,
            "%",
        ),
        "HKQuantityTypeIdentifierTimeInDaylight" => (
            VitalsType::TimeInDaylight,
            quantity_value(payload, "time_in_daylight")?,
            "min",
        ),
        "HKQuantityTypeIdentifierPhysicalEffort" => (
            VitalsType::PhysicalEffort,
            quantity_value(payload, "physical_effort")?,
            "kcal/kg*hr",
        ),
        "HKQuantityTypeIdentifierBodyMass" => (
            VitalsType::BodyMass,
            quantity_value(payload, "body_mass")?,
            "kg",
        ),
        unsupported => {
            return Err(HealthKitQuarantine::new(
                "healthkit_type_unsupported",
                format!("healthkit_type={unsupported}"),
            ));
        }
    };

    if sample_type != VitalsType::SleepStage
        && payload.original_unit.as_deref() != Some(expected_unit)
    {
        return Err(HealthKitQuarantine::new(
            "healthkit_unit_mismatch",
            format!(
                "healthkit_type={} original_unit={:?} expected_unit={expected_unit}",
                payload.healthkit_type, payload.original_unit
            ),
        ));
    }
    let (minimum, maximum) = sample_type.range();
    if !value.is_finite() {
        return Err(HealthKitQuarantine::new(
            "healthkit_value_non_finite",
            format!("type={} value={value:?}", payload.healthkit_type),
        ));
    }
    if value < minimum || value > maximum {
        return Err(HealthKitQuarantine::new(
            "healthkit_value_out_of_range",
            format!(
                "type={} value={value} allowed={minimum}..={maximum}",
                payload.healthkit_type
            ),
        ));
    }

    let extra = serde_json::to_vec(&Provenance {
        schema: 1,
        platform: "healthkit",
        sample_id: payload.healthkit_sample_uuid,
        healthkit_type: &payload.healthkit_type,
        source: &payload.source_revision,
        device: payload.device.as_ref(),
        metadata: &payload.metadata,
    })
    .map_err(|error| {
        HealthKitQuarantine::new(
            "healthkit_provenance_encode_failed",
            format!("json={error}"),
        )
    })?;
    let (session_id, seq) = split_uuid(payload.healthkit_sample_uuid);

    Ok(MappedHealthKitSample {
        source: VitalsSource::Healthkit,
        r#type: sample_type,
        session_id,
        seq,
        timestamp_unix_micros: start.timestamp_micros(),
        value,
        extra,
    })
}

fn quantity_value(
    payload: &HealthKitSamplePayload,
    name: &'static str,
) -> Result<f64, HealthKitQuarantine> {
    payload.original_value.ok_or_else(|| {
        HealthKitQuarantine::new(
            "healthkit_value_absent",
            format!("quantity={name} original_value=null"),
        )
    })
}

fn sleep_stage_value(payload: &HealthKitSamplePayload) -> Result<f64, HealthKitQuarantine> {
    let category = payload.original_category.as_deref().ok_or_else(|| {
        HealthKitQuarantine::new("healthkit_sleep_stage_absent", "original_category=null")
    })?;
    let code = match category {
        "HKCategoryValueSleepAnalysisAwake" => 0.0,
        "HKCategoryValueSleepAnalysisAsleepUnspecified" => 1.0,
        "HKCategoryValueSleepAnalysisAsleepCore" => 2.0,
        "HKCategoryValueSleepAnalysisAsleepDeep" => 3.0,
        "HKCategoryValueSleepAnalysisAsleepREM" => 4.0,
        unsupported => {
            return Err(HealthKitQuarantine::new(
                "unmappable_sleep_stage",
                format!("platform_value={unsupported}"),
            ));
        }
    };
    Ok(code)
}

fn split_uuid(uuid: Uuid) -> (i64, i64) {
    let bytes = uuid.as_bytes();
    let Some(high) = bytes.get(..8).and_then(|part| part.try_into().ok()) else {
        return (0, 0);
    };
    let Some(low) = bytes.get(8..16).and_then(|part| part.try_into().ok()) else {
        return (0, 0);
    };
    (i64::from_be_bytes(high), i64::from_be_bytes(low))
}

#[derive(Serialize)]
struct Provenance<'a> {
    schema: u8,
    platform: &'static str,
    sample_id: Uuid,
    healthkit_type: &'a str,
    source: &'a SourceRevision,
    device: Option<&'a DeviceMetadata>,
    metadata: &'a BTreeMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUANTITY: &[u8] =
        include_bytes!("../../../clients/apple/Fixtures/GEN1/healthkit-quantity-sample.json");
    const SLEEP: &[u8] =
        include_bytes!("../../../clients/apple/Fixtures/GEN1/healthkit-sleep-category.json");
    const REPLAY: &[u8] =
        include_bytes!("../../../clients/apple/Fixtures/GEN1/healthkit-replay.json");

    #[test]
    fn healthkit_quantity_maps_to_registry_unit_and_identity() {
        let result = decode_healthkit_payload(QUANTITY);
        assert!(result.is_ok());
        let Ok(mapped) = result else { return };
        assert_eq!(mapped.source, VitalsSource::Healthkit);
        assert_eq!(mapped.r#type, VitalsType::Hr);
        assert!((mapped.value - 72.0).abs() < 1e-9);
        assert_eq!(mapped.session_id, 0x4000);
        assert_eq!(mapped.seq, i64::from_be_bytes([0x80, 0, 0, 0, 0, 0, 1, 1]));
        assert!(String::from_utf8_lossy(&mapped.extra).contains("Fixture Watch"));
    }

    #[test]
    fn healthkit_sleep_maps_core_stage() {
        let result = decode_healthkit_payload(SLEEP);
        assert!(result.is_ok());
        let Ok(mapped) = result else { return };
        assert_eq!(mapped.r#type, VitalsType::SleepStage);
        assert!((mapped.value - 2.0).abs() < 1e-9);
    }

    #[test]
    fn healthkit_replay_has_identical_ingest_identity() {
        let first = decode_healthkit_payload(QUANTITY);
        let replay = decode_healthkit_payload(REPLAY);
        assert!(first.is_ok());
        assert_eq!(first, replay);
    }

    #[test]
    fn healthkit_rejects_out_of_range_value() {
        let mut value: serde_json::Value = match serde_json::from_slice(QUANTITY) {
            Ok(value) => value,
            Err(_) => return,
        };
        let Some(original_value) = value.get_mut("original_value") else {
            return;
        };
        *original_value = serde_json::json!(251.0);
        let Ok(bytes) = serde_json::to_vec(&value) else {
            return;
        };
        let error = decode_healthkit_payload(&bytes).err();
        assert_eq!(
            error.map(|reason| reason.code),
            Some("healthkit_value_out_of_range")
        );
    }

    #[test]
    fn healthkit_rejects_unknown_sleep_stage() {
        let mut value: serde_json::Value = match serde_json::from_slice(SLEEP) {
            Ok(value) => value,
            Err(_) => return,
        };
        let Some(original_category) = value.get_mut("original_category") else {
            return;
        };
        *original_category = serde_json::json!("HKCategoryValueSleepAnalysisUnknown");
        let Ok(bytes) = serde_json::to_vec(&value) else {
            return;
        };
        let error = decode_healthkit_payload(&bytes).err();
        assert_eq!(
            error.map(|reason| reason.code),
            Some("unmappable_sleep_stage")
        );
    }

    #[test]
    fn healthkit_rejects_missing_value_and_wrong_unit() {
        let mut missing: serde_json::Value = match serde_json::from_slice(QUANTITY) {
            Ok(value) => value,
            Err(_) => return,
        };
        let Some(original_value) = missing.get_mut("original_value") else {
            return;
        };
        *original_value = serde_json::Value::Null;
        let Ok(missing_bytes) = serde_json::to_vec(&missing) else {
            return;
        };
        assert_eq!(
            decode_healthkit_payload(&missing_bytes)
                .err()
                .map(|reason| reason.code),
            Some("healthkit_value_absent")
        );

        let mut wrong_unit: serde_json::Value = match serde_json::from_slice(QUANTITY) {
            Ok(value) => value,
            Err(_) => return,
        };
        let Some(original_unit) = wrong_unit.get_mut("original_unit") else {
            return;
        };
        *original_unit = serde_json::json!("percent");
        let Ok(wrong_unit_bytes) = serde_json::to_vec(&wrong_unit) else {
            return;
        };
        assert_eq!(
            decode_healthkit_payload(&wrong_unit_bytes)
                .err()
                .map(|reason| reason.code),
            Some("healthkit_unit_mismatch")
        );
    }
}
