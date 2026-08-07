//! The vitals admin console HTTP service.
//!
//! Serves the console API (`/api/health`, `/api/version`), the real-data
//! read API (`/api/query`, `/api/latest`, `/api/nightly`), and the host
//! status endpoints (`/api/services`, `/api/host`, `/api/display`,
//! `/api/recovery`, `/api/clock`). The daemon wire protocol, host
//! inspection, and `PostgreSQL` access live in the `client` and `api`
//! modules so the handlers stay thin.

/// Real-data read API: sample queries, host status, and recovery state.
pub mod api;
/// Client for the `vitalsd` Unix-socket protocol.
pub mod client;
/// Route handlers and shared application state.
pub mod handlers;
/// HTTP response view-model types.
pub mod models;
