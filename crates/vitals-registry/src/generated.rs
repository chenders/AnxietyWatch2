// GENERATED FROM registry.toml — DO NOT EDIT
//! Types and sources generated from `vitals/registry.toml`.
//!
//! Ids are permanent: never reuse, never renumber. A reused id silently
//! reinterprets every historical row that carries it.

/// A measurement type. The canonical unit and valid range come from the
/// registry, never from a literal at the call site.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VitalsType {
    /// `hr` — `bpm`, valid 20.0..250.0
    Hr = 1,
    /// `spo2` — `percent`, valid 50.0..100.0
    Spo2 = 2,
    /// `accel` — `g`, valid -16.0..16.0
    Accel = 3,
    /// `hrv_rmssd` — `ms`, valid 1.0..500.0
    HrvRmssd = 4,
    /// `rr_interval` — `seconds`, valid 0.2..3.0
    RrInterval = 5,
    /// `bp_systolic` — `mmHg`, valid 50.0..260.0
    BpSystolic = 6,
    /// `bp_diastolic` — `mmHg`, valid 30.0..160.0
    BpDiastolic = 7,
    /// `bp_map` — `mmHg`, valid 40.0..200.0
    BpMap = 8,
    /// `sleep_stage` — `stage_code`, valid 0.0..4.0
    SleepStage = 9,
    /// `resting_hr` — `bpm`, valid 20.0..250.0
    RestingHr = 10,
    /// `respiratory_rate` — `breaths/min`, valid 1.0..100.0
    RespiratoryRate = 11,
    /// `wrist_temperature` — `degC`, valid -20.0..60.0
    WristTemperature = 12,
    /// `step_count` — `count`, valid 0.0..1000000.0
    StepCount = 13,
    /// `active_energy` — `kcal`, valid 0.0..100000.0
    ActiveEnergy = 14,
    /// `exercise_time` — `min`, valid 0.0..1440.0
    ExerciseTime = 15,
    /// `environmental_audio` — `dBASPL`, valid 0.0..200.0
    EnvironmentalAudio = 16,
    /// `blood_glucose` — `mg/dL`, valid 1.0..1000.0
    BloodGlucose = 17,
    /// `vo2_max` — `mL/kg*min`, valid 1.0..150.0
    Vo2Max = 18,
    /// `walking_hr` — `bpm`, valid 20.0..250.0
    WalkingHr = 19,
    /// `headphone_audio` — `dBASPL`, valid 0.0..200.0
    HeadphoneAudio = 20,
    /// `walking_steadiness` — `percent`, valid 0.0..100.0
    WalkingSteadiness = 21,
    /// `afib_burden` — `percent`, valid 0.0..100.0
    AfibBurden = 22,
    /// `walking_speed` — `m/s`, valid 0.0..20.0
    WalkingSpeed = 23,
    /// `walking_step_length` — `m`, valid 0.0..5.0
    WalkingStepLength = 24,
    /// `walking_double_support` — `percent`, valid 0.0..100.0
    WalkingDoubleSupport = 25,
    /// `walking_asymmetry` — `percent`, valid 0.0..100.0
    WalkingAsymmetry = 26,
    /// `time_in_daylight` — `min`, valid 0.0..1440.0
    TimeInDaylight = 27,
    /// `physical_effort` — `kcal/kg*hr`, valid 0.0..100.0
    PhysicalEffort = 28,
    /// `body_mass` — `kg`, valid 0.1..500.0
    BodyMass = 29,
    /// `patient_flow` — `L/s`, valid -20.0..20.0
    PatientFlow = 30,
    /// `mask_pressure` — `cmH2O`, valid 0.0..50.0
    MaskPressure = 31,
    /// `mask_pressure_2s` — `cmH2O`, valid 0.0..50.0
    MaskPressure2s = 32,
    /// `inspiratory_pressure` — `cmH2O`, valid 0.0..50.0
    InspiratoryPressure = 33,
    /// `expiratory_pressure` — `cmH2O`, valid 0.0..50.0
    ExpiratoryPressure = 34,
    /// `leak` — `L/min`, valid 0.0..300.0
    Leak = 35,
    /// `tidal_volume` — `mL`, valid 0.0..5000.0
    TidalVolume = 36,
    /// `minute_ventilation` — `L/min`, valid 0.0..60.0
    MinuteVentilation = 37,
    /// `target_minute_ventilation` — `L/min`, valid 0.0..60.0
    TargetMinuteVentilation = 38,
    /// `ie_ratio` — `ratio`, valid 0.0..10.0
    IeRatio = 39,
    /// `snore_index` — `index`, valid 0.0..100.0
    SnoreIndex = 40,
    /// `flow_limitation` — `ratio`, valid 0.0..1.0
    FlowLimitation = 41,
    /// `inspiratory_duration` — `seconds`, valid 0.0..10.0
    InspiratoryDuration = 42,
    /// `trigger_cycle_event` — `code`, valid 0.0..255.0
    TriggerCycleEvent = 43,
}

