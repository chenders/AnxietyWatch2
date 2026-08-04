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

pub use generated::{VitalsSource, VitalsType, ALL_SOURCES, ALL_TYPES};

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids are permanent, so a renumbering must break the build rather than
    /// silently reinterpret every historical row carrying the old value.
    #[test]
    fn ids_are_pinned() {
        assert_eq!(VitalsType::Hr.id(), 1);
        assert_eq!(VitalsType::BpSystolic.id(), 6);
        assert_eq!(VitalsSource::Emay.id(), 0);
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
}
