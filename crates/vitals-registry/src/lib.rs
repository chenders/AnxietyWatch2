//! Types and sources generated from `vitals/registry.toml`.
//!
//! `registry.toml` is the single source of truth for every source, type,
//! canonical unit and valid range. A source label like `"polar_h10"` exists in
//! exactly one file; everywhere else it is [`VitalsSource::PolarH10`].
//!
//! Regenerate with `python3 vitals/tools/gen_registry.py`. CI runs `--check`,
//! so a hand-edited generated file cannot survive.
#![no_std]
#![forbid(unsafe_code)]

mod generated;

pub use generated::{ALL_SOURCES, ALL_TYPES, Cadence, VitalsSource, VitalsType};

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids are permanent, so a renumbering must break the build rather than
    /// silently reinterpret every historical row carrying the old value.
    #[test]
    fn ids_are_pinned() {
        assert_eq!(VitalsType::Hr.id(), 1);
        assert_eq!(VitalsType::BpSystolic.id(), 6);
        assert_eq!(VitalsType::RespiratoryRate.id(), 11);
        assert_eq!(VitalsSource::Emay.id(), 0);
        assert_eq!(VitalsSource::AcconeerXe125.id(), 4);
        assert_eq!(VitalsSource::AdUa651ble.id(), 6);
    }

    /// The wire and the database both hand us plain integers. An unknown id is
    /// rejected, never transmuted into a variant that does not exist.
    #[test]
    fn unknown_ids_are_rejected() {
        assert!(VitalsType::from_id(9999).is_none());
        assert!(VitalsSource::from_id(-1).is_none());
        for t in ALL_TYPES {
            assert_eq!(VitalsType::from_id(t.id()), Some(t));
        }
        for s in ALL_SOURCES {
            assert_eq!(VitalsSource::from_id(s.id()), Some(s));
        }
    }

    /// An inverted or degenerate range would quarantine everything or nothing,
    /// and both fail silently.
    #[test]
    fn ranges_are_ordered_and_units_present() {
        for t in ALL_TYPES {
            let (lo, hi) = t.range();
            assert!(lo < hi, "{t:?} has range {lo}..{hi}");
            assert!(!t.unit().is_empty(), "{t:?} has no unit");
        }
    }

    /// `source` and `type` are both small integers drawn from overlapping
    /// ranges, so a transposed pair is invisible in a debugger. These are the
    /// ids T-D6's collector and the sqitch reference data must agree on.
    #[test]
    fn bp_source_and_types_match_the_briefs() {
        assert_eq!(VitalsSource::AdUa651ble.id(), 6);
        assert_eq!(VitalsType::BpSystolic.id(), 6);
        assert_eq!(VitalsType::BpDiastolic.id(), 7);
        assert_eq!(VitalsType::BpMap.id(), 8);
        // T-D6 maps the cuff's pulse to Hr; there is no VitalsType::Pulse.
        assert_eq!(VitalsType::Hr.unit(), "bpm");
    }

    /// AirSense uses permanent ids for source-backed stream rows. The source
    /// and type enums are what prevent a numeric source/type transposition.
    #[test]
    fn air11_source_and_types_match_the_contract() {
        assert_eq!(VitalsSource::ResmedAir11.id(), 5);
        assert_eq!(VitalsType::RespiratoryRate.id(), 11);
        assert_eq!(VitalsType::PatientFlow.id(), 30);
        assert_eq!(VitalsType::MaskPressure.id(), 31);
        assert_eq!(VitalsType::MaskPressure2s.id(), 32);
        assert_eq!(VitalsType::Leak.id(), 35);
        assert_eq!(VitalsType::TidalVolume.id(), 36);
        assert_eq!(VitalsType::TriggerCycleEvent.id(), 43);
        assert_eq!(VitalsType::Leak.unit(), "L/min");
        assert_eq!(VitalsType::TidalVolume.unit(), "mL");
        assert_eq!(
            VitalsSource::ResmedAir11.expected_cadence(),
            Cadence::EverySeconds(1)
        );
    }

    /// The declared cadences come straight from `registry.toml`; the liveness
    /// law (SPEC-admin-console §2.5) evaluates age against exactly these
    /// numbers, so a drift here silently changes when a source turns UNKNOWN.
    #[test]
    fn cadences_match_the_registry() {
        assert_eq!(
            VitalsSource::Emay.expected_cadence(),
            Cadence::EverySeconds(60)
        );
        assert_eq!(
            VitalsSource::PolarH10.expected_cadence(),
            Cadence::EverySeconds(1)
        );
        assert_eq!(
            VitalsSource::Healthkit.expected_cadence(),
            Cadence::EverySeconds(21_600),
        );
        assert_eq!(VitalsSource::Derived.expected_cadence(), Cadence::OnDemand);
        assert_eq!(
            VitalsSource::AdUa651ble.expected_cadence(),
            Cadence::OnDemand
        );
        // The emulator mirrors the physical strap it simulates.
        assert_eq!(
            VitalsSource::PolarH10Emulator.expected_cadence(),
            Cadence::EverySeconds(1)
        );
    }

    /// Every source declares a cadence — the generated match has no wildcard
    /// arm, so this is exhaustive by construction and a new source or a new
    /// `Cadence` variant fails to compile rather than silently defaulting.
    /// Iterating `ALL_SOURCES` also pins the array to the same set as the
    /// cadence match: a mismatch between them cannot compile.
    #[test]
    fn every_source_declares_a_cadence() {
        let mut periodic = 0u32;
        let mut on_demand = 0u32;
        for s in ALL_SOURCES {
            match s.expected_cadence() {
                Cadence::EverySeconds(n) => {
                    assert!(n > 0, "{s:?} declared non-positive cadence {n}");
                    periodic += 1;
                }
                Cadence::OnDemand => on_demand += 1,
            }
        }
        assert_eq!(periodic, 5);
        assert_eq!(on_demand, 3);
    }
}
