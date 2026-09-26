# Linux BlueZ broker acceptance gates

**Status:** portable/fake gates recorded; Linux/Pi gates remain open.
**Scope:** T-BT-7 deterministic policy evidence only. No Pi, SSH, service, firmware, overlay,
Wellue, or physical H10 operation was performed.

## Portable evidence

The broker's host-side tests cover stable controller resolution, lease serialization and expiry,
BP capability rejection, generation invalidation, raw-journal-before-sync-ack ordering, malformed
protocol frames, bounded queues, and the deterministic absent-Wellue/BP policy trace.

The absent-Wellue regression is intentionally policy-only: Wellue is disabled, therefore an absent
optional target cannot retain a discovery lease; a BP `KernelObserve` lease can be granted, and the
frame ledger must progress through:

```text
Received -> JournalSynced -> SyncAppendPending -> DurableAcknowledged
```

A timeout or crash instead remains `DurabilityUnknown`. This proves neither a physical cuff radio
nor a BlueZ event trace.

## Required commands

Run from the repository root. Exit status is the acceptance evidence.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

## Open Linux/Pi gates

These remain **unknown/not run** and cannot be substituted by the portable tests:

- stable controller identity and current `hciN` alias;
- BlueZ/D-Bus contract, adapter generation, hotplug, and cancellation behavior;
- kernel BP accept-list registration and short-advertisement delivery;
- absent-Wellue plus real BP measurement and durable spool evidence;
- notification gaps, queue delay, reconnects, and finite-window coexistence;
- manager restart, `bluetoothd` restart, reboot, and USB hotplug recovery;
- fresh physical H10 measurements and acceptance;
- Wellue enablement (remains disabled).

No process-start, stale `Connected`, fake frame, or emulator result closes a physical gate.
