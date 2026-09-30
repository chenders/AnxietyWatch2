//! Linux integration tests for the typed broker client.

#![cfg(all(feature = "linux", target_os = "linux"))]
#![expect(
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "the scripted Unix peer keeps protocol fixtures concise"
)]

use std::{path::PathBuf, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
use vitals_ble_broker::{
    BrokerClient, Capability, CapabilitySet, Generation, LeaseId, Priority, ServerMessage, Source,
    decode, encode_server,
};

const SELECTOR: &str = "bus=usb;vid_pid=0a12:0001";
const TARGET: &str = "target";

fn socket_path(test_name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "vitals-ble-broker-client-{test_name}-{}",
        std::process::id()
    ))
}

async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Vec<u8> {
    let mut header = [0_u8; 11];
    reader
        .read_exact(&mut header)
        .await
        .expect("client frame header");
    let length = u32::from_be_bytes([header[7], header[8], header[9], header[10]]) as usize;
    let mut frame = Vec::with_capacity(11 + length);
    frame.extend_from_slice(&header);
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .expect("client frame payload");
    frame.extend_from_slice(&payload);
    frame
}

async fn send_server(stream: &mut UnixStream, message: ServerMessage) {
    let frame = encode_server(&message).expect("server test frame");
    stream.write_all(&frame).await.expect("server response");
}

async fn expect_operation(stream: &mut UnixStream, operation_id: u64) {
    let message = decode(&read_frame(stream).await).expect("client operation");
    assert!(matches!(
        message,
        vitals_ble_broker::ClientMessage::Operation(operation)
            if operation.operation_id == operation_id
    ));
    send_server(stream, ServerMessage::OperationAccepted { operation_id }).await;
}

