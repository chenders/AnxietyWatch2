//! Linux `BlueZ` broker daemon entry point.

#[cfg(not(all(feature = "linux", target_os = "linux")))]
fn main() {
    eprintln!("vitals-ble-broker requires the Linux feature and runs on the Pi");
    std::process::exit(64);
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn main() {
    let config = match vitals_ble_broker::server::BrokerConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("vitals-ble-broker: {error}");
            std::process::exit(64);
        }
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("vitals-ble-broker: could not start tokio runtime");
        std::process::exit(70);
    };
    if let Err(error) = runtime.block_on(vitals_ble_broker::server::run(config)) {
        eprintln!("vitals-ble-broker: {error}");
        std::process::exit(70);
    }
}
