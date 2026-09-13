//! Pure decoders for physiological sensor wire formats.
//!
//! Every function here parses **attacker-controlled input** — a malformed
//! advertisement or a truncated GATT indication is what `BlueZ` hands you when a
//! device browns out mid-packet. The crate is therefore deliberately minimal:
//!
//! - `#![no_std]`, no allocation, no I/O, no global state.
//! - `#![forbid(unsafe_code)]` — slice indexing is bounds-checked by the
//!   compiler rather than by reviewer attention.
//! - Every decoder is `(&[u8]) -> Option<T>` or `(u16) -> Option<f64>`: a
//!   malformed input yields `None`, never a partially-populated value.
//!
//! `Option` is load-bearing, not stylistic. The system's core invariant is that
//! absent data contributes nothing and is never coerced to a number. In C that
//! is a convention enforced by sentinels — and a sentinel *is* a value, so
//! nothing stops it being read as one. Here it is enforced by the type system.

#![no_std]
#![forbid(unsafe_code)]
// unwrap/panic are denied in library code — a decoder must never abort on
// hostile input. Tests are the one place asserting-by-panic is the point.
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod bp;
pub mod emay;
pub mod events;
pub mod fig;
pub mod hr;
pub mod sfloat;
pub mod stream;

pub use emay::decode_data;
pub use hr::decode_2a37;
pub use sfloat::decode_sfloat;
