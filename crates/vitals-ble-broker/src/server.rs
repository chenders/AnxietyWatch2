//! Linux broker daemon transport and typed operation dispatch.
//!
//! The server is the only process-facing owner of the Linux adapter owner. It
//! authenticates workers by the private socket and typed hello, validates every
//! lease against the negotiated capability set, and never forwards a raw
//! `BlueZ` object or D-Bus method to a worker.

#![cfg(all(feature = "linux", target_os = "linux"))]

use std::{
    collections::HashMap,
    fs, io,
    os::unix::fs::{FileTypeExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    str::FromStr as _,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::bluez::{ControllerBus, ControllerEvidence, ControllerSelector, LinuxBluezAdapterOwner};
use crate::{
    Acquire, Capability, CapabilitySet, ClientMessage, DeviceId, Generation, Intent, Lease,
    LeaseId, LeaseRequest, Operation, OperationKind, ProtocolError, Release, ServerMessage, Source,
    encode_server,
};
use futures::StreamExt as _;
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _},
    net::{UnixListener, UnixStream},
    sync::{Mutex, mpsc},
};

const HEADER_BYTES: usize = 11;
const SOCKET_QUEUE: usize = 32;
const SOCKET_MODE: u32 = 0o660;
const DISCOVERY_DURATION: Duration = Duration::from_secs(10);
const OPERATION_ERROR: u8 = 2;
const AUTH_ERROR: u8 = 3;
const OWNER_ERROR: u8 = 4;
const QUEUED_ERROR: u8 = 5;

/// Configuration for one broker process.
#[derive(Clone, Debug)]
pub struct BrokerConfig {
    /// Private worker socket path.
    pub socket: PathBuf,
    /// Stable selector text expected from every worker.
    pub selector_text: String,
    /// Stable selector resolved against the local controller inventory.
    pub selector: ControllerSelector,
    /// Optional peer uid allowed to use the socket.
    pub allowed_uid: Option<u32>,
}

impl BrokerConfig {
    /// Read configuration without selecting a runtime `hciN` alias.
    ///
    /// # Errors
    ///
    /// Returns a named configuration error when the stable selector is absent
    /// or malformed.
    pub fn from_env() -> Result<Self, String> {
        let selector_text = std::env::var("VITALS_BLE_ADAPTER_SELECTOR")
            .map_err(|_| "VITALS_BLE_ADAPTER_SELECTOR is unset".to_owned())?;
        let selector = parse_selector(&selector_text)?;
        let allowed_uid = std::env::var("VITALS_BLE_ALLOWED_UID")
            .ok()
            .map(|value| {
                value
                    .parse::<u32>()
                    .map_err(|_| format!("VITALS_BLE_ALLOWED_UID is not a uid: {value}"))
            })
            .transpose()?;
        Ok(Self {
            socket: std::env::var_os("VITALS_BLE_BROKER_SOCKET")
                .map_or_else(|| "/run/vitals/ble-broker.sock".into(), PathBuf::from),
            selector_text,
            selector,
            allowed_uid,
        })
    }
}

/// Run the broker until the listener or an accepted client fails fatally.
///
/// # Errors
///
/// Returns a named setup or listener error. Individual clients are isolated;
/// a malformed or disconnected worker does not stop the broker.
pub async fn run(config: BrokerConfig) -> Result<(), String> {
    let inventory = controller_inventory(Path::new("/sys/class/bluetooth"))
        .map_err(|error| format!("controller inventory failed: {error}"))?;
    let generation = Generation::new(1);
    let owner = LinuxBluezAdapterOwner::open(&config.selector, &inventory, generation)
        .await
        .map_err(|error| format!("broker adapter open failed: {error}"))?;
    prepare_socket(&config.socket).map_err(|error| {
        format!(
            "broker socket setup failed for {}: {error}",
            config.socket.display()
        )
    })?;
    let listener = UnixListener::bind(&config.socket)
        .map_err(|error| format!("broker socket bind failed: {error}"))?;
    fs::set_permissions(&config.socket, fs::Permissions::from_mode(SOCKET_MODE))
        .map_err(|error| format!("broker socket permissions failed: {error}"))?;

    let state = Arc::new(BrokerState {
        owner: Mutex::new(owner),
        selector_text: config.selector_text,
        allowed_uid: config.allowed_uid,
        started: Instant::now(),
        clients: Mutex::new(HashMap::new()),
        leases: Mutex::new(HashMap::new()),
        handles: Mutex::new(HashMap::new()),
        next_client: Mutex::new(1),
    });

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("broker socket accept failed: {error}"))?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(error) = serve_client(stream, state).await {
                eprintln!("vitals-ble-broker: client ended: {error}");
            }
        });
    }
}

