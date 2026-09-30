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
    #![allow(clippy::missing_errors_doc)]

    use super::{
        AcquireResult, AdapterHealth, AdapterPolicy, BluezFailureClass, ControllerEvidence,
        ControllerSelector, DeviceObservation, DiscoveryReport, Generation, Lease, LeaseId,
        LeaseRequest, OpenError, OwnerError, PolicyError, ReleaseOutcome, ResolvedController,
        SchedulerEvent, resolve_controller,
    };
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use bluer::{Adapter, Address, Session, gatt::remote::Characteristic};
    use futures::StreamExt;
    use tokio::sync::{Mutex, OwnedMutexGuard};

    /// A Linux `BlueZ` adapter owner. All application-level `BlueZ` operations pass
    /// through this value; workers receive only typed results from its methods.
    pub struct BluezAdapterOwner {
        _session: Session,
        adapter: Adapter,
        controller: ResolvedController,
        policy: AdapterPolicy,
        writes: Mutex<HashMap<Address, Arc<Mutex<()>>>>,
    }

    impl BluezAdapterOwner {
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
            Ok(Self {
                _session: session,
                adapter,
                controller,
                policy: AdapterPolicy::new(generation),
                writes: Mutex::new(HashMap::new()),
            })
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
            self.adapter
                .set_powered(powered)
                .await
                .map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "set adapter power",
                })
        }

        /// Read adapter health without inferring readiness from process state.
        pub async fn health(&self) -> Result<AdapterHealth, OwnerError> {
            let powered = self
                .adapter
                .is_powered()
                .await
                .map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DaemonUnavailable,
                    operation: "read Powered",
                })?;
            let discovering =
                self.adapter
                    .is_discovering()
                    .await
                    .map_err(|_| OwnerError::Bluez {
                        class: BluezFailureClass::DaemonUnavailable,
                        operation: "read Discovering",
                    })?;
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
                if adapter
                    .is_discovering()
                    .await
                    .map_err(|_| OwnerError::Bluez {
                        class: BluezFailureClass::DaemonUnavailable,
                        operation: "read Discovering",
                    })?
                {
                    return Err(OwnerError::Bluez {
                        class: BluezFailureClass::DiscoveryBusy,
                        operation: "start discovery",
                    });
                }
                let previous = adapter.discovery_filter().await;
                adapter
                    .set_discovery_filter(filter)
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "set discovery filter",
                    })?;
                let mut events = match adapter.discover_devices().await {
                    Ok(events) => events,
                    Err(error) => {
                        let restored = adapter.set_discovery_filter(previous).await.is_ok();
                        return if restored {
                            Err(OwnerError::Bluez {
                                class: classify_bluer_error(&error),
                                operation: "start discovery",
                            })
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
            let device = self
                .adapter
                .device(address)
                .map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DeviceMissing,
                    operation: "materialize device",
                })?;
            let connected = device
                .is_connected()
                .await
                .map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "read Connected",
                })?;
            let services_resolved =
                device
                    .is_services_resolved()
                    .await
                    .map_err(|error| OwnerError::Bluez {
                        class: classify_bluer_error(&error),
                        operation: "read ServicesResolved",
                    })?;
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
            let device = self
                .adapter
                .device(address)
                .map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DeviceMissing,
                    operation: "materialize device",
                })?;
            if let Ok(result) =
                tokio::time::timeout(duration, bluer::Device::connect(&device)).await
            {
                result.map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "connect device",
                })
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
            let device = self
                .adapter
                .device(address)
                .map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DeviceMissing,
                    operation: "materialize device",
                })?;
            device
                .disconnect()
                .await
                .map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "disconnect device",
                })
        }

        /// Resolve one typed remote characteristic by service and characteristic UUID.
        pub async fn resolve(
            &self,
            lease: &Lease,
            address: Address,
            service_uuid: bluer::Uuid,
            characteristic_uuid: bluer::Uuid,
        ) -> Result<Characteristic, OwnerError> {
            self.check_gatt_lease(lease)?;
            let device = self
                .adapter
                .device(address)
                .map_err(|_| OwnerError::Bluez {
                    class: BluezFailureClass::DeviceMissing,
                    operation: "materialize device",
                })?;
            let services = device.services().await.map_err(|error| OwnerError::Bluez {
                class: classify_bluer_error(&error),
                operation: "resolve services",
            })?;
            for service in services {
                if service.uuid().await.map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "read service UUID",
                })? != service_uuid
                {
                    continue;
                }
                let characteristics =
                    service
                        .characteristics()
                        .await
                        .map_err(|error| OwnerError::Bluez {
                            class: classify_bluer_error(&error),
                            operation: "resolve characteristics",
                        })?;
                for characteristic in characteristics {
                    if characteristic
                        .uuid()
                        .await
                        .map_err(|error| OwnerError::Bluez {
                            class: classify_bluer_error(&error),
                            operation: "read characteristic UUID",
                        })?
                        == characteristic_uuid
                    {
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
            characteristic: &Characteristic,
        ) -> Result<impl futures::Stream<Item = Vec<u8>> + use<>, OwnerError> {
            self.check_gatt_lease(lease)?;
            characteristic
                .notify()
                .await
                .map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "subscribe characteristic",
                })
        }

        /// Serialize writes to one peer while allowing different peers to proceed independently.
        pub async fn write(
            &self,
            lease: &Lease,
            address: Address,
            characteristic: &Characteristic,
            payload: &[u8],
        ) -> Result<(), OwnerError> {
            self.check_lease(lease, crate::lease::Intent::Gatt)?;
            let guard = self.write_guard(address).await;
            let result = characteristic
                .write(payload)
                .await
                .map_err(|error| OwnerError::Bluez {
                    class: classify_bluer_error(&error),
                    operation: "write characteristic",
                });
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

    pub use BluezAdapterOwner as LinuxBluezAdapterOwner;
}

#[cfg(all(feature = "linux", target_os = "linux"))]
pub use runtime::LinuxBluezAdapterOwner;

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
