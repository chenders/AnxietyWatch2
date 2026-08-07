#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::bp::bp_decode_2a35;

fuzz_target!(|data: &[u8]| {
    let _ = bp_decode_2a35(data);
});
