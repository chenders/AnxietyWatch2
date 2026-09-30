//! Deterministic coexistence and failure-boundary tests.
//!
//! These tests prove policy ordering only. They are not `BlueZ`, Pi, or physical
//! acceptance evidence.

use vitals_ble_broker::bp::{BpFrameLedger, FrameDurability, bp_capabilities};
use vitals_ble_broker::{
    AcquireResult, CapabilitySet, DeviceId, Generation, Intent, LeaseRequest, LeaseScheduler,
    Priority, Source,
};

#[test]
fn absent_wellue_cannot_hold_discovery_and_bp_can_become_durable() {
    let mut scheduler = LeaseScheduler::new(Generation::new(1));
    let wellue = LeaseRequest::new(
        Source::Wellue,
        Intent::Discover,
        Priority::new(1),
        20,
        5,
        None,
        CapabilitySet::empty(),
    )
    .with_reason("optional Wellue target absent");
    assert_eq!(
        scheduler.acquire(wellue, 0),
        Err(vitals_ble_broker::PolicyError::WellueDisabled)
    );

    let bp = LeaseRequest::new(
        Source::Bp,
        Intent::KernelObserve,
        Priority::new(10),
        100,
        20,
        Some(DeviceId::new("bp-cuff")),
        bp_capabilities(),
    )
    .with_reason("kernel-created BP connection");
    let lease = scheduler.acquire(bp, 1);
    assert!(matches!(lease, Ok(AcquireResult::Granted(_))));

    let mut ledger = BpFrameLedger::received();
    assert!(ledger.journal_synced());
    assert!(ledger.sync_append_sent());
    assert!(ledger.durable_reply());
    assert_eq!(ledger.state(), FrameDurability::DurableAcknowledged);
}

#[test]
fn worker_release_dispatches_waiting_owner_without_generation_overlap() {
    let mut scheduler = LeaseScheduler::new(Generation::new(3));
    let first = LeaseRequest::new(
        Source::H10,
        Intent::Stream,
        Priority::new(5),
        100,
        20,
        Some(DeviceId::new("h10")),
        CapabilitySet::empty(),
    );
    let Ok(AcquireResult::Granted(first_lease)) = scheduler.acquire(first, 0) else {
        return;
    };
    let second = LeaseRequest::new(
        Source::H10,
        Intent::Stream,
        Priority::new(5),
        100,
        20,
        Some(DeviceId::new("h10")),
        CapabilitySet::empty(),
    );
    assert!(matches!(
        scheduler.acquire(second, 1),
        Ok(AcquireResult::Queued { .. })
    ));

    let released = scheduler.release(first_lease.id(), 2);
    assert_eq!(released.granted.len(), 1);
    assert_eq!(scheduler.active().len(), 1);
    let Some(active) = scheduler.active().first() else {
        return;
    };
    assert_eq!(active.generation(), Generation::new(3));
}

#[test]
fn controller_generation_change_revokes_all_source_leases() {
    let mut scheduler = LeaseScheduler::new(Generation::new(7));
    let request = LeaseRequest::new(
        Source::Bp,
        Intent::KernelObserve,
        Priority::new(10),
        100,
        20,
        Some(DeviceId::new("bp-cuff")),
        bp_capabilities(),
    );
    assert!(matches!(
        scheduler.acquire(request, 0),
        Ok(AcquireResult::Granted(_))
    ));
    let events_result = scheduler.invalidate_generation(Generation::new(8));
    assert!(events_result.is_ok());
    let Ok(events) = events_result else {
        return;
    };
    assert_eq!(events.len(), 1);
    assert!(scheduler.active().is_empty());
}