#[tokio::test]
async fn typed_client_reports_deadlines_and_cancellation_blockers() -> Result<(), String> {
    let path = socket_path("timeouts");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let _ = read_frame(&mut stream).await;
        send_server(
            &mut stream,
            ServerMessage::HelloAccepted {
                generation: Generation::new(1),
            },
        )
        .await;
        let _ = read_frame(&mut stream).await;
        send_server(
            &mut stream,
            ServerMessage::LeaseGranted {
                lease_id: LeaseId::new(1),
                generation: Generation::new(1),
                expires_at: 10,
            },
        )
        .await;

        for (_operation_id, cancellation) in [
            (1_u64, ServerMessage::Cancelled { operation_id: 1 }),
            (
                2,
                ServerMessage::Error {
                    operation_id: 2,
                    code: 4,
                    detail: "cancel rejected".to_owned(),
                },
            ),
            (3, ServerMessage::OperationAccepted { operation_id: 3 }),
        ] {
            let _ = read_frame(&mut stream).await;
            let _ = read_frame(&mut stream).await;
            send_server(&mut stream, cancellation).await;
        }
        let _ = read_frame(&mut stream).await;
        let _ = read_frame(&mut stream).await;
        Ok::<(), String>(())
    });

    let capabilities = CapabilitySet::empty()
        .with(Capability::Connect)
        .with(Capability::Subscribe)
        .with(Capability::Write);
    let mut client =
        BrokerClient::connect_source(Source::AirSense, &path, SELECTOR, TARGET, capabilities)
            .await
            .map_err(|error| error.to_string())?;
    client
        .acquire_lease(
            vitals_ble_broker::Intent::Connect,
            Priority::new(1),
            10,
            10,
            TARGET,
            capabilities,
            "timeout test",
        )
        .await
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        client
            .connect_device_bounded(TARGET, Duration::from_millis(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::DeadlineExceeded {
            operation: "connect"
        })
    ));
    assert!(matches!(
        client
            .discover_target_bounded(TARGET, Duration::from_millis(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::CancellationFailed {
            operation: "discover",
            ..
        })
    ));
    assert!(matches!(
        client
            .subscribe_bounded("handle", Duration::from_millis(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::CancellationFailed {
            operation: "subscribe",
            ..
        })
    ));
    assert!(matches!(
        client
            .write_bounded("handle", &[1], Duration::from_millis(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::CancellationFailed {
            operation: "write",
            ..
        })
    ));
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[tokio::test]
async fn typed_client_covers_successful_lease_operations_and_events() -> Result<(), String> {
    let path = socket_path("success");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let hello = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(hello, vitals_ble_broker::ClientMessage::Hello(_)));
        send_server(
            &mut stream,
            ServerMessage::HelloAccepted {
                generation: Generation::new(3),
            },
        )
        .await;

        let acquire = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(
            acquire,
            vitals_ble_broker::ClientMessage::Acquire(_)
        ));
        send_server(
            &mut stream,
            ServerMessage::LeaseGranted {
                lease_id: LeaseId::new(7),
                generation: Generation::new(3),
                expires_at: 99,
            },
        )
        .await;
        let acquire = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(
            acquire,
            vitals_ble_broker::ClientMessage::Acquire(_)
        ));
        send_server(
            &mut stream,
            ServerMessage::LeaseGranted {
                lease_id: LeaseId::new(8),
                generation: Generation::new(3),
                expires_at: 100,
            },
        )
        .await;

        expect_operation(&mut stream, 1).await;
        expect_operation(&mut stream, 2).await;
        let resolve = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(
            resolve,
            vitals_ble_broker::ClientMessage::Operation(operation)
                if operation.operation_id == 3
                    && operation.kind == vitals_ble_broker::OperationKind::Resolve
        ));
        send_server(
            &mut stream,
            ServerMessage::Resolved {
                operation_id: 3,
                handle: "opaque-handle".to_owned(),
            },
        )
        .await;
        expect_operation(&mut stream, 4).await;
        let write = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(
            write,
            vitals_ble_broker::ClientMessage::Operation(operation)
                if operation.operation_id == 5
                    && operation.payload == vec![1, 2, 3]
        ));
        send_server(
            &mut stream,
            ServerMessage::OperationAccepted { operation_id: 5 },
        )
        .await;

        let observe = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(
            observe,
            vitals_ble_broker::ClientMessage::Operation(operation)
                if operation.operation_id == 6
                    && operation.kind == vitals_ble_broker::OperationKind::ObserveDevice
        ));
        send_server(
            &mut stream,
            ServerMessage::DeviceState {
                operation_id: 6,
                connected: true,
                services_resolved: true,
            },
        )
        .await;

        send_server(
            &mut stream,
            ServerMessage::Frame {
                lease_id: LeaseId::new(8),
                operation_id: 4,
                indication: true,
                payload: vec![9],
            },
        )
        .await;
        send_server(
            &mut stream,
            ServerMessage::DeviceState {
                operation_id: 4,
                connected: true,
                services_resolved: false,
            },
        )
        .await;
        send_server(
            &mut stream,
            ServerMessage::Disconnected {
                operation_id: 4,
                reason: "test disconnect".to_owned(),
            },
        )
        .await;

        let release = decode(&read_frame(&mut stream).await).map_err(|error| error.to_string())?;
        assert!(matches!(
            release,
            vitals_ble_broker::ClientMessage::Release(_)
        ));
        send_server(
            &mut stream,
            ServerMessage::Released {
                lease_id: LeaseId::new(8),
            },
        )
        .await;
        Ok::<(), String>(())
    });

    let capabilities = CapabilitySet::empty()
        .with(Capability::Connect)
        .with(Capability::Resolve)
        .with(Capability::Subscribe)
        .with(Capability::Write);
    let mut client =
        BrokerClient::connect_source(Source::AirSense, &path, SELECTOR, TARGET, capabilities)
            .await
            .map_err(|error| error.to_string())?;
    let lease = client
        .acquire_discovery(Priority::new(2), 10, 20, TARGET)
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(lease.id, LeaseId::new(7));
    let lease = client
        .acquire_connect(Priority::new(2), 10, 20, TARGET)
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(lease.id, LeaseId::new(8));
    client
        .connect_device(TARGET)
        .await
        .map_err(|error| error.to_string())?;
    client
        .discover_target_bounded(TARGET, Duration::from_secs(1))
        .await
        .map_err(|error| error.to_string())?;
    let handle = client
        .resolve_characteristic("service", "characteristic")
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(handle, "opaque-handle");
    client
        .subscribe(handle.clone())
        .await
        .map_err(|error| error.to_string())?;
    client
        .write_bounded(handle, &[1, 2, 3], Duration::from_secs(1))
        .await
        .map_err(|error| error.to_string())?;
    assert_eq!(
        client
            .observe_device_bounded(TARGET, Duration::from_secs(1))
            .await
            .map_err(|error| error.to_string())?,
        (true, true)
    );
    assert!(matches!(
        client.next_event().await.map_err(|error| error.to_string())?,
        Some(vitals_ble_broker::BrokerEvent::Frame {
            payload,
            indication: true
        }) if payload == vec![9]
    ));
    assert!(matches!(
        client
            .next_event()
            .await
            .map_err(|error| error.to_string())?,
        Some(vitals_ble_broker::BrokerEvent::DeviceState {
            connected: true,
            services_resolved: false
        })
    ));
    assert!(matches!(
        client.next_event().await.map_err(|error| error.to_string())?,
        Some(vitals_ble_broker::BrokerEvent::Disconnected { reason })
            if reason == "test disconnect"
    ));
    client
        .release("test complete")
        .await
        .map_err(|error| error.to_string())?;
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}
