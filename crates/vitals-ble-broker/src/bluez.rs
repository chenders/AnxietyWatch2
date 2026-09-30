//! Stable-controller resolution and the Linux `BlueZ` adapter owner.
//!
//! The selector and failure policy are portable so they can be tested without a
//! controller. The D-Bus owner is compiled only for Linux with the `linux`
//! feature. It receives controller evidence from the host inventory layer; it
//! never treats `hciN` as hardware identity and never exposes a generic D-Bus
//! operation.

use std::fmt;

use crate::lease::{
    AcquireResult, Generation, Lease, LeaseId, LeaseRequest, LeaseScheduler, PolicyError,
    ReleaseOutcome, SchedulerEvent,
};

/// Stable bus kind for controller evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerBus {
    /// A USB-attached controller.
    Usb,
    /// The Pi's UART-attached controller.
    Uart,
    /// A bus reported by inventory but not classified by this policy.
    Other,
}

/// A configured controller identity. `runtime_name` is deliberately absent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControllerSelector {
    /// The controller Bluetooth address, when validated by inventory.
    pub address: Option<String>,
    /// The stable sysfs device path or physical USB port.
    pub sysfs_path: Option<String>,
    /// The USB serial, when the hardware actually reports one.
    pub serial: Option<String>,
    /// The expected bus.
    pub bus: Option<ControllerBus>,
    /// The expected USB vendor/product pair.
    pub vid_pid: Option<(u16, u16)>,
}

impl ControllerSelector {
    /// Return whether at least one genuinely stable identity field is present.
    ///
    /// A USB controller with no readable serial or sysfs address can still be
    /// selected by its transport and VID/PID. Resolution remains fail-closed:
    /// two matching controllers are ambiguous, while moving one controller to
    /// another USB port changes only its sysfs path and not this identity.
    #[must_use]
    pub fn has_stable_identity(&self) -> bool {
        self.address.is_some()
            || self.sysfs_path.is_some()
            || self.serial.is_some()
            || (self.bus == Some(ControllerBus::Usb) && self.vid_pid.is_some())
    }
}

/// Runtime evidence for one controller. The runtime name is only an alias.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerEvidence {
    /// Current `BlueZ` adapter name, such as `hci0`.
    pub runtime_name: String,
    /// Controller Bluetooth address.
    pub address: Option<String>,
    /// Stable sysfs path or physical USB port.
    pub sysfs_path: Option<String>,
    /// USB serial, if present.
    pub serial: Option<String>,
    /// Physical bus.
    pub bus: ControllerBus,
    /// USB vendor/product pair, when applicable.
    pub vid_pid: Option<(u16, u16)>,
}

impl ControllerEvidence {
    fn matches(&self, selector: &ControllerSelector) -> bool {
        selector
            .address
            .as_ref()
            .is_none_or(|value| self.address.as_ref() == Some(value))
            && selector
                .sysfs_path
                .as_ref()
                .is_none_or(|value| self.sysfs_path.as_ref() == Some(value))
            && selector
                .serial
                .as_ref()
                .is_none_or(|value| self.serial.as_ref() == Some(value))
            && selector.bus.is_none_or(|value| self.bus == value)
            && selector
                .vid_pid
                .is_none_or(|value| self.vid_pid == Some(value))
    }
}

/// The selected controller and its current runtime alias.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedController {
    /// The validated stable selector.
    pub selector: ControllerSelector,
    /// Current `BlueZ` adapter alias, retained as runtime evidence only.
    pub runtime_name: String,
}

/// Stable-controller resolution failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolutionError {
    /// No stable selector field was configured.
    EmptySelector,
    /// No inventory record matched the selector.
    NoMatch {
        /// Number of inventory records considered.
        candidates: usize,
    },
    /// More than one inventory record matched the selector.
    Ambiguous {
        /// Number of matching inventory records.
        matches: usize,
    },
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySelector => formatter.write_str(
                "controller resolution refused: selector has no stable address, sysfs path, or serial",
            ),
            Self::NoMatch { candidates } => write!(
                formatter,
                "controller resolution refused: no stable match among {candidates} inventory records",
            ),
            Self::Ambiguous { matches } => write!(
                formatter,
                "controller resolution refused: stable selector matched {matches} controllers",
            ),
        }
    }
}

impl std::error::Error for ResolutionError {}

/// Resolve exactly one controller without consulting its volatile `hciN` name.
///
/// # Errors
///
/// Returns a named failure for an empty, missing, or ambiguous stable match.
pub fn resolve_controller(
    selector: &ControllerSelector,
    inventory: &[ControllerEvidence],
) -> Result<ResolvedController, ResolutionError> {
    if !selector.has_stable_identity() {
        return Err(ResolutionError::EmptySelector);
    }
    let matches: Vec<&ControllerEvidence> = inventory
        .iter()
        .filter(|evidence| evidence.matches(selector))
        .collect();
    match matches.as_slice() {
        [] => Err(ResolutionError::NoMatch {
            candidates: inventory.len(),
        }),
        [evidence] => Ok(ResolvedController {
            selector: selector.clone(),
            runtime_name: evidence.runtime_name.clone(),
        }),
        _ => Err(ResolutionError::Ambiguous {
            matches: matches.len(),
        }),
    }
}

/// Failure class used in broker status and telemetry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BluezFailureClass {
    /// The D-Bus daemon or its connection is unavailable.
    DaemonUnavailable,
    /// The selected adapter disappeared.
    AdapterMissing,
    /// Another owner already has discovery active.
    DiscoveryBusy,
    /// The peer object is absent or was removed.
    DeviceMissing,
    /// `BlueZ` rejected a connection attempt.
    ConnectRejected,
    /// The peer connected but services did not resolve.
    ServicesUnresolved,
    /// A GATT operation or subscription was rejected.
    GattRejected,
    /// The controller was reset, unplugged, or otherwise lost.
    ControllerLost,
    /// An operation exceeded its caller-supplied deadline.
    DeadlineExceeded,
    /// An error did not fit a more specific class.
    Unknown,
}

/// Failure while opening a selected Linux adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpenError {
    /// Stable inventory resolution failed.
    Resolution(ResolutionError),
    /// `BlueZ` could not enumerate or expose the selected runtime adapter.
    Bluez {
        /// Classified failure.
        class: BluezFailureClass,
        /// Operation that failed.
        operation: &'static str,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolution(error) => error.fmt(formatter),
            Self::Bluez { class, operation } => {
                write!(formatter, "BlueZ {operation} failed: {class:?}")
            }
        }
    }
}

impl std::error::Error for OpenError {}

impl From<ResolutionError> for OpenError {
    fn from(error: ResolutionError) -> Self {
        Self::Resolution(error)
    }
}

/// A portable description of adapter health.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterHealth {
    /// Current broker generation.
    pub generation: Generation,
    /// Current runtime adapter alias, never used as identity.
    pub runtime_name: String,
    /// Whether `BlueZ` reports the adapter powered.
    pub powered: bool,
    /// Whether `BlueZ` reports discovery active.
    pub discovering: bool,
}

/// A discovery operation result after the prior filter was restored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryReport {
    /// Number of device events observed.
    pub events: usize,
    /// Whether the requested address was observed, if one was supplied.
    pub target_found: bool,
    /// Whether restoration of the prior filter succeeded.
    pub filter_restored: bool,
}

/// Observation of a device created by `BlueZ` or kernel policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceObservation {
    /// Whether `BlueZ` reports the peer connected.
    pub connected: bool,
    /// Whether `BlueZ` has resolved the peer's GATT services.
    pub services_resolved: bool,
}