/// Where a sample came from. Provenance, and it must survive aggregation.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VitalsSource {
    /// `emay` — `ble`
    Emay = 0,
    /// `polar_h10` — `ble`, service `0x180D`
    PolarH10 = 1,
    /// `healthkit` — `push`
    Healthkit = 2,
    /// `derived` — `internal`
    Derived = 3,
    /// `acconeer_xe125` — `usb_serial`
    AcconeerXe125 = 4,
    /// `resmed_air11` — `ble`, service `0xFD56`
    ResmedAir11 = 5,
    /// `ad_ua651ble` — `ble`, service `0x1810` (indication)
    AdUa651ble = 6,
    /// `polar_h10_emulator` — `internal`
    PolarH10Emulator = 7,
    /// `resmed_air11_sd` — `sd_edf`
    ResmedAir11Sd = 8,
}

/// How often a source is expected to deliver, declared in `registry.toml`.
/// The liveness law (SPEC-admin-console §2.5) compares a source's age against
/// this; a source without a declaration is a build-time error, never a
/// silently-`LIVE` card (§9 M0 criterion 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cadence {
    /// The source is expected to attempt at least once per this many seconds;
    /// silence longer than that degrades it to `UNKNOWN`/`FAILING`, never
    /// `LIVE`. The upper bound when the true cadence is variable.
    EverySeconds(u32),
    /// Event-driven — a manual upload, an SD card. State is the outcome and
    /// age of the last event, never time-degraded.
    OnDemand,
}

impl VitalsType {
    /// Valid range from `registry.toml`. Values outside it are **quarantined
    /// with a reason, never clamped** — a clamped reading is indistinguishable
    /// from a real one downstream.
    #[must_use]
    // Arms are one-per-variant on purpose. Several types share a unit or a
    // range, and collapsing them would make it non-obvious which type maps
    // where -- exactly the ambiguity this enum exists to remove.
    #[allow(clippy::match_same_arms)]
    pub const fn range(self) -> (f64, f64) {
        match self {
            Self::Hr => (20.0, 250.0),
            Self::Spo2 => (50.0, 100.0),
            Self::Accel => (-16.0, 16.0),
            Self::HrvRmssd => (1.0, 500.0),
            Self::RrInterval => (0.2, 3.0),
            Self::BpSystolic => (50.0, 260.0),
            Self::BpDiastolic => (30.0, 160.0),
            Self::BpMap => (40.0, 200.0),
            Self::SleepStage => (0.0, 4.0),
            Self::RestingHr => (20.0, 250.0),
            Self::RespiratoryRate => (1.0, 100.0),
            Self::WristTemperature => (-20.0, 60.0),
            Self::StepCount => (0.0, 1_000_000.0),
            Self::ActiveEnergy => (0.0, 100_000.0),
            Self::ExerciseTime => (0.0, 1_440.0),
            Self::EnvironmentalAudio => (0.0, 200.0),
            Self::BloodGlucose => (1.0, 1_000.0),
            Self::Vo2Max => (1.0, 150.0),
            Self::WalkingHr => (20.0, 250.0),
            Self::HeadphoneAudio => (0.0, 200.0),
            Self::WalkingSteadiness => (0.0, 100.0),
            Self::AfibBurden => (0.0, 100.0),
            Self::WalkingSpeed => (0.0, 20.0),
            Self::WalkingStepLength => (0.0, 5.0),
            Self::WalkingDoubleSupport => (0.0, 100.0),
            Self::WalkingAsymmetry => (0.0, 100.0),
            Self::TimeInDaylight => (0.0, 1_440.0),
            Self::PhysicalEffort => (0.0, 100.0),
            Self::BodyMass => (0.1, 500.0),
            Self::PatientFlow => (-20.0, 20.0),
            Self::MaskPressure => (0.0, 50.0),
            Self::MaskPressure2s => (0.0, 50.0),
            Self::InspiratoryPressure => (0.0, 50.0),
            Self::ExpiratoryPressure => (0.0, 50.0),
            Self::Leak => (0.0, 300.0),
            Self::TidalVolume => (0.0, 5_000.0),
            Self::MinuteVentilation => (0.0, 60.0),
            Self::TargetMinuteVentilation => (0.0, 60.0),
            Self::IeRatio => (0.0, 10.0),
            Self::SnoreIndex => (0.0, 100.0),
            Self::FlowLimitation => (0.0, 1.0),
            Self::InspiratoryDuration => (0.0, 10.0),
            Self::TriggerCycleEvent => (0.0, 255.0),
        }
    }

