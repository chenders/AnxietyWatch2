//! Bluetooth SIG Heart Rate Measurement, characteristic `0x2A37`.
//!
//! Pure and `no_std`, like every decoder here: the wire format is where the bugs
//! live, and it should be testable and fuzzable without a radio.
//!
//! # Layout
//!
//! ```text
//! byte 0   flags
//!            bit0  0 = HR is uint8, 1 = HR is uint16
//!            bit1  sensor contact status
//!            bit2  sensor contact supported
//!            bit3  energy expended present (uint16, kJ)
//!            bit4  RR intervals present
//! then     HR         (1 or 2 bytes, little-endian)
//! then     energy     (2 bytes, only if bit3)
//! then     RR         (0..n x 2 bytes, little-endian, units of 1/1024 s)
//! ```
//!
//! # Why RR intervals are the point
//!
//! Several arrive in one notification, and they are the reason the store keys on
//! `(source, type, session_id, seq)` rather than on time: anchoring them to
//! packet arrival makes them collide, and `ON CONFLICT DO NOTHING` then discards
//! real measurements while every health metric reads normal (amendment 05 §1).
//!
//! The units are **1/1024 s, not milliseconds.** Treating them as milliseconds
//! yields intervals about 2.4% short — an HRV figure wrong in a way no range
//! check would ever flag. The registry's canonical unit for `rr_interval` is
//! **seconds**, and per the one-conversion rule that conversion happens here
//! and nowhere else.

/// Most RR intervals a single notification can carry.
///
/// A 20-byte ATT MTU leaves at most 9 after the flags and a uint16 HR; the Polar
/// H10 negotiates larger MTUs, so this is generous. Bounded because the decoder
/// is `no_std` and must not allocate on attacker-controlled input.
pub const MAX_RR: usize = 32;

/// One `0x2A37` notification.
#[derive(Debug, Clone, PartialEq)]
pub struct HeartRate {
    /// Beats per minute.
    pub bpm: u16,
    /// How many RR intervals were decoded into [`Self::rr_s`].
    pub rr_len: usize,
    /// RR intervals in **seconds** — the registry's canonical unit for
    /// `rr_interval` — converted once here so nothing downstream sees
    /// 1/1024 s units. Only the first [`Self::rr_len`] entries are meaningful.
    pub rr_s: [f64; MAX_RR],
    /// Intervals present in the notification but beyond [`MAX_RR`].
    ///
    /// Reported rather than silently discarded: a nonzero value means data was
    /// lost, and lost data must never be invisible.
    pub rr_truncated: usize,
    /// Whether the sensor reports skin contact.
    ///
    /// `None` when the sensor does not support the feature — distinct from
    /// `Some(false)`, which means it supports it and reports no contact. A
    /// device that cannot tell you must never read as telling you "fine".
    pub contact: Option<bool>,
    /// Energy expended in kJ, if present.
    pub energy_kj: Option<u16>,
}

impl HeartRate {
    /// The decoded RR intervals, in seconds.
    #[must_use]
    pub fn rr(&self) -> &[f64] {
        self.rr_s.get(..self.rr_len).unwrap_or(&[])
    }
}

