//! Shared value types, the `PostgreSQL` binary `COPY` encoder, and the append-only spool.
//!
//! Implemented by T-D3. The `COPY` encoder is pure and portable — no database
//! handle, no I/O — so it unit-tests on macOS as well as on the Pi.
#![forbid(unsafe_code)]
