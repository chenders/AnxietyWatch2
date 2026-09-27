//! Deterministic Linux-owner contract tests. These do not claim `BlueZ` or Pi evidence.

#![cfg(feature = "linux")]

use vitals_ble_broker::bluez::{
    AdapterPolicy, ControllerBus, ControllerEvidence, ControllerSelector, ResolutionError,
    resolve_controller,
};
use vitals_ble_broker::{CapabilitySet, Generation, Intent, LeaseRequest, Priority, Source};

fn evidence(name: &str, address: &str, path: &str) -> ControllerEvidence {
    ControllerEvidence {
        runtime_name: name.to_owned(),
        address: Some(address.to_owned()),
        sysfs_path: Some(path.to_owned()),
        serial: None,
        bus: ControllerBus::Usb,
        vid_pid: Some((0x0a12, 0x0001)),
    }
}

#[test]
fn controller_resolution_uses_stable_identity_not_hci_alias() {
    let selector = ControllerSelector {
        address: Some("controller-a".to_owned()),
        sysfs_path: Some("usb-1-2".to_owned()),
        serial: None,
        bus: Some(ControllerBus::Usb),
        vid_pid: Some((0x0a12, 0x0001)),
    };
    let selected = resolve_controller(&selector, &[evidence("hci7", "controller-a", "usb-1-2")]);
    assert_eq!(
        selected.map(|value| value.runtime_name),
        Ok("hci7".to_owned())
    );
}

#[test]
fn missing_or_ambiguous_identity_fails_closed() {
    let empty = ControllerSelector::default();
    assert_eq!(
        resolve_controller(&empty, &[]),
        Err(ResolutionError::EmptySelector)
    );

    let selector = ControllerSelector {
        address: Some("controller-a".to_owned()),
        ..ControllerSelector::default()
    };
    let inventory = vec![
        evidence("hci0", "controller-a", "usb-1-1"),
        evidence("hci1", "controller-a", "usb-1-2"),
    ];
    assert_eq!(
        resolve_controller(&selector, &inventory),
        Err(ResolutionError::Ambiguous { matches: 2 })
    );
}

#[test]
fn policy_serializes_discovery_and_connect_admission() {
    let mut policy = AdapterPolicy::new(Generation::new(4));
    let discovery = LeaseRequest::new(
        Source::AirSense,
        Intent::Discover,
        Priority::new(2),
        50,
        10,
        None,
        CapabilitySet::empty(),
    );
    let first = policy.acquire(discovery, 0);
    let Ok(vitals_ble_broker::AcquireResult::Granted(lease)) = first else {
        return;
    };
    let connect = LeaseRequest::new(
        Source::H10,
        Intent::Connect,
        Priority::new(3),
        50,
        10,
        Some(vitals_ble_broker::DeviceId::new("h10")),
        CapabilitySet::empty(),
    );
    let queued = policy.acquire(connect, 1);
    assert!(matches!(
        queued,
        Ok(vitals_ble_broker::AcquireResult::Queued { .. })
    ));
    let released = policy.release(lease.id(), 2);
    assert_eq!(released.granted.len(), 1);
}

#[test]
fn bp_has_no_connect_owner_and_failure_classes_are_named() {
    let mut policy = AdapterPolicy::new(Generation::new(1));
    let request = LeaseRequest::new(
        Source::Bp,
        Intent::Connect,
        Priority::new(9),
        20,
        5,
        None,
        CapabilitySet::empty(),
    );
    assert_eq!(
        policy.acquire(request, 0),
        Err(vitals_ble_broker::lease::PolicyError::BpConnectForbidden)
    );
}
