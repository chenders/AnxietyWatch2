//! AirSense FIG framing fuzz target.
#![no_main]

use libfuzzer_sys::fuzz_target;
use vitals_decode::fig::{FigReassembler, decode_fig};

fuzz_target!(|data: &[u8]| {
    let _ = decode_fig(data);
    let mut reassembler = FigReassembler::new();
    if reassembler.feed(data) {
        let _ = reassembler.take();
    }
});
