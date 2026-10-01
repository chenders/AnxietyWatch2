#![cfg(all(feature = "linux", target_os = "linux"))]
#![allow(clippy::panic, missing_docs)]

use dbus::{
    arg::{RefArg, Variant},
    channel::{MatchingReceiver, Sender},
    message::MatchRule,
    strings::Path,
};
use dbus_tokio::connection;
use std::{
    collections::HashMap,
    io::{BufRead as _, BufReader},
    process::{Child, Command, Stdio},
};
use vitals_ble_broker::Generation;
use vitals_ble_broker::bluez::{
    AdapterIo, BluezAdapterOwner, CharacteristicIo, ControllerBus, ControllerEvidence,
    ControllerSelector, DeviceIo, ServiceIo,
};

const CHILD_ENV: &str = "VITALS_BLE_FAKE_DBUS_CHILD";
const ADAPTER_PATH: &str = "/org/bluez/adapter_test";
const ERROR_ADAPTER_PATH: &str = "/org/bluez/adapter_error";
const DEVICE_PATH: &str = "/org/bluez/adapter_test/dev_01_02_03_04_05_06";
const ERROR_DEVICE_PATH: &str = "/org/bluez/adapter_test/dev_01_02_03_04_05_07";
const SERVICE_PATH: &str = "/org/bluez/adapter_test/dev_01_02_03_04_05_06/service0001";
const CHARACTERISTIC_PATH: &str =
    "/org/bluez/adapter_test/dev_01_02_03_04_05_06/service0001/char0001";

struct BusProcess(Child);