    /// Canonical unit. Conversion happens once, at the decoder, so the store
    /// holds exactly one unit per type.
    #[must_use]
    // Arms are one-per-variant on purpose. Several types share a unit or a
    // range, and collapsing them would make it non-obvious which type maps
    // where -- exactly the ambiguity this enum exists to remove.
    #[allow(clippy::match_same_arms)]
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Hr => "bpm",
            Self::Spo2 => "percent",
            Self::Accel => "g",
            Self::HrvRmssd => "ms",
            Self::RrInterval => "seconds",
            Self::BpSystolic => "mmHg",
            Self::BpDiastolic => "mmHg",
            Self::BpMap => "mmHg",
            Self::SleepStage => "stage_code",
            Self::RestingHr => "bpm",
            Self::RespiratoryRate => "breaths/min",
            Self::WristTemperature => "degC",
            Self::StepCount => "count",
            Self::ActiveEnergy => "kcal",
            Self::ExerciseTime => "min",
            Self::EnvironmentalAudio => "dBASPL",
            Self::BloodGlucose => "mg/dL",
            Self::Vo2Max => "mL/kg*min",
            Self::WalkingHr => "bpm",
            Self::HeadphoneAudio => "dBASPL",
            Self::WalkingSteadiness => "percent",
            Self::AfibBurden => "percent",
            Self::WalkingSpeed => "m/s",
            Self::WalkingStepLength => "m",
            Self::WalkingDoubleSupport => "percent",
            Self::WalkingAsymmetry => "percent",
            Self::TimeInDaylight => "min",
            Self::PhysicalEffort => "kcal/kg*hr",
            Self::BodyMass => "kg",
            Self::PatientFlow => "L/s",
            Self::MaskPressure => "cmH2O",
            Self::MaskPressure2s => "cmH2O",
            Self::InspiratoryPressure => "cmH2O",
            Self::ExpiratoryPressure => "cmH2O",
            Self::Leak => "L/min",
            Self::TidalVolume => "mL",
            Self::MinuteVentilation => "L/min",
            Self::TargetMinuteVentilation => "L/min",
            Self::IeRatio => "ratio",
            Self::SnoreIndex => "index",
            Self::FlowLimitation => "ratio",
            Self::InspiratoryDuration => "seconds",
            Self::TriggerCycleEvent => "code",
        }
    }

    /// Rejects an unknown id rather than transmuting it. The wire and the
    /// database both hand us plain integers.
    #[must_use]
    pub const fn from_id(id: i32) -> Option<Self> {
        match id {
            1 => Some(Self::Hr),
            2 => Some(Self::Spo2),
            3 => Some(Self::Accel),
            4 => Some(Self::HrvRmssd),
            5 => Some(Self::RrInterval),
            6 => Some(Self::BpSystolic),
            7 => Some(Self::BpDiastolic),
            8 => Some(Self::BpMap),
            9 => Some(Self::SleepStage),
            10 => Some(Self::RestingHr),
            11 => Some(Self::RespiratoryRate),
            12 => Some(Self::WristTemperature),
            13 => Some(Self::StepCount),
            14 => Some(Self::ActiveEnergy),
            15 => Some(Self::ExerciseTime),
            16 => Some(Self::EnvironmentalAudio),
            17 => Some(Self::BloodGlucose),
            18 => Some(Self::Vo2Max),
            19 => Some(Self::WalkingHr),
            20 => Some(Self::HeadphoneAudio),
            21 => Some(Self::WalkingSteadiness),
            22 => Some(Self::AfibBurden),
            23 => Some(Self::WalkingSpeed),
            24 => Some(Self::WalkingStepLength),
            25 => Some(Self::WalkingDoubleSupport),
            26 => Some(Self::WalkingAsymmetry),
            27 => Some(Self::TimeInDaylight),
            28 => Some(Self::PhysicalEffort),
            29 => Some(Self::BodyMass),
            30 => Some(Self::PatientFlow),
            31 => Some(Self::MaskPressure),
            32 => Some(Self::MaskPressure2s),
            33 => Some(Self::InspiratoryPressure),
            34 => Some(Self::ExpiratoryPressure),
            35 => Some(Self::Leak),
            36 => Some(Self::TidalVolume),
            37 => Some(Self::MinuteVentilation),
            38 => Some(Self::TargetMinuteVentilation),
            39 => Some(Self::IeRatio),
            40 => Some(Self::SnoreIndex),
            41 => Some(Self::FlowLimitation),
            42 => Some(Self::InspiratoryDuration),
            43 => Some(Self::TriggerCycleEvent),
            _ => None,
        }
    }

    /// The stable integer id. Permanent — see the module docs.
    #[must_use]
    pub const fn id(self) -> i32 {
        self as i32
    }
}

