//! Linux `BlueZ` broker daemon entry point.

#[cfg(all(feature = "linux", target_os = "linux"))]
pub use vitals_ble_broker::server::entrypoint as main;

#[cfg(not(all(feature = "linux", target_os = "linux")))]
fn main() {
    eprintln!("vitals-ble-broker requires the Linux feature and runs on the Pi");
    std::process::exit(64);
}
