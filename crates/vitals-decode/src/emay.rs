//! EMAY `SleepO2` real-time BLE protocol.
//!
//! Reverse-engineered and verified against the device on 2026-07-09. No public
//! decoder for the live protocol existed; this was derived from the vendor
//! Android app plus a captured iOS trace.
//!
//! Vendor-serial over `0xFF12`: write `0xFF01`, notify `0xFF02`. The device
//! connects with **no pairing, no bonding and no encryption**, which is why it
//! cannot hit the SMP Security-Request failure that blocks a bonded reconnect on
//! kernel 6.12.x (amendment 06 §3) — there is no bond to resume.
//!
//! # The checksum gotcha
//!
//! `checksum = sum(payload) & 0x7F` — masked with **0x7F, not 0xFF**. A decoder
//! written the obvious way accepts roughly half of all corrupt frames, because
//! the high bit is simply never compared.
//!
//! # Why `SpO₂` absence must be absence
//!
//! This is a **primary** signal: amendment 05 §2.1 allows only `SpO₂` and
//! respiratory rate to raise a tier past watch or to clear a raised one. A
//! no-finger reading coerced to a number would be a primary signal fabricated
//! out of nothing, which is the single worst input this system can receive. So
//! out-of-range values are absent, never clamped and never zero.

/// Physiologically possible `SpO₂`, in percent.
///
/// Below 50 the device is reporting a fault, not a measurement — and a real
/// desaturation that deep is beyond what a fingertip sensor tracks reliably.
pub const SPO2_RANGE: (u8, u8) = (50, 100);

/// Physiologically possible pulse rate, in bpm.
pub const PULSE_RANGE: (u8, u8) = (25, 250);

/// Opcode of a real-time data notification.
const OP_DATA: u8 = 0xEB;

/// One real-time reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// `SpO₂` in percent, or `None` when the device reported no finger or an
    /// implausible value.
    pub spo2: Option<u8>,
    /// Pulse rate in bpm, or `None` for the same reasons.
    pub pulse_bpm: Option<u8>,
}

impl Reading {
    /// Whether either field carries a usable measurement.
    ///
    /// A frame with neither is a valid *frame* reporting no *reading* — the
    /// device is connected and saying "no finger", which is information, not an
    /// error.
    #[must_use]
    pub const fn is_measurement(&self) -> bool {
        self.spo2.is_some() || self.pulse_bpm.is_some()
    }
}

/// Verifies the trailing checksum.
///
/// The mask is `0x7F`. Using `0xFF` compares a byte the device never sets and
/// silently accepts corrupt frames.
#[must_use]
pub fn checksum_ok(frame: &[u8]) -> bool {
    let Some((&given, payload)) = frame.split_last() else {
        return false;
    };
    if payload.is_empty() {
        return false;
    }
    let sum = payload.iter().fold(0u32, |a, b| a + u32::from(*b));
    // try_from rather than `as`: the mask makes the value provably small, but
    // saying so with a fallible conversion costs nothing and does not need an
    // #[expect] that a later edit could invalidate.
    u8::try_from(sum & 0x7F).is_ok_and(|want| given == want)
}

/// Decodes a real-time data notification.
///
/// Expected shape: `EB 01 05 <PR> <SPO2> 7F 00 <cks>`, 8 bytes.
///
/// Returns `None` when the frame is not a data notification, is the wrong
/// length, or fails its checksum. A frame that fails verification is *absent*,
/// never partially trusted — the bytes are radio input and the values are a
/// primary clinical signal.
#[must_use]
pub fn decode_data(frame: &[u8]) -> Option<Reading> {
    if frame.len() != 8 || *frame.first()? != OP_DATA || !checksum_ok(frame) {
        return None;
    }
    let pulse = *frame.get(3)?;
    let spo2 = *frame.get(4)?;
    Some(Reading {
        spo2: in_range(spo2, SPO2_RANGE),
        pulse_bpm: in_range(pulse, PULSE_RANGE),
    })
}

