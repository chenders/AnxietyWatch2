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
        send_server(&mut stream, ServerMessage::Cancelled { operation_id: 4 }).await;
        let _ = read_frame(&mut stream).await;
        let _ = read_frame(&mut stream).await;
        send_server(&mut stream, ServerMessage::Cancelled { operation_id: 5 }).await;
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
        Err(vitals_ble_broker::BrokerClientError::DeadlineExceeded { operation: "write" })
    ));
    assert!(matches!(
        client
            .observe_device_bounded(TARGET, Duration::from_millis(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::DeadlineExceeded {
            operation: "observe"
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

async fn connect_with_response(
    name: &str,
    response: ServerMessage,
) -> Result<vitals_ble_broker::BrokerClientError, String> {
    let path = socket_path(name);
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let _ = read_frame(&mut stream).await;
        send_server(&mut stream, response).await;
        Ok::<(), String>(())
    });
    let Err(result) = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await
    else {
        return Err("scripted handshake unexpectedly succeeded".to_owned());
    };
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(result)
}

async fn acquire_test(
    client: &mut BrokerClient,
) -> Result<(), vitals_ble_broker::BrokerClientError> {
    client
        .acquire_lease(
            vitals_ble_broker::Intent::Connect,
            Priority::new(1),
            1,
            1,
            TARGET,
            CapabilitySet::empty(),
            "test",
        )
        .await
        .map(|_| ())
}

#[tokio::test]
async fn typed_client_reports_handshake_rejections() -> Result<(), String> {
    assert!(matches!(
        connect_with_response(
            "hello-error",
            ServerMessage::Error {
                operation_id: 0,
                code: 3,
                detail: "bad hello".to_owned(),
            },
        )
        .await?,
        vitals_ble_broker::BrokerClientError::Broker { code: 3, .. }
    ));
    assert!(matches!(
        connect_with_response(
            "hello-unexpected",
            ServerMessage::Released {
                lease_id: LeaseId::new(2),
            },
        )
        .await?,
        vitals_ble_broker::BrokerClientError::UnexpectedResponse("HelloAccepted")
    ));
    Ok(())
}

#[tokio::test]
async fn typed_client_reports_operation_rejections() -> Result<(), String> {
    let path = socket_path("rejections");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let _ = read_frame(&mut stream).await;
        send_server(
            &mut stream,
            ServerMessage::HelloAccepted {
                generation: Generation::new(4),
            },
        )
        .await;
        for response in [
            ServerMessage::LeaseQueued { ticket: 11 },
            ServerMessage::Error {
                operation_id: 0,
                code: 4,
                detail: "lease rejected".to_owned(),
            },
            ServerMessage::DeviceState {
                operation_id: 0,
                connected: false,
                services_resolved: false,
            },
            ServerMessage::LeaseGranted {
                lease_id: LeaseId::new(12),
                generation: Generation::new(4),
                expires_at: 44,
            },
        ] {
            let _ = read_frame(&mut stream).await;
            send_server(&mut stream, response).await;
        }
        for response in [
            ServerMessage::Error {
                operation_id: 1,
                code: 5,
                detail: "observe rejected".to_owned(),
            },
            ServerMessage::OperationAccepted { operation_id: 2 },
            ServerMessage::OperationAccepted { operation_id: 3 },
            ServerMessage::Error {
                operation_id: 4,
                code: 6,
                detail: "resolve rejected".to_owned(),
            },
            ServerMessage::Error {
                operation_id: 5,
                code: 7,
                detail: "subscribe rejected".to_owned(),
            },
            ServerMessage::DeviceState {
                operation_id: 6,
                connected: true,
                services_resolved: false,
            },
        ] {
            let _ = read_frame(&mut stream).await;
            send_server(&mut stream, response).await;
        }
        send_server(
            &mut stream,
            ServerMessage::Error {
                operation_id: 0,
                code: 8,
                detail: "event stream ended".to_owned(),
            },
        )
        .await;
        let _ = read_frame(&mut stream).await;
        send_server(
            &mut stream,
            ServerMessage::Error {
                operation_id: 0,
                code: 9,
                detail: "release rejected".to_owned(),
            },
        )
        .await;
        Ok::<(), String>(())
    });
    let mut client = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await
    .map_err(|error| error.to_string())?;
    assert!(matches!(
        acquire_test(&mut client).await,
        Err(vitals_ble_broker::BrokerClientError::Queued { ticket: 11 })
    ));
    assert!(matches!(
        acquire_test(&mut client).await,
        Err(vitals_ble_broker::BrokerClientError::Broker { code: 4, .. })
    ));
    assert!(matches!(
        acquire_test(&mut client).await,
        Err(vitals_ble_broker::BrokerClientError::UnexpectedResponse(
            "LeaseGranted"
        ))
    ));
    acquire_test(&mut client)
        .await
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        client
            .observe_device_bounded(TARGET, Duration::from_secs(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::Broker { code: 5, .. })
    ));
    assert!(matches!(
        client
            .observe_device_bounded(TARGET, Duration::from_secs(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::UnexpectedResponse(
            "DeviceState"
        ))
    ));
    client
        .connect_device_bounded(TARGET, Duration::from_secs(1))
        .await
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        client
            .resolve_characteristic_bounded("service", "characteristic", Duration::from_secs(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::Broker { code: 6, .. })
    ));
    assert!(matches!(
        client
            .subscribe_bounded("handle", Duration::from_secs(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::Broker { code: 7, .. })
    ));
    assert!(matches!(
        client
            .write_bounded("handle", &[], Duration::from_secs(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::UnexpectedResponse(
            "OperationAccepted"
        ))
    ));
    assert!(matches!(
        client.next_event().await,
        Err(vitals_ble_broker::BrokerClientError::Broker { code: 8, .. })
    ));
    assert!(matches!(
        client.release("rejected").await,
        Err(vitals_ble_broker::BrokerClientError::Broker { code: 9, .. })
    ));
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}

async fn run_single_operation_with_disconnect(
    name: &str,
    resolve: bool,
) -> Result<vitals_ble_broker::BrokerClientError, String> {
    let path = socket_path(name);
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
        let _ = read_frame(&mut stream).await;
        drop(stream);
        Ok::<(), String>(())
    });
    let mut client = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await
    .map_err(|error| error.to_string())?;
    acquire_test(&mut client)
        .await
        .map_err(|error| error.to_string())?;
    let result = if resolve {
        client
            .resolve_characteristic_bounded("service", "characteristic", Duration::from_secs(1))
            .await
            .map(|_| ())
    } else {
        client
            .observe_device_bounded(TARGET, Duration::from_secs(1))
            .await
            .map(|_| ())
    };
    let error = result
        .err()
        .ok_or_else(|| "server did not close the operation stream".to_owned())?;
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(error)
}

