//! Deterministic adapter and device lease policy.

use std::cmp::Ordering;
use std::fmt;

/// A caller-supplied monotonic time unit used by the policy state machine.
pub type Tick = u64;

/// A source requesting access to the broker.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Source {
    /// The A&D store-and-forward blood-pressure cuff.
    Bp,
    /// The Polar H10 stream.
    H10,
    /// The EMAY `SleepO2` stream.
    SleepO2,
    /// The `ResMed` `AirSense` transport.
    AirSense,
    /// The optional Wellue transport.
    Wellue,
    /// An explicitly operator-authorized diagnostic client.
    Diagnostics,
}

/// A requested broker operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intent {
    /// Observe a kernel-created connection without initiating one.
    KernelObserve,
    /// Own the controller discovery operation.
    Discover,
    /// Admit one new connection attempt.
    Connect,
    /// Resolve or operate on a device's GATT objects.
    Gatt,
    /// Own an established device stream.
    Stream,
    /// Perform an explicitly authorized controller recovery.
    PowerRecovery,
}

impl Intent {
    fn is_device_operation(self) -> bool {
        matches!(
            self,
            Self::KernelObserve | Self::Connect | Self::Gatt | Self::Stream
        )
    }

    fn is_global_operation(self) -> bool {
        matches!(self, Self::Discover | Self::Connect | Self::PowerRecovery)
    }
}

/// A capability that a worker may request at handshake time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    /// Observe a kernel-created device connection.
    KernelObserve,
    /// Request a bounded discovery lease.
    Discover,
    /// Initiate a device connection.
    Connect,
    /// Resolve GATT services and characteristics.
    Resolve,
    /// Subscribe to notifications or indications.
    Subscribe,
    /// Write a bounded GATT value.
    Write,
}

/// A compact set of capabilities requested by one worker.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CapabilitySet(u8);

impl CapabilitySet {
    const KERNEL_OBSERVE: u8 = 1 << 0;
    const DISCOVER: u8 = 1 << 1;
    const CONNECT: u8 = 1 << 2;
    const RESOLVE: u8 = 1 << 3;
    const SUBSCRIBE: u8 = 1 << 4;
    const WRITE: u8 = 1 << 5;

    /// Return an empty capability set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Return a set containing one capability.
    #[must_use]
    pub const fn from_capability(capability: Capability) -> Self {
        Self(0).with(capability)
    }

    /// Return a copy of this set with `capability` included.
    #[must_use]
    pub const fn with(self, capability: Capability) -> Self {
        Self(self.0 | Self::bit(capability))
    }

    /// Return whether this set contains `capability`.
    #[must_use]
    pub const fn contains(self, capability: Capability) -> bool {
        self.0 & Self::bit(capability) != 0
    }

    /// Return the stable wire representation of this set.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Construct a set from its stable wire representation.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        let known = Self::KERNEL_OBSERVE
            | Self::DISCOVER
            | Self::CONNECT
            | Self::RESOLVE
            | Self::SUBSCRIBE
            | Self::WRITE;
        if bits & !known == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    const fn bit(capability: Capability) -> u8 {
        match capability {
            Capability::KernelObserve => Self::KERNEL_OBSERVE,
            Capability::Discover => Self::DISCOVER,
            Capability::Connect => Self::CONNECT,
            Capability::Resolve => Self::RESOLVE,
            Capability::Subscribe => Self::SUBSCRIBE,
            Capability::Write => Self::WRITE,
        }
    }
}

/// An opaque configured peer identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeviceId(String);