/// Decodes a Heart Rate Measurement.
///
/// Returns `None` on any truncation of a required field. A partially-decoded
/// notification must be *absent*, never a reading with invented fields — the
/// same rule the SFLOAT sentinels follow.
#[must_use]
pub fn decode_2a37(buf: &[u8]) -> Option<HeartRate> {
    let flags = *buf.first()?;
    let mut off = 1usize;

    let bpm: u16 = if flags & 0x01 == 0 {
        let v = u16::from(*buf.get(off)?);
        off += 1;
        v
    } else {
        let b = buf.get(off..off + 2)?;
        let v = u16::from_le_bytes([*b.first()?, *b.get(1)?]);
        off += 2;
        v
    };

    // bit2 says the feature exists; bit1 is the reading. Without bit2 the
    // reading is meaningless, so it is None rather than false.
    let contact = if flags & 0x04 == 0 {
        None
    } else {
        Some(flags & 0x02 != 0)
    };

    let energy_kj = if flags & 0x08 == 0 {
        None
    } else {
        let b = buf.get(off..off + 2)?;
        let v = u16::from_le_bytes([*b.first()?, *b.get(1)?]);
        off += 2;
        Some(v)
    };

    let mut rr_s = [0.0f64; MAX_RR];
    let mut rr_len = 0usize;
    let mut rr_truncated = 0usize;
    if flags & 0x10 != 0 {
        while let Some(b) = buf.get(off..off + 2) {
            let raw = u16::from_le_bytes([*b.first()?, *b.get(1)?]);
            if let Some(slot) = rr_s.get_mut(rr_len) {
                // 1/1024 s, NOT milliseconds and NOT thousandths. The /1024 is
                // the whole difference between a correct HRV figure and one
                // 2.4% short.
                *slot = f64::from(raw) / 1024.0;
                rr_len += 1;
            } else {
                rr_truncated += 1;
            }
            off += 2;
        }
    }

    Some(HeartRate {
        bpm,
        rr_len,
        rr_s,
        rr_truncated,
        contact,
        energy_kj,
    })
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn uint8_heart_rate_no_rr() {
        let h = decode_2a37(&[0x00, 60]).expect("decodes");
        assert_eq!(h.bpm, 60);
        assert!(h.rr().is_empty());
        assert_eq!(h.contact, None);
        assert_eq!(h.energy_kj, None);
    }

    #[test]
    fn uint16_heart_rate_is_little_endian() {
        // 0x0140 = 320. Big-endian would read 0x4001 = 16385.
        let h = decode_2a37(&[0x01, 0x40, 0x01]).expect("decodes");
        assert_eq!(h.bpm, 320);
    }

    /// The unit conversion this decoder exists to get right.
    #[test]
    fn rr_intervals_are_1024ths_of_a_second_not_milliseconds() {
        // 1024 units = exactly one second.
        let h = decode_2a37(&[0x10, 60, 0x00, 0x04]).expect("decodes");
        assert_eq!(h.rr().len(), 1);
        assert!(close(h.rr()[0], 1.0), "got {}", h.rr()[0]);

        // A typical resting interval: 900 units = 0.87890625 s, not 0.9 s.
        let h = decode_2a37(&[0x10, 60, 0x84, 0x03]).expect("decodes");
        assert!(close(h.rr()[0], 900.0 / 1024.0));
        assert!(
            !close(h.rr()[0], 0.9),
            "reading raw units as thousandths is 2.4% wrong"
        );
    }

    #[test]
    fn several_rr_intervals_arrive_in_one_notification() {
        let h = decode_2a37(&[0x10, 60, 0x00, 0x04, 0x84, 0x03, 0x00, 0x03]).expect("decodes");
        assert_eq!(h.rr().len(), 3);
    }

    /// Energy sits between HR and the RR intervals. Skipping it wrongly shifts
    /// every RR value by one byte and produces plausible garbage.
    #[test]
    fn energy_is_skipped_at_the_right_offset() {
        // flags 0x18 = energy + RR. energy = 0x0102, then one RR of 1024.
        let h = decode_2a37(&[0x18, 60, 0x02, 0x01, 0x00, 0x04]).expect("decodes");
        assert_eq!(h.energy_kj, Some(0x0102));
        assert_eq!(h.rr().len(), 1);
        assert!(close(h.rr()[0], 1.0));
    }

    /// "Cannot report contact" and "reports no contact" are different claims. A
    /// sensor that does not support the feature must not read as reassuring.
    #[test]
    fn unsupported_contact_is_none_not_false() {
        assert_eq!(decode_2a37(&[0x00, 60]).expect("decodes").contact, None);
        // bit2 set, bit1 clear: supported, and NOT in contact.
        assert_eq!(
            decode_2a37(&[0x04, 60]).expect("decodes").contact,
            Some(false)
        );
        // bit2 and bit1 set: supported, in contact.
        assert_eq!(
            decode_2a37(&[0x06, 60]).expect("decodes").contact,
            Some(true)
        );
    }

    #[test]
    fn truncation_is_absence_not_a_partial_reading() {
        assert!(decode_2a37(&[]).is_none(), "empty");
        assert!(decode_2a37(&[0x00]).is_none(), "flags but no HR");
        assert!(
            decode_2a37(&[0x01, 0x40]).is_none(),
            "uint16 HR with one byte"
        );
        assert!(
            decode_2a37(&[0x08, 60, 0x01]).is_none(),
            "energy with one byte"
        );
    }

    /// A trailing odd byte in the RR region is ignored rather than rejected: the
    /// intervals that fully arrived are real measurements, and discarding them
    /// would lose more than it protects.
    #[test]
    fn a_trailing_odd_byte_does_not_discard_good_intervals() {
        let h = decode_2a37(&[0x10, 60, 0x00, 0x04, 0xFF]).expect("decodes");
        assert_eq!(h.rr().len(), 1);
    }

    /// Beyond `MAX_RR` the tail is dropped, but the loss is REPORTED. Silent
    /// truncation would be indistinguishable from a shorter notification.
    #[test]
    fn oversized_notifications_report_what_they_dropped() {
        // Fixed array, not a Vec: this crate is no_std.
        const EXTRA: usize = 5;
        let mut buf = [0u8; 2 + (MAX_RR + EXTRA) * 2];
        if let Some(f) = buf.first_mut() {
            *f = 0x10;
        }
        if let Some(b) = buf.get_mut(1) {
            *b = 60;
        }
        // Every interval is 1024 units = one second.
        for i in 0..(MAX_RR + EXTRA) {
            if let Some(hi) = buf.get_mut(2 + i * 2 + 1) {
                *hi = 0x04;
            }
        }
        let h = decode_2a37(&buf).expect("decodes");
        assert_eq!(h.rr().len(), MAX_RR);
        assert_eq!(h.rr_truncated, EXTRA, "dropped intervals must be visible");
    }

    /// No input may panic. The buffer is attacker-controlled radio data.
    #[test]
    fn no_input_panics() {
        for flags in 0u16..=0xFF {
            for len in 0..24usize {
                let mut buf = [0u8; 24];
                if let Some(f) = buf.first_mut() {
                    *f = u8::try_from(flags).unwrap_or(0);
                }
                let _ = decode_2a37(buf.get(..len).unwrap_or(&[]));
            }
        }
    }
}
