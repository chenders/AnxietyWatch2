//! Pure vitalsd state and health rules.
//!
//! The socket/runtime binary is intentionally separate so these safety rules
//! can be covered without treating an uninstrumented child process as proof.

pub mod state;
