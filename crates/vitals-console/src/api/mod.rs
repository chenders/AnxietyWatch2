//! Real-data read API for the console and dashboard.
//!
//! Each submodule collects one class of facts — samples from the daemon or
//! `PostgreSQL`, host throttling, display state, unit health, backups — and
//! exposes an axum handler for the corresponding `/api/*` route. The `ops`
//! module injects system primitives so every collector is testable off-host.

/// Clock synchronization status (`/api/clock`).
pub mod clock;
/// Display/compositor/browser state (`/api/display`).
pub mod display;
/// Host health: disk, thermal, throttling, network (`/api/host`).
pub mod host;
/// Injected system-command and filesystem primitives.
pub mod ops;
/// `PostgreSQL` access: latest sample and nightly aggregates.
pub mod postgres;
/// Sample query over the `vitalsd` Unix socket (`/api/query`).
pub mod query;
/// `pgbackrest` backup and restore-verify state (`/api/recovery`).
pub mod recovery;
/// systemd unit states (`/api/services`).
pub mod services;