impl DeviceId {
    /// Construct a peer identity from a broker configuration reference.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the redacted/configuration reference held by this value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A broker generation. A controller restart invalidates leases from the old generation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Generation(u64);

impl Generation {
    /// Construct a generation number.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the numeric generation.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// An opaque active lease identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LeaseId(u64);

impl LeaseId {
    /// Construct an identifier from its protocol representation.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the numeric identifier for telemetry or protocol encoding.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_wire(value: u64) -> Self {
        Self::new(value)
    }
}

/// An opaque queued-request identifier.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct QueueTicket(u64);

impl QueueTicket {
    /// Return the numeric identifier for telemetry or protocol encoding.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A scheduler priority. Larger values run before smaller values.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Priority(u8);

impl Priority {
    /// Construct a priority from a policy value.
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Return the numeric policy value.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// A request for an adapter or device lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeaseRequest {
    source: Source,
    intent: Intent,
    priority: Priority,
    deadline: Tick,
    max_hold: Tick,
    preemptible: bool,
    device: Option<DeviceId>,
    capabilities: CapabilitySet,
    reason: String,
}

impl LeaseRequest {
    /// Construct a lease request with a monotonic deadline and maximum hold time.
    #[must_use]
    pub fn new(
        source: Source,
        intent: Intent,
        priority: Priority,
        deadline: Tick,
        max_hold: Tick,
        device: Option<DeviceId>,
        capabilities: CapabilitySet,
    ) -> Self {
        Self {
            source,
            intent,
            priority,
            deadline,
            max_hold,
            preemptible: false,
            device,
            capabilities,
            reason: String::new(),
        }
    }

    /// Attach a human-readable reason to this request.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Mark this request as safe to preempt before its operation reaches a safe boundary.
    #[must_use]
    pub const fn preemptible(mut self, value: bool) -> Self {
        self.preemptible = value;
        self
    }

    /// Return the request source.
    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
    }

    /// Return the requested operation.
    #[must_use]
    pub const fn intent(&self) -> Intent {
        self.intent
    }

    /// Return the request priority.
    #[must_use]
    pub const fn priority(&self) -> Priority {
        self.priority
    }

    /// Return the monotonic deadline.
    #[must_use]
    pub const fn deadline(&self) -> Tick {
        self.deadline
    }

    /// Return the maximum duration of a granted lease.
    #[must_use]
    pub const fn max_hold(&self) -> Tick {
        self.max_hold
    }

    /// Return the optional peer identity.
    #[must_use]
    pub fn device(&self) -> Option<&DeviceId> {
        self.device.as_ref()
    }

    /// Return the capabilities requested by the worker.
    #[must_use]
    pub const fn capabilities(&self) -> CapabilitySet {
        self.capabilities
    }

    /// Return whether the requested operation is safe to preempt.
    #[must_use]
    pub const fn is_preemptible(&self) -> bool {
        self.preemptible
    }

    /// Return the human-readable reason attached to the request.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// A granted lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lease {
    id: LeaseId,
    source: Source,
    intent: Intent,
    priority: Priority,
    generation: Generation,
    acquired_at: Tick,
    expires_at: Tick,
    preemptible: bool,
    device: Option<DeviceId>,
    reason: String,
}

impl Lease {
    /// Return the lease identifier.
    #[must_use]
    pub const fn id(&self) -> LeaseId {
        self.id
    }

    /// Return the source that owns the lease.
    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
    }

    /// Return the operation protected by the lease.
    #[must_use]
    pub const fn intent(&self) -> Intent {
        self.intent
    }

    /// Return the scheduler priority.
    #[must_use]
    pub const fn priority(&self) -> Priority {
        self.priority
    }

    /// Return the broker generation at acquisition.
    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Return when the lease was acquired.
    #[must_use]
    pub const fn acquired_at(&self) -> Tick {
        self.acquired_at
    }

    /// Return the lease expiry tick.
    #[must_use]
    pub const fn expires_at(&self) -> Tick {
        self.expires_at
    }

    /// Return the optional peer identity.
    #[must_use]
    pub fn device(&self) -> Option<&DeviceId> {
        self.device.as_ref()
    }

    /// Return the operation reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Return whether this lease may be preempted at a safe boundary.
    #[must_use]
    pub const fn is_preemptible(&self) -> bool {
        self.preemptible
    }
}

