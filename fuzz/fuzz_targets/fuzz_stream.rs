//! AirSense StreamData JSON fuzz target.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::stream::decode_stream;

fuzz_target!(|data: &[u8]| {
    let _ = decode_stream(data);
});
