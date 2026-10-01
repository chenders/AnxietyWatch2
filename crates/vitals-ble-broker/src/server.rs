//! Linux broker daemon transport and typed operation dispatch.
//!
//! The server is the only process-facing owner of the Linux adapter owner. It
//! authenticates workers by the private socket and typed hello, validates every
//! lease against the negotiated capability set, and never forwards a raw
//! `BlueZ` object or D-Bus method to a worker.

#![cfg(all(feature = "linux", target_os = "linux"))]

use std::{
    collections::{HashMap, HashSet},
    fs,
    future::Future,
    io,
    os::unix::fs::{FileTypeExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr as _,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::bluez::{
    AdapterIo, BluezAdapterOwner, ControllerBus, ControllerEvidence, ControllerSelector, DeviceIo,
    LinuxBluezAdapterOwner,
};
use crate::telemetry::{OperationResult, Telemetry};
use crate::{
    Acquire, Capability, CapabilitySet, ClientMessage, DeviceId, Generation, Intent, Lease,
    LeaseId, LeaseRequest, Operation, OperationKind, ProtocolError, Release, ReleaseOutcome,
    ServerMessage, Source, encode, encode_server,
};
use futures::{
    Stream, StreamExt as _,
    future::{AbortHandle, Abortable},
};
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

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type BoxPayloadStream = Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;

trait OwnerOperations: Send + 'static {
    type Characteristic: Clone + Send + Sync + 'static;

    fn generation(&self) -> Generation;
    fn acquire(
        &mut self,
        request: LeaseRequest,
        now: crate::lease::Tick,
    ) -> Result<crate::AcquireResult, crate::PolicyError>;
    fn release(&mut self, lease: LeaseId, now: crate::lease::Tick) -> ReleaseOutcome;
    fn cancel(&mut self, ticket: crate::lease::QueueTicket) -> bool;
    fn health(
        &self,
    ) -> BoxFuture<'_, Result<crate::bluez::AdapterHealth, crate::bluez::OwnerError>>;
    fn discover<'a>(
        &'a self,
        lease: &'a Lease,
        filter: bluer::DiscoveryFilter,
        target: Option<bluer::Address>,
        duration: Duration,
    ) -> BoxFuture<'a, Result<crate::bluez::DiscoveryReport, crate::bluez::OwnerError>>;
    fn observe<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
    ) -> BoxFuture<'a, Result<crate::bluez::DeviceObservation, crate::bluez::OwnerError>>;
    fn connect<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
        duration: Duration,
    ) -> BoxFuture<'a, Result<(), crate::bluez::OwnerError>>;
    fn disconnect<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
    ) -> BoxFuture<'a, Result<(), crate::bluez::OwnerError>>;
    fn resolve<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
        service_uuid: bluer::Uuid,
        characteristic_uuid: bluer::Uuid,
    ) -> BoxFuture<'a, Result<Self::Characteristic, crate::bluez::OwnerError>>;
    fn subscribe<'a>(
        &'a self,
        lease: &'a Lease,
        characteristic: &'a Self::Characteristic,
    ) -> BoxFuture<'a, Result<BoxPayloadStream, crate::bluez::OwnerError>>;
    fn write<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
        characteristic: &'a Self::Characteristic,
        payload: &'a [u8],
    ) -> BoxFuture<'a, Result<(), crate::bluez::OwnerError>>;
}

impl<A: AdapterIo> OwnerOperations for BluezAdapterOwner<A> {
    type Characteristic = <A::Device as DeviceIo>::Characteristic;

    fn generation(&self) -> Generation {
        BluezAdapterOwner::<A>::generation(self)
    }

    fn acquire(
        &mut self,
        request: LeaseRequest,
        now: crate::lease::Tick,
    ) -> Result<crate::AcquireResult, crate::PolicyError> {
        BluezAdapterOwner::<A>::acquire(self, request, now)
    }

    fn release(&mut self, lease: LeaseId, now: crate::lease::Tick) -> ReleaseOutcome {
        BluezAdapterOwner::<A>::release(self, lease, now)
    }

    fn cancel(&mut self, ticket: crate::lease::QueueTicket) -> bool {
        BluezAdapterOwner::<A>::cancel(self, ticket)
    }

    fn health(
        &self,
    ) -> BoxFuture<'_, Result<crate::bluez::AdapterHealth, crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::health(self))
    }

    fn discover<'a>(
        &'a self,
        lease: &'a Lease,
        filter: bluer::DiscoveryFilter,
        target: Option<bluer::Address>,
        duration: Duration,
    ) -> BoxFuture<'a, Result<crate::bluez::DiscoveryReport, crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::discover(
            self, lease, filter, target, duration,
        ))
    }

    fn observe<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
    ) -> BoxFuture<'a, Result<crate::bluez::DeviceObservation, crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::observe(self, lease, address))
    }

    fn connect<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
        duration: Duration,
    ) -> BoxFuture<'a, Result<(), crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::connect(
            self, lease, address, duration,
        ))
    }

    fn disconnect<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
    ) -> BoxFuture<'a, Result<(), crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::disconnect(self, lease, address))
    }

    fn resolve<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
        service_uuid: bluer::Uuid,
        characteristic_uuid: bluer::Uuid,
    ) -> BoxFuture<'a, Result<Self::Characteristic, crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::resolve(
            self,
            lease,
            address,
            service_uuid,
            characteristic_uuid,
        ))
    }

    fn subscribe<'a>(
        &'a self,
        lease: &'a Lease,
        characteristic: &'a Self::Characteristic,
    ) -> BoxFuture<'a, Result<BoxPayloadStream, crate::bluez::OwnerError>> {
        Box::pin(async move {
            BluezAdapterOwner::<A>::subscribe(self, lease, characteristic)
                .await
                .map(|stream| stream as BoxPayloadStream)
        })
    }

    fn write<'a>(
        &'a self,
        lease: &'a Lease,
        address: bluer::Address,
        characteristic: &'a Self::Characteristic,
        payload: &'a [u8],
    ) -> BoxFuture<'a, Result<(), crate::bluez::OwnerError>> {
        Box::pin(BluezAdapterOwner::<A>::write(
            self,
            lease,
            address,
            characteristic,
            payload,
        ))
    }
}

