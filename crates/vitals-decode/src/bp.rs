//! Bluetooth SIG Blood Pressure Measurement, characteristic `0x2A35`.
//!
//! The characteristic is an **indication**. A client must use the GATT
//! subscription API (which selects CCCD `0x0002`); writing notification bit
//! `0x0001` succeeds but delivers nothing.

use crate::decode_sfloat;

const KPA_TO_MMHG: f64 = 7.500_62;

/// One decoded Blood Pressure Measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BpMeasurement {
    /// Systolic pressure in mmHg.
    pub systolic: Option<f64>,
    /// Diastolic pressure in mmHg.
    pub diastolic: Option<f64>,
    /// Mean arterial pressure in mmHg.
    pub map: Option<f64>,
    /// Pulse rate in beats per minute.
    pub pulse_bpm: Option<f64>,
    /// Device event time as UTC Unix seconds, when present and valid.
    pub timestamp: Option<i64>,
    /// Device user id, when present.
    pub user_id: Option<u8>,
    /// Measurement Status bit 2.
    pub irregular_pulse: bool,
    /// Measurement Status bit 0.
    pub body_movement: bool,
    /// Measurement Status bit 1.
    pub cuff_too_loose: bool,
}

fn take_u16(buf: &[u8], off: &mut usize) -> Option<u16> {
    let bytes: [u8; 2] = buf.get(*off..off.checked_add(2)?)?.try_into().ok()?;
    *off = off.checked_add(2)?;
    Some(u16::from_le_bytes(bytes))
}

fn take_u8(buf: &[u8], off: &mut usize) -> Option<u8> {
    let value = *buf.get(*off)?;
    *off = off.checked_add(1)?;
    Some(value)
}

fn pressure(raw: u16, kpa: bool) -> Option<f64> {
    decode_sfloat(raw).map(|value| if kpa { value * KPA_TO_MMHG } else { value })
}

