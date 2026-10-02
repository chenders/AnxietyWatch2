//! BP policy tests. These prove the software boundary, not physical kernel evidence.

use vitals_ble_broker::bluez::{ControllerBus, ControllerEvidence, ControllerSelector};
use vitals_ble_broker::bp::{
    AddressType, BpFrameLedger, FrameDurability, KernelPolicyBoundary, RegistrationFailure,
    RegistrationState, bp_capabilities, validate_bp_capabilities,
};
use vitals_ble_broker::{Capability, Generation};

fn controller(runtime_name: &str) -> ControllerEvidence {
    ControllerEvidence {
        runtime_name: runtime_name.to_owned(),
        address: Some("controller-a".to_owned()),
        sysfs_path: Some("usb-1-2".to_owned()),
        serial: None,
        bus: ControllerBus::Usb,
        vid_pid: Some((0x0a12, 0x0001)),
    }
}

fn selector() -> ControllerSelector {
    ControllerSelector {
        address: Some("controller-a".to_owned()),
        sysfs_path: Some("usb-1-2".to_owned()),
        serial: None,
        bus: Some(ControllerBus::Usb),
        vid_pid: Some((0x0a12, 0x0001)),
    }
}

#[test]
fn bp_capability_set_cannot_encode_userspace_connect() {
    let capabilities = bp_capabilities();
    assert!(capabilities.contains(Capability::KernelObserve));
    assert!(!capabilities.contains(Capability::Connect));
    assert!(validate_bp_capabilities(capabilities).is_ok());
    assert_eq!(
        validate_bp_capabilities(capabilities.with(Capability::Connect)),
        Err(vitals_ble_broker::lease::PolicyError::BpConnectForbidden)
    );
}

#[test]
fn kernel_registration_uses_stable_controller_and_re_registers_on_hotplug() {
    let mut boundary = KernelPolicyBoundary::new(selector(), "bp-cuff", AddressType::Random);
    let request = boundary.prepare_registration(&[controller("hci7")], Generation::new(3));
    assert!(request.is_ok());
    let Ok(request) = request else {
        return;
    };
    assert_eq!(request.controller.runtime_name, "hci7");
    boundary.record_result(&request, Ok(()));
    assert_eq!(
        boundary.state(),
        &RegistrationState::Registered {
            runtime_name: "hci7".to_owned(),
            generation: Generation::new(3)
        }
    );

    let hotplug = boundary.prepare_hotplug(&[controller("hci0")], Generation::new(4));
    assert!(hotplug.is_ok());
    let Ok(hotplug) = hotplug else {
        return;
    };
    assert_eq!(hotplug.controller.runtime_name, "hci0");
    boundary.record_result(&hotplug, Err(RegistrationFailure::ManagementRejected));
    assert_eq!(
        boundary.state(),
        &RegistrationState::Failed {
            failure: RegistrationFailure::ManagementRejected
        }
    );
}

#[test]
fn raw_journal_precedes_sync_append_and_ack() {
    let mut ledger = BpFrameLedger::received();
    assert!(!ledger.sync_append_sent());
    assert!(ledger.journal_synced());
    assert!(ledger.sync_append_sent());
    assert!(ledger.durable_reply());
    assert_eq!(ledger.state(), FrameDurability::DurableAcknowledged);

    let mut crashed = BpFrameLedger::received();
    crashed.durability_unknown();
    assert_eq!(crashed.state(), FrameDurability::DurabilityUnknown);
    assert!(!crashed.durable_reply());
}

#[test]
fn registration_failures_and_ledger_rejections_are_named() {
    for failure in [
        RegistrationFailure::ManagementRejected,
        RegistrationFailure::ControllerUnavailable,
        RegistrationFailure::PolicyRejected,
    ] {
        assert!(!failure.to_string().is_empty());
    }
    let mut ledger = BpFrameLedger::received();
    assert!(ledger.journal_synced());
    assert!(!ledger.journal_synced());
    assert!(ledger.sync_append_sent());
    assert!(!ledger.sync_append_sent());
    assert!(ledger.durable_reply());
    assert!(!ledger.durable_reply());
}

#[test]
fn registration_failure_is_observable_and_not_success() {
    let mut boundary = KernelPolicyBoundary::new(selector(), "bp-cuff", AddressType::Random);
    let request = boundary.prepare_registration(&[controller("hci0")], Generation::new(1));
    assert!(request.is_ok());
    let Ok(request) = request else {
        return;
    };
    boundary.record_result(&request, Err(RegistrationFailure::ControllerUnavailable));
    assert_eq!(
        boundary.state(),
        &RegistrationState::Failed {
            failure: RegistrationFailure::ControllerUnavailable
        }
    );
    assert_ne!(boundary.state(), &RegistrationState::Unregistered);
}