/// A named blocker returned when a request cannot start immediately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NamedBlocker {
    /// Another request owns the singleton discovery operation.
    DiscoveryOwned {
        /// The current discovery owner.
        owner: LeaseId,
        /// The current broker generation.
        generation: Generation,
        /// When the owner expires.
        deadline: Tick,
    },
    /// Another request owns the bounded new-connect admission slot.
    ConnectAdmissionOwned {
        /// The current connect owner.
        owner: LeaseId,
        /// The current broker generation.
        generation: Generation,
        /// When the owner expires.
        deadline: Tick,
    },
    /// Another operation owns the requested peer.
    DeviceOwned {
        /// The current device owner.
        owner: LeaseId,
        /// The current broker generation.
        generation: Generation,
        /// When the owner expires.
        deadline: Tick,
    },
    /// A controller recovery lease excludes all normal operations.
    RecoveryOwned {
        /// The recovery owner.
        owner: LeaseId,
        /// The current broker generation.
        generation: Generation,
        /// When recovery expires.
        deadline: Tick,
    },
}

impl fmt::Display for NamedBlocker {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiscoveryOwned {
                owner,
                generation,
                deadline,
            } => write!(
                formatter,
                "discovery lease denied: owner={}, generation={}, deadline={}",
                owner.value(),
                generation.value(),
                deadline
            ),
            Self::ConnectAdmissionOwned {
                owner,
                generation,
                deadline,
            } => write!(
                formatter,
                "connect admission denied: owner={}, generation={}, deadline={}",
                owner.value(),
                generation.value(),
                deadline
            ),
            Self::DeviceOwned {
                owner,
                generation,
                deadline,
            } => write!(
                formatter,
                "device lease denied: owner={}, generation={}, deadline={}",
                owner.value(),
                generation.value(),
                deadline
            ),
            Self::RecoveryOwned {
                owner,
                generation,
                deadline,
            } => write!(
                formatter,
                "recovery lease denied: owner={}, generation={}, deadline={}",
                owner.value(),
                generation.value(),
                deadline
            ),
        }
    }
}

/// A policy error that prevents a request from entering the queue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    /// A request's deadline is not after the current time.
    DeadlineExpired {
        /// The current policy tick.
        now: Tick,
        /// The rejected deadline.
        deadline: Tick,
    },
    /// A lease cannot have zero hold time.
    ZeroHold,
    /// The BP worker attempted to request userspace connection authority.
    BpConnectForbidden,
    /// Wellue is disabled by policy.
    WellueDisabled,
    /// Controller power recovery is reserved for an explicit diagnostic owner.
    RecoveryNotAuthorized {
        /// Source that attempted recovery.
        source: Source,
    },
    /// A generation change did not advance the controller generation.
    GenerationNotAdvanced {
        /// The current generation.
        current: Generation,
        /// The rejected generation.
        requested: Generation,
    },
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadlineExpired { now, deadline } => {
                write!(
                    formatter,
                    "lease rejected: deadline={deadline} is not after now={now}"
                )
            }
            Self::ZeroHold => formatter.write_str("lease rejected: max_hold=0"),
            Self::BpConnectForbidden => {
                formatter.write_str("lease rejected: BP has no userspace connect capability")
            }
            Self::WellueDisabled => formatter.write_str("lease rejected: Wellue is disabled"),
            Self::RecoveryNotAuthorized { source } => write!(
                formatter,
                "lease rejected: power recovery source={source:?} is not Diagnostics",
            ),
            Self::GenerationNotAdvanced { current, requested } => write!(
                formatter,
                "generation change rejected: current={}, requested={}",
                current.value(),
                requested.value()
            ),
        }
    }
}

/// The result of attempting to acquire a lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcquireResult {
    /// The request owns a lease immediately.
    Granted(Lease),
    /// The request is queued behind a named blocker.
    Queued {
        /// The queue ticket used to observe or preempt the request.
        ticket: QueueTicket,
        /// The active operation blocking the request.
        blocker: NamedBlocker,
    },
}

impl AcquireResult {
    /// Return the granted lease, if the request started immediately.
    #[must_use]
    pub fn lease(&self) -> Option<&Lease> {
        match self {
            Self::Granted(lease) => Some(lease),
            Self::Queued { .. } => None,
        }
    }

