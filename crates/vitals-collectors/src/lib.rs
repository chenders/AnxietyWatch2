//! Shared, host-independent policy for the BLE collectors.
//!
//! The `BlueZ` transports live in the binaries and are Linux-only; the
//! sample-building gates live here so they compile and test anywhere.

pub mod as11_crypto;
pub mod as11_rpc;
pub mod bp_build;
pub mod h10_build;
pub mod h10_reconnect;