// Howard Hinnant's civil-date conversion. Input validation is kept separate so
// an invalid device date is absent rather than silently normalised.
fn unix_seconds(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<i64> {
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
        || !(1970..=9999).contains(&year)
    {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_len = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day > month_len {
        return None;
    }

    let adjusted_year = year - i32::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = i32::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i32::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = i64::from(era * 146_097 + day_of_era - 719_468);
    days.checked_mul(86_400)?
        .checked_add(i64::from(hour) * 3_600)?
        .checked_add(i64::from(minute) * 60)?
        .checked_add(i64::from(second))
}

/// Decodes a Bluetooth SIG `0x2A35` Blood Pressure Measurement.
///
/// Returns `None` for malformed, truncated, or trailing data. Reserved SFLOAT
/// values make only their field absent; they do not invalidate the frame.
#[must_use]
pub fn bp_decode_2a35(buf: &[u8]) -> Option<BpMeasurement> {
    let flags = *buf.first()?;
    // Bits 5..7 are reserved. Accepting them would claim a layout we do not
    // understand and could shift every optional field.
    if flags & 0xe0 != 0 {
        return None;
    }
    let mut off = 1usize;
    let kpa = flags & 0x01 != 0;
    let systolic = pressure(take_u16(buf, &mut off)?, kpa);
    let diastolic = pressure(take_u16(buf, &mut off)?, kpa);
    let map = pressure(take_u16(buf, &mut off)?, kpa);

    let timestamp = if flags & 0x02 != 0 {
        let year = i32::from(take_u16(buf, &mut off)?);
        let month = take_u8(buf, &mut off)?;
        let day = take_u8(buf, &mut off)?;
        let hour = take_u8(buf, &mut off)?;
        let minute = take_u8(buf, &mut off)?;
        let second = take_u8(buf, &mut off)?;
        unix_seconds(year, month, day, hour, minute, second)
    } else {
        None
    };
    let pulse_bpm = if flags & 0x04 != 0 {
        decode_sfloat(take_u16(buf, &mut off)?)
    } else {
        None
    };
    let user_id = if flags & 0x08 != 0 {
        Some(take_u8(buf, &mut off)?)
    } else {
        None
    };
    let status = if flags & 0x10 != 0 {
        take_u16(buf, &mut off)?
    } else {
        0
    };
    if off != buf.len() {
        return None;
    }

    Some(BpMeasurement {
        systolic,
        diastolic,
        map,
        pulse_bpm,
        timestamp,
        user_id,
        irregular_pulse: status & 0x0004 != 0,
        body_movement: status & 0x0001 != 0,
        cuff_too_loose: status & 0x0002 != 0,
    })
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let text = core::str::from_utf8(pair).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }

    fn close(actual: Option<f64>, expected: f64) {
        assert!(
            (actual.unwrap() - expected).abs() < 0.01,
            "{actual:?} != {expected}"
        );
    }

    #[test]
    fn decodes_four_hardware_frames() {
        // Byte layout of the four 2026-08-03 hardware frames (flags 0x14, 11 bytes,
        // exponent 0); the readings themselves are substituted, not the owner's.
        // Frame 3's systolic low byte (0x84) keeps bit 7 set, as the captured
        // frames' did, so an in-range value still crosses the sign-bit path.
        // See crates/vitals-decode/fixtures/ua651ble-2a35.md.
        for (raw, sys, dia, map, pulse) in [
            ("1476004c005a003e000000", 118.0, 76.0, 90.0, 62.0),
            ("147a004e005d0040000000", 122.0, 78.0, 93.0, 64.0),
            ("1484005600650044000000", 132.0, 86.0, 101.0, 68.0),
            ("14780050005d0046000000", 120.0, 80.0, 93.0, 70.0),
        ] {
            let b = bp_decode_2a35(&hex(raw)).unwrap();
            close(b.systolic, sys);
            close(b.diastolic, dia);
            close(b.map, map);
            close(b.pulse_bpm, pulse);
            assert_eq!(b.timestamp, None);
            assert_eq!(b.user_id, None);
            assert!(!b.irregular_pulse && !b.body_movement && !b.cuff_too_loose);
        }
    }

    #[test]
    fn minimum_frame_has_no_optional_fields() {
        let b = bp_decode_2a35(&hex("0076004a005900")).unwrap();
        close(b.systolic, 118.0);
        assert_eq!(b.pulse_bpm, None);
    }

    #[test]
    fn converts_kpa_and_negative_exponents() {
        let b = bp_decode_2a35(&hex("014af03cf042f0")).unwrap();
        close(b.systolic, 7.4 * KPA_TO_MMHG);
        close(b.diastolic, 6.0 * KPA_TO_MMHG);
        close(b.map, 6.6 * KPA_TO_MMHG);
    }

    #[test]
    fn all_optional_fields_fit_nineteen_bytes() {
        let b = bp_decode_2a35(&hex("1e76004a005900e80701020304053c00070400")).unwrap();
        assert_eq!(b.timestamp, Some(1_704_164_645));
        close(b.pulse_bpm, 60.0);
        assert_eq!(b.user_id, Some(7));
        assert!(b.irregular_pulse);
    }

    #[test]
    fn reserved_sfloat_is_absent() {
        let b = bp_decode_2a35(&hex("00ff074a005900")).unwrap();
        assert_eq!(b.systolic, None);
        close(b.diastolic, 74.0);
    }

    #[test]
    fn status_bits_are_exposed() {
        let b = bp_decode_2a35(&hex("1076004a0059000700")).unwrap();
        assert!(b.irregular_pulse && b.body_movement && b.cuff_too_loose);
    }

    #[test]
    fn every_truncation_and_trailing_byte_is_rejected() {
        let full = hex("1e76004a005900e80701020304053c00070400");
        for len in 0..full.len() {
            assert_eq!(bp_decode_2a35(&full[..len]), None, "accepted length {len}");
        }
        let mut trailing = full;
        trailing.push(0);
        assert_eq!(bp_decode_2a35(&trailing), None);
    }

    #[test]
    fn timestamp_flag_with_short_payload_is_rejected() {
        assert_eq!(bp_decode_2a35(&hex("0276004a00590000")), None);
    }

    #[test]
    fn invalid_calendar_date_is_absent_not_normalised() {
        let b = bp_decode_2a35(&hex("0276004a005900e807021e000000")).unwrap();
        assert_eq!(b.timestamp, None);
    }
}