#[tokio::test]
async fn typed_client_reports_operation_receive_failures() -> Result<(), String> {
    assert!(matches!(
        run_single_operation_with_disconnect("observe-io", false).await?,
        vitals_ble_broker::BrokerClientError::Io(_)
    ));
    assert!(matches!(
        run_single_operation_with_disconnect("resolve-io", true).await?,
        vitals_ble_broker::BrokerClientError::Io(_)
    ));
    Ok(())
}

#[tokio::test]
async fn typed_client_skips_unrelated_events_and_rejects_release_response() -> Result<(), String> {
    let path = socket_path("event-and-release");
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
        send_server(
            &mut stream,
            ServerMessage::OperationAccepted { operation_id: 1 },
        )
        .await;
        send_server(
            &mut stream,
            ServerMessage::OperationAccepted { operation_id: 99 },
        )
        .await;
        send_server(
            &mut stream,
            ServerMessage::Frame {
                lease_id: LeaseId::new(1),
                operation_id: 1,
                indication: false,
                payload: vec![1],
            },
        )
        .await;
        let _ = read_frame(&mut stream).await;
        send_server(
            &mut stream,
            ServerMessage::OperationAccepted { operation_id: 99 },
        )
        .await;
        Ok::<(), String>(())
    });
    let mut client = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await
    .map_err(|error| error.to_string())?;
    acquire_test(&mut client)
        .await
        .map_err(|error| error.to_string())?;
    client
        .connect_device(TARGET)
        .await
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        client
            .next_event()
            .await
            .map_err(|error| error.to_string())?,
        Some(vitals_ble_broker::BrokerEvent::Frame { payload, .. }) if payload == vec![1]
    ));
    assert!(matches!(
        client.release("unexpected").await,
        Err(vitals_ble_broker::BrokerClientError::UnexpectedResponse(
            "Released"
        ))
    ));
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[tokio::test]
async fn typed_client_reports_cancel_send_failure() -> Result<(), String> {
    let path = socket_path("cancel-send-failure");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let (mut reader, writer) = stream.into_split();
        let _ = read_frame(&mut reader).await;
        send_server_split(
            &writer,
            &ServerMessage::HelloAccepted {
                generation: Generation::new(1),
            },
        )?;
        let _ = read_frame(&mut reader).await;
        send_server_split(
            &writer,
            &ServerMessage::LeaseGranted {
                lease_id: LeaseId::new(1),
                generation: Generation::new(1),
                expires_at: 10,
            },
        )?;
        let _ = read_frame(&mut reader).await;
        drop(reader);
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok::<(), String>(())
    });
    let mut client = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await
    .map_err(|error| error.to_string())?;
    acquire_test(&mut client)
        .await
        .map_err(|error| error.to_string())?;
    let result = client
        .connect_device_bounded(TARGET, Duration::from_millis(20))
        .await;
    assert!(matches!(
        result,
        Err(vitals_ble_broker::BrokerClientError::CancellationFailed { .. })
    ));
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}

