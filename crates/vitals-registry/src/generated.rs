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
    /// `ad_ua651ble` — `ble`, service `0x1810` (indication)
    AdUa651ble = 6,
    /// `polar_h10_emulator` — `internal`
    PolarH10Emulator = 7,
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
            6 => Some(Self::AdUa651ble),
            7 => Some(Self::PolarH10Emulator),
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
            Self::AdUa651ble => Cadence::OnDemand,
            Self::PolarH10Emulator => Cadence::EverySeconds(1),
        }
    }
}

/// Every type, for exhaustive iteration in tests and admin views.
pub const ALL_TYPES: [VitalsType; 9] = [
    VitalsType::Hr,
    VitalsType::Spo2,
    VitalsType::Accel,
    VitalsType::HrvRmssd,
    VitalsType::RrInterval,
    VitalsType::BpSystolic,
    VitalsType::BpDiastolic,
    VitalsType::BpMap,
    VitalsType::SleepStage,
];

/// Every source.
pub const ALL_SOURCES: [VitalsSource; 6] = [
    VitalsSource::Emay,
    VitalsSource::PolarH10,
    VitalsSource::Healthkit,
    VitalsSource::Derived,
    VitalsSource::AdUa651ble,
    VitalsSource::PolarH10Emulator,
];
