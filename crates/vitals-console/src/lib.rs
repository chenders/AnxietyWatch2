//! The vitals admin console HTTP service.
//!
//! Serves the console API (`/api/health`, `/api/version`), the encrypted
//! envelope write API (`/api/envelope`, `/api/envelope/register`), attempt
//! views (`/api/attempts`), and — behind a runtime flag — a plaintext test
//! route and page. The daemon wire protocol, the cryptography, and the
//! persistence are separated into the `client`, `crypto`, and `db` modules so
//! the HTTP layer stays presentation-only.

/// Client for the `vitalsd` Unix-socket protocol (`vitals_proto`).
pub mod client;

/// XSalsa20-Poly1305 secretbox encryption for envelope payloads.
pub mod crypto;

/// `PostgreSQL` persistence: clients, domain records, and attempts.
pub mod db;

/// Route handlers and shared application state.
pub mod handlers;

/// HTTP request/response view-model types.
pub mod models;

/// Static HTML test page for the envelope API.
pub mod test_page;
