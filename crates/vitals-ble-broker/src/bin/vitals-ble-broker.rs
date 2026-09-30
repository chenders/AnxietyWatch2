//! Linux `BlueZ` broker daemon entry point.

#[cfg(not(all(feature = "linux", target_os = "linux")))]
fn main() {
    eprintln!("vitals-ble-broker requires the Linux feature and runs on the Pi");
    std::process::exit(64);
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn main() -> std::process::ExitCode {
    match startup(vitals_ble_broker::server::BrokerConfig::from_env()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err((code, error)) => {
            eprintln!("vitals-ble-broker: {error}");
            std::process::ExitCode::from(code)
        }
    }
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn startup(
    config: Result<vitals_ble_broker::server::BrokerConfig, String>,
) -> Result<(), (u8, String)> {
    let config = config.map_err(|error| (64, error))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| (70, format!("could not start tokio runtime: {error}")))?;
    runtime
        .block_on(vitals_ble_broker::server::run(config))
        .map_err(|error| (70, error))
}

#[cfg(all(test, feature = "linux", target_os = "linux"))]
mod tests {
    #[test]
    fn startup_reports_configuration_failure_without_starting_bluetooth() {
        assert_eq!(
            super::startup(Err("VITALS_BLE_ADAPTER_SELECTOR is unset".to_owned())),
            Err((64, "VITALS_BLE_ADAPTER_SELECTOR is unset".to_owned()))
        );
    }
}
