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
/// Versioned Apple-client routes and generated OpenAPI contract.
pub mod client_api;
/// Secretbox envelope encryption for the legacy loopback test API.
pub mod crypto;
/// `PostgreSQL` access for attempts and domain records.
pub mod db;
/// Canonicalization and immutable metadata rules for domain records.
pub mod domain;
/// Route handlers and shared application state.
pub mod handlers;
/// HTTP response view-model types.
pub mod models;
/// Operator authentication, CSRF, sessions, throttling, and request limits.
pub mod security;
/// The deliberately-crude envelope test page (not part of the console).
pub mod test_page;