impl VitalsSource {
    /// Rejects an unknown id rather than transmuting it.
    #[must_use]
    pub const fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Emay),
            1 => Some(Self::PolarH10),
            2 => Some(Self::Healthkit),
            3 => Some(Self::Derived),
            4 => Some(Self::AcconeerXe125),
            5 => Some(Self::ResmedAir11),
            6 => Some(Self::AdUa651ble),
            7 => Some(Self::PolarH10Emulator),
            8 => Some(Self::ResmedAir11Sd),
            _ => None,
        }
    }

    /// The stable integer id. Permanent — see the module docs.
    #[must_use]
    pub const fn id(self) -> i32 {
        self as i32
    }

    /// The declared cadence from `registry.toml`, used by the liveness
    /// evaluator. Every source has one — the generator rejects a source
    /// without it, and arms are one-per-variant on purpose so a new source
    /// without a cadence cannot compile.
    #[must_use]
    // Arms are one-per-variant on purpose: several sources share a cadence,
    // and collapsing them would make it non-obvious which source maps where.
    #[allow(clippy::match_same_arms)]
    pub const fn expected_cadence(self) -> Cadence {
        match self {
            Self::Emay => Cadence::EverySeconds(60),
            Self::PolarH10 => Cadence::EverySeconds(1),
            Self::Healthkit => Cadence::EverySeconds(21600),
            Self::Derived => Cadence::OnDemand,
            Self::AcconeerXe125 => Cadence::OnDemand,
            Self::ResmedAir11 => Cadence::EverySeconds(1),
            Self::AdUa651ble => Cadence::OnDemand,
            Self::PolarH10Emulator => Cadence::EverySeconds(1),
            Self::ResmedAir11Sd => Cadence::OnDemand,
        }
    }

    /// Source-to-canonical conversion factor from `registry.toml`.
    #[must_use]
    pub const fn conversion(self, r#type: VitalsType) -> f64 {
        match (self, r#type) {
            (Self::ResmedAir11, VitalsType::Leak) => 60.0,
            (Self::ResmedAir11, VitalsType::TidalVolume) => 1_000.0,
            _ => 1.0,
        }
    }
}

/// Every type, for exhaustive iteration in tests and admin views.
pub const ALL_TYPES: [VitalsType; 43] = [
    VitalsType::Hr,
    VitalsType::Spo2,
    VitalsType::Accel,
    VitalsType::HrvRmssd,
    VitalsType::RrInterval,
    VitalsType::BpSystolic,
    VitalsType::BpDiastolic,
    VitalsType::BpMap,
    VitalsType::SleepStage,
    VitalsType::RestingHr,
    VitalsType::RespiratoryRate,
    VitalsType::WristTemperature,
    VitalsType::StepCount,
    VitalsType::ActiveEnergy,
    VitalsType::ExerciseTime,
    VitalsType::EnvironmentalAudio,
    VitalsType::BloodGlucose,
    VitalsType::Vo2Max,
    VitalsType::WalkingHr,
    VitalsType::HeadphoneAudio,
    VitalsType::WalkingSteadiness,
    VitalsType::AfibBurden,
    VitalsType::WalkingSpeed,
    VitalsType::WalkingStepLength,
    VitalsType::WalkingDoubleSupport,
    VitalsType::WalkingAsymmetry,
    VitalsType::TimeInDaylight,
    VitalsType::PhysicalEffort,
    VitalsType::BodyMass,
    VitalsType::PatientFlow,
    VitalsType::MaskPressure,
    VitalsType::MaskPressure2s,
    VitalsType::InspiratoryPressure,
    VitalsType::ExpiratoryPressure,
    VitalsType::Leak,
    VitalsType::TidalVolume,
    VitalsType::MinuteVentilation,
    VitalsType::TargetMinuteVentilation,
    VitalsType::IeRatio,
    VitalsType::SnoreIndex,
    VitalsType::FlowLimitation,
    VitalsType::InspiratoryDuration,
    VitalsType::TriggerCycleEvent,
];

/// Every source.
pub const ALL_SOURCES: [VitalsSource; 9] = [
    VitalsSource::Emay,
    VitalsSource::PolarH10,
    VitalsSource::Healthkit,
    VitalsSource::Derived,
    VitalsSource::AcconeerXe125,
    VitalsSource::ResmedAir11,
    VitalsSource::AdUa651ble,
    VitalsSource::PolarH10Emulator,
    VitalsSource::ResmedAir11Sd,
];