struct BrokerState {
    owner: Mutex<LinuxBluezAdapterOwner>,
    selector_text: String,
    allowed_uid: Option<u32>,
    started: Instant,
    clients: Mutex<HashMap<u64, ClientInfo>>,
    leases: Mutex<HashMap<LeaseId, LeaseInfo>>,
    handles: Mutex<HashMap<String, bluer::gatt::remote::Characteristic>>,
    next_client: Mutex<u64>,
}

struct ClientInfo {
    source: Source,
    capabilities: CapabilitySet,
    leases: Vec<LeaseId>,
}

#[derive(Clone)]
struct LeaseInfo {
    lease: Lease,
    client_id: u64,
    capabilities: CapabilitySet,
    address: Option<bluer::Address>,
}

async fn serve_client(stream: UnixStream, state: Arc<BrokerState>) -> Result<(), String> {
    if let Some(expected_uid) = state.allowed_uid {
        let credentials = stream
            .peer_cred()
            .map_err(|error| format!("peer credential lookup failed: {error}"))?;
        if credentials.uid() != expected_uid {
            return Err(format!(
                "worker authentication rejected: uid={} expected={expected_uid}",
                credentials.uid()
            ));
        }
    }

    let (mut reader, mut writer) = stream.into_split();
    let (sender, mut responses) = mpsc::channel::<ServerMessage>(SOCKET_QUEUE);
    let writer_task = tokio::spawn(async move {
        while let Some(message) = responses.recv().await {
            let frame = encode_server(&message).map_err(|error| error.to_string())?;
            writer
                .write_all(&frame)
                .await
                .map_err(|error| error.to_string())?;
        }
        Ok::<(), String>(())
    });

    let hello = read_message(&mut reader).await?;
    let ClientMessage::Hello(hello) = hello else {
        let _ = sender
            .send(error_message(0, AUTH_ERROR, "worker handshake required"))
            .await;
        return Err("worker handshake required".to_owned());
    };
    validate_hello(&state, &hello)?;
    let client_id = {
        let mut next = state.next_client.lock().await;
        let id = *next;
        *next = next.saturating_add(1);
        id
    };
    let generation = state.owner.lock().await.generation();
    let source = hello.source;
    let capabilities = hello.capabilities;
    state.clients.lock().await.insert(
        client_id,
        ClientInfo {
            source,
            capabilities,
            leases: Vec::new(),
        },
    );
    send_response(&sender, ServerMessage::HelloAccepted { generation }).await?;

    let result = loop {
        let message = match read_message(&mut reader).await {
            Ok(message) => message,
            Err(error) if error == "broker socket closed" => break Ok(()),
            Err(error) => break Err(error),
        };
        if let Err(error) = process_message(client_id, message, &state, &sender).await {
            break Err(error);
        }
    };

    cleanup_client(client_id, &state).await;
    drop(sender);
    if let Ok(Err(error)) = writer_task.await {
        return Err(format!("broker response writer failed: {error}"));
    }
    result
}

fn validate_hello(state: &BrokerState, hello: &crate::Hello) -> Result<(), String> {
    if hello.adapter_selector != state.selector_text {
        return Err(format!(
            "worker authentication rejected: adapter selector mismatch for source={:?}",
            hello.source
        ));
    }
    if hello.device_selector.as_deref().is_none_or(str::is_empty) {
        return Err(format!(
            "worker authentication rejected: empty device selector for source={:?}",
            hello.source
        ));
    }
    if hello.source == Source::Bp
        && (hello.capabilities.contains(Capability::Connect)
            || hello.capabilities.contains(Capability::Discover)
            || hello.capabilities.contains(Capability::Write))
    {
        return Err(
            "BP worker authentication rejected: requested capability includes userspace authority"
                .to_owned(),
        );
    }
    Ok(())
}