fn send_server_split(
    writer: &tokio::net::unix::OwnedWriteHalf,
    message: &ServerMessage,
) -> Result<(), String> {
    let frame = encode_server(message).map_err(|error| error.to_string())?;
    writer
        .try_write(&frame)
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tokio::test]
async fn typed_client_reports_resolve_unexpected_and_timeout() -> Result<(), String> {
    let path = socket_path("resolve-variants");
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
        let _ = read_frame(&mut stream).await;
        send_server(
            &mut stream,
            ServerMessage::OperationAccepted { operation_id: 1 },
        )
        .await;
        let _ = read_frame(&mut stream).await;
        let _ = read_frame(&mut stream).await;
        send_server(&mut stream, ServerMessage::Cancelled { operation_id: 2 }).await;
        Ok::<(), String>(())
    });
    let mut client = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await
    .map_err(|error| error.to_string())?;
    acquire_test(&mut client)
        .await
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        client
            .resolve_characteristic_bounded("service", "characteristic", Duration::from_secs(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::UnexpectedResponse(
            "Resolved"
        ))
    ));
    assert!(matches!(
        client
            .resolve_characteristic_bounded("service", "characteristic", Duration::from_millis(1))
            .await,
        Err(vitals_ble_broker::BrokerClientError::DeadlineExceeded {
            operation: "resolve"
        })
    ));
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[tokio::test]
async fn typed_client_rejects_an_oversized_server_frame() -> Result<(), String> {
    let path = socket_path("oversized");
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).map_err(|error| error.to_string())?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let _ = read_frame(&mut stream).await;
        let mut header = [0_u8; 11];
        let length =
            u32::try_from(vitals_ble_broker::MAX_FRAME_BYTES).map_err(|error| error.to_string())?;
        header[7..11].copy_from_slice(&length.to_be_bytes());
        stream
            .write_all(&header)
            .await
            .map_err(|error| error.to_string())?;
        Ok::<(), String>(())
    });
    let result = BrokerClient::connect_source(
        Source::AirSense,
        &path,
        SELECTOR,
        TARGET,
        CapabilitySet::empty(),
    )
    .await;
    assert!(matches!(
        result,
        Err(vitals_ble_broker::BrokerClientError::Protocol(
            vitals_ble_broker::ProtocolError::FrameTooLarge { .. }
        ))
    ));
    server.await.map_err(|error| error.to_string())??;
    let _ = std::fs::remove_file(path);
    Ok(())
}
