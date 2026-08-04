//! Bluetooth SIG Blood Pressure Measurement, characteristic `0x2A35`.
//!
//! Implemented by T-D6. The characteristic is an **indication**, so the client
//! writes `0x0002` to the CCCD — writing `0x0001` yields a successful
//! subscription that silently delivers nothing, forever.