    /// Return the queue ticket, if the request was queued.
    #[must_use]
    pub const fn ticket(&self) -> Option<QueueTicket> {
        match self {
            Self::Granted(_) => None,
            Self::Queued { ticket, .. } => Some(*ticket),
        }
    }
}

/// A request to consider preempting an active lease at a safe boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreemptionRequest {
    /// The queued request's ticket.
    pub ticket: QueueTicket,
    /// The active lease that may be preempted.
    pub active_lease: LeaseId,
    /// The source currently holding the active lease.
    pub active_source: Source,
}

/// Leases granted after a release or time advancement.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReleaseOutcome {
    /// Lease identifiers that expired before dispatch.
    pub expired: Vec<LeaseId>,
    /// Leases granted from the queue.
    pub granted: Vec<Lease>,
}

/// A scheduler event produced by generation invalidation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchedulerEvent {
    /// An active lease was revoked by a controller-generation change.
    LeaseRevoked(LeaseId),
    /// A queued request was discarded by a controller-generation change.
    RequestDropped(QueueTicket),
}

#[derive(Clone, Debug)]
struct QueuedRequest {
    ticket: QueueTicket,
    request: LeaseRequest,
    queued_at: Tick,
}

/// Pure adapter/device lease scheduler.
#[derive(Clone, Debug)]
pub struct LeaseScheduler {
    generation: Generation,
    next_lease: u64,
    next_ticket: u64,
    active: Vec<Lease>,
    queued: Vec<QueuedRequest>,
    wellue_enabled: bool,
}

impl LeaseScheduler {
    /// Construct a scheduler with Wellue disabled and the supplied generation.
    #[must_use]
    pub const fn new(generation: Generation) -> Self {
        Self {
            generation,
            next_lease: 1,
            next_ticket: 1,
            active: Vec::new(),
            queued: Vec::new(),
            wellue_enabled: false,
        }
    }

    /// Enable or disable the optional Wellue policy path.
    pub const fn set_wellue_enabled(&mut self, enabled: bool) {
        self.wellue_enabled = enabled;
    }

    /// Return the current controller generation.
    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Return the currently active leases.
    #[must_use]
    pub fn active(&self) -> &[Lease] {
        &self.active
    }

    /// Return the number of queued requests.
    #[must_use]
    pub const fn queued_len(&self) -> usize {
        self.queued.len()
    }

    /// Return an active lease by identifier.
    #[must_use]
    pub fn active_lease(&self, lease_id: LeaseId) -> Option<&Lease> {
        self.active.iter().find(|lease| lease.id == lease_id)
    }

    /// Try to acquire a lease or queue it behind a named active owner.
    ///
    /// # Errors
    ///
    /// Returns a [`PolicyError`] when the request is expired, has zero hold time,
    /// violates BP's no-connect capability, or targets disabled Wellue.
    pub fn acquire(
        &mut self,
        request: LeaseRequest,
        now: Tick,
    ) -> Result<AcquireResult, PolicyError> {
        self.expire(now);
        Self::validate(&request, now, self.wellue_enabled)?;
        if let Some(blocker) = self.blocker_for(&request) {
            let ticket = QueueTicket(self.next_ticket);
            self.next_ticket = self.next_ticket.saturating_add(1);
            self.queued.push(QueuedRequest {
                ticket,
                request,
                queued_at: now,
            });
            return Ok(AcquireResult::Queued { ticket, blocker });
        }
        Ok(AcquireResult::Granted(self.grant(request, now)))
    }

    /// Cancel one queued request without touching active leases.
    pub fn cancel(&mut self, ticket: QueueTicket) -> bool {
        let before = self.queued.len();
        self.queued.retain(|request| request.ticket != ticket);
        self.queued.len() != before
    }

    /// Release a lease and grant the highest-priority queued requests that now fit.
    pub fn release(&mut self, lease_id: LeaseId, now: Tick) -> ReleaseOutcome {
        let mut outcome = ReleaseOutcome::default();
        self.expire_into(now, &mut outcome.expired);
        if let Some(index) = self.active.iter().position(|lease| lease.id == lease_id) {
            self.active.remove(index);
        }
        self.dispatch(now, &mut outcome.granted);
        outcome
    }