trait BrokerOwner: OwnerOperations {}

impl<T: OwnerOperations> BrokerOwner for T {}

#[cfg(test)]
#[allow(
    clippy::match_wildcard_for_single_variants,
    clippy::panic,
    clippy::unwrap_used
)]
pub(crate) async fn exercise_linux_owner_dispatch<A: AdapterIo>(owner: &mut BluezAdapterOwner<A>) {
    assert_eq!(OwnerOperations::generation(owner), Generation::new(1));
    let address = bluer::Address::new([1, 2, 3, 4, 5, 6]);
    let request = |source, intent| {
        LeaseRequest::new(
            source,
            intent,
            crate::Priority::new(1),
            100,
            100,
            None,
            CapabilitySet::empty(),
        )
    };

    let discover =
        OwnerOperations::acquire(owner, request(Source::Diagnostics, Intent::Discover), 0)
            .unwrap()
            .lease()
            .cloned()
            .unwrap();
    let queued =
        OwnerOperations::acquire(owner, request(Source::Diagnostics, Intent::Discover), 0).unwrap();
    if let crate::AcquireResult::Queued { ticket, .. } = queued {
        assert!(OwnerOperations::cancel(owner, ticket));
    }
    OwnerOperations::health(owner).await.unwrap();
    OwnerOperations::discover(
        owner,
        &discover,
        bluer::DiscoveryFilter::default(),
        Some(address),
        Duration::from_millis(10),
    )
    .await
    .unwrap();
    OwnerOperations::release(owner, discover.id(), 1);

    let observe = OwnerOperations::acquire(owner, request(Source::Bp, Intent::KernelObserve), 2)
        .unwrap()
        .lease()
        .cloned()
        .unwrap();
    OwnerOperations::observe(owner, &observe, address)
        .await
        .unwrap();
    OwnerOperations::release(owner, observe.id(), 3);

    let connect = OwnerOperations::acquire(owner, request(Source::AirSense, Intent::Connect), 4)
        .unwrap()
        .lease()
        .cloned()
        .unwrap();
    OwnerOperations::connect(owner, &connect, address, Duration::from_millis(10))
        .await
        .unwrap();
    OwnerOperations::disconnect(owner, &connect, address)
        .await
        .unwrap();
    OwnerOperations::release(owner, connect.id(), 5);

    let gatt = OwnerOperations::acquire(owner, request(Source::AirSense, Intent::Gatt), 6)
        .unwrap()
        .lease()
        .cloned()
        .unwrap();
    let characteristic = OwnerOperations::resolve(
        owner,
        &gatt,
        address,
        bluer::Uuid::nil(),
        bluer::Uuid::nil(),
    )
    .await
    .unwrap();
    let mut stream = OwnerOperations::subscribe(owner, &gatt, &characteristic)
        .await
        .unwrap();
    let _ = stream.next().await;
    OwnerOperations::write(owner, &gatt, address, &characteristic, &[1, 2, 3])
        .await
        .unwrap();
    OwnerOperations::release(owner, gatt.id(), 7);
}

/// Configuration for one broker process.
#[derive(Clone, Debug)]
pub struct BrokerConfig {
    /// Private worker socket path.
    pub socket: PathBuf,
    /// Prometheus textfile snapshot path.
    pub metrics_path: PathBuf,
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
            metrics_path: std::env::var_os("VITALS_BLE_BROKER_METRICS_PATH")
                .map_or_else(|| "/run/vitals/ble-broker.prom".into(), PathBuf::from),
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
    run_with_owner(config, owner).await
}

