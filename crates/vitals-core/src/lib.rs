//! HLC, the PostgreSQL binary `COPY` encoder, and the append-only spool.
//!
//! Implemented by T-D3. The COPY encoder is pure and portable, so unlike the
//! SQLite store it replaces, it unit-tests on macOS as well as Linux.
#![forbid(unsafe_code)]
