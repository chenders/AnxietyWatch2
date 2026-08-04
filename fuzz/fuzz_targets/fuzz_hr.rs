//! Heart Rate Measurement decoder, fuzzed.
//!
//! `0x2A37` is variable-length by construction: the flags byte decides whether
//! HR is one byte or two, whether an energy field intervenes, and how many RR
//! intervals follow. Every one of those is an offset the decoder computes from
//! attacker-controlled input, so the whole surface is reachable from the radio.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::hr::{decode_2a37, MAX_RR};

fuzz_target!(|data: &[u8]| {
    if let Some(h) = decode_2a37(data) {
        // A decode that succeeds must produce usable numbers, not NaN or a
        // length that would index out of bounds.
        assert!(h.rr_len <= MAX_RR, "rr_len {} exceeds MAX_RR", h.rr_len);
        assert_eq!(h.rr().len(), h.rr_len);
        for v in h.rr() {
            assert!(v.is_finite(), "RR interval {v} is not finite");
            assert!(*v >= 0.0, "RR interval {v} is negative");
        }
    }
});
