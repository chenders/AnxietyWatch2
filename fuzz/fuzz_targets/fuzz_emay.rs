//! EMAY SleepO2 real-time frame decoder, fuzzed.
//!
//! Every byte arrives over an unauthenticated, unencrypted BLE link — the
//! device requires no pairing at all, so anything in radio range can write
//! frames to this decoder. SpO2 is a primary signal, so a fabricated value here
//! could clear a raised alarm tier.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::emay::{decode_data, PULSE_RANGE, SPO2_RANGE};

fuzz_target!(|data: &[u8]| {
    if let Some(r) = decode_data(data) {
        // A frame that verified must never yield a value outside the
        // physiologically possible range. Absence is fine; a wrong number is
        // not.
        if let Some(s) = r.spo2 {
            assert!(s >= SPO2_RANGE.0 && s <= SPO2_RANGE.1, "SpO2 {s} out of range");
        }
        if let Some(p) = r.pulse_bpm {
            assert!(p >= PULSE_RANGE.0 && p <= PULSE_RANGE.1, "pulse {p} out of range");
        }
    }
});
