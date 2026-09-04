//! EMAY SleepO2 real-time frame decoder, fuzzed.
//!
//! Every byte arrives over an unauthenticated, unencrypted BLE link — the
//! device requires no pairing at all, so anything in radio range can write
//! frames to this decoder. SpO2 is a primary signal, so a fabricated value here
//! could clear a raised alarm tier.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::emay::{
    FRAME_LEN, SESSION_DESCRIPTOR_LEN, STORED_RECORD_LEN, decode_data, decode_session_descriptor,
    decode_stored_record, is_empty_session_descriptor, scan_frame, storage_ordinal,
};

fuzz_target!(|data: &[u8]| {
    if let Some(r) = decode_data(data) {
        // There is deliberately no plausibility floor: a pulse of 25 or an SpO2
        // of 45 is exactly the event this system exists to detect. The only
        // invariant is the one genuine physical impossibility.
        if let Some(s) = r.spo2 {
            assert!(s <= 100, "SpO2 {s}% is not physically possible");
            assert_ne!(s, 0, "0 is a sentinel and must decode as absent");
        }
        if let Some(p) = r.pulse_bpm {
            assert_ne!(p, 0, "0 is a sentinel and must decode as absent");
            assert_ne!(p, 0xFF, "0xFF is a sentinel and must decode as absent");
        }
    }

    // The scanner must never claim to consume more than it was given, or a
    // caller draining that many bytes would discard live data.
    if let Some((_, used)) = scan_frame(data) {
        assert!(
            used <= data.len(),
            "scan_frame consumed {used} of {}",
            data.len()
        );
        assert!(used >= FRAME_LEN, "a match must consume at least one frame");
    }

    if data.len() == SESSION_DESCRIPTOR_LEN {
        let _ = decode_session_descriptor(data);
        let _ = is_empty_session_descriptor(data);
    }
    if data.len() == STORED_RECORD_LEN {
        if let Some(record) = decode_stored_record(data) {
            assert!(storage_ordinal(record.cursor).is_some());
            if let Some(value) = record.value {
                assert_ne!(value, 0, "zero is a stored sentinel");
                assert_ne!(value, 0x7f, "0x7f is a stored sentinel");
                assert_ne!(value, 0xff, "0xff is a stored sentinel");
            }
        }
    }
});
