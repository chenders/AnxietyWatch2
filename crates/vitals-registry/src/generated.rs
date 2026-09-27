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
    /// `respiratory_rate` — `breaths_per_minute`, valid 1.0..120.0
    RespiratoryRate = 9,
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
    /// `ad_ua651ble` — `ble`, service `0x1810` (indication)
    AdUa651ble = 6,
    /// `polar_h10_emulator` — `internal`
    PolarH10Emulator = 7,
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
            Self::RespiratoryRate => (1.0, 120.0),
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
            Self::RespiratoryRate => "breaths_per_minute",
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
            9 => Some(Self::RespiratoryRate),
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
    VitalsType::RespiratoryRate,
];

/// Every source.
pub const ALL_SOURCES: [VitalsSource; 7] = [
    VitalsSource::Emay,
    VitalsSource::PolarH10,
    VitalsSource::Healthkit,
    VitalsSource::Derived,
    VitalsSource::AcconeerXe125,
    VitalsSource::AdUa651ble,
    VitalsSource::PolarH10Emulator,
];