async fn process_message(
    client_id: u64,
    message: ClientMessage,
    state: &Arc<BrokerState>,
    sender: &mpsc::Sender<ServerMessage>,
) -> Result<(), String> {
    match message {
        ClientMessage::Hello(_) => {
            send_response(
                sender,
                error_message(0, AUTH_ERROR, "worker handshake already completed"),
            )
            .await
        }
        ClientMessage::Health => {
            let health = state
                .owner
                .lock()
                .await
                .health()
                .await
                .map_err(|error| error.to_string())?;
            send_response(
                sender,
                ServerMessage::Health {
                    generation: health.generation,
                    queue_depth: 0,
                },
            )
            .await
        }
        ClientMessage::Acquire(acquire) => acquire_lease(client_id, acquire, state, sender).await,
        ClientMessage::Operation(operation) => {
            process_operation(client_id, operation, state, sender).await
        }
        ClientMessage::Release(release) => release_lease(client_id, release, state, sender).await,
        ClientMessage::Cancel(cancel) => {
            // BlueZ methods in the owner have their own bounded deadlines. A
            // cancellation arriving at this protocol boundary is therefore a
            // safe no-op for an already completed operation, never a claim that
            // a device operation was rolled back.
            send_response(
                sender,
                ServerMessage::Cancelled {
                    operation_id: cancel.operation_id,
                },
            )
            .await
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn acquire_lease(
    client_id: u64,
    acquire: Acquire,
    state: &Arc<BrokerState>,
    sender: &mpsc::Sender<ServerMessage>,
) -> Result<(), String> {
    let client = state
        .clients
        .lock()
        .await
        .get(&client_id)
        .map(|client| (client.source, client.capabilities))
        .ok_or_else(|| "lease blocked: worker is not authenticated".to_owned())?;
    if acquire.source != client.0 {
        return send_response(
            sender,
            error_message(0, AUTH_ERROR, "lease source does not match handshake"),
        )
        .await;
    }
    if acquire.capabilities.bits() & !client.1.bits() != 0 {
        return send_response(
            sender,
            error_message(
                0,
                AUTH_ERROR,
                "lease requests capability not granted at handshake",
            ),
        )
        .await;
    }
    if acquire.source == Source::Bp
        && (acquire.capabilities.contains(Capability::Connect)
            || acquire.intent == Intent::Connect
            || acquire.capabilities.contains(Capability::Discover)
            || acquire.capabilities.contains(Capability::Write))
    {
        return send_response(
            sender,
            error_message(
                0,
                AUTH_ERROR,
                "BP lease has no userspace connect capability",
            ),
        )
        .await;
    }
    let device = acquire.device_selector.clone().map(DeviceId::new);
    let request = LeaseRequest::new(
        acquire.source,
        acquire.intent,
        acquire.priority,
        acquire.deadline,
        acquire.max_hold,
        device,
        acquire.capabilities,
    )
    .with_reason(acquire.reason);
    let now = elapsed_tick(state.started);
    let result = state.owner.lock().await.acquire(request, now);
    match result {
        Ok(crate::AcquireResult::Granted(lease)) => {
            let address = address_for_lease(&lease);
            let id = lease.id();
            if let Some(client) = state.clients.lock().await.get_mut(&client_id) {
                client.leases.push(id);
            }
            send_response(
                sender,
                ServerMessage::LeaseGranted {
                    lease_id: id,
                    generation: lease.generation(),
                    expires_at: lease.expires_at(),
                },
            )
            .await?;
            insert_lease(
                state,
                LeaseInfo {
                    lease,
                    client_id,
                    capabilities: acquire.capabilities,
                    address,
                },
            )
            .await;
            Ok(())
        }
        Ok(crate::AcquireResult::Queued { ticket, blocker }) => {
            // Source clients currently use bounded retries rather than holding
            // an unbounded socket wait. Cancel the scheduler entry immediately
            // so a dead worker can never strand a lease request.
            state.owner.lock().await.cancel(ticket);
            send_response(sender, error_message(0, QUEUED_ERROR, &blocker.to_string())).await
        }
        Err(error) => {
            send_response(
                sender,
                error_message(0, OPERATION_ERROR, &error.to_string()),
            )
            .await
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn process_operation(
    client_id: u64,
    operation: Operation,
    state: &Arc<BrokerState>,
    sender: &mpsc::Sender<ServerMessage>,
) -> Result<(), String> {
    let info = match lease_info(client_id, operation.lease_id, state).await {
        Ok(info) => info,
        Err(error) => {
            return send_response(
                sender,
                error_message(operation.operation_id, AUTH_ERROR, &error),
            )
            .await;
        }
    };
    let needed = capability_for(operation.kind);
    if !info.capabilities.contains(needed) {
        return send_response(
            sender,
            error_message(
                operation.operation_id,
                AUTH_ERROR,
                "operation blocked: lease lacks the requested typed capability",
            ),
        )
        .await;
    }
    if info.lease.source() == Source::Bp && operation.kind == OperationKind::Connect {
        return send_response(
            sender,
            error_message(
                operation.operation_id,
                AUTH_ERROR,
                "operation blocked: BP has no userspace Connect capability",
            ),
        )
        .await;
    }

    match operation.kind {
        OperationKind::ObserveDevice => {
            let address = operation_address(&operation.target, info.address)?;
            let observation = state
                .owner
                .lock()
                .await
                .observe(&info.lease, address)
                .await
                .map_err(|error| error.to_string());
            match observation {
                Ok(observation) => {
                    send_response(
                        sender,
                        ServerMessage::DeviceState {
                            operation_id: operation.operation_id,
                            connected: observation.connected,
                            services_resolved: observation.services_resolved,
                        },
                    )
                    .await
                }
                Err(error) => {
                    send_response(
                        sender,
                        error_message(operation.operation_id, OWNER_ERROR, &error),
                    )
                    .await
                }
            }
        }
        OperationKind::DiscoverTarget => {
            let target = bluer::Address::from_str(&operation.target)
                .map_err(|_| "discovery blocked: target address is invalid".to_owned())?;
            let result = state
                .owner
                .lock()
                .await
                .discover(
                    &info.lease,
                    bluer::DiscoveryFilter::default(),
                    Some(target),
                    DISCOVERY_DURATION,
                )
                .await;
            match result {
                Ok(_) => {
                    send_response(
                        sender,
                        ServerMessage::OperationAccepted {
                            operation_id: operation.operation_id,
                        },
                    )
                    .await
                }
                Err(error) => {
                    send_response(
                        sender,
                        error_message(operation.operation_id, OWNER_ERROR, &error.to_string()),
                    )
                    .await
                }
            }
        }
        OperationKind::Connect => {
            let address = operation_address(&operation.target, info.address)?;
            let result = state
                .owner
                .lock()
                .await
                .connect(&info.lease, address, Duration::from_secs(30))
                .await;
            operation_result(sender, operation.operation_id, result).await
        }
        OperationKind::Resolve => {
            let (service, characteristic) = operation.target.split_once('|').ok_or_else(|| {
                "resolve blocked: target must be service|characteristic".to_owned()
            })?;
            let service = bluer::Uuid::from_str(service)
                .map_err(|_| "resolve blocked: service UUID is invalid".to_owned())?;
            let characteristic = bluer::Uuid::from_str(characteristic)
                .map_err(|_| "resolve blocked: characteristic UUID is invalid".to_owned())?;
            let address = info
                .address
                .ok_or_else(|| "resolve blocked: lease has no device selector".to_owned())?;
            let resolved = state
                .owner
                .lock()
                .await
                .resolve(&info.lease, address, service, characteristic)
                .await;
            match resolved {
                Ok(characteristic) => {
                    let handle = format!("c{}-o{}", client_id, operation.operation_id);
                    state
                        .handles
                        .lock()
                        .await
                        .insert(handle.clone(), characteristic);
                    send_response(
                        sender,
                        ServerMessage::Resolved {
                            operation_id: operation.operation_id,
                            handle,
                        },
                    )
                    .await
                }
                Err(error) => {
                    send_response(
                        sender,
                        error_message(operation.operation_id, OWNER_ERROR, &error.to_string()),
                    )
                    .await
                }
            }
        }
        OperationKind::Subscribe => {
            let characteristic = state
                .handles
                .lock()
                .await
                .get(&operation.target)
                .cloned()
                .ok_or_else(|| "subscribe blocked: characteristic handle is unknown".to_owned())?;
            let stream = state
                .owner
                .lock()
                .await
                .subscribe(&info.lease, &characteristic)
                .await;
            match stream {
                Ok(stream) => {
                    send_response(
                        sender,
                        ServerMessage::OperationAccepted {
                            operation_id: operation.operation_id,
                        },
                    )
                    .await?;
                    let sender = sender.clone();
                    let lease_id = operation.lease_id;
                    let operation_id = operation.operation_id;
                    tokio::spawn(async move {
                        futures::pin_mut!(stream);
                        while let Some(payload) = stream.next().await {
                            if sender
                                .send(ServerMessage::Frame {
                                    lease_id,
                                    operation_id,
                                    indication: true,
                                    payload,
                                })
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                        let _ = sender
                            .send(ServerMessage::Disconnected {
                                operation_id,
                                reason: "BlueZ notification stream ended".to_owned(),
                            })
                            .await;
                    });
                    Ok(())
                }
                Err(error) => {
                    send_response(
                        sender,
                        error_message(operation.operation_id, OWNER_ERROR, &error.to_string()),
                    )
                    .await
                }
            }
        }
        OperationKind::Write => {
            let characteristic = state
                .handles
                .lock()
                .await
                .get(&operation.target)
                .cloned()
                .ok_or_else(|| "write blocked: characteristic handle is unknown".to_owned())?;
            let address = info
                .address
                .ok_or_else(|| "write blocked: lease has no device selector".to_owned())?;
            let result = state
                .owner
                .lock()
                .await
                .write(&info.lease, address, &characteristic, &operation.payload)
                .await;
            operation_result(sender, operation.operation_id, result).await
        }
    }
}

async fn release_lease(
    client_id: u64,
    release: Release,
    state: &Arc<BrokerState>,
    sender: &mpsc::Sender<ServerMessage>,
) -> Result<(), String> {
    let info = match lease_info(client_id, release.lease_id, state).await {
        Ok(info) => info,
        Err(error) => return send_response(sender, error_message(0, AUTH_ERROR, &error)).await,
    };
    if info.lease.source() != Source::Bp
        && info.lease.intent() == Intent::Connect
        && let Some(address) = info.address
    {
        let _ = state
            .owner
            .lock()
            .await
            .disconnect(&info.lease, address)
            .await;
    }
    let outcome = state
        .owner
        .lock()
        .await
        .release(release.lease_id, elapsed_tick(state.started));
    remove_lease(client_id, release.lease_id, state).await;
    send_response(
        sender,
        ServerMessage::Released {
            lease_id: release.lease_id,
        },
    )
    .await?;
    if !outcome.granted.is_empty() {
        // Queued leases are cancelled at admission in this first daemon
        // revision, so this is only a defensive health signal.
        eprintln!("vitals-ble-broker: scheduler dispatched an unowned lease");
    }
    Ok(())
}

async fn cleanup_client(client_id: u64, state: &Arc<BrokerState>) {
    let leases = state
        .clients
        .lock()
        .await
        .remove(&client_id)
        .map_or_else(Vec::new, |client| client.leases);
    for lease_id in leases {
        if let Ok(info) = lease_info(client_id, lease_id, state).await {
            if info.lease.source() != Source::Bp
                && info.lease.intent() == Intent::Connect
                && let Some(address) = info.address
            {
                let _ = state
                    .owner
                    .lock()
                    .await
                    .disconnect(&info.lease, address)
                    .await;
            }
            let _ = state
                .owner
                .lock()
                .await
                .release(lease_id, elapsed_tick(state.started));
            remove_lease(client_id, lease_id, state).await;
        }
    }
    state
        .handles
        .lock()
        .await
        .retain(|handle, _| !handle.starts_with(&format!("c{client_id}-")));
}

async fn lease_info(
    client_id: u64,
    lease_id: LeaseId,
    state: &Arc<BrokerState>,
) -> Result<LeaseInfo, String> {
    state
        .leases
        .lock()
        .await
        .get(&lease_id)
        .filter(|info| info.client_id == client_id)
        .cloned()
        .ok_or_else(|| {
            format!(
                "lease blocked: lease={} is not owned by this worker",
                lease_id.value()
            )
        })
}

async fn insert_lease(state: &Arc<BrokerState>, info: LeaseInfo) {
    state.leases.lock().await.insert(info.lease.id(), info);
}

async fn remove_lease(client_id: u64, lease_id: LeaseId, state: &Arc<BrokerState>) {
    state.leases.lock().await.remove(&lease_id);
    if let Some(client) = state.clients.lock().await.get_mut(&client_id) {
        client.leases.retain(|id| *id != lease_id);
    }
}

async fn operation_result<E: std::fmt::Display>(
    sender: &mpsc::Sender<ServerMessage>,
    operation_id: u64,
    result: Result<(), E>,
) -> Result<(), String> {
    match result {
        Ok(()) => send_response(sender, ServerMessage::OperationAccepted { operation_id }).await,
        Err(error) => {
            send_response(
                sender,
                error_message(operation_id, OWNER_ERROR, &error.to_string()),
            )
            .await
        }
    }
}

fn capability_for(kind: OperationKind) -> Capability {
    match kind {
        OperationKind::ObserveDevice => Capability::KernelObserve,
        OperationKind::DiscoverTarget => Capability::Discover,
        OperationKind::Connect => Capability::Connect,
        OperationKind::Resolve => Capability::Resolve,
        OperationKind::Subscribe => Capability::Subscribe,
        OperationKind::Write => Capability::Write,
    }
}

fn operation_address(
    target: &str,
    lease_address: Option<bluer::Address>,
) -> Result<bluer::Address, String> {
    let address = bluer::Address::from_str(target)
        .map_err(|_| "operation blocked: target address is invalid".to_owned())?;
    if lease_address.is_some_and(|expected| expected != address) {
        return Err("operation blocked: target differs from lease device selector".to_owned());
    }
    Ok(address)
}

fn address_for_lease(lease: &Lease) -> Option<bluer::Address> {
    lease
        .device()
        .and_then(|device| bluer::Address::from_str(device.as_str()).ok())
}

fn elapsed_tick(started: Instant) -> u64 {
    started.elapsed().as_secs()
}

fn error_message(operation_id: u64, code: u8, detail: &str) -> ServerMessage {
    ServerMessage::Error {
        operation_id,
        code,
        detail: detail.to_owned(),
    }
}

async fn send_response(
    sender: &mpsc::Sender<ServerMessage>,
    message: ServerMessage,
) -> Result<(), String> {
    sender
        .send(message)
        .await
        .map_err(|_| "broker response blocked: worker response queue is closed".to_owned())
}

async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> Result<ClientMessage, String> {
    let mut header = [0_u8; HEADER_BYTES];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
            return Err("broker socket closed".to_owned());
        }
        Err(error) => return Err(format!("broker frame header read failed: {error}")),
    }
    if &header[..4] != b"VBT1" {
        return Err(ProtocolError::BadMagic.to_string());
    }
    let payload_length = usize::try_from(u32::from_be_bytes([
        header[7], header[8], header[9], header[10],
    ]))
    .map_err(|_| ProtocolError::InvalidPayload.to_string())?;
    let frame_length = HEADER_BYTES
        .checked_add(payload_length)
        .ok_or(ProtocolError::FrameTooLarge {
            length: usize::MAX,
            max: crate::MAX_FRAME_BYTES,
        })
        .map_err(|error| error.to_string())?;
    if frame_length > crate::MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: frame_length,
            max: crate::MAX_FRAME_BYTES,
        }
        .to_string());
    }
    let mut frame = Vec::with_capacity(frame_length);
    frame.extend_from_slice(&header);
    let mut payload = vec![0_u8; payload_length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|error| format!("broker frame payload read failed: {error}"))?;
    frame.extend_from_slice(&payload);
    crate::decode(&frame).map_err(|error| error.to_string())
}

fn prepare_socket(path: &Path) -> io::Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.file_type().is_socket() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "refusing to replace a non-socket path",
            ));
        }
        fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn parse_selector(value: &str) -> Result<ControllerSelector, String> {
    if value.is_empty() {
        return Err("adapter selector is empty".to_owned());
    }
    let mut selector = ControllerSelector::default();
    if !value.contains('=') {
        bluer::Address::from_str(value).map_err(|_| {
            "adapter selector must be address=... or a Bluetooth address".to_owned()
        })?;
        selector.address = Some(value.to_owned());
        return Ok(selector);
    }
    for field in value.split(';') {
        let (key, field_value) = field
            .split_once('=')
            .ok_or_else(|| "adapter selector field lacks '='".to_owned())?;
        if field_value.is_empty() {
            return Err(format!("adapter selector field is empty: {key}"));
        }
        match key {
            "address" => {
                bluer::Address::from_str(field_value)
                    .map_err(|_| "adapter selector address is invalid".to_owned())?;
                selector.address = Some(field_value.to_owned());
            }
            "sysfs_path" => selector.sysfs_path = Some(field_value.to_owned()),
            "serial" => selector.serial = Some(field_value.to_owned()),
            "bus" => {
                selector.bus = Some(match field_value {
                    "usb" => ControllerBus::Usb,
                    "uart" => ControllerBus::Uart,
                    "other" => ControllerBus::Other,
                    _ => return Err(format!("adapter selector bus is unknown: {field_value}")),
                });
            }
            "vid_pid" => {
                let (vid, pid) = field_value
                    .split_once(':')
                    .ok_or_else(|| "adapter selector vid_pid must be VID:PID".to_owned())?;
                let vid = u16::from_str_radix(vid, 16)
                    .map_err(|_| "adapter selector VID is invalid".to_owned())?;
                let pid = u16::from_str_radix(pid, 16)
                    .map_err(|_| "adapter selector PID is invalid".to_owned())?;
                selector.vid_pid = Some((vid, pid));
            }
            other => return Err(format!("adapter selector field is unknown: {other}")),
        }
    }
    if selector.has_stable_identity() {
        Ok(selector)
    } else {
        Err("adapter selector has no stable identity field".to_owned())
    }
}