impl Drop for BusProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_private_bus() -> Result<(BusProcess, String), String> {
    let mut child = Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--print-address=1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("start fake D-Bus daemon: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "fake D-Bus daemon stdout was unavailable".to_owned())?;
    let mut lines = BufReader::new(stdout).lines();
    let address = lines
        .next()
        .ok_or_else(|| "fake D-Bus daemon did not publish an address".to_owned())
        .and_then(|line| line.map_err(|error| format!("read fake D-Bus address: {error}")))?;
    Ok((BusProcess(child), address))
}

type Properties = HashMap<String, Variant<Box<dyn RefArg + 'static>>>;
type Interfaces = HashMap<String, Properties>;
type ManagedObjects = HashMap<Path<'static>, Interfaces>;

fn managed_objects() -> ManagedObjects {
    let mut objects = ManagedObjects::new();
    for (path, interface) in [
        (ADAPTER_PATH, "org.bluez.Adapter1"),
        (ERROR_ADAPTER_PATH, "org.bluez.Adapter1"),
        (DEVICE_PATH, "org.bluez.Device1"),
        (ERROR_DEVICE_PATH, "org.bluez.Device1"),
        (SERVICE_PATH, "org.bluez.GattService1"),
        (CHARACTERISTIC_PATH, "org.bluez.GattCharacteristic1"),
    ] {
        let mut interfaces = Interfaces::new();
        interfaces.insert(interface.to_owned(), Properties::new());
        let Ok(path) = Path::new(path) else {
            continue;
        };
        objects.insert(path.clone(), interfaces);
    }
    objects
}

async fn install_fake_bluez(
    connection: &std::sync::Arc<dbus::nonblock::SyncConnection>,
) -> Result<(), String> {
    let request = connection
        .request_name("org.bluez", false, true, false)
        .await
        .map_err(|error| format!("claim fake BlueZ name: {error}"))?;
    if request != dbus::nonblock::stdintf::org_freedesktop_dbus::RequestNameReply::PrimaryOwner {
        return Err("fake BlueZ did not become the primary owner".to_owned());
    }
    let rule = MatchRule::new_method_call();
    connection.start_receive(
        rule,
        Box::new(|message, connection| {
            let interface = message.interface().map(|value| value.to_string());
            let member = message.member().map(|value| value.to_string());
            if interface.as_deref() == Some("org.freedesktop.DBus.ObjectManager")
                && member.as_deref() == Some("GetManagedObjects")
            {
                let reply = message.method_return().append1(managed_objects());
                let _ = connection.send(reply);
                return true;
            }
            if interface.as_deref() == Some("org.freedesktop.DBus.Properties")
                && member.as_deref() == Some("Get")
            {
                let property = message.get2::<String, String>().1;
                let path = message.path().map(|value| value.to_string());
                if property.as_deref() == Some("UUID")
                    || path.as_deref() == Some(ERROR_ADAPTER_PATH)
                    || path.as_deref() == Some(ERROR_DEVICE_PATH)
                {
                    let error_name: dbus::strings::ErrorName<'static> =
                        "org.bluez.Error.Failed".into();
                    let Ok(error_message) =
                        std::ffi::CString::new("deterministic fake BlueZ property failure")
                    else {
                        return true;
                    };
                    let reply = message.error(&error_name, &error_message);
                    let _ = connection.send(reply);
                    return true;
                }
                let value: Variant<Box<dyn RefArg + 'static>> = match property.as_deref() {
                    Some("Powered" | "Connected" | "ServicesResolved") => Variant(Box::new(true)),
                    Some("UUID") => {
                        Variant(Box::new("0000180d-0000-1000-8000-00805f9b34fb".to_owned()))
                    }
                    _ => Variant(Box::new(false)),
                };
                let reply = message.method_return().append1(value);
                let _ = connection.send(reply);
                return true;
            }
            let error_name: dbus::strings::ErrorName<'static> = "org.bluez.Error.Failed".into();
            let Ok(error_message) =
                std::ffi::CString::new("deterministic fake BlueZ rejects this operation")
            else {
                return true;
            };
            let reply = message.error(&error_name, &error_message);
            let _ = connection.send(reply);
            true
        }),
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn exercise_real_bluer_wrappers() -> Result<(), String> {
    let (resource, connection) = connection::new_system_sync()
        .map_err(|error| format!("connect fake system bus: {error}"))?;
    tokio::spawn(async move {
        let _ = resource.await;
    });
    install_fake_bluez(&connection).await?;

    let session = bluer::Session::new()
        .await
        .map_err(|error| format!("open bluer session: {error}"))?;
    let adapter = session
        .adapter("adapter_test")
        .map_err(|error| format!("construct adapter proxy: {error}"))?;
    let evidence = ControllerEvidence {
        runtime_name: "adapter_test".to_owned(),
        address: Some("controller-test".to_owned()),
        sysfs_path: None,
        serial: None,
        bus: ControllerBus::Other,
        vid_pid: None,
    };
    let selector = ControllerSelector {
        address: Some("controller-test".to_owned()),
        ..ControllerSelector::default()
    };
    assert!(
        BluezAdapterOwner::open(&selector, &[evidence], Generation::new(1))
            .await
            .is_ok()
    );
    let missing_evidence = ControllerEvidence {
        runtime_name: "adapter_missing".to_owned(),
        address: Some("missing-controller".to_owned()),
        sysfs_path: None,
        serial: None,
        bus: ControllerBus::Other,
        vid_pid: None,
    };
    let missing_selector = ControllerSelector {
        address: Some("missing-controller".to_owned()),
        ..ControllerSelector::default()
    };
    assert!(
        BluezAdapterOwner::open(&missing_selector, &[missing_evidence], Generation::new(1),)
            .await
            .is_err()
    );
    assert!(AdapterIo::powered(&adapter).await.is_ok());
    assert!(AdapterIo::discovering(&adapter).await.is_ok());
    let error_adapter = session
        .adapter("adapter_error")
        .map_err(|error| format!("construct error adapter proxy: {error}"))?;
    assert!(AdapterIo::powered(&error_adapter).await.is_err());
    assert!(AdapterIo::discovering(&error_adapter).await.is_err());
    assert_eq!(
        AdapterIo::discovery_filter(&adapter).await,
        bluer::DiscoveryFilter::default()
    );
    assert!(
        AdapterIo::set_discovery_filter(&adapter, bluer::DiscoveryFilter::default())
            .await
            .is_ok()
    );
    assert!(AdapterIo::discover_devices(&adapter).await.is_err());
    assert!(AdapterIo::set_powered(&adapter, true).await.is_err());

    let address = bluer::Address::new([1, 2, 3, 4, 5, 6]);
    let device = adapter
        .device(address)
        .map_err(|error| format!("construct device proxy: {error}"))?;
    assert!(DeviceIo::connected(&device).await.is_ok());
    assert!(DeviceIo::services_resolved(&device).await.is_ok());
    let error_device = adapter
        .device(bluer::Address::new([1, 2, 3, 4, 5, 7]))
        .map_err(|error| format!("construct error device proxy: {error}"))?;
    assert!(DeviceIo::connected(&error_device).await.is_err());
    assert!(DeviceIo::services_resolved(&error_device).await.is_err());
    assert!(DeviceIo::services(&error_device).await.is_err());
    assert!(DeviceIo::connect(&device).await.is_err());
    assert!(DeviceIo::disconnect(&device).await.is_err());
    let services = DeviceIo::services(&device)
        .await
        .map_err(|error| format!("resolve fake services: {error}"))?;
    let Some(service) = services.into_iter().next() else {
        return Err("fake BlueZ returned no service".to_owned());
    };
    assert!(ServiceIo::uuid(&service).await.is_err());
    let characteristics = ServiceIo::characteristics(&service)
        .await
        .map_err(|error| format!("resolve fake characteristics: {error}"))?;
    let Some(characteristic) = characteristics.into_iter().next() else {
        return Err("fake BlueZ returned no characteristic".to_owned());
    };
    assert!(CharacteristicIo::uuid(&characteristic).await.is_err());
    assert!(CharacteristicIo::notify(&characteristic).await.is_err());
    assert!(
        CharacteristicIo::write(&characteristic, &[1, 2])
            .await
            .is_err()
    );
    Ok(())
}

#[test]
fn production_bluer_wrappers_run_against_deterministic_fake_dbus() {
    if std::env::var_os(CHILD_ENV).is_none() {
        let (bus, address) = match start_private_bus() {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        };
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => panic!("locate test executable: {error}"),
        };
        let status = Command::new(executable)
            .arg("--exact")
            .arg("production_bluer_wrappers_run_against_deterministic_fake_dbus")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .env("DBUS_SYSTEM_BUS_ADDRESS", address)
            .status();
        drop(bus);
        assert!(status.is_ok_and(|status| status.success()));
        return;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => panic!("build test runtime: {error}"),
    };
    let result = runtime.block_on(exercise_real_bluer_wrappers());
    assert!(result.is_ok(), "fake BlueZ wrapper test failed: {result:?}");
}