/// Owner-level error, including the value that blocked an operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnerError {
    /// The lease is no longer active or belongs to a prior generation.
    LeaseNotOwned {
        /// Lease identifier that failed ownership validation.
        lease: LeaseId,
    },
    /// The lease intent cannot authorize this operation.
    WrongIntent {
        /// Lease identifier that requested the operation.
        lease: LeaseId,
        /// Operation name required by the owner.
        expected: &'static str,
    },
    /// BP has no userspace connection path.
    BpConnectForbidden,
    /// A `BlueZ` operation failed with a classified cause.
    Bluez {
        /// Classified failure.
        class: BluezFailureClass,
        /// Operation name, without a peer identifier.
        operation: &'static str,
    },
    /// The prior discovery filter could not be restored.
    FilterRestoreFailed,
    /// The broker task failed before it could restore state.
    WorkerStopped,
    /// A bounded operation exceeded its deadline.
    DeadlineExceeded {
        /// Operation that exceeded its deadline.
        operation: &'static str,
    },
}

impl fmt::Display for OwnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LeaseNotOwned { lease } => {
                write!(
                    formatter,
                    "operation blocked: lease={} is not active",
                    lease.value()
                )
            }
            Self::WrongIntent { lease, expected } => write!(
                formatter,
                "operation blocked: lease={} does not authorize {expected}",
                lease.value()
            ),
            Self::BpConnectForbidden => {
                formatter.write_str("operation blocked: BP has no userspace connect capability")
            }
            Self::Bluez { class, operation } => {
                write!(formatter, "BlueZ {operation} failed: {class:?}")
            }
            Self::FilterRestoreFailed => {
                formatter.write_str("discovery blocked: prior filter restoration failed")
            }
            Self::WorkerStopped => formatter
                .write_str("discovery blocked: broker worker stopped before filter restoration"),
            Self::DeadlineExceeded { operation } => {
                write!(formatter, "operation deadline exceeded: {operation}")
            }
        }
    }
}

impl std::error::Error for OwnerError {}

/// Pure contract surface used by the Linux owner and its tests.
#[derive(Clone, Debug)]
pub struct AdapterPolicy {
    scheduler: LeaseScheduler,
}

impl AdapterPolicy {
    /// Construct policy for one controller generation.
    #[must_use]
    pub const fn new(generation: Generation) -> Self {
        Self {
            scheduler: LeaseScheduler::new(generation),
        }
    }

    /// Acquire an adapter or device lease.
    ///
    /// # Errors
    ///
    /// Returns the scheduler policy error when the request is invalid.
    pub fn acquire(
        &mut self,
        request: LeaseRequest,
        now: crate::lease::Tick,
    ) -> Result<AcquireResult, PolicyError> {
        self.scheduler.acquire(request, now)
    }

    /// Release a lease and dispatch queued work.
    pub fn release(&mut self, lease: LeaseId, now: crate::lease::Tick) -> ReleaseOutcome {
        self.scheduler.release(lease, now)
    }

    /// Cancel a queued request when its worker disconnects or declines to wait.
    pub fn cancel(&mut self, ticket: crate::lease::QueueTicket) -> bool {
        self.scheduler.cancel(ticket)
    }

    /// Return the current scheduler generation.
    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.scheduler.generation()
    }

    /// Advance expiry and dispatch queued work.
    pub fn advance(&mut self, now: crate::lease::Tick) -> ReleaseOutcome {
        self.scheduler.advance(now)
    }

    /// Invalidate all work on a controller generation change.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied generation is not newer.
    pub fn invalidate_generation(
        &mut self,
        generation: Generation,
    ) -> Result<Vec<SchedulerEvent>, PolicyError> {
        self.scheduler.invalidate_generation(generation)
    }

    /// Return whether a lease is active in the current generation.
    #[must_use]
    pub fn owns(&self, lease: &Lease) -> bool {
        self.scheduler
            .active_lease(lease.id())
            .is_some_and(|active| active.generation() == lease.generation())
    }
}

#[cfg(all(feature = "linux", target_os = "linux"))]
mod runtime {
    #![allow(clippy::elidable_lifetime_names, clippy::missing_errors_doc)]

    use super::{
        AcquireResult, AdapterHealth, AdapterPolicy, BluezFailureClass, ControllerEvidence,
        ControllerSelector, DeviceObservation, DiscoveryReport, Generation, Lease, LeaseId,
        LeaseRequest, OpenError, OwnerError, PolicyError, ReleaseOutcome, ResolvedController,
        SchedulerEvent, resolve_controller,
    };
    use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

    use bluer::{Adapter, Address, Session, gatt::remote::Characteristic};
    use futures::{Stream, StreamExt};
    use tokio::sync::{Mutex, OwnedMutexGuard};

    type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
    pub type BackendStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