    /// Expire leases and dispatch queued requests at `now`.
    pub fn advance(&mut self, now: Tick) -> ReleaseOutcome {
        let mut outcome = ReleaseOutcome::default();
        self.expire_into(now, &mut outcome.expired);
        self.dispatch(now, &mut outcome.granted);
        outcome
    }

    /// Return a safe-boundary preemption candidate for a queued request.
    #[must_use]
    pub fn preemption_for(
        &self,
        ticket: QueueTicket,
        requester_priority: Priority,
    ) -> Option<PreemptionRequest> {
        let queued = self.queued.iter().find(|entry| entry.ticket == ticket)?;
        if queued.request.priority != requester_priority {
            return None;
        }
        self.active
            .iter()
            .filter(|lease| lease.preemptible && lease.priority < requester_priority)
            .min_by_key(|lease| lease.priority)
            .map(|lease| PreemptionRequest {
                ticket,
                active_lease: lease.id,
                active_source: lease.source,
            })
    }

    /// Invalidate active and queued work when the controller generation changes.
    ///
    /// # Errors
    ///
    /// Returns an error when `generation` is not greater than the current
    /// generation, because stale controller events must fail closed.
    pub fn invalidate_generation(
        &mut self,
        generation: Generation,
    ) -> Result<Vec<SchedulerEvent>, PolicyError> {
        if generation <= self.generation {
            return Err(PolicyError::GenerationNotAdvanced {
                current: self.generation,
                requested: generation,
            });
        }
        self.generation = generation;
        let mut events = Vec::with_capacity(self.active.len() + self.queued.len());
        for lease in self.active.drain(..) {
            events.push(SchedulerEvent::LeaseRevoked(lease.id));
        }
        for request in self.queued.drain(..) {
            events.push(SchedulerEvent::RequestDropped(request.ticket));
        }
        Ok(events)
    }

    fn validate(
        request: &LeaseRequest,
        now: Tick,
        wellue_enabled: bool,
    ) -> Result<(), PolicyError> {
        if request.deadline <= now {
            return Err(PolicyError::DeadlineExpired {
                now,
                deadline: request.deadline,
            });
        }
        if request.max_hold == 0 {
            return Err(PolicyError::ZeroHold);
        }
        if request.source == Source::Bp
            && (request.intent == Intent::Connect
                || request.capabilities.contains(Capability::Connect))
        {
            return Err(PolicyError::BpConnectForbidden);
        }
        if request.source == Source::Wellue && !wellue_enabled {
            return Err(PolicyError::WellueDisabled);
        }
        if request.intent == Intent::PowerRecovery && request.source != Source::Diagnostics {
            return Err(PolicyError::RecoveryNotAuthorized {
                source: request.source,
            });
        }
        Ok(())
    }

    fn grant(&mut self, request: LeaseRequest, now: Tick) -> Lease {
        let expires_at = now.saturating_add(request.max_hold);
        let lease = Lease {
            id: LeaseId(self.next_lease),
            source: request.source,
            intent: request.intent,
            priority: request.priority,
            generation: self.generation,
            acquired_at: now,
            expires_at,
            preemptible: request.preemptible,
            device: request.device,
            reason: request.reason,
        };
        self.next_lease = self.next_lease.saturating_add(1);
        self.active.push(lease.clone());
        lease
    }