fn controller_inventory(root: &Path) -> io::Result<Vec<ControllerEvidence>> {
    let mut records = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("hci") {
            continue;
        }
        let class_path = entry.path();
        let device_path = fs::canonicalize(class_path.join("device"))?;
        let address = fs::read_to_string(class_path.join("address"))
            .ok()
            .map(|value| value.trim().to_owned());
        let (serial, vid_pid) = usb_identity(&device_path);
        let bus = if device_path.to_string_lossy().contains("/usb") {
            ControllerBus::Usb
        } else if device_path.to_string_lossy().contains("serial")
            || device_path.to_string_lossy().contains("uart")
        {
            ControllerBus::Uart
        } else {
            ControllerBus::Other
        };
        records.push(ControllerEvidence {
            runtime_name: name,
            address,
            sysfs_path: Some(device_path.to_string_lossy().into_owned()),
            serial,
            bus,
            vid_pid,
        });
    }
    Ok(records)
}

fn usb_identity(path: &Path) -> (Option<String>, Option<(u16, u16)>) {
    for ancestor in path.ancestors() {
        let serial = fs::read_to_string(ancestor.join("serial")).ok();
        let vendor = fs::read_to_string(ancestor.join("idVendor")).ok();
        let product = fs::read_to_string(ancestor.join("idProduct")).ok();
        if serial.is_some() || vendor.is_some() || product.is_some() {
            let vid_pid = vendor
                .and_then(|vendor| u16::from_str_radix(vendor.trim(), 16).ok())
                .zip(product.and_then(|product| u16::from_str_radix(product.trim(), 16).ok()));
            return (serial.map(|value| value.trim().to_owned()), vid_pid);
        }
    }
    (None, None)
}
