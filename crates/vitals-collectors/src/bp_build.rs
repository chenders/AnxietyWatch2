//! Store-row policy for the A&D UA-651BLE collector and its journal
//! replayer. Pure and portable: both binaries must apply identical
//! registry gates, and a copy would drift.

use core::fmt::Write as _;
use vitals_decode::bp::BpMeasurement;
use vitals_proto::WireSample;
use vitals_registry::{VitalsSource, VitalsType};

/// The rows to append plus every named rejection.
pub struct Built {
    /// Rows that passed every gate.
    pub samples: Vec<WireSample>,
    /// Why anything absent from `samples` is absent, with values named.
    pub rejected: Vec<String>,
}

/// The `extra` payload for every row of one measurement.
///
/// `event_time=arrival_estimate` is unconditional: amendment 06 §1.4 keeps
/// arrival authoritative until the step-5 clock investigation resolves
/// what the cuff's Date Time characteristic actually means (its clock is
/// civil local time, so a decoded "UTC" instant is off by the zone
/// offset). A device timestamp, when the cuff ever sends one, is recorded
/// here as `device_time_unvalidated` rather than trusted as the row's
/// event time.
fn extra(m: &BpMeasurement) -> Vec<u8> {
    let mut s = format!(
        "event_time=arrival_estimate;irregular_pulse={};body_movement={};\
         cuff_too_loose={};user_id={}",
        m.irregular_pulse,
        m.body_movement,
        m.cuff_too_loose,
        m.user_id
            .map_or_else(|| "absent".to_owned(), |v| v.to_string())
    );
    if let Some(device_secs) = m.timestamp {
        let _ = write!(s, ";device_time_unvalidated={device_secs}");
    }
    s.into_bytes()
}

/// Builds store rows from one measurement, enforcing the registry gates.
///
/// Every present, in-range field becomes one row; all rows of one
/// indication share `seq` (type is part of the identity, so they cannot
/// collide). Out-of-range values are rejected with the value and range
/// named, never clamped. Absent fields contribute nothing.
#[must_use]
pub fn samples(m: &BpMeasurement, session_id: i64, seq: i64, arrival_micros: i64) -> Built {
    let detail = Some(extra(m));
    let mut built = Built {
        samples: Vec::with_capacity(4),
        rejected: Vec::new(),
    };
    for (kind, label, value) in [
        (VitalsType::BpSystolic, "systolic", m.systolic),
        (VitalsType::BpDiastolic, "diastolic", m.diastolic),
        (VitalsType::BpMap, "map", m.map),
        (VitalsType::Hr, "pulse", m.pulse_bpm),
    ] {
        let Some(value) = value else { continue };
        let (lo, hi) = kind.range();
        if value < lo || value > hi {
            built.rejected.push(format!(
                "{label}={value} outside registry range {lo}..{hi} \
                 {unit}: rejected, never clamped",
                unit = kind.unit()
            ));
            continue;
        }
        built.samples.push(WireSample {
            source: VitalsSource::AdUa651ble.id(),
            r#type: kind.id(),
            session_id,
            seq,
            timestamp_unix_micros: arrival_micros,
            value,
            extra: detail.clone(),
            batch_id: None,
        });
    }
    built
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions should panic loudly on the fixed frames used"
)]
mod tests {
    use super::*;
    use vitals_decode::bp::bp_decode_2a35;

    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = core::str::from_utf8(pair).expect("utf8");
                u8::from_str_radix(text, 16).expect("hex")
            })
            .collect()
    }

    #[test]
    fn a_real_frame_yields_four_rows_sharing_one_seq() {
        let m = bp_decode_2a35(&hex("1476004c005a003e000000")).expect("decodes");
        let b = samples(&m, 7, 2, 5_000_000);
        assert_eq!(b.samples.len(), 4);
        assert!(b.rejected.is_empty());
        assert!(b.samples.iter().all(|s| s.seq == 2 && s.session_id == 7));
        assert!(
            b.samples
                .iter()
                .all(|s| s.timestamp_unix_micros == 5_000_000),
            "event time is arrival, always (amendment 06 §1.4)"
        );
    }

    #[test]
    fn an_absent_field_contributes_no_row() {
        // Systolic is the 0x07FF NaN sentinel.
        let m = bp_decode_2a35(&hex("14ff0759006e0043000000")).expect("decodes");
        let b = samples(&m, 1, 0, 0);
        assert_eq!(b.samples.len(), 3);
        assert!(b.rejected.is_empty(), "absent is not rejected");
        assert!(
            !b.samples
                .iter()
                .any(|s| s.r#type == vitals_registry::VitalsType::BpSystolic.id())
        );
    }

    #[test]
    fn out_of_range_is_rejected_by_name_never_clamped() {
        // Systolic SFLOAT 0x2076 = 118 × 10² = 11800 mmHg: in-spec
        // encoding, physiologically impossible.
        let m = bp_decode_2a35(&hex("1476204c005a003e000000")).expect("decodes");
        let b = samples(&m, 1, 0, 0);
        assert_eq!(b.samples.len(), 3, "the other fields still land");
        assert_eq!(b.rejected.len(), 1);
        assert!(b.rejected[0].contains("systolic=11800"));
        assert!(b.rejected[0].contains("50..260"));
    }

    #[test]
    fn device_timestamp_is_annotation_not_event_time() {
        // Flags 0x02: timestamp present (2024-01-02 03:04:05).
        let m = bp_decode_2a35(&hex("0276004a005900e8070102030405")).expect("decodes");
        let arrival = 9_000_000_i64;
        let b = samples(&m, 1, 0, arrival);
        assert!(b.samples.iter().all(|s| s.timestamp_unix_micros == arrival));
        let extra = b.samples[0].extra.as_deref().expect("extra present");
        let text = core::str::from_utf8(extra).expect("utf8");
        assert!(text.contains("event_time=arrival_estimate"));
        assert!(text.contains("device_time_unvalidated=1704164645"));
    }

    #[test]
    fn status_flags_travel_in_extra() {
        let m = bp_decode_2a35(&hex("1076004a0059000700")).expect("decodes");
        let b = samples(&m, 1, 0, 0);
        let text = String::from_utf8(b.samples[0].extra.clone().expect("extra")).expect("utf8");
        assert!(text.contains("irregular_pulse=true"));
        assert!(text.contains("body_movement=true"));
        assert!(text.contains("cuff_too_loose=true"));
    }
}