    fn blocker_for(&self, request: &LeaseRequest) -> Option<NamedBlocker> {
        self.active.iter().find_map(|lease| {
            if !Self::conflicts(lease, request) {
                return None;
            }
            let blocker = match (lease.intent, request.intent) {
                (Intent::PowerRecovery, _) | (_, Intent::PowerRecovery) => {
                    NamedBlocker::RecoveryOwned {
                        owner: lease.id,
                        generation: lease.generation,
                        deadline: lease.expires_at,
                    }
                }
                (Intent::Discover, _) | (_, Intent::Discover) => NamedBlocker::DiscoveryOwned {
                    owner: lease.id,
                    generation: lease.generation,
                    deadline: lease.expires_at,
                },
                (Intent::Connect, _) | (_, Intent::Connect) => {
                    NamedBlocker::ConnectAdmissionOwned {
                        owner: lease.id,
                        generation: lease.generation,
                        deadline: lease.expires_at,
                    }
                }
                _ => NamedBlocker::DeviceOwned {
                    owner: lease.id,
                    generation: lease.generation,
                    deadline: lease.expires_at,
                },
            };
            Some(blocker)
        })
    }

    fn conflicts(active: &Lease, request: &LeaseRequest) -> bool {
        if active.intent == Intent::PowerRecovery || request.intent == Intent::PowerRecovery {
            return true;
        }
        if active.intent.is_global_operation() && request.intent.is_global_operation() {
            if active.intent == Intent::Discover || request.intent == Intent::Discover {
                return true;
            }
            if active.intent == Intent::Connect && request.intent == Intent::Connect {
                return true;
            }
        }
        if active.intent == Intent::Discover && request.intent == Intent::Connect
            || active.intent == Intent::Connect && request.intent == Intent::Discover
        {
            return true;
        }
        matches!(
            (active.device(), request.device()),
            (Some(active_device), Some(request_device))
                if active_device == request_device
                    && active.intent.is_device_operation()
                    && request.intent.is_device_operation()
        )
    }

    fn expire(&mut self, now: Tick) {
        self.active.retain(|lease| lease.expires_at > now);
        self.queued.retain(|request| request.request.deadline > now);
    }

    fn expire_into(&mut self, now: Tick, expired: &mut Vec<LeaseId>) {
        let mut retained = Vec::with_capacity(self.active.len());
        for lease in self.active.drain(..) {
            if lease.expires_at > now {
                retained.push(lease);
            } else {
                expired.push(lease.id);
            }
        }
        self.active = retained;
        self.queued.retain(|request| request.request.deadline > now);
    }

    fn dispatch(&mut self, now: Tick, granted: &mut Vec<Lease>) {
        loop {
            let Some(index) = self.best_ready_request(now) else {
                return;
            };
            let request = self.queued.remove(index);
            granted.push(self.grant(request.request, now));
        }
    }

