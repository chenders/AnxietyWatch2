#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::events::decode_event;

fuzz_target!(|data: &[u8]| {
    let _ = decode_event(data);
});