    #[allow(missing_docs)]
    pub trait CharacteristicIo: Clone + Send + Sync + 'static {
        fn uuid<'a>(&'a self) -> BackendFuture<'a, Result<bluer::Uuid, OwnerError>>;
        fn notify<'a>(&'a self) -> BackendFuture<'a, Result<BackendStream<Vec<u8>>, OwnerError>>;
        fn write<'a>(&'a self, payload: &'a [u8]) -> BackendFuture<'a, Result<(), OwnerError>>;
    }

    #[allow(missing_docs)]
    pub trait ServiceIo: Clone + Send + Sync + 'static {
        type Characteristic: CharacteristicIo;
        fn uuid<'a>(&'a self) -> BackendFuture<'a, Result<bluer::Uuid, OwnerError>>;
        fn characteristics<'a>(
            &'a self,
        ) -> BackendFuture<'a, Result<Vec<Self::Characteristic>, OwnerError>>;
    }

    #[allow(missing_docs)]
    pub trait DeviceIo: Clone + Send + Sync + 'static {
        type Service: ServiceIo<Characteristic = Self::Characteristic>;
        type Characteristic: CharacteristicIo;
        fn connected<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>>;
        fn services_resolved<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>>;
        fn connect<'a>(&'a self) -> BackendFuture<'a, Result<(), OwnerError>>;
        fn disconnect<'a>(&'a self) -> BackendFuture<'a, Result<(), OwnerError>>;
        fn services<'a>(&'a self) -> BackendFuture<'a, Result<Vec<Self::Service>, OwnerError>>;
    }

    #[allow(missing_docs)]
    pub trait AdapterIo: Clone + Send + Sync + 'static {
        type Device: DeviceIo<Characteristic = Self::Characteristic>;
        type Characteristic: CharacteristicIo;
        fn powered<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>>;
        fn discovering<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>>;
        fn discovery_filter<'a>(&'a self) -> BackendFuture<'a, bluer::DiscoveryFilter>;
        fn set_discovery_filter<'a>(
            &'a self,
            filter: bluer::DiscoveryFilter,
        ) -> BackendFuture<'a, Result<(), OwnerError>>;
        fn discover_devices<'a>(
            &'a self,
        ) -> BackendFuture<'a, Result<BackendStream<bluer::AdapterEvent>, OwnerError>>;
        fn device(&self, address: Address) -> Result<Self::Device, OwnerError>;
        fn set_powered<'a>(&'a self, powered: bool) -> BackendFuture<'a, Result<(), OwnerError>>;
    }

    impl CharacteristicIo for Characteristic {
        fn uuid<'a>(&'a self) -> BackendFuture<'a, Result<bluer::Uuid, OwnerError>> {
            Box::pin(async move {
                self.uuid().await.map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "read characteristic UUID",
                })
            })
        }

        fn notify<'a>(&'a self) -> BackendFuture<'a, Result<BackendStream<Vec<u8>>, OwnerError>> {
            Box::pin(async move {
                self.notify()
                    .await
                    .map(|stream| Box::pin(stream) as BackendStream<Vec<u8>>)
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "subscribe characteristic",
                    })
            })
        }

        fn write<'a>(&'a self, payload: &'a [u8]) -> BackendFuture<'a, Result<(), OwnerError>> {
            Box::pin(async move {
                self.write(payload)
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "write characteristic",
                    })
            })
        }
    }

    impl ServiceIo for bluer::gatt::remote::Service {
        type Characteristic = Characteristic;

        fn uuid<'a>(&'a self) -> BackendFuture<'a, Result<bluer::Uuid, OwnerError>> {
            Box::pin(async move {
                self.uuid().await.map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "read service UUID",
                })
            })
        }

        fn characteristics<'a>(
            &'a self,
        ) -> BackendFuture<'a, Result<Vec<Self::Characteristic>, OwnerError>> {
            Box::pin(async move {
                self.characteristics()
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "resolve characteristics",
                    })
            })
        }
    }

    impl DeviceIo for bluer::Device {
        type Service = bluer::gatt::remote::Service;
        type Characteristic = Characteristic;

        fn connected<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
            Box::pin(async move {
                self.is_connected()
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "read Connected",
                    })
            })
        }

        fn services_resolved<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
            Box::pin(async move {
                self.is_services_resolved()
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "read ServicesResolved",
                    })
            })
        }

        fn connect<'a>(&'a self) -> BackendFuture<'a, Result<(), OwnerError>> {
            Box::pin(async move {
                bluer::Device::connect(self)
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "connect device",
                    })
            })
        }

        fn disconnect<'a>(&'a self) -> BackendFuture<'a, Result<(), OwnerError>> {
            Box::pin(async move {
                self.disconnect().await.map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "disconnect device",
                })
            })
        }

        fn services<'a>(&'a self) -> BackendFuture<'a, Result<Vec<Self::Service>, OwnerError>> {
            Box::pin(async move {
                self.services().await.map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "resolve services",
                })
            })
        }
    }

    impl AdapterIo for Adapter {
        type Device = bluer::Device;
        type Characteristic = Characteristic;

        fn powered<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
            Box::pin(async move {
                self.is_powered().await.map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DaemonUnavailable,
                    operation: "read Powered",
                })
            })
        }

        fn discovering<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
            Box::pin(async move {
                self.is_discovering().await.map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DaemonUnavailable,
                    operation: "read Discovering",
                })
            })
        }

        fn discovery_filter<'a>(&'a self) -> BackendFuture<'a, bluer::DiscoveryFilter> {
            Box::pin(async move { self.discovery_filter().await })
        }

        fn set_discovery_filter<'a>(
            &'a self,
            filter: bluer::DiscoveryFilter,
        ) -> BackendFuture<'a, Result<(), OwnerError>> {
            Box::pin(async move {
                self.set_discovery_filter(filter)
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "set discovery filter",
                    })
            })
        }

        fn discover_devices<'a>(
            &'a self,
        ) -> BackendFuture<'a, Result<BackendStream<bluer::AdapterEvent>, OwnerError>> {
            Box::pin(async move {
                self.discover_devices()
                    .await
                    .map(|stream| Box::pin(stream) as BackendStream<bluer::AdapterEvent>)
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "start discovery",
                    })
            })
        }

        fn device(&self, address: Address) -> Result<Self::Device, OwnerError> {
            self.device(address).map_err(|_| OwnerError::Bluez {
                class: BluezFailureClass::DeviceMissing,
                operation: "materialize device",
            })
        }

        fn set_powered<'a>(&'a self, powered: bool) -> BackendFuture<'a, Result<(), OwnerError>> {
            Box::pin(async move {
                self.set_powered(powered)
                    .await
                    .map_err(|_| OwnerError::Bluez {
                        class: BluezFailureClass::DaemonUnavailable,
                        operation: "set adapter power",
                    })
            })
        }
    }

    /// A Linux `BlueZ` adapter owner. All application-level `BlueZ` operations pass
    /// through this value; workers receive only typed results from its methods.
    pub struct BluezAdapterOwner<A: AdapterIo> {
        session: Option<Session>,
        adapter: A,
        controller: ResolvedController,
        policy: AdapterPolicy,
        writes: Mutex<HashMap<Address, Arc<Mutex<()>>>>,
    }

    impl BluezAdapterOwner<Adapter> {
        /// Open a `BlueZ` session only after stable inventory resolution.
        ///
        /// # Errors
        ///
        /// Fails closed if the selector is missing/ambiguous or the resolved
        /// runtime adapter is no longer present in `BlueZ`.
        pub async fn open(
            selector: &ControllerSelector,
            inventory: &[ControllerEvidence],
            generation: Generation,
        ) -> Result<Self, OpenError> {
            let controller = resolve_controller(selector, inventory)?;
            let session = Session::new().await.map_err(|_| OpenError::Bluez {
                class: BluezFailureClass::DaemonUnavailable,
                operation: "open D-Bus session",
            })?;
            let names = session
                .adapter_names()
                .await
                .map_err(|_| OpenError::Bluez {
                    class: BluezFailureClass::DaemonUnavailable,
                    operation: "enumerate adapters",
                })?;
            if !names.iter().any(|name| name == &controller.runtime_name) {
                return Err(OpenError::Bluez {
                    class: BluezFailureClass::AdapterMissing,
                    operation: "resolve selected adapter alias",
                });
            }
            let adapter =
                session
                    .adapter(&controller.runtime_name)
                    .map_err(|_| OpenError::Bluez {
                        class: BluezFailureClass::AdapterMissing,
                        operation: "construct selected adapter",
                    })?;
            let mut owner = Self::from_backend(adapter, controller, generation);
            owner.session = Some(session);
            Ok(owner)
        }
    }

    impl<A: AdapterIo> BluezAdapterOwner<A> {
        pub(crate) fn from_backend(
            adapter: A,
            controller: ResolvedController,
            generation: Generation,
        ) -> Self {
            Self {
                session: None,
                adapter,
                controller,
                policy: AdapterPolicy::new(generation),
                writes: Mutex::new(HashMap::new()),
            }
        }

        /// Return the validated controller and current runtime alias.
        #[must_use]
        pub fn controller(&self) -> &ResolvedController {
            &self.controller
        }

        /// Acquire a lease through the broker scheduler.
        pub fn acquire(
            &mut self,
            request: LeaseRequest,
            now: crate::lease::Tick,
        ) -> Result<AcquireResult, PolicyError> {
            self.policy.acquire(request, now)
        }

        /// Release a lease and obtain any newly dispatched leases.
        pub fn release(&mut self, lease: LeaseId, now: crate::lease::Tick) -> ReleaseOutcome {
            self.policy.release(lease, now)
        }

        /// Cancel a queued request when its worker disconnects or declines to wait.
        pub fn cancel(&mut self, ticket: crate::lease::QueueTicket) -> bool {
            self.policy.cancel(ticket)
        }

        /// Return the current scheduler generation.
        #[must_use]
        pub const fn generation(&self) -> Generation {
            self.policy.generation()
        }

        /// Invalidate all leases after a controller or `BlueZ` generation change.
        pub fn invalidate_generation(
            &mut self,
            generation: Generation,
        ) -> Result<Vec<SchedulerEvent>, PolicyError> {
            self.policy.invalidate_generation(generation)
        }

        /// Change adapter power only under an explicit recovery lease.
        pub async fn set_powered(&self, lease: &Lease, powered: bool) -> Result<(), OwnerError> {
            self.check_lease(lease, crate::lease::Intent::PowerRecovery)?;
            self.adapter.set_powered(powered).await
        }

        /// Read adapter health without inferring readiness from process state.
        pub async fn health(&self) -> Result<AdapterHealth, OwnerError> {
            let powered = self.adapter.powered().await?;
            let discovering = self.adapter.discovering().await?;
            Ok(AdapterHealth {
                generation: self.policy.generation(),
                runtime_name: self.controller.runtime_name.clone(),
                powered,
                discovering,
            })
        }

        /// Run one bounded, broker-owned discovery and restore the prior filter.
        pub async fn discover(
            &self,
            lease: &Lease,
            filter: bluer::DiscoveryFilter,
            target: Option<Address>,
            duration: Duration,
        ) -> Result<DiscoveryReport, OwnerError> {
            self.check_lease(lease, crate::lease::Intent::Discover)?;
            let adapter = self.adapter.clone();
            let task = tokio::spawn(async move {
                if adapter.discovering().await? {
                    return Err(OwnerError::Bluez {
                        class: BluezFailureClass::DiscoveryBusy,
                        operation: "start discovery",
                    });
                }
                let previous = adapter.discovery_filter().await;
                adapter.set_discovery_filter(filter).await?;
                let mut events = match adapter.discover_devices().await {
                    Ok(events) => events,
                    Err(error) => {
                        let restored = adapter.set_discovery_filter(previous).await.is_ok();
                        return if restored {
                            Err(error)
                        } else {
                            Err(OwnerError::FilterRestoreFailed)
                        };
                    }
                };
                let scan = async {
                    let mut count = 0_usize;
                    let mut found = false;
                    while let Some(event) = events.next().await {
                        count = count.saturating_add(1);
                        if let bluer::AdapterEvent::DeviceAdded(address) = event
                            && target == Some(address)
                        {
                            found = true;
                            break;
                        }
                    }
                    (count, found)
                };
                let (events_seen, target_found) = tokio::time::timeout(duration, scan)
                    .await
                    .unwrap_or((0, false));
                drop(events);
                if adapter.set_discovery_filter(previous).await.is_err() {
                    return Err(OwnerError::FilterRestoreFailed);
                }
                Ok(DiscoveryReport {
                    events: events_seen,
                    target_found,
                    filter_restored: true,
                })
            });
            task.await.map_err(|_| OwnerError::WorkerStopped)?
        }

        /// Observe a kernel-created connection without initiating one.
        pub async fn observe(
            &self,
            lease: &Lease,
            address: Address,
        ) -> Result<DeviceObservation, OwnerError> {
            self.check_lease(lease, crate::lease::Intent::KernelObserve)?;
            let device = self.adapter.device(address)?;
            let connected = device.connected().await?;
            let services_resolved = device.services_resolved().await?;
            Ok(DeviceObservation {
                connected,
                services_resolved,
            })
        }

        /// Initiate a permitted non-BP connection with a cancellation deadline.
        pub async fn connect(
            &self,
            lease: &Lease,
            address: Address,
            duration: Duration,
        ) -> Result<(), OwnerError> {
            self.check_lease(lease, crate::lease::Intent::Connect)?;
            if lease.source() == crate::lease::Source::Bp {
                return Err(OwnerError::BpConnectForbidden);
            }
            let device = self.adapter.device(address)?;
            if let Ok(result) = tokio::time::timeout(duration, device.connect()).await {
                result
            } else {
                let _ = device.disconnect().await;
                Err(OwnerError::DeadlineExceeded {
                    operation: "connect device",
                })
            }
        }

        /// Disconnect a device under its device lease.
        pub async fn disconnect(&self, lease: &Lease, address: Address) -> Result<(), OwnerError> {
            self.check_lease(lease, crate::lease::Intent::Connect)?;
            let device = self.adapter.device(address)?;
            device.disconnect().await
        }

        /// Resolve one typed remote characteristic by service and characteristic UUID.
        pub async fn resolve(
            &self,
            lease: &Lease,
            address: Address,
            service_uuid: bluer::Uuid,
            characteristic_uuid: bluer::Uuid,
        ) -> Result<<A::Device as DeviceIo>::Characteristic, OwnerError> {
            self.check_gatt_lease(lease)?;
            let device = self.adapter.device(address)?;
            let services = device.services().await?;
            for service in services {
                if service.uuid().await? != service_uuid {
                    continue;
                }
                for characteristic in service.characteristics().await? {
                    if characteristic.uuid().await? == characteristic_uuid {
                        return Ok(characteristic);
                    }
                }
            }
            Err(OwnerError::Bluez {
                class: BluezFailureClass::GattRejected,
                operation: "resolve requested characteristic",
            })
        }

        /// Subscribe to notifications or indications through a typed GATT handle.
        pub async fn subscribe(
            &self,
            lease: &Lease,
            characteristic: &<A::Device as DeviceIo>::Characteristic,
        ) -> Result<BackendStream<Vec<u8>>, OwnerError> {
            self.check_gatt_lease(lease)?;
            characteristic.notify().await
        }

        /// Serialize writes to one peer while allowing different peers to proceed independently.
        pub async fn write(
            &self,
            lease: &Lease,
            address: Address,
            characteristic: &<A::Device as DeviceIo>::Characteristic,
            payload: &[u8],
        ) -> Result<(), OwnerError> {
            self.check_lease(lease, crate::lease::Intent::Gatt)?;
            let guard = self.write_guard(address).await;
            let result = characteristic.write(payload).await;
            drop(guard);
            result
        }

        async fn write_guard(&self, address: Address) -> OwnedMutexGuard<()> {
            let mutex = {
                let mut guards = self.writes.lock().await;
                guards
                    .entry(address)
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .clone()
            };
            mutex.lock_owned().await
        }

        fn check_gatt_lease(&self, lease: &Lease) -> Result<(), OwnerError> {
            if lease.intent() == crate::lease::Intent::KernelObserve
                && lease.source() == crate::lease::Source::Bp
            {
                if !self.policy.owns(lease) {
                    return Err(OwnerError::LeaseNotOwned { lease: lease.id() });
                }
                return Ok(());
            }
            if lease.intent() == crate::lease::Intent::Connect
                || lease.intent() == crate::lease::Intent::Gatt
            {
                if !self.policy.owns(lease) {
                    return Err(OwnerError::LeaseNotOwned { lease: lease.id() });
                }
                return Ok(());
            }
            self.check_lease(lease, crate::lease::Intent::Gatt)
        }

        fn check_lease(
            &self,
            lease: &Lease,
            expected: crate::lease::Intent,
        ) -> Result<(), OwnerError> {
            if !self.policy.owns(lease) {
                return Err(OwnerError::LeaseNotOwned { lease: lease.id() });
            }
            if lease.intent() != expected {
                return Err(OwnerError::WrongIntent {
                    lease: lease.id(),
                    expected: match expected {
                        crate::lease::Intent::Discover => "discovery",
                        crate::lease::Intent::Connect => "connection",
                        crate::lease::Intent::Gatt => "GATT",
                        crate::lease::Intent::PowerRecovery => "adapter power recovery",
                        _ => "typed operation",
                    },
                });
            }
            Ok(())
        }
    }

    fn classify_bluer_error(error: &bluer::Error) -> BluezFailureClass {
        use bluer::ErrorKind;
        match error.kind {
            ErrorKind::NotFound | ErrorKind::DoesNotExist => BluezFailureClass::DeviceMissing,
            ErrorKind::DiscoveryActive => BluezFailureClass::DiscoveryBusy,
            ErrorKind::ServicesUnresolved => BluezFailureClass::ServicesUnresolved,
            ErrorKind::ConnectionAttemptFailed
            | ErrorKind::AuthenticationCanceled
            | ErrorKind::AuthenticationFailed
            | ErrorKind::AuthenticationRejected
            | ErrorKind::AuthenticationTimeout
            | ErrorKind::InProgress
            | ErrorKind::NotReady => BluezFailureClass::ConnectRejected,
            ErrorKind::Internal(bluer::InternalErrorKind::DBusConnectionLost) => {
                BluezFailureClass::ControllerLost
            }
            ErrorKind::NotAvailable | ErrorKind::Internal(_) => {
                BluezFailureClass::DaemonUnavailable
            }
            ErrorKind::NotPermitted | ErrorKind::NotAuthorized | ErrorKind::NotSupported => {
                BluezFailureClass::GattRejected
            }
            _ => BluezFailureClass::Unknown,
        }
    }

    /// Production `BlueZ` owner backed by the real Linux adapter.
    pub type LinuxBluezAdapterOwner = BluezAdapterOwner<Adapter>;

    #[cfg(test)]
    mod tests {
        #![allow(
            clippy::bool_assert_comparison,
            clippy::panic,
            clippy::too_many_lines,
            clippy::unwrap_used
        )]

        use super::*;
        use crate::bluez::ControllerBus;

        fn error(kind: bluer::ErrorKind) -> bluer::Error {
            bluer::Error {
                kind,
                message: String::new(),
            }
        }

        #[derive(Clone)]
        struct FakeCharacteristic {
            state: Arc<std::sync::Mutex<FakeCharacteristicState>>,
        }

        #[derive(Clone)]
        struct FakeCharacteristicState {
            uuid: bluer::Uuid,
            notify: Result<Vec<Vec<u8>>, OwnerError>,
            write: Result<(), OwnerError>,
        }

        #[derive(Clone)]
        struct FakeService {
            state: Arc<std::sync::Mutex<FakeServiceState>>,
        }

        #[derive(Clone)]
        struct FakeServiceState {
            uuid: Result<bluer::Uuid, OwnerError>,
            characteristics: Result<Vec<FakeCharacteristic>, OwnerError>,
        }

        #[derive(Clone)]
        struct FakeDevice {
            state: Arc<std::sync::Mutex<FakeDeviceState>>,
        }

        #[derive(Clone)]
        struct FakeDeviceState {
            connected: Result<bool, OwnerError>,
            services_resolved: Result<bool, OwnerError>,
            connect: Result<(), OwnerError>,
            connect_pending: bool,
            disconnect: Result<(), OwnerError>,
            services: Result<Vec<FakeService>, OwnerError>,
        }

        #[derive(Clone)]
        struct FakeAdapter {
            state: Arc<std::sync::Mutex<FakeAdapterState>>,
        }

        struct FakeAdapterState {
            powered: Result<bool, OwnerError>,
            discovering: Result<bool, OwnerError>,
            set_powered: Result<(), OwnerError>,
            set_filter: Result<(), OwnerError>,
            set_filter_calls: usize,
            fail_set_filter_call: Option<usize>,
            discovery: Result<Vec<bluer::AdapterEvent>, OwnerError>,
            discovery_pending: bool,
            device: Result<FakeDevice, OwnerError>,
        }

        impl CharacteristicIo for FakeCharacteristic {
            fn uuid<'a>(&'a self) -> BackendFuture<'a, Result<bluer::Uuid, OwnerError>> {
                let uuid = self.state.lock().unwrap().uuid;
                Box::pin(async move { Ok(uuid) })
            }

            fn notify<'a>(
                &'a self,
            ) -> BackendFuture<'a, Result<BackendStream<Vec<u8>>, OwnerError>> {
                let result = self.state.lock().unwrap().notify.clone();
                Box::pin(async move {
                    result.map(|payloads| {
                        Box::pin(futures::stream::iter(payloads)) as BackendStream<_>
                    })
                })
            }

            fn write<'a>(
                &'a self,
                _payload: &'a [u8],
            ) -> BackendFuture<'a, Result<(), OwnerError>> {
                let result = self.state.lock().unwrap().write.clone();
                Box::pin(async move { result })
            }
        }

        impl ServiceIo for FakeService {
            type Characteristic = FakeCharacteristic;

            fn uuid<'a>(&'a self) -> BackendFuture<'a, Result<bluer::Uuid, OwnerError>> {
                let result = self.state.lock().unwrap().uuid.clone();
                Box::pin(async move { result })
            }

            fn characteristics<'a>(
                &'a self,
            ) -> BackendFuture<'a, Result<Vec<Self::Characteristic>, OwnerError>> {
                let result = self.state.lock().unwrap().characteristics.clone();
                Box::pin(async move { result })
            }
        }

        impl DeviceIo for FakeDevice {
            type Service = FakeService;
            type Characteristic = FakeCharacteristic;

            fn connected<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
                let result = self.state.lock().unwrap().connected.clone();
                Box::pin(async move { result })
            }

            fn services_resolved<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
                let result = self.state.lock().unwrap().services_resolved.clone();
                Box::pin(async move { result })
            }

            fn connect<'a>(&'a self) -> BackendFuture<'a, Result<(), OwnerError>> {
                if self.state.lock().unwrap().connect_pending {
                    return Box::pin(async {
                        futures::future::pending::<Result<(), OwnerError>>().await
                    });
                }
                let result = self.state.lock().unwrap().connect.clone();
                Box::pin(async move { result })
            }

            fn disconnect<'a>(&'a self) -> BackendFuture<'a, Result<(), OwnerError>> {
                let result = self.state.lock().unwrap().disconnect.clone();
                Box::pin(async move { result })
            }

            fn services<'a>(&'a self) -> BackendFuture<'a, Result<Vec<Self::Service>, OwnerError>> {
                let result = self.state.lock().unwrap().services.clone();
                Box::pin(async move { result })
            }
        }

        impl AdapterIo for FakeAdapter {
            type Device = FakeDevice;
            type Characteristic = FakeCharacteristic;

            fn powered<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
                let result = self.state.lock().unwrap().powered.clone();
                Box::pin(async move { result })
            }

            fn discovering<'a>(&'a self) -> BackendFuture<'a, Result<bool, OwnerError>> {
                let result = self.state.lock().unwrap().discovering.clone();
                Box::pin(async move { result })
            }

            fn discovery_filter<'a>(&'a self) -> BackendFuture<'a, bluer::DiscoveryFilter> {
                Box::pin(async { bluer::DiscoveryFilter::default() })
            }

            fn set_discovery_filter<'a>(
                &'a self,
                _filter: bluer::DiscoveryFilter,
            ) -> BackendFuture<'a, Result<(), OwnerError>> {
                let mut state = self.state.lock().unwrap();
                state.set_filter_calls = state.set_filter_calls.saturating_add(1);
                let result = if state.fail_set_filter_call == Some(state.set_filter_calls) {
                    Err(backend_error("set discovery filter"))
                } else {
                    state.set_filter.clone()
                };
                Box::pin(async move { result })
            }

            fn discover_devices<'a>(
                &'a self,
            ) -> BackendFuture<'a, Result<BackendStream<bluer::AdapterEvent>, OwnerError>>
            {
                let state = self.state.lock().unwrap();
                if state.discovery_pending {
                    return Box::pin(async {
                        Ok(Box::pin(futures::stream::pending()) as BackendStream<_>)
                    })
                        as BackendFuture<
                            'a,
                            Result<BackendStream<bluer::AdapterEvent>, OwnerError>,
                        >;
                }
                let result = state.discovery.clone();
                Box::pin(async move {
                    result.map(|events| Box::pin(futures::stream::iter(events)) as BackendStream<_>)
                })
            }

            fn device(&self, _address: Address) -> Result<Self::Device, OwnerError> {
                self.state.lock().unwrap().device.clone()
            }

            fn set_powered<'a>(
                &'a self,
                _powered: bool,
            ) -> BackendFuture<'a, Result<(), OwnerError>> {
                let result = self.state.lock().unwrap().set_powered.clone();
                Box::pin(async move { result })
            }
        }

        fn backend_error(operation: &'static str) -> OwnerError {
            OwnerError::Bluez {
                class: BluezFailureClass::DaemonUnavailable,
                operation,
            }
        }

        fn fake_owner(adapter: FakeAdapter) -> BluezAdapterOwner<FakeAdapter> {
            BluezAdapterOwner::from_backend(
                adapter,
                ResolvedController {
                    selector: ControllerSelector::default(),
                    runtime_name: "fake".to_owned(),
                },
                Generation::new(1),
            )
        }

        fn fake_lease(
            owner: &mut BluezAdapterOwner<FakeAdapter>,
            source: crate::lease::Source,
            intent: crate::lease::Intent,
        ) -> Lease {
            match owner.acquire(
                LeaseRequest::new(
                    source,
                    intent,
                    crate::Priority::new(1),
                    100,
                    100,
                    None,
                    crate::CapabilitySet::empty(),
                ),
                0,
            ) {
                Ok(AcquireResult::Granted(lease)) => lease,
                other => panic!("expected fake lease, got {other:?}"),
            }
        }

        fn fake_characteristic(uuid: bluer::Uuid) -> FakeCharacteristic {
            FakeCharacteristic {
                state: Arc::new(std::sync::Mutex::new(FakeCharacteristicState {
                    uuid,
                    notify: Ok(vec![vec![1, 2, 3]]),
                    write: Ok(()),
                })),
            }
        }

        fn fake_adapter() -> (FakeAdapter, FakeDevice, FakeCharacteristic) {
            let characteristic = fake_characteristic(bluer::Uuid::nil());
            let service = FakeService {
                state: Arc::new(std::sync::Mutex::new(FakeServiceState {
                    uuid: Ok(bluer::Uuid::nil()),
                    characteristics: Ok(vec![characteristic.clone()]),
                })),
            };
            let device = FakeDevice {
                state: Arc::new(std::sync::Mutex::new(FakeDeviceState {
                    connected: Ok(true),
                    services_resolved: Ok(true),
                    connect: Ok(()),
                    connect_pending: false,
                    disconnect: Ok(()),
                    services: Ok(vec![service]),
                })),
            };
            let adapter = FakeAdapter {
                state: Arc::new(std::sync::Mutex::new(FakeAdapterState {
                    powered: Ok(true),
                    discovering: Ok(false),
                    set_powered: Ok(()),
                    set_filter: Ok(()),
                    set_filter_calls: 0,
                    fail_set_filter_call: None,
                    discovery: Ok(vec![bluer::AdapterEvent::DeviceAdded(Address::new([
                        1, 2, 3, 4, 5, 6,
                    ]))]),
                    discovery_pending: false,
                    device: Ok(device.clone()),
                })),
            };
            (adapter, device, characteristic)
        }

        #[tokio::test]
        async fn fake_backend_runs_successful_owner_operations() {
            let (adapter, _device, _characteristic) = fake_adapter();
            let mut owner = fake_owner(adapter);
            crate::server::exercise_linux_owner_dispatch(&mut owner).await;
            assert!(owner.health().await.unwrap().powered);

            let power = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::PowerRecovery,
            );
            owner.set_powered(&power, true).await.unwrap();
            owner.release(power.id(), 1);

            let discover = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            let report = owner
                .discover(
                    &discover,
                    bluer::DiscoveryFilter::default(),
                    Some(Address::new([1, 2, 3, 4, 5, 6])),
                    Duration::from_millis(10),
                )
                .await
                .unwrap();
            assert!(report.target_found);
            owner.release(discover.id(), 2);

            let observe = fake_lease(
                &mut owner,
                crate::lease::Source::Bp,
                crate::lease::Intent::KernelObserve,
            );
            assert_eq!(
                owner
                    .observe(&observe, Address::new([1, 2, 3, 4, 5, 6]))
                    .await
                    .unwrap(),
                DeviceObservation {
                    connected: true,
                    services_resolved: true,
                }
            );
            owner.release(observe.id(), 3);

            let connect = fake_lease(
                &mut owner,
                crate::lease::Source::AirSense,
                crate::lease::Intent::Connect,
            );
            owner
                .connect(
                    &connect,
                    Address::new([1, 2, 3, 4, 5, 6]),
                    Duration::from_millis(10),
                )
                .await
                .unwrap();
            owner
                .disconnect(&connect, Address::new([1, 2, 3, 4, 5, 6]))
                .await
                .unwrap();
            owner.release(connect.id(), 4);

            let gatt = fake_lease(
                &mut owner,
                crate::lease::Source::AirSense,
                crate::lease::Intent::Gatt,
            );
            let characteristic = owner
                .resolve(
                    &gatt,
                    Address::new([1, 2, 3, 4, 5, 6]),
                    bluer::Uuid::nil(),
                    bluer::Uuid::nil(),
                )
                .await
                .unwrap();
            assert_eq!(
                owner
                    .subscribe(&gatt, &characteristic)
                    .await
                    .unwrap()
                    .collect::<Vec<_>>()
                    .await,
                vec![vec![1, 2, 3]]
            );
            owner
                .write(
                    &gatt,
                    Address::new([1, 2, 3, 4, 5, 6]),
                    &characteristic,
                    &[9, 8, 7],
                )
                .await
                .unwrap();
        }

        #[tokio::test]
        async fn fake_backend_exercises_discovery_failures_and_deadlines() {
            let (adapter, _device, _characteristic) = fake_adapter();
            adapter.state.lock().unwrap().discovering = Ok(true);
            let mut owner = fake_owner(adapter);
            let lease = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            assert!(matches!(
                owner
                    .discover(
                        &lease,
                        bluer::DiscoveryFilter::default(),
                        None,
                        Duration::from_millis(1),
                    )
                    .await,
                Err(OwnerError::Bluez {
                    class: BluezFailureClass::DiscoveryBusy,
                    ..
                })
            ));

            let (adapter, _device, _characteristic) = fake_adapter();
            adapter.state.lock().unwrap().set_filter = Err(backend_error("set discovery filter"));
            let mut owner = fake_owner(adapter);
            let lease = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            assert!(
                owner
                    .discover(
                        &lease,
                        bluer::DiscoveryFilter::default(),
                        None,
                        Duration::from_millis(1)
                    )
                    .await
                    .is_err()
            );

            let (adapter, _device, _characteristic) = fake_adapter();
            adapter.state.lock().unwrap().discovery = Err(backend_error("start discovery"));
            let mut owner = fake_owner(adapter);
            let lease = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            assert!(matches!(
                owner
                    .discover(
                        &lease,
                        bluer::DiscoveryFilter::default(),
                        None,
                        Duration::from_millis(1)
                    )
                    .await,
                Err(OwnerError::Bluez { .. })
            ));

            let (adapter, _device, _characteristic) = fake_adapter();
            {
                let mut state = adapter.state.lock().unwrap();
                state.discovery = Err(backend_error("start discovery"));
                state.fail_set_filter_call = Some(2);
            }
            let mut owner = fake_owner(adapter);
            let lease = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            assert_eq!(
                owner
                    .discover(
                        &lease,
                        bluer::DiscoveryFilter::default(),
                        None,
                        Duration::from_millis(1)
                    )
                    .await,
                Err(OwnerError::FilterRestoreFailed)
            );

            let (adapter, _device, _characteristic) = fake_adapter();
            adapter.state.lock().unwrap().discovery_pending = true;
            let mut owner = fake_owner(adapter);
            let lease = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            let report = owner
                .discover(
                    &lease,
                    bluer::DiscoveryFilter::default(),
                    None,
                    Duration::from_millis(1),
                )
                .await
                .unwrap();
            assert_eq!((report.events, report.target_found), (0, false));

            let (adapter, _device, _characteristic) = fake_adapter();
            {
                let mut state = adapter.state.lock().unwrap();
                state.discovery_pending = true;
                state.fail_set_filter_call = Some(2);
            }
            let mut owner = fake_owner(adapter);
            let lease = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            assert_eq!(
                owner
                    .discover(
                        &lease,
                        bluer::DiscoveryFilter::default(),
                        None,
                        Duration::from_millis(1),
                    )
                    .await,
                Err(OwnerError::FilterRestoreFailed)
            );
        }

        #[tokio::test]
        async fn fake_backend_exercises_device_and_gatt_failures() {
            let (adapter, _device, _characteristic) = fake_adapter();
            adapter.state.lock().unwrap().device = Err(backend_error("materialize device"));
            let mut owner = fake_owner(adapter);
            let observe = fake_lease(
                &mut owner,
                crate::lease::Source::Bp,
                crate::lease::Intent::KernelObserve,
            );
            assert!(
                owner
                    .observe(&observe, Address::new([1, 2, 3, 4, 5, 6]))
                    .await
                    .is_err()
            );

            let (adapter, device, _characteristic) = fake_adapter();
            device.state.lock().unwrap().connected = Err(backend_error("read Connected"));
            let mut owner = fake_owner(adapter);
            let observe = fake_lease(
                &mut owner,
                crate::lease::Source::Bp,
                crate::lease::Intent::KernelObserve,
            );
            assert!(
                owner
                    .observe(&observe, Address::new([1, 2, 3, 4, 5, 6]))
                    .await
                    .is_err()
            );

            let (adapter, _device, _characteristic) = fake_adapter();
            let mut owner = fake_owner(adapter);
            assert_eq!(
                owner.acquire(
                    LeaseRequest::new(
                        crate::lease::Source::Bp,
                        crate::lease::Intent::Connect,
                        crate::Priority::new(1),
                        100,
                        100,
                        None,
                        crate::CapabilitySet::empty(),
                    ),
                    0,
                ),
                Err(PolicyError::BpConnectForbidden)
            );

            let (adapter, device, _characteristic) = fake_adapter();
            device.state.lock().unwrap().connect = Err(backend_error("connect device"));
            let mut owner = fake_owner(adapter);
            let connect = fake_lease(
                &mut owner,
                crate::lease::Source::AirSense,
                crate::lease::Intent::Connect,
            );
            assert!(
                owner
                    .connect(
                        &connect,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        Duration::from_millis(1)
                    )
                    .await
                    .is_err()
            );

            let (adapter, device, _characteristic) = fake_adapter();
            device.state.lock().unwrap().connect_pending = true;
            let mut owner = fake_owner(adapter);
            let connect = fake_lease(
                &mut owner,
                crate::lease::Source::AirSense,
                crate::lease::Intent::Connect,
            );
            assert_eq!(
                owner
                    .connect(
                        &connect,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        Duration::from_millis(1)
                    )
                    .await,
                Err(OwnerError::DeadlineExceeded {
                    operation: "connect device"
                })
            );

            let (adapter, device, _characteristic) = fake_adapter();
            device.state.lock().unwrap().disconnect = Err(backend_error("disconnect device"));
            let mut owner = fake_owner(adapter);
            let connect = fake_lease(
                &mut owner,
                crate::lease::Source::AirSense,
                crate::lease::Intent::Connect,
            );
            assert!(
                owner
                    .disconnect(&connect, Address::new([1, 2, 3, 4, 5, 6]))
                    .await
                    .is_err()
            );

            let (adapter, device, characteristic) = fake_adapter();
            device.state.lock().unwrap().services = Err(backend_error("resolve services"));
            let mut owner = fake_owner(adapter);
            let gatt = fake_lease(
                &mut owner,
                crate::lease::Source::AirSense,
                crate::lease::Intent::Gatt,
            );
            assert!(
                owner
                    .resolve(
                        &gatt,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await
                    .is_err()
            );

            device.state.lock().unwrap().services = Ok(vec![]);
            assert!(matches!(
                owner
                    .resolve(
                        &gatt,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await,
                Err(OwnerError::Bluez {
                    class: BluezFailureClass::GattRejected,
                    ..
                })
            ));
            device.state.lock().unwrap().services = Ok(vec![FakeService {
                state: Arc::new(std::sync::Mutex::new(FakeServiceState {
                    uuid: Ok(bluer::Uuid::from_u128(1)),
                    characteristics: Ok(vec![characteristic.clone()]),
                })),
            }]);
            assert!(
                owner
                    .resolve(
                        &gatt,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await
                    .is_err()
            );
            device.state.lock().unwrap().services = Ok(vec![FakeService {
                state: Arc::new(std::sync::Mutex::new(FakeServiceState {
                    uuid: Ok(bluer::Uuid::nil()),
                    characteristics: Ok(vec![fake_characteristic(bluer::Uuid::from_u128(1))]),
                })),
            }]);
            assert!(
                owner
                    .resolve(
                        &gatt,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await
                    .is_err()
            );
            device.state.lock().unwrap().services = Ok(vec![FakeService {
                state: Arc::new(std::sync::Mutex::new(FakeServiceState {
                    uuid: Err(backend_error("read service UUID")),
                    characteristics: Ok(vec![characteristic.clone()]),
                })),
            }]);
            assert!(
                owner
                    .resolve(
                        &gatt,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await
                    .is_err()
            );
            characteristic.state.lock().unwrap().notify =
                Err(backend_error("subscribe characteristic"));
            assert!(owner.subscribe(&gatt, &characteristic).await.is_err());
            characteristic.state.lock().unwrap().write = Err(backend_error("write characteristic"));
            assert!(
                owner
                    .write(
                        &gatt,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        &characteristic,
                        &[1]
                    )
                    .await
                    .is_err()
            );

            let (adapter, _device, _characteristic) = fake_adapter();
            let mut owner = fake_owner(adapter);
            let observe = fake_lease(
                &mut owner,
                crate::lease::Source::Bp,
                crate::lease::Intent::KernelObserve,
            );
            assert!(
                owner
                    .resolve(
                        &observe,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await
                    .is_ok()
            );
        }

        #[tokio::test]
        async fn fake_backend_exercises_scheduler_helpers() {
            let (adapter, _device, _characteristic) = fake_adapter();
            let mut owner = fake_owner(adapter);
            assert_eq!(owner.controller().runtime_name, "fake");
            let active = fake_lease(
                &mut owner,
                crate::lease::Source::Diagnostics,
                crate::lease::Intent::Discover,
            );
            let queued = owner
                .acquire(
                    LeaseRequest::new(
                        crate::lease::Source::Diagnostics,
                        crate::lease::Intent::Discover,
                        crate::Priority::new(1),
                        100,
                        100,
                        None,
                        crate::CapabilitySet::empty(),
                    ),
                    0,
                )
                .unwrap();
            if let AcquireResult::Queued { ticket, .. } = queued {
                assert!(owner.cancel(ticket));
            }
            owner.release(active.id(), 1);
            assert!(owner.invalidate_generation(Generation::new(2)).is_ok());
            assert_eq!(owner.generation(), Generation::new(2));
        }

        #[test]
        fn classify_bluer_errors_preserves_safety_categories() {
            for kind in [bluer::ErrorKind::NotFound, bluer::ErrorKind::DoesNotExist] {
                assert_eq!(
                    classify_bluer_error(&error(kind)),
                    BluezFailureClass::DeviceMissing
                );
            }
            assert_eq!(
                classify_bluer_error(&error(bluer::ErrorKind::DiscoveryActive)),
                BluezFailureClass::DiscoveryBusy
            );
            assert_eq!(
                classify_bluer_error(&error(bluer::ErrorKind::ServicesUnresolved)),
                BluezFailureClass::ServicesUnresolved
            );
            for kind in [
                bluer::ErrorKind::ConnectionAttemptFailed,
                bluer::ErrorKind::AuthenticationCanceled,
                bluer::ErrorKind::AuthenticationFailed,
                bluer::ErrorKind::AuthenticationRejected,
                bluer::ErrorKind::AuthenticationTimeout,
                bluer::ErrorKind::InProgress,
                bluer::ErrorKind::NotReady,
            ] {
                assert_eq!(
                    classify_bluer_error(&error(kind)),
                    BluezFailureClass::ConnectRejected
                );
            }
            assert_eq!(
                classify_bluer_error(&error(bluer::ErrorKind::Internal(
                    bluer::InternalErrorKind::DBusConnectionLost,
                ))),
                BluezFailureClass::ControllerLost
            );
            for kind in [
                bluer::ErrorKind::NotAvailable,
                bluer::ErrorKind::Internal(bluer::InternalErrorKind::InvalidValue),
            ] {
                assert_eq!(
                    classify_bluer_error(&error(kind)),
                    BluezFailureClass::DaemonUnavailable
                );
            }
            for kind in [
                bluer::ErrorKind::NotPermitted,
                bluer::ErrorKind::NotAuthorized,
                bluer::ErrorKind::NotSupported,
            ] {
                assert_eq!(
                    classify_bluer_error(&error(kind)),
                    BluezFailureClass::GattRejected
                );
            }
            assert_eq!(
                classify_bluer_error(&error(bluer::ErrorKind::AlreadyConnected)),
                BluezFailureClass::Unknown
            );
        }

        #[tokio::test]
        async fn owner_policy_and_lease_guards_run_against_the_selected_adapter() {
            let Ok(inventory) = std::fs::read_dir("/sys/class/bluetooth") else {
                return;
            };
            let Some(name) = inventory
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .find(|name| name.starts_with("hci") && !name.contains(':'))
            else {
                return;
            };
            let address =
                match std::fs::read_to_string(format!("/sys/class/bluetooth/{name}/address")) {
                    Ok(address) => address.trim().to_owned(),
                    Err(_) => return,
                };
            let evidence = ControllerEvidence {
                runtime_name: name,
                address: Some(address.clone()),
                sysfs_path: None,
                serial: None,
                bus: ControllerBus::Other,
                vid_pid: None,
            };
            let selector = ControllerSelector {
                address: Some(address),
                ..ControllerSelector::default()
            };
            let Ok(mut owner) =
                BluezAdapterOwner::open(&selector, &[evidence], Generation::new(1)).await
            else {
                return;
            };
            assert_eq!(owner.generation(), Generation::new(1));
            assert!(!owner.controller().runtime_name.is_empty());
            let _ = owner.health().await;
            let request = LeaseRequest::new(
                crate::Source::Diagnostics,
                crate::Intent::PowerRecovery,
                crate::Priority::new(1),
                10,
                10,
                None,
                crate::CapabilitySet::empty(),
            );
            let Ok(AcquireResult::Granted(lease)) = owner.acquire(request, 0) else {
                return;
            };
            let stale = lease;
            assert!(owner.release(stale.id(), 1).granted.is_empty());
            assert!(matches!(
                owner.set_powered(&stale, true).await,
                Err(OwnerError::LeaseNotOwned { .. })
            ));
            assert!(matches!(
                owner
                    .observe(&stale, Address::new([1, 2, 3, 4, 5, 6]))
                    .await,
                Err(OwnerError::LeaseNotOwned { .. })
            ));
            assert!(matches!(
                owner
                    .connect(
                        &stale,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        Duration::from_millis(1)
                    )
                    .await,
                Err(OwnerError::LeaseNotOwned { .. })
            ));
            assert!(matches!(
                owner
                    .disconnect(&stale, Address::new([1, 2, 3, 4, 5, 6]))
                    .await,
                Err(OwnerError::LeaseNotOwned { .. })
            ));
            assert!(matches!(
                owner
                    .resolve(
                        &stale,
                        Address::new([1, 2, 3, 4, 5, 6]),
                        bluer::Uuid::nil(),
                        bluer::Uuid::nil()
                    )
                    .await,
                Err(OwnerError::LeaseNotOwned { .. })
            ));
        }

        #[tokio::test]
        async fn open_rejects_a_resolved_runtime_alias_that_is_not_present() {
            let selector = ControllerSelector {
                address: Some("controller-that-is-not-present".to_owned()),
                ..ControllerSelector::default()
            };
            let inventory = [ControllerEvidence {
                runtime_name: "alias-that-is-not-present".to_owned(),
                address: Some("controller-that-is-not-present".to_owned()),
                sysfs_path: None,
                serial: None,
                bus: ControllerBus::Other,
                vid_pid: None,
            }];
            assert!(matches!(
                BluezAdapterOwner::open(&selector, &inventory, Generation::new(1)).await,
                Err(OpenError::Bluez { .. })
            ));
        }
    }
}

#[cfg(all(feature = "linux", target_os = "linux"))]
#[doc(hidden)]
pub use runtime::{
    AdapterIo, BluezAdapterOwner, CharacteristicIo, DeviceIo, LinuxBluezAdapterOwner,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lease::LeaseId;

    fn evidence() -> ControllerEvidence {
        ControllerEvidence {
            runtime_name: "hci-test".to_owned(),
            address: Some("controller".to_owned()),
            sysfs_path: Some("usb-test".to_owned()),
            serial: Some("serial-test".to_owned()),
            bus: ControllerBus::Usb,
            vid_pid: Some((0x0a12, 0x0001)),
        }
    }

    #[test]
    fn stable_identity_accepts_each_allowed_identity_and_rejects_unstable_bus_pair() {
        for selector in [
            ControllerSelector {
                address: Some("controller".to_owned()),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                sysfs_path: Some("usb-test".to_owned()),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                serial: Some("serial-test".to_owned()),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                bus: Some(ControllerBus::Usb),
                vid_pid: Some((0x0a12, 0x0001)),
                ..ControllerSelector::default()
            },
        ] {
            assert!(selector.has_stable_identity());
        }
        assert!(
            !ControllerSelector {
                bus: Some(ControllerBus::Uart),
                vid_pid: Some((0x0a12, 0x0001)),
                ..ControllerSelector::default()
            }
            .has_stable_identity()
        );
    }

    #[test]
    fn selector_matching_checks_every_evidence_field() {
        let base = evidence();
        for selector in [
            ControllerSelector {
                address: Some("wrong".to_owned()),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                sysfs_path: Some("wrong".to_owned()),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                serial: Some("wrong".to_owned()),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                address: Some("controller".to_owned()),
                bus: Some(ControllerBus::Uart),
                vid_pid: Some((0x0a12, 0x0001)),
                ..ControllerSelector::default()
            },
            ControllerSelector {
                bus: Some(ControllerBus::Usb),
                vid_pid: Some((1, 2)),
                ..ControllerSelector::default()
            },
        ] {
            assert_eq!(
                resolve_controller(&selector, std::slice::from_ref(&base)),
                Err(ResolutionError::NoMatch { candidates: 1 })
            );
        }
    }

    #[test]
    fn resolution_and_owner_error_messages_name_blockers() {
        assert!(
            ResolutionError::EmptySelector
                .to_string()
                .contains("no stable")
        );
        assert!(
            ResolutionError::NoMatch { candidates: 2 }
                .to_string()
                .contains("2 inventory")
        );
        assert!(
            ResolutionError::Ambiguous { matches: 3 }
                .to_string()
                .contains("3 controllers")
        );
        assert!(
            OpenError::Resolution(ResolutionError::EmptySelector)
                .to_string()
                .contains("no stable")
        );
        assert!(
            OpenError::Bluez {
                class: BluezFailureClass::AdapterMissing,
                operation: "open",
            }
            .to_string()
            .contains("AdapterMissing")
        );
        let lease = LeaseId::new(7);
        for error in [
            OwnerError::LeaseNotOwned { lease },
            OwnerError::WrongIntent {
                lease,
                expected: "GATT",
            },
            OwnerError::BpConnectForbidden,
            OwnerError::Bluez {
                class: BluezFailureClass::Unknown,
                operation: "test",
            },
            OwnerError::FilterRestoreFailed,
            OwnerError::WorkerStopped,
            OwnerError::DeadlineExceeded { operation: "test" },
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn adapter_policy_advances_and_invalidates_generation() {
        let mut policy = AdapterPolicy::new(Generation::new(1));
        assert_eq!(policy.generation(), Generation::new(1));
        assert!(policy.advance(1).granted.is_empty());
        assert!(policy.invalidate_generation(Generation::new(2)).is_ok());
        assert_eq!(policy.generation(), Generation::new(2));
        assert_eq!(
            policy.invalidate_generation(Generation::new(2)),
            Err(PolicyError::GenerationNotAdvanced {
                current: Generation::new(2),
                requested: Generation::new(2),
            })
        );
    }
}