/// `None` outside the range. Not clamped: a clamped physiological value is
/// indistinguishable from a real one downstream.
const fn in_range(v: u8, (lo, hi): (u8, u8)) -> Option<u8> {
    if v >= lo && v <= hi { Some(v) } else { None }
}

/// Commands the host writes to `0xFF01`.
///
/// Kept here beside the decoder so the protocol lives in one file rather than
/// being split between a spec document and a binary nobody can test.
pub mod cmd {
    /// Hello. Device replies with a `0xF9` frame.
    pub const HELLO: [u8; 2] = [0x89, 0x09];
    /// Device state. Replies `0xFE`.
    pub const DEVICE_STATE: [u8; 3] = [0x8E, 0x05, 0x13];
    /// Start the real-time stream. Data begins immediately on `0xFF02`.
    pub const START_REALTIME: [u8; 3] = [0x9B, 0x01, 0x1C];
    /// Battery level. Replies `0xF6 <batt> <cks>`.
    pub const BATTERY: [u8; 2] = [0x86, 0x06];
    /// Keepalive. **Must be resent about every 1.5 s** or the stream stops.
    pub const KEEPALIVE: [u8; 2] = [0x9A, 0x1A];
    /// Stop the real-time stream, before disconnecting.
    pub const STOP_REALTIME: [u8; 3] = [0x9B, 0x7F, 0x1A];
}

