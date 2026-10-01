//! Linux `BlueZ` broker daemon entry point.

#[cfg(not(all(feature = "linux", target_os = "linux")))]
fn main() {
    eprintln!("vitals-ble-broker requires the Linux feature and runs on the Pi");
    std::process::exit(64);
}

#[cfg(all(feature = "linux", target_os = "linux", not(test)))]
fn main() -> std::process::ExitCode {
    finish(startup(vitals_ble_broker::server::BrokerConfig::from_env()))
}

#[cfg(all(feature = "linux", target_os = "linux", test))]
fn main() -> std::process::ExitCode {
    finish(Err((64, "test startup rejection".to_owned())))
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn finish(result: Result<(), (u8, String)>) -> std::process::ExitCode {
    match result {
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
    use std::path::PathBuf;

    use vitals_ble_broker::bluez::ControllerSelector;
    use vitals_ble_broker::server::BrokerConfig;

    #[test]
    fn startup_reports_configuration_failure_without_starting_bluetooth() {
        assert_eq!(
            super::startup(Err("VITALS_BLE_ADAPTER_SELECTOR is unset".to_owned())),
            Err((64, "VITALS_BLE_ADAPTER_SELECTOR is unset".to_owned()))
        );
    }

    #[test]
    fn startup_reports_runtime_failure_without_binding_a_socket() {
        let config = BrokerConfig {
            socket: PathBuf::from("/tmp/vitals-ble-broker-coverage.sock"),
            metrics_path: PathBuf::from("/tmp/vitals-ble-broker-coverage.prom"),
            selector_text: "address=00:00:00XX:XX:XX".to_owned(),
            selector: ControllerSelector {
                address: Some("00:00:00XX:XX:XX".to_owned()),
                ..ControllerSelector::default()
            },
            allowed_uid: None,
        };
        let result = super::startup(Ok(config));
        assert!(matches!(result, Err((70, _))));
    }

    #[test]
    fn test_entrypoint_and_exit_codes_are_deterministic() {
        assert_eq!(super::main(), std::process::ExitCode::from(64));
        assert_eq!(super::finish(Ok(())), std::process::ExitCode::SUCCESS);
        assert_eq!(
            super::finish(Err((70, "broker stopped".to_owned()))),
            std::process::ExitCode::from(70)
        );
    }
}
