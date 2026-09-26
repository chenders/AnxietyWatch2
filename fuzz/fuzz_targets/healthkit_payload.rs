#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_console::healthkit::decode_healthkit_payload;

fuzz_target!(|bytes: &[u8]| {
    let _ = decode_healthkit_payload(bytes);
});
