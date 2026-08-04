//! IEEE-11073 SFLOAT decoder, fuzzed.
//!
//! The decoder's whole job is to survive bytes off a radio. A malformed
//! advertisement or a truncated GATT indication is what BlueZ hands you when a
//! device browns out mid-packet, so every 16-bit input must either produce a
//! finite number or say the value is absent — never a panic, and never a NaN or
//! infinity dressed up as a measurement.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::sfloat::decode_sfloat;

fuzz_target!(|data: &[u8]| {
    let Some(chunk) = data.get(..2) else { return };
    let raw = u16::from_le_bytes([chunk[0], chunk[1]]);
    if let Some(v) = decode_sfloat(raw) {
        // A decode that succeeds must produce a finite number. A sentinel
        // reaching this branch as NaN or infinity is the exact failure the
        // design forbids: it is indistinguishable from a real reading by the
        // time it reaches the detection engine.
        assert!(v.is_finite(), "decode_sfloat({raw:#06x}) -> {v}");
    }
});
