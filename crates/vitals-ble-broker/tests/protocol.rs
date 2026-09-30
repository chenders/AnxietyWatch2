#![allow(missing_docs)]

use vitals_ble_broker::{
    Acquire, Capability, CapabilitySet, ClientMessage, FakeReply, FakeTransport, Generation, Hello,
    Intent, Operation, OperationKind, PROTOCOL_VERSION, Priority, ProtocolError, Source, decode,
    encode,
};

fn hello(source: Source, capabilities: CapabilitySet) -> ClientMessage {
    ClientMessage::Hello(Hello {
        protocol_version: PROTOCOL_VERSION,
        source,
        adapter_selector: "verified-sena".to_owned(),
        device_selector: Some("peer".to_owned()),
        capabilities,
    })
}

#[test]
fn typed_operation_round_trips_without_arbitrary_dbus_fields() {
    let message = ClientMessage::Operation(Operation {
        lease_id: vitals_ble_broker::LeaseId::new(4),
        operation_id: 9,
        kind: OperationKind::Write,
        target: "rx-characteristic".to_owned(),
        payload: vec![0, 1, 2, 255],
    });
    let frame_result = encode(&message);
    assert!(frame_result.is_ok());
    let Ok(frame) = frame_result else {
        return;
    };
    assert_eq!(decode(&frame), Ok(message));
}

#[test]
fn handshake_and_queue_capacity_are_enforced() {
    let mut transport = FakeTransport::new(Generation::new(3), 1);
    let handshake_result = encode(&hello(Source::H10, CapabilitySet::empty()));
    assert!(handshake_result.is_ok());
    let Ok(handshake) = handshake_result else {
        return;
    };
    assert_eq!(
        transport.send(&handshake),
        Ok(FakeReply::HelloAccepted {
            generation: Generation::new(3)
        })
    );
    let acquire = ClientMessage::Acquire(Acquire {
        source: Source::H10,
        intent: Intent::Connect,
        priority: Priority::new(2),
        deadline: 100,
        max_hold: 10,
        preemptible: false,
        device_selector: Some("h10".to_owned()),
        capabilities: CapabilitySet::from_capability(Capability::Connect),
        reason: "test".to_owned(),
    });
    let frame_result = encode(&acquire);
    assert!(frame_result.is_ok());
    let Ok(frame) = frame_result else {
        return;
    };
    assert_eq!(transport.send(&frame), Ok(FakeReply::Accepted));
    assert_eq!(
        transport.send(&frame),
        Err(ProtocolError::QueueFull { capacity: 1 })
    );
}

#[test]
fn bp_cannot_negotiate_or_send_connect() {
    let mut transport = FakeTransport::new(Generation::new(1), 4);
    let forbidden = hello(
        Source::Bp,
        CapabilitySet::from_capability(Capability::Connect),
    );
    let frame_result = encode(&forbidden);
    assert!(frame_result.is_ok());
    let Ok(frame) = frame_result else {
        return;
    };
    assert_eq!(
        transport.send(&frame),
        Err(ProtocolError::BpConnectForbidden)
    );

    let allowed = hello(
        Source::Bp,
        CapabilitySet::from_capability(Capability::KernelObserve),
    );
    let frame_result = encode(&allowed);
    assert!(frame_result.is_ok());
    let Ok(frame) = frame_result else {
        return;
    };
    assert!(matches!(
        transport.send(&frame),
        Ok(FakeReply::HelloAccepted { .. })
    ));
    let operation = ClientMessage::Operation(Operation {
        lease_id: vitals_ble_broker::LeaseId::new(1),
        operation_id: 1,
        kind: OperationKind::Connect,
        target: "bp".to_owned(),
        payload: Vec::new(),
    });
    let frame_result = encode(&operation);
    assert!(frame_result.is_ok());
    let Ok(frame) = frame_result else {
        return;
    };
    assert_eq!(
        transport.send(&frame),
        Err(ProtocolError::BpConnectForbidden)
    );
}
