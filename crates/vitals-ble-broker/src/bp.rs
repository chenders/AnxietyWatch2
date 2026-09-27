//! Pure policy seams for the BP kernel-autoconnect boundary.
//!
//! This module intentionally has no `BlueZ`, D-Bus, filesystem, or management
//! socket code. A privileged helper supplies registration results; the broker
//! and BP worker can only consume the typed evidence and durability state.

use std::fmt;

use crate::{
    bluez::{ControllerEvidence, ControllerSelector, ResolvedController, resolve_controller},
    lease::{Capability, CapabilitySet, Generation},
};

/// The address type supplied to the kernel accept-list operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressType {
    /// A public device address.
    Public,
    /// A random static or resolvable private address handled by the bond.
    Random,
}

/// A bounded BP kernel-policy registration request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrationRequest {
    /// The resolved controller and its current runtime alias.
    pub controller: ResolvedController,
    /// Opaque configured peer reference; not a D-Bus object path.
    pub peer_selector: String,
    /// Address type passed to the management boundary.
    pub address_type: AddressType,
    /// Broker generation for which this request is valid.
    pub generation: Generation,
}

/// A registration failure reported by the privileged helper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistrationFailure {
    /// The management operation returned a non-success status.
    ManagementRejected,
    /// The controller disappeared before registration completed.
    ControllerUnavailable,
    /// The helper refused the request because its policy was not satisfied.
    PolicyRejected,
}

impl fmt::Display for RegistrationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ManagementRejected => "kernel registration rejected: management result=failed",
            Self::ControllerUnavailable => {
                "kernel registration blocked: controller availability=unknown"
            }
            Self::PolicyRejected => "kernel registration blocked: helper policy=result-rejected",
        })
    }
}

/// Observable result of one accept-list registration attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistrationState {
    /// No attempt has been made for the current generation.
    Unregistered,
    /// The helper reported successful registration.
    Registered {
        /// Runtime adapter alias observed at registration time.
        runtime_name: String,
        /// Generation associated with the result.
        generation: Generation,
    },
    /// The helper reported a named failure.
    Failed {
        /// Classified failure.
        failure: RegistrationFailure,
    },
}

/// Pure BP kernel-policy boundary state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelPolicyBoundary {
    selector: ControllerSelector,
    peer_selector: String,
    address_type: AddressType,
    state: RegistrationState,
}

impl KernelPolicyBoundary {
    /// Construct an unregistered boundary for one opaque peer.
    #[must_use]
    pub fn new(
        selector: ControllerSelector,
        peer_selector: impl Into<String>,
        address_type: AddressType,
    ) -> Self {
        Self {
            selector,
            peer_selector: peer_selector.into(),
            address_type,
            state: RegistrationState::Unregistered,
        }
    }

    /// Resolve the selected controller and prepare a helper request.
    ///
    /// # Errors
    ///
    /// Returns the stable-identity resolution error instead of falling back to
    /// a default adapter or an `hciN` name.
    pub fn prepare_registration(
        &self,
        inventory: &[ControllerEvidence],
        generation: Generation,
    ) -> Result<RegistrationRequest, crate::bluez::ResolutionError> {
        let controller = resolve_controller(&self.selector, inventory)?;
        Ok(RegistrationRequest {
            controller,
            peer_selector: self.peer_selector.clone(),
            address_type: self.address_type,
            generation,
        })
    }

    /// Record the privileged helper result for the request's generation.
    pub fn record_result(
        &mut self,
        request: &RegistrationRequest,
        result: Result<(), RegistrationFailure>,
    ) {
        self.state = match result {
            Ok(()) => RegistrationState::Registered {
                runtime_name: request.controller.runtime_name.clone(),
                generation: request.generation,
            },
            Err(failure) => RegistrationState::Failed { failure },
        };
    }

    /// Prepare a fresh registration after controller hotplug or generation change.
    ///
    /// The previous result is not reused as current evidence.
    ///
    /// # Errors
    ///
    /// Returns the stable-identity resolution error if the controller is absent
    /// or ambiguous.
    pub fn prepare_hotplug(
        &mut self,
        inventory: &[ControllerEvidence],
        generation: Generation,
    ) -> Result<RegistrationRequest, crate::bluez::ResolutionError> {
        self.state = RegistrationState::Unregistered;
        self.prepare_registration(inventory, generation)
    }

    /// Return the current registration state.
    #[must_use]
    pub const fn state(&self) -> &RegistrationState {
        &self.state
    }
}

/// The only capabilities a BP worker may request.
#[must_use]
pub const fn bp_capabilities() -> CapabilitySet {
    CapabilitySet::empty()
        .with(Capability::KernelObserve)
        .with(Capability::Resolve)
        .with(Capability::Subscribe)
}

/// Reject a BP handshake or lease capability set that can initiate a connection.
///
/// # Errors
///
/// Returns [`crate::lease::PolicyError::BpConnectForbidden`] when the request
/// includes the userspace connect capability.
pub fn validate_bp_capabilities(
    capabilities: CapabilitySet,
) -> Result<(), crate::lease::PolicyError> {
    if capabilities.contains(Capability::Connect) {
        return Err(crate::lease::PolicyError::BpConnectForbidden);
    }
    Ok(())
}

/// Ordered durability state for one raw BP frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameDurability {
    /// The frame has arrived but is not yet recoverable.
    Received,
    /// The raw bytes have been journaled and synced before decode.
    JournalSynced,
    /// `append { sync: true }` was sent and awaits its reply.
    SyncAppendPending,
    /// `vitalsd` acknowledged the synced append.
    DurableAcknowledged,
    /// A crash, timeout, or journal error prevents a success claim.
    DurabilityUnknown,
}

/// A small state machine enforcing BP raw-journal and synced-append order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BpFrameLedger {
    state: FrameDurability,
}

impl BpFrameLedger {
    /// Begin tracking one received raw frame.
    #[must_use]
    pub const fn received() -> Self {
        Self {
            state: FrameDurability::Received,
        }
    }

    /// Sync the raw journal before decoding.
    pub fn journal_synced(&mut self) -> bool {
        if self.state != FrameDurability::Received {
            return false;
        }
        self.state = FrameDurability::JournalSynced;
        true
    }

    /// Mark a `sync: true` append as sent after journaling.
    pub fn sync_append_sent(&mut self) -> bool {
        if self.state != FrameDurability::JournalSynced {
            return false;
        }
        self.state = FrameDurability::SyncAppendPending;
        true
    }

    /// Accept the durable reply from `vitalsd`.
    pub fn durable_reply(&mut self) -> bool {
        if self.state != FrameDurability::SyncAppendPending {
            return false;
        }
        self.state = FrameDurability::DurableAcknowledged;
        true
    }

    /// Mark the frame unknown after a crash, timeout, or journal failure.
    pub const fn durability_unknown(&mut self) {
        self.state = FrameDurability::DurabilityUnknown;
    }

    /// Return the current state.
    #[must_use]
    pub const fn state(self) -> FrameDurability {
        self.state
    }
}