    fn best_ready_request(&self, now: Tick) -> Option<usize> {
        self.queued
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.request.deadline > now && self.blocker_for(&entry.request).is_none()
            })
            .max_by(|(_, left), (_, right)| {
                left.request
                    .priority
                    .cmp(&right.request.priority)
                    .then_with(|| right.queued_at.cmp(&left.queued_at))
                    .then_with(|| right.ticket.cmp(&left.ticket))
                    .then(Ordering::Equal)
            })
            .map(|(index, _)| index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(source: Source, intent: Intent, priority: u8, device: Option<&str>) -> LeaseRequest {
        LeaseRequest::new(
            source,
            intent,
            Priority::new(priority),
            100,
            20,
            device.map(DeviceId::new),
            CapabilitySet::empty(),
        )
        .with_reason("test request")
    }

    #[test]
    fn granted_lease_uses_max_hold_not_request_deadline() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let mut lease_request = request(Source::AirSense, Intent::Discover, 1, None);
        lease_request = LeaseRequest::new(
            lease_request.source,
            lease_request.intent,
            lease_request.priority,
            101,
            20,
            lease_request.device,
            lease_request.capabilities,
        );
        let result = scheduler.acquire(lease_request, 100);
        let Ok(AcquireResult::Granted(lease)) = result else {
            return;
        };
        assert_eq!(lease.expires_at(), 120);
    }

    #[test]
    fn discovery_is_singleton_and_names_owner() {
        let mut scheduler = LeaseScheduler::new(Generation::new(7));
        let first = scheduler.acquire(request(Source::AirSense, Intent::Discover, 2, None), 0);
        assert!(matches!(first, Ok(AcquireResult::Granted(_))));
        scheduler.set_wellue_enabled(true);
        let second = scheduler.acquire(request(Source::Wellue, Intent::Discover, 1, None), 1);
        assert!(matches!(&second, Ok(AcquireResult::Queued { .. })));
        let Ok(AcquireResult::Queued { blocker, .. }) = second else {
            return;
        };
        let text = blocker.to_string();
        assert!(text.contains("owner=1"));
        assert!(text.contains("generation=7"));
        assert!(text.contains("deadline=20"));
    }

    #[test]
    fn higher_priority_request_dispatches_first() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let first = scheduler.acquire(request(Source::AirSense, Intent::Discover, 1, None), 0);
        assert!(matches!(&first, Ok(AcquireResult::Granted(_))));
        let Ok(AcquireResult::Granted(lease)) = first else {
            return;
        };
        let low = scheduler.acquire(request(Source::Diagnostics, Intent::Discover, 1, None), 1);
        let high = scheduler.acquire(request(Source::Bp, Intent::KernelObserve, 9, Some("bp")), 2);
        assert!(matches!(low, Ok(AcquireResult::Queued { .. })));
        assert!(matches!(high, Ok(AcquireResult::Granted(_))));
        let outcome = scheduler.release(lease.id(), 3);
        assert_eq!(outcome.granted.len(), 1);
        assert_eq!(
            outcome.granted.first().map(Lease::source),
            Some(Source::Diagnostics)
        );
        assert_eq!(scheduler.queued_len(), 0);
    }

    #[test]
    fn queued_same_resource_uses_priority_then_fifo() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let first = scheduler.acquire(request(Source::AirSense, Intent::Discover, 1, None), 0);
        assert!(matches!(&first, Ok(AcquireResult::Granted(_))));
        let Ok(AcquireResult::Granted(lease)) = first else {
            return;
        };
        let low = scheduler.acquire(request(Source::Diagnostics, Intent::Discover, 1, None), 1);
        scheduler.set_wellue_enabled(true);
        let high = scheduler.acquire(request(Source::Wellue, Intent::Discover, 3, None), 2);
        assert!(matches!(&low, Ok(AcquireResult::Queued { .. })));
        let Ok(AcquireResult::Queued {
            ticket: low_ticket, ..
        }) = low
        else {
            return;
        };
        assert!(matches!(&high, Ok(AcquireResult::Queued { .. })));
        let Ok(AcquireResult::Queued {
            ticket: high_ticket,
            ..
        }) = high
        else {
            return;
        };
        let outcome = scheduler.release(lease.id(), 3);
        assert_eq!(outcome.granted.len(), 1);
        let Some(granted) = outcome.granted.first() else {
            return;
        };
        assert_eq!(granted.source(), Source::Wellue);
        assert_ne!(low_ticket, high_ticket);
    }

    #[test]
    fn bp_cannot_request_userspace_connect() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let result = scheduler.acquire(request(Source::Bp, Intent::Connect, 9, Some("bp")), 0);
        assert_eq!(result, Err(PolicyError::BpConnectForbidden));
        let connect_capability = request(Source::Bp, Intent::Gatt, 9, Some("bp"));
        let connect_capability = LeaseRequest::new(
            connect_capability.source,
            connect_capability.intent,
            connect_capability.priority,
            connect_capability.deadline,
            connect_capability.max_hold,
            connect_capability.device,
            CapabilitySet::from_capability(Capability::Connect),
        )
        .with_reason("test connect capability");
        assert_eq!(
            scheduler.acquire(connect_capability, 0),
            Err(PolicyError::BpConnectForbidden)
        );
    }

    #[test]
    fn power_recovery_is_reserved_for_diagnostics() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let h10_request = request(Source::H10, Intent::PowerRecovery, 9, None);
        assert_eq!(
            scheduler.acquire(h10_request, 0),
            Err(PolicyError::RecoveryNotAuthorized {
                source: Source::H10
            })
        );
        let diagnostic_request = request(Source::Diagnostics, Intent::PowerRecovery, 9, None);
        assert!(matches!(
            scheduler.acquire(diagnostic_request, 0),
            Ok(AcquireResult::Granted(_))
        ));
    }

    #[test]
    fn wellue_is_disabled_until_explicitly_enabled() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let request = request(Source::Wellue, Intent::Discover, 1, None);
        assert_eq!(
            scheduler.acquire(request.clone(), 0),
            Err(PolicyError::WellueDisabled)
        );
        scheduler.set_wellue_enabled(true);
        assert!(matches!(
            scheduler.acquire(request, 0),
            Ok(AcquireResult::Granted(_))
        ));
    }

    #[test]
    fn expiry_releases_owner_and_dispatches_queue() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let mut first_request = request(Source::AirSense, Intent::Discover, 1, None);
        first_request = LeaseRequest::new(
            first_request.source,
            first_request.intent,
            first_request.priority,
            10,
            2,
            first_request.device,
            first_request.capabilities,
        )
        .with_reason(first_request.reason);
        let first = scheduler.acquire(first_request, 0);
        assert!(matches!(&first, Ok(AcquireResult::Granted(_))));
        let Ok(AcquireResult::Granted(first_lease)) = first else {
            return;
        };
        let queued = scheduler.acquire(request(Source::Diagnostics, Intent::Discover, 2, None), 1);
        assert!(matches!(queued, Ok(AcquireResult::Queued { .. })));
        let outcome = scheduler.advance(2);
        assert_eq!(outcome.expired, vec![first_lease.id()]);
        assert_eq!(outcome.granted.len(), 1);
    }

    #[test]
    fn preemption_is_only_a_request_and_does_not_stop_active_work() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let active_request =
            request(Source::Diagnostics, Intent::Discover, 1, None).preemptible(true);
        let active = scheduler.acquire(active_request, 0);
        assert!(matches!(&active, Ok(AcquireResult::Granted(_))));
        let Ok(AcquireResult::Granted(active_lease)) = active else {
            return;
        };
        let queued = scheduler.acquire(request(Source::AirSense, Intent::Discover, 5, None), 1);
        assert!(matches!(&queued, Ok(AcquireResult::Queued { .. })));
        let Ok(AcquireResult::Queued { ticket, .. }) = queued else {
            return;
        };
        let preemption = scheduler.preemption_for(ticket, Priority::new(5));
        assert!(preemption.is_some());
        let Some(preemption) = preemption else {
            return;
        };
        assert_eq!(preemption.active_lease, active_lease.id());
        assert_eq!(scheduler.active().len(), 1);
    }

    #[test]
    fn cancelling_a_queued_request_does_not_release_active_work() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let first = scheduler.acquire(request(Source::AirSense, Intent::Discover, 1, None), 0);
        assert!(matches!(first, Ok(AcquireResult::Granted(_))));
        let queued = scheduler.acquire(request(Source::Diagnostics, Intent::Discover, 1, None), 1);
        let Ok(AcquireResult::Queued { ticket, .. }) = queued else {
            return;
        };
        assert!(scheduler.cancel(ticket));
        assert_eq!(scheduler.queued_len(), 0);
        assert_eq!(scheduler.active().len(), 1);
        assert!(!scheduler.cancel(ticket));
    }

    #[test]
    fn generation_change_revokes_active_and_queued_work() {
        let mut scheduler = LeaseScheduler::new(Generation::new(1));
        let active = scheduler.acquire(request(Source::AirSense, Intent::Discover, 1, None), 0);
        assert!(matches!(active, Ok(AcquireResult::Granted(_))));
        let queued = scheduler.acquire(request(Source::Diagnostics, Intent::Discover, 1, None), 1);
        assert!(matches!(queued, Ok(AcquireResult::Queued { .. })));
        let events = scheduler.invalidate_generation(Generation::new(2));
        assert!(events.is_ok());
        let Ok(events) = events else {
            return;
        };
        assert_eq!(events.len(), 2);
        assert!(scheduler.active().is_empty());
        assert_eq!(scheduler.queued_len(), 0);
    }
}