/// A response opcode is its command opcode **XOR** `0x70`.
///
/// Not addition. For every `0x8x` command the two agree — `0x89 + 0x70` and
/// `0x89 ^ 0x70` are both `0xF9` — so a note derived from those alone records
/// "+ 0x70" and is wrong exactly once: `0x9B + 0x70` wraps to `0x0B`, while the
/// device answers `0xEB`. `0x9B` is `START_REALTIME`, the command whose reply
/// carries every reading.
#[must_use]
pub const fn response_opcode(command: u8) -> u8 {
    command ^ 0x70
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
mod tests {
    use super::*;

    /// Captured from the device: PR 62, `SpO2` 95.
    const REAL_FRAME: [u8; 8] = [0xEB, 0x01, 0x05, 0x3E, 0x5F, 0x7F, 0x00, 0x0D];

    #[test]
    fn decodes_a_frame_captured_from_the_device() {
        let r = decode_data(&REAL_FRAME).expect("decodes");
        assert_eq!(r.pulse_bpm, Some(62));
        assert_eq!(r.spo2, Some(95));
        assert!(r.is_measurement());
    }

    /// The gotcha this decoder exists to get right. With a 0xFF mask the
    /// captured frame's checksum would not verify at all.
    #[test]
    fn checksum_is_masked_with_0x7f_not_0xff() {
        assert!(checksum_ok(&REAL_FRAME));

        // The captured frame cannot demonstrate the difference: its payload
        // sums to 0x20D, and 0x20D & 0x7F == 0x20D & 0xFF == 0x0D. The masks
        // only diverge when bit 7 of the sum is set, which is why a decoder
        // written with 0xFF can pass a hand-check against one real frame and
        // still be wrong.
        let diverging = [0xEB, 0x01, 0x05, 0x3E, 50, 0x7F, 0x00, 0x60];
        let sum: u32 = diverging
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        assert_eq!(sum, 0x1E0);
        assert_eq!(sum & 0x7F, 0x60, "the device's mask");
        assert_eq!(
            sum & 0xFF,
            0xE0,
            "an 0xFF mask would compute a different byte"
        );
        assert!(checksum_ok(&diverging), "0x7F is the correct mask");

        let mut wrong = diverging;
        if let Some(last) = wrong.last_mut() {
            *last = 0xE0; // what an 0xFF-masked encoder would emit
        }
        assert!(!checksum_ok(&wrong), "an 0xFF-masked frame must not verify");
    }

    #[test]
    fn a_corrupt_checksum_is_rejected() {
        let mut bad = REAL_FRAME;
        if let Some(last) = bad.last_mut() {
            *last ^= 0x01;
        }
        assert!(decode_data(&bad).is_none());
    }

    #[test]
    fn a_corrupt_payload_is_rejected() {
        let mut bad = REAL_FRAME;
        if let Some(b) = bad.get_mut(4) {
            *b = 0x60; // SpO2 96 without fixing the checksum
        }
        assert!(decode_data(&bad).is_none());
    }

    /// No-finger and fault values must be absent, never coerced. This is a
    /// primary signal: a fabricated `SpO2` could clear a raised alarm tier.
    #[test]
    fn out_of_range_values_are_absent_not_clamped() {
        // 0xFF in both fields, checksum recomputed so the frame itself is valid.
        let mut f = [0xEB, 0x01, 0x05, 0xFF, 0xFF, 0x7F, 0x00, 0x00];
        let sum: u32 = f
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        let r = decode_data(&f).expect("the frame is well-formed");
        assert_eq!(r.spo2, None, "0xFF must not become 255%");
        assert_eq!(r.pulse_bpm, None);
        assert!(!r.is_measurement(), "a valid frame reporting no reading");
    }

    #[test]
    fn zero_is_absent_too() {
        let mut f = [0xEB, 0x01, 0x05, 0x00, 0x00, 0x7F, 0x00, 0x00];
        let sum: u32 = f
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        let r = decode_data(&f).expect("well-formed");
        assert_eq!(r.spo2, None, "0% SpO2 is a fault code, not a measurement");
        assert_eq!(r.pulse_bpm, None);
    }

    /// The boundaries themselves are valid readings.
    #[test]
    fn range_boundaries_are_inclusive() {
        for (pr, spo2, want_pr, want_spo2) in [
            (25u8, 50u8, Some(25u8), Some(50u8)),
            (250, 100, Some(250), Some(100)),
            (24, 49, None, None),
        ] {
            let mut f = [0xEB, 0x01, 0x05, pr, spo2, 0x7F, 0x00, 0x00];
            let sum: u32 = f
                .get(..7)
                .expect("payload")
                .iter()
                .map(|b| u32::from(*b))
                .sum();
            if let Some(last) = f.last_mut() {
                *last = u8::try_from(sum & 0x7F).unwrap_or(0);
            }
            let r = decode_data(&f).expect("well-formed");
            assert_eq!(r.pulse_bpm, want_pr, "pr {pr}");
            assert_eq!(r.spo2, want_spo2, "spo2 {spo2}");
        }
    }

    #[test]
    fn non_data_opcodes_are_not_readings() {
        // A battery reply is a valid frame but not a measurement.
        let mut f = [0xF6, 0x64, 0x00];
        let sum: u32 = f
            .get(..2)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        assert!(decode_data(&f).is_none());
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(decode_data(&[]).is_none());
        assert!(decode_data(&[0xEB]).is_none());
        assert!(decode_data(REAL_FRAME.get(..7).expect("short")).is_none());
        let mut long = REAL_FRAME.to_vec();
        long.push(0x00);
        assert!(decode_data(&long).is_none());
    }

    #[test]
    fn response_opcodes_are_command_plus_0x70() {
        assert_eq!(response_opcode(0x89), 0xF9);
        assert_eq!(response_opcode(0x8E), 0xFE);
        assert_eq!(response_opcode(0x9B), 0xEB);
        assert_eq!(response_opcode(0x86), 0xF6);
    }

    /// No input may panic; every byte comes off a radio.
    #[test]
    fn no_input_panics() {
        for len in 0..12usize {
            for b in 0u16..=0xFF {
                let mut buf = [0u8; 12];
                for (i, slot) in buf.iter_mut().enumerate() {
                    *slot = u8::try_from((b as usize + i) % 256).unwrap_or(0);
                }
                let _ = decode_data(buf.get(..len).unwrap_or(&[]));
                let _ = checksum_ok(buf.get(..len).unwrap_or(&[]));
            }
        }
    }
}