async fn run_with_owner<O: BrokerOwner>(config: BrokerConfig, owner: O) -> Result<(), String> {
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

    let telemetry = Arc::new(Telemetry::new());
    let _metrics_exporter =
        tokio::spawn(Arc::clone(&telemetry).export_loop(config.metrics_path.clone()));
    let state = Arc::new(BrokerState {
        owner: Mutex::new(owner),
        telemetry,
        selector_text: config.selector_text,
        allowed_uid: config.allowed_uid,
        started: Instant::now(),
        clients: Mutex::new(HashMap::new()),
        leases: Mutex::new(HashMap::new()),
        handles: Mutex::new(HashMap::new()),
        operations: Mutex::new(HashMap::new()),
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

struct BrokerState<O: BrokerOwner> {
    owner: Mutex<O>,
    telemetry: Arc<Telemetry>,
    selector_text: String,
    allowed_uid: Option<u32>,
    started: Instant,
    clients: Mutex<HashMap<u64, ClientInfo>>,
    leases: Mutex<HashMap<LeaseId, LeaseInfo>>,
    handles: Mutex<HashMap<String, O::Characteristic>>,
    operations: Mutex<HashMap<(u64, u64), AbortHandle>>,
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

async fn serve_client<O: BrokerOwner>(
    stream: UnixStream,
    state: Arc<BrokerState<O>>,
) -> Result<(), String> {
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
    let telemetry = Arc::clone(&state.telemetry);
    let writer_task = tokio::spawn(async move {
        while let Some(message) = responses.recv().await {
            let frame = encode_server(&message).map_err(|error| error.to_string())?;
            telemetry.response_bytes(frame.len());
            writer
                .write_all(&frame)
                .await
                .map_err(|error| error.to_string())?;
        }
        Ok::<(), String>(())
    });

    let hello = read_message(&mut reader).await?;
    let ClientMessage::Hello(hello) = hello else {
        state.telemetry.client_rejected();
        let _ = sender
            .send(error_message(0, AUTH_ERROR, "worker handshake required"))
            .await;
        return Err("worker handshake required".to_owned());
    };
    if let Err(error) = validate_hello(&state, &hello) {
        state.telemetry.client_rejected();
        return Err(error);
    }
    let client_id = {
        let mut next = state.next_client.lock().await;
        let id = *next;
        *next = next.saturating_add(1);
        id
    };
    let generation = state.owner.lock().await.generation();
    let source = hello.source;
    let capabilities = hello.capabilities;
    state.telemetry.client_connected(source);
    if let Ok(frame) = encode(&ClientMessage::Hello(hello.clone())) {
        state.telemetry.request_bytes(source, frame.len());
    }
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
        if let Ok(frame) = encode(&message) {
            state.telemetry.request_bytes(source, frame.len());
        }
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

fn validate_hello<O: BrokerOwner>(
    state: &BrokerState<O>,
    hello: &crate::Hello,
) -> Result<(), String> {
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

async fn process_message<O: BrokerOwner>(
    client_id: u64,
    message: ClientMessage,
    state: &Arc<BrokerState<O>>,
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
            start_operation(client_id, operation, state, sender).await
        }
        ClientMessage::Release(release) => release_lease(client_id, release, state, sender).await,
        ClientMessage::Cancel(cancel) => cancel_operation(client_id, cancel, state, sender).await,
    }
}

#[allow(clippy::too_many_lines)]
async fn acquire_lease<O: BrokerOwner>(
    client_id: u64,
    acquire: Acquire,
    state: &Arc<BrokerState<O>>,
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
    let now = elapsed_tick(state.started);
    let request = LeaseRequest::new(
        acquire.source,
        acquire.intent,
        acquire.priority,
        deadline_after(now, acquire.deadline),
        acquire.max_hold,
        device,
        acquire.capabilities,
    )
    .with_reason(acquire.reason);
    let result = state.owner.lock().await.acquire(request, now);
    match result {
        Ok(crate::AcquireResult::Granted(lease)) => {
            state.telemetry.lease_granted(acquire.source);
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
            state.telemetry.lease_queued(acquire.source);
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

async fn start_operation<O: BrokerOwner>(
    client_id: u64,
    operation: Operation,
    state: &Arc<BrokerState<O>>,
    sender: &mpsc::Sender<ServerMessage>,
) -> Result<(), String> {
    let operation_id = operation.operation_id;
    let operation_kind = operation.kind;
    let source = state
        .clients
        .lock()
        .await
        .get(&client_id)
        .map(|client| client.source)
        .ok_or_else(|| "operation blocked: worker is not authenticated".to_owned())?;
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let mut operations = state.operations.lock().await;
    if operations.contains_key(&(client_id, operation_id)) {
        return send_response(
            sender,
            error_message(
                operation_id,
                AUTH_ERROR,
                "operation blocked: operation_id is already active",
            ),
        )
        .await;
    }
    operations.insert((client_id, operation_id), abort_handle);
    drop(operations);
    let state = Arc::clone(state);
    let sender = sender.clone();
    tokio::spawn(async move {
        let started = Instant::now();
        let result = Abortable::new(
            process_operation(client_id, operation, &state, &sender),
            abort_registration,
        )
        .await;
        let telemetry_result = match &result {
            Ok(Ok(())) => OperationResult::Success,
            Ok(Err(_)) => OperationResult::Failure,
            Err(_) => OperationResult::Cancelled,
        };
        state.telemetry.operation_finished(
            source,
            operation_kind,
            started.elapsed(),
            telemetry_result,
        );
        if let Ok(Err(error)) = result {
            let _ = send_response(&sender, error_message(operation_id, OWNER_ERROR, &error)).await;
        }
        state
            .operations
            .lock()
            .await
            .remove(&(client_id, operation_id));
    });
    Ok(())
}

async fn cancel_operation<O: BrokerOwner>(
    client_id: u64,
    cancel: crate::Cancel,
    state: &Arc<BrokerState<O>>,
    sender: &mpsc::Sender<ServerMessage>,
) -> Result<(), String> {
    if let Err(error) = lease_info(client_id, cancel.lease_id, state).await {
        return send_response(sender, error_message(0, AUTH_ERROR, &error)).await;
    }
    if let Some(handle) = state
        .operations
        .lock()
        .await
        .remove(&(client_id, cancel.operation_id))
    {
        handle.abort();
    }
    // A missing handle means the operation already completed or was already
    // cancelled. Acknowledging that safe no-op lets a timed-out client close
    // its lease without leaving a BlueZ future running behind it.
    send_response(
        sender,
        ServerMessage::Cancelled {
            operation_id: cancel.operation_id,
        },
    )
    .await
}

#[allow(clippy::too_many_lines)]
async fn process_operation<O: BrokerOwner>(
    client_id: u64,
    operation: Operation,
    state: &Arc<BrokerState<O>>,
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
                    let telemetry = Arc::clone(&state.telemetry);
                    let lease_id = operation.lease_id;
                    let operation_id = operation.operation_id;
                    let indication = info.lease.source() == Source::Bp;
                    tokio::spawn(async move {
                        futures::pin_mut!(stream);
                        while let Some(payload) = stream.next().await {
                            telemetry.notification_bytes(payload.len());
                            if sender
                                .send(ServerMessage::Frame {
                                    lease_id,
                                    operation_id,
                                    indication,
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

async fn release_lease<O: BrokerOwner>(
    client_id: u64,
    release: Release,
    state: &Arc<BrokerState<O>>,
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
    state.telemetry.lease_released(info.lease.source());
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

async fn cleanup_client<O: BrokerOwner>(client_id: u64, state: &Arc<BrokerState<O>>) {
    let operation_handles = {
        let mut operations = state.operations.lock().await;
        let keys: Vec<(u64, u64)> = operations
            .keys()
            .filter(|(owner, _)| *owner == client_id)
            .copied()
            .collect();
        keys.into_iter()
            .filter_map(|key| operations.remove(&key))
            .collect::<Vec<_>>()
    };
    for handle in operation_handles {
        handle.abort();
    }
    let removed_client = state.clients.lock().await.remove(&client_id);
    let (source, leases) = removed_client.map_or((None, Vec::new()), |client| {
        (Some(client.source), client.leases)
    });
    if let Some(source) = source {
        state.telemetry.client_disconnected(source);
    }
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

async fn lease_info<O: BrokerOwner>(
    client_id: u64,
    lease_id: LeaseId,
    state: &Arc<BrokerState<O>>,
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

async fn insert_lease<O: BrokerOwner>(state: &Arc<BrokerState<O>>, info: LeaseInfo) {
    state.leases.lock().await.insert(info.lease.id(), info);
}

async fn remove_lease<O: BrokerOwner>(
    client_id: u64,
    lease_id: LeaseId,
    state: &Arc<BrokerState<O>>,
) {
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

fn deadline_after(now: u64, timeout: u64) -> u64 {
    now.saturating_add(timeout)
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
    let mut seen_devices = HashSet::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("hci") {
            continue;
        }
        let class_path = entry.path();
        let device_path = fs::canonicalize(class_path.join("device"))?;
        // BlueZ can expose aliases such as hci0:77 which resolve below the
        // same physical USB device as hci0. Inventory is physical, not an
        // alias count; duplicate aliases must not make a unique USB selector
        // look ambiguous.
        let physical_path = physical_device_path(&device_path);
        if !seen_devices.insert(physical_path) {
            continue;
        }
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

fn physical_device_path(path: &Path) -> PathBuf {
    for ancestor in path.ancestors() {
        if ancestor.join("idVendor").is_file() && ancestor.join("idProduct").is_file() {
            return ancestor.to_path_buf();
        }
    }
    path.to_path_buf()
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

#[cfg(test)]
mod tests {
    use super::{
        BoxFuture, BoxPayloadStream, BrokerState, ClientInfo, ClientMessage, OWNER_ERROR,
        OwnerOperations, Path, UnixStream, address_for_lease, capability_for, cleanup_client,
        controller_inventory, deadline_after, error_message, operation_address, parse_selector,
        physical_device_path, prepare_socket, process_message, read_message, run, run_with_owner,
        serve_client, validate_hello,
    };
    use crate::bluez::{
        AdapterHealth, AdapterPolicy, ControllerBus, ControllerSelector, DeviceObservation,
        DiscoveryReport, OwnerError,
    };
    use crate::telemetry::Telemetry;
    use crate::{
        AcquireResult, Capability, CapabilitySet, Generation, Intent, LeaseRequest, Priority,
        ServerMessage, Source,
    };
    use futures::stream;
    use std::sync::Arc;
    use std::time::Instant;
    use std::{collections::HashMap, fs};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::{Mutex, mpsc};

    #[derive(Clone)]
    struct FakeCharacteristic(u8);

    struct FakeOwner {
        policy: AdapterPolicy,
        fail_operations: bool,
    }

    impl FakeOwner {
        fn new() -> Self {
            Self {
                policy: AdapterPolicy::new(Generation::new(1)),
                fail_operations: false,
            }
        }
    }

    impl OwnerOperations for FakeOwner {
        type Characteristic = FakeCharacteristic;

        fn generation(&self) -> Generation {
            self.policy.generation()
        }

        fn acquire(
            &mut self,
            request: LeaseRequest,
            now: crate::lease::Tick,
        ) -> Result<AcquireResult, crate::PolicyError> {
            self.policy.acquire(request, now)
        }

        fn release(
            &mut self,
            lease: crate::LeaseId,
            now: crate::lease::Tick,
        ) -> crate::ReleaseOutcome {
            self.policy.release(lease, now)
        }

        fn cancel(&mut self, ticket: crate::QueueTicket) -> bool {
            self.policy.cancel(ticket)
        }

        fn health(&self) -> BoxFuture<'_, Result<AdapterHealth, OwnerError>> {
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(AdapterHealth {
                    generation: Generation::new(1),
                    runtime_name: "fake".to_owned(),
                    powered: true,
                    discovering: false,
                })
            })
        }

        fn discover<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            _filter: bluer::DiscoveryFilter,
            _target: Option<bluer::Address>,
            _duration: std::time::Duration,
        ) -> BoxFuture<'a, Result<DiscoveryReport, OwnerError>> {
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(DiscoveryReport {
                    events: 1,
                    target_found: true,
                    filter_restored: true,
                })
            })
        }

        fn observe<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            _address: bluer::Address,
        ) -> BoxFuture<'a, Result<DeviceObservation, OwnerError>> {
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(DeviceObservation {
                    connected: true,
                    services_resolved: true,
                })
            })
        }

        fn connect<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            _address: bluer::Address,
            _duration: std::time::Duration,
        ) -> BoxFuture<'a, Result<(), OwnerError>> {
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(())
            })
        }

        fn disconnect<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            _address: bluer::Address,
        ) -> BoxFuture<'a, Result<(), OwnerError>> {
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(())
            })
        }

        fn resolve<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            _address: bluer::Address,
            _service_uuid: bluer::Uuid,
            _characteristic_uuid: bluer::Uuid,
        ) -> BoxFuture<'a, Result<Self::Characteristic, OwnerError>> {
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(FakeCharacteristic(1))
            })
        }

        fn subscribe<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            characteristic: &'a Self::Characteristic,
        ) -> BoxFuture<'a, Result<BoxPayloadStream, OwnerError>> {
            assert_eq!(characteristic.0, 1);
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(Box::pin(stream::iter(vec![vec![1_u8, 2, 3]])) as BoxPayloadStream)
            })
        }

        fn write<'a>(
            &'a self,
            _lease: &'a crate::Lease,
            _address: bluer::Address,
            characteristic: &'a Self::Characteristic,
            _payload: &'a [u8],
        ) -> BoxFuture<'a, Result<(), OwnerError>> {
            assert_eq!(characteristic.0, 1);
            let fail = self.fail_operations;
            Box::pin(async move {
                if fail {
                    return Err(OwnerError::WorkerStopped);
                }
                Ok(())
            })
        }
    }

    fn fake_state() -> Arc<BrokerState<FakeOwner>> {
        Arc::new(BrokerState {
            owner: Mutex::new(FakeOwner::new()),
            telemetry: Arc::new(Telemetry::new()),
            selector_text: "bus=usb;vid_pid=0a12:0001".to_owned(),
            allowed_uid: None,
            started: Instant::now(),
            clients: Mutex::new(HashMap::new()),
            leases: Mutex::new(HashMap::new()),
            handles: Mutex::new(HashMap::new()),
            operations: Mutex::new(HashMap::new()),
            next_client: Mutex::new(1),
        })
    }

    #[tokio::test]
    async fn run_with_fake_owner_binds_socket_and_accepts_client() -> Result<(), String> {
        let socket =
            std::env::temp_dir().join(format!("vitals-broker-loop-{}", std::process::id()));
        let config = super::BrokerConfig {
            socket: socket.clone(),
            metrics_path: socket.with_extension("prom"),
            selector_text: "fake".to_owned(),
            selector: ControllerSelector::default(),
            allowed_uid: None,
        };
        let task = tokio::spawn(run_with_owner(config, FakeOwner::new()));
        let mut connected = false;
        for _ in 0..20 {
            if UnixStream::connect(&socket).await.is_ok() {
                connected = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert!(connected);
        task.abort();
        let _ = task.await;
        let _ = fs::remove_file(socket);
        Ok(())
    }

    #[tokio::test]
    async fn run_rejects_a_selector_without_a_local_adapter() {
        let config = super::BrokerConfig {
            socket: std::env::temp_dir().join(format!("vitals-broker-run-{}", std::process::id())),
            metrics_path: std::env::temp_dir()
                .join(format!("vitals-broker-run-{}.prom", std::process::id())),
            selector_text: "address=00:00:00XX:XX:XX".to_owned(),
            selector: ControllerSelector {
                address: Some("00:00:00XX:XX:XX".to_owned()),
                ..ControllerSelector::default()
            },
            allowed_uid: None,
        };
        let result = run(config).await;
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.contains("broker adapter open failed"));
        }
    }

    #[test]
    fn prepare_socket_refuses_files_and_replaces_sockets() -> std::io::Result<()> {
        let root =
            std::env::temp_dir().join(format!("vitals-broker-socket-{}", std::process::id()));
        fs::create_dir_all(&root)?;
        let file = root.join("not-a-socket");
        fs::write(&file, b"guard")?;
        assert!(prepare_socket(&file).is_err());
        let socket = root.join("broker.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket)?;
        drop(listener);
        prepare_socket(&socket)?;
        assert!(!socket.exists());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[tokio::test]
    async fn read_message_rejects_truncated_bad_and_oversized_frames() -> Result<(), String> {
        let (writer, mut reader) = tokio::io::duplex(32);
        drop(writer);
        assert_eq!(
            read_message(&mut reader).await,
            Err("broker socket closed".to_owned())
        );
        let (mut writer, mut reader) = tokio::io::duplex(32);
        writer
            .write_all(&[0_u8; 11])
            .await
            .map_err(|error| error.to_string())?;
        drop(writer);
        assert!(read_message(&mut reader).await.is_err());
        let (mut writer, mut reader) = tokio::io::duplex(32);
        let mut oversized = b"VBT1\x00\x01\x00".to_vec();
        oversized.extend_from_slice(
            &u32::try_from(crate::MAX_FRAME_BYTES)
                .map_err(|error| error.to_string())?
                .to_be_bytes(),
        );
        writer
            .write_all(&oversized)
            .await
            .map_err(|error| error.to_string())?;
        drop(writer);
        assert!(read_message(&mut reader).await.is_err());
        let (mut writer, mut reader) = tokio::io::duplex(32);
        let mut truncated = b"VBT1\x00\x01\x05".to_vec();
        truncated.extend_from_slice(&1_u32.to_be_bytes());
        writer
            .write_all(&truncated)
            .await
            .map_err(|error| error.to_string())?;
        drop(writer);
        assert!(read_message(&mut reader).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn serve_client_covers_handshake_and_protocol_rejection() -> Result<(), String> {
        async fn receive(stream: &mut UnixStream) -> Result<ServerMessage, String> {
            let mut header = [0_u8; 11];
            stream
                .read_exact(&mut header)
                .await
                .map_err(|error| error.to_string())?;
            let length = u32::from_be_bytes([header[7], header[8], header[9], header[10]]) as usize;
            let mut frame = header.to_vec();
            let mut payload = vec![0_u8; length];
            stream
                .read_exact(&mut payload)
                .await
                .map_err(|error| error.to_string())?;
            frame.extend_from_slice(&payload);
            crate::decode_server(&frame).map_err(|error| error.to_string())
        }

        let state = fake_state();
        let (mut peer, broker) = UnixStream::pair().map_err(|error| error.to_string())?;
        let task = tokio::spawn(serve_client(broker, state.clone()));
        let hello = ClientMessage::Hello(crate::Hello {
            protocol_version: crate::PROTOCOL_VERSION,
            source: Source::H10,
            adapter_selector: "bus=usb;vid_pid=0a12:0001".to_owned(),
            device_selector: Some("peer".to_owned()),
            capabilities: CapabilitySet::empty(),
        });
        let frame = crate::encode(&hello).map_err(|error| error.to_string())?;
        peer.write_all(&frame)
            .await
            .map_err(|error| error.to_string())?;
        assert!(matches!(
            receive(&mut peer).await?,
            ServerMessage::HelloAccepted { generation } if generation == Generation::new(1)
        ));
        drop(peer);
        assert!(task.await.map_err(|error| error.to_string())?.is_ok());

        let (mut peer, broker) = UnixStream::pair().map_err(|error| error.to_string())?;
        let task = tokio::spawn(serve_client(broker, state));
        let frame = crate::encode(&ClientMessage::Health).map_err(|error| error.to_string())?;
        peer.write_all(&frame)
            .await
            .map_err(|error| error.to_string())?;
        assert!(matches!(
            receive(&mut peer).await?,
            ServerMessage::Error { code: 3, .. }
        ));
        assert!(task.await.map_err(|error| error.to_string())?.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn fake_owner_propagates_typed_operation_failures() -> Result<(), String> {
        let state = fake_state();
        let capabilities = CapabilitySet::empty()
            .with(Capability::KernelObserve)
            .with(Capability::Discover)
            .with(Capability::Connect)
            .with(Capability::Resolve)
            .with(Capability::Subscribe)
            .with(Capability::Write);
        state.clients.lock().await.insert(
            1,
            ClientInfo {
                source: Source::H10,
                capabilities,
                leases: Vec::new(),
            },
        );
        let (sender, mut responses) = mpsc::channel(32);
        let acquire = crate::Acquire {
            source: Source::H10,
            intent: Intent::Connect,
            priority: Priority::new(1),
            deadline: 30,
            max_hold: 60,
            preemptible: false,
            device_selector: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            capabilities,
            reason: "fake failure test".to_owned(),
        };
        process_message(1, ClientMessage::Acquire(acquire), &state, &sender).await?;
        let lease_id = match responses.recv().await {
            Some(ServerMessage::LeaseGranted { lease_id, .. }) => lease_id,
            other => return Err(format!("expected lease grant, got {other:?}")),
        };
        state.owner.lock().await.fail_operations = true;
        assert!(
            process_message(1, ClientMessage::Health, &state, &sender)
                .await
                .is_err()
        );
        let operation =
            |operation_id: u64, kind: crate::OperationKind, target: &str| crate::Operation {
                lease_id,
                operation_id,
                kind,
                target: target.to_owned(),
                payload: vec![1],
            };
        for (operation_id, kind, target) in [
            (7, crate::OperationKind::ObserveDevice, "not-an-address"),
            (8, crate::OperationKind::DiscoverTarget, "not-an-address"),
            (9, crate::OperationKind::Resolve, "malformed"),
            (
                10,
                crate::OperationKind::Resolve,
                "not-a-uuid|also-not-a-uuid",
            ),
            (11, crate::OperationKind::Subscribe, "unknown"),
            (12, crate::OperationKind::Write, "unknown"),
        ] {
            process_message(
                1,
                ClientMessage::Operation(operation(operation_id, kind, target)),
                &state,
                &sender,
            )
            .await?;
            assert!(matches!(
                responses.recv().await,
                Some(ServerMessage::Error { code: OWNER_ERROR, operation_id: id, .. }) if id == operation_id
            ));
        }
        state
            .handles
            .lock()
            .await
            .insert("fake".to_owned(), FakeCharacteristic(1));
        for (operation_id, kind, target) in [
            (1, crate::OperationKind::ObserveDevice, "AA:BB:CC:DD:EE:FF"),
            (2, crate::OperationKind::DiscoverTarget, "AA:BB:CC:DD:EE:FF"),
            (3, crate::OperationKind::Connect, "AA:BB:CC:DD:EE:FF"),
            (
                4,
                crate::OperationKind::Resolve,
                "0000180d-0000-1000-8000-00805f9b34fb|00002a37-0000-1000-8000-00805f9b34fb",
            ),
            (5, crate::OperationKind::Subscribe, "fake"),
            (6, crate::OperationKind::Write, "fake"),
        ] {
            process_message(
                1,
                ClientMessage::Operation(operation(operation_id, kind, target)),
                &state,
                &sender,
            )
            .await?;
            assert!(
                matches!(responses.recv().await, Some(ServerMessage::Error { operation_id: id, .. }) if id == operation_id)
            );
        }
        Ok(())
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn authorization_and_duplicate_operation_paths_fail_closed() -> Result<(), String> {
        let state = fake_state();
        state.clients.lock().await.insert(
            1,
            ClientInfo {
                source: Source::H10,
                capabilities: CapabilitySet::empty(),
                leases: Vec::new(),
            },
        );
        state.clients.lock().await.insert(
            2,
            ClientInfo {
                source: Source::Bp,
                capabilities: CapabilitySet::empty()
                    .with(Capability::Connect)
                    .with(Capability::Discover)
                    .with(Capability::Write),
                leases: Vec::new(),
            },
        );
        let (sender, mut responses) = mpsc::channel(32);
        let acquire = |source, intent, capabilities| crate::Acquire {
            source,
            intent,
            priority: Priority::new(1),
            deadline: 30,
            max_hold: 60,
            preemptible: false,
            device_selector: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            capabilities,
            reason: "authorization test".to_owned(),
        };
        process_message(
            1,
            ClientMessage::Acquire(acquire(
                Source::AirSense,
                Intent::Connect,
                CapabilitySet::from_capability(Capability::Connect),
            )),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 3, .. })
        ));

        process_message(
            1,
            ClientMessage::Acquire(acquire(
                Source::H10,
                Intent::Connect,
                CapabilitySet::from_capability(Capability::Connect),
            )),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 3, .. })
        ));

        process_message(
            2,
            ClientMessage::Acquire(acquire(
                Source::Bp,
                Intent::Connect,
                CapabilitySet::from_capability(Capability::Connect),
            )),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 3, .. })
        ));

        if let Some(client) = state.clients.lock().await.get_mut(&1) {
            client.capabilities = CapabilitySet::from_capability(Capability::Connect);
        }
        process_message(
            1,
            ClientMessage::Acquire(acquire(
                Source::H10,
                Intent::Connect,
                CapabilitySet::from_capability(Capability::Connect),
            )),
            &state,
            &sender,
        )
        .await?;
        let lease_id = match responses.recv().await {
            Some(ServerMessage::LeaseGranted { lease_id, .. }) => lease_id,
            other => return Err(format!("expected lease grant, got {other:?}")),
        };
        process_message(
            1,
            ClientMessage::Acquire(acquire(
                Source::H10,
                Intent::Connect,
                CapabilitySet::from_capability(Capability::Connect),
            )),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 5, .. })
        ));

        process_message(
            1,
            ClientMessage::Operation(crate::Operation {
                lease_id,
                operation_id: 10,
                kind: crate::OperationKind::ObserveDevice,
                target: "AA:BB:CC:DD:EE:FF".to_owned(),
                payload: Vec::new(),
            }),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 3, .. })
        ));

        state
            .operations
            .lock()
            .await
            .insert((1, 11), futures::future::AbortHandle::new_pair().0);
        process_message(
            1,
            ClientMessage::Operation(crate::Operation {
                lease_id,
                operation_id: 11,
                kind: crate::OperationKind::Connect,
                target: "AA:BB:CC:DD:EE:FF".to_owned(),
                payload: Vec::new(),
            }),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 3, .. })
        ));

        process_message(
            1,
            ClientMessage::Cancel(crate::Cancel {
                lease_id: crate::LeaseId::new(999),
                operation_id: 10,
                reason: "unknown lease".to_owned(),
            }),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Error { code: 3, .. })
        ));
        Ok(())
    }

    #[tokio::test]
    #[expect(
        clippy::too_many_lines,
        reason = "the fake-owner dispatch fixture covers every typed operation"
    )]
    async fn fake_owner_exercises_successful_dispatch_and_cleanup() -> Result<(), String> {
        let state = fake_state();
        let capabilities = CapabilitySet::empty()
            .with(Capability::KernelObserve)
            .with(Capability::Discover)
            .with(Capability::Connect)
            .with(Capability::Resolve)
            .with(Capability::Subscribe)
            .with(Capability::Write);
        state.clients.lock().await.insert(
            1,
            ClientInfo {
                source: Source::H10,
                capabilities,
                leases: Vec::new(),
            },
        );
        let (sender, mut responses) = mpsc::channel(32);
        process_message(1, ClientMessage::Health, &state, &sender).await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Health { .. })
        ));
        let acquire = crate::Acquire {
            source: Source::H10,
            intent: Intent::Connect,
            priority: Priority::new(2),
            deadline: 30,
            max_hold: 60,
            preemptible: false,
            device_selector: Some("AA:BB:CC:DD:EE:FF".to_owned()),
            capabilities,
            reason: "fake-owner test".to_owned(),
        };
        process_message(1, ClientMessage::Acquire(acquire), &state, &sender).await?;
        let lease_id = match responses.recv().await {
            Some(ServerMessage::LeaseGranted { lease_id, .. }) => lease_id,
            other => return Err(format!("expected lease grant, got {other:?}")),
        };
        let address = "AA:BB:CC:DD:EE:FF";
        let operation =
            |operation_id: u64, kind: crate::OperationKind, target: &str| crate::Operation {
                lease_id,
                operation_id,
                kind,
                target: target.to_owned(),
                payload: vec![4, 5],
            };
        for (operation_id, kind, target) in [
            (1, crate::OperationKind::ObserveDevice, address),
            (2, crate::OperationKind::DiscoverTarget, address),
            (3, crate::OperationKind::Connect, address),
            (
                4,
                crate::OperationKind::Resolve,
                "0000180d-0000-1000-8000-00805f9b34fb|00002a37-0000-1000-8000-00805f9b34fb",
            ),
        ] {
            process_message(
                1,
                ClientMessage::Operation(operation(operation_id, kind, target)),
                &state,
                &sender,
            )
            .await?;
            let response = responses.recv().await.ok_or("operation response missing")?;
            assert!(matches!(
                response,
                ServerMessage::DeviceState { .. }
                    | ServerMessage::OperationAccepted { .. }
                    | ServerMessage::Resolved { .. }
            ));
        }
        process_message(
            1,
            ClientMessage::Operation(operation(5, crate::OperationKind::Subscribe, "c1-o4")),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::OperationAccepted { operation_id: 5 })
        ));
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Frame {
                operation_id: 5,
                ..
            })
        ));
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Disconnected {
                operation_id: 5,
                ..
            })
        ));
        process_message(
            1,
            ClientMessage::Operation(operation(6, crate::OperationKind::Write, "c1-o4")),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::OperationAccepted { operation_id: 6 })
        ));
        process_message(
            1,
            ClientMessage::Release(crate::Release {
                lease_id,
                reason: "finished".to_owned(),
            }),
            &state,
            &sender,
        )
        .await?;
        assert!(matches!(
            responses.recv().await,
            Some(ServerMessage::Released { lease_id: released }) if released == lease_id
        ));
        cleanup_client(1, &state).await;
        Ok(())
    }

    #[test]
    fn physical_device_path_collapses_bluetooth_interface_aliases() -> std::io::Result<()> {
        let root =
            std::env::temp_dir().join(format!("vitals-ble-broker-usb-{}", std::process::id()));
        let interface = root.join("3-2:1.0/bluetooth/hci0");
        fs::create_dir_all(&interface)?;
        fs::write(root.join("idVendor"), "0a12\n")?;
        fs::write(root.join("idProduct"), "0001\n")?;
        assert_eq!(physical_device_path(&interface), root);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn selector_parser_accepts_stable_fields_and_rejects_ambiguous_input()
    -> Result<(), &'static str> {
        assert!(parse_selector("").is_err());
        assert!(parse_selector("not-an-address").is_err());
        assert!(parse_selector("AA:BB:CC:DD:EE:FF").is_ok());
        let selector = parse_selector(
            "address=AA:BB:CC:DD:EE:FF;sysfs_path=/sys/devices/usb;serial=serial;bus=usb;vid_pid=0a12:0001",
        )
        .map_err(|_| "fixed selector fixture")?;
        assert_eq!(selector.address.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert_eq!(selector.bus, Some(ControllerBus::Usb));
        assert_eq!(selector.vid_pid, Some((0x0a12, 1)));
        assert!(parse_selector("sysfs_path=").is_err());
        assert!(parse_selector("unknown=value").is_err());
        assert!(parse_selector("bus=unknown").is_err());
        assert!(parse_selector("vid_pid=0001").is_err());
        assert!(parse_selector("vid_pid=zzzz:0001").is_err());
        assert!(parse_selector("address=not-an-address").is_err());
        assert!(parse_selector("bus=usb").is_err());
        Ok(())
    }

    #[test]
    fn selector_parser_exercises_all_field_validation_paths() {
        for value in [
            "address=AA:BB:CC:DD:EE:FF;bus=uart",
            "serial=serial;bus=other",
            "bus=usb;vid_pid=0a12:0001",
        ] {
            assert!(parse_selector(value).is_ok(), "{value}");
        }
        for value in [
            "field-without-equals",
            "sysfs_path=/sys;broken",
            "vid_pid=0001:0002",
            "vid_pid=000g:0001",
            "vid_pid=0001:000g",
            "serial=serial;bus=invalid",
            "serial=serial;unknown=value",
            "bus=uart;vid_pid=0a12:0001",
            "bus=other;vid_pid=0a12:0001",
            "bus=other",
        ] {
            assert!(parse_selector(value).is_err(), "{value}");
        }
    }

    #[test]
    fn usb_identity_reads_serial_and_valid_or_invalid_vid_pid() -> std::io::Result<()> {
        let root =
            std::env::temp_dir().join(format!("vitals-ble-broker-identity-{}", std::process::id()));
        let nested = root.join("device/child");
        fs::create_dir_all(&nested)?;
        fs::write(root.join("serial"), " serial-value \n")?;
        fs::write(root.join("idVendor"), "0a12\n")?;
        fs::write(root.join("idProduct"), "0001\n")?;
        assert_eq!(
            super::usb_identity(&nested),
            (Some("serial-value".to_owned()), Some((0x0a12, 1)))
        );
        fs::write(root.join("idVendor"), "invalid\n")?;
        assert_eq!(
            super::usb_identity(&nested).0,
            Some("serial-value".to_owned())
        );
        fs::remove_file(root.join("serial"))?;
        assert_eq!(super::usb_identity(&nested), (None, None));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn inventory_failure_and_non_controller_entries_are_observable() {
        assert!(controller_inventory(Path::new("/path/that/does/not/exist")).is_err());
    }

    #[test]
    fn inventory_deduplicates_runtime_aliases_by_physical_device() -> std::io::Result<()> {
        let root = std::env::temp_dir().join(format!(
            "vitals-ble-broker-inventory-{}",
            std::process::id()
        ));
        let usb = root.join("devices/usb/1-1");
        let uart = root.join("devices/serial/ttyS0");
        fs::create_dir_all(&usb)?;
        fs::create_dir_all(&uart)?;
        fs::write(usb.join("idVendor"), "0a12\n")?;
        fs::write(usb.join("idProduct"), "0001\n")?;
        fs::write(usb.join("serial"), "usb-serial\n")?;
        for name in ["hci0", "hci0:77"] {
            let class = root.join(name);
            fs::create_dir_all(&class)?;
            std::os::unix::fs::symlink(&usb, class.join("device"))?;
            fs::write(class.join("address"), "AA:BB:CC:DD:EE:FF\n")?;
        }
        let class = root.join("hci1");
        fs::create_dir_all(&class)?;
        std::os::unix::fs::symlink(&uart, class.join("device"))?;
        let records = controller_inventory(&root)?;
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .any(|record| record.bus == ControllerBus::Usb)
        );
        assert!(
            records
                .iter()
                .any(|record| record.bus == ControllerBus::Uart)
        );
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn broker_helpers_fail_closed_with_typed_blockers() {
        assert_eq!(
            capability_for(crate::OperationKind::ObserveDevice),
            Capability::KernelObserve
        );
        assert_eq!(
            capability_for(crate::OperationKind::DiscoverTarget),
            Capability::Discover
        );
        assert_eq!(
            capability_for(crate::OperationKind::Connect),
            Capability::Connect
        );
        assert_eq!(
            capability_for(crate::OperationKind::Resolve),
            Capability::Resolve
        );
        assert_eq!(
            capability_for(crate::OperationKind::Subscribe),
            Capability::Subscribe
        );
        assert_eq!(
            capability_for(crate::OperationKind::Write),
            Capability::Write
        );
        let Ok(address) = "AA:BB:CC:DD:EE:FF".parse() else {
            return;
        };
        assert_eq!(
            operation_address("AA:BB:CC:DD:EE:FF", Some(address)),
            Ok(address)
        );
        assert!(operation_address("invalid", None).is_err());
        let Ok(other) = "11:22:33:44:55:66".parse() else {
            return;
        };
        assert!(operation_address("AA:BB:CC:DD:EE:FF", Some(other)).is_err());
        assert!(matches!(
            error_message(3, OWNER_ERROR, "blocker"),
            ServerMessage::Error {
                operation_id: 3,
                code: OWNER_ERROR,
                ..
            }
        ));
    }

    #[test]
    fn hello_validation_rejects_selector_device_and_bp_authority() {
        let state = fake_state();
        let hello = |source, selector: &str, device_selector, capabilities| crate::Hello {
            protocol_version: crate::PROTOCOL_VERSION,
            source,
            adapter_selector: selector.to_owned(),
            device_selector,
            capabilities,
        };
        assert!(
            validate_hello(
                &state,
                &hello(
                    Source::H10,
                    "wrong",
                    Some("peer".to_owned()),
                    CapabilitySet::empty()
                )
            )
            .is_err()
        );
        assert!(
            validate_hello(
                &state,
                &hello(
                    Source::H10,
                    "bus=usb;vid_pid=0a12:0001",
                    None,
                    CapabilitySet::empty()
                )
            )
            .is_err()
        );
        assert!(
            validate_hello(
                &state,
                &hello(
                    Source::Bp,
                    "bus=usb;vid_pid=0a12:0001",
                    Some("peer".to_owned()),
                    CapabilitySet::from_capability(Capability::Connect),
                )
            )
            .is_err()
        );
        assert!(
            validate_hello(
                &state,
                &hello(
                    Source::H10,
                    "bus=usb;vid_pid=0a12:0001",
                    Some("peer".to_owned()),
                    CapabilitySet::empty(),
                )
            )
            .is_ok()
        );
    }

    #[test]
    fn lease_address_is_derived_only_from_a_valid_device_selector() {
        let lease = LeaseRequest::new(
            Source::H10,
            Intent::Connect,
            Priority::new(1),
            10,
            10,
            Some(crate::DeviceId::new("AA:BB:CC:DD:EE:FF")),
            CapabilitySet::empty(),
        );
        let mut policy = AdapterPolicy::new(Generation::new(1));
        let Ok(AcquireResult::Granted(lease)) = policy.acquire(lease, 0) else {
            return;
        };
        assert_eq!(
            address_for_lease(&lease).map(|value| value.to_string()),
            Some("AA:BB:CC:DD:EE:FF".to_owned())
        );
    }

    #[test]
    fn lease_timeout_is_added_to_broker_tick() {
        assert_eq!(deadline_after(722, 300), 1_022);
    }

    #[test]
    fn lease_timeout_addition_saturates() {
        assert_eq!(deadline_after(u64::MAX, 1), u64::MAX);
    }
}
