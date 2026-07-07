//! Shared transport error type. Backends map internal failures here; protocol
//! crates wrap this via `#[from]`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("bind failed for {addr}: {reason}")]
    BindFailed { addr: String, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("buffer pool exhausted (in_use {in_use} / capacity {capacity})")]
    PoolExhausted { in_use: usize, capacity: usize },

    #[error("ring full (capacity {capacity})")]
    RingFull { capacity: usize },

    #[error("backend {name} unavailable: {reason}")]
    BackendUnavailable { name: &'static str, reason: String },

    #[error("operation not supported by {name}: {reason}")]
    Unsupported {
        name: &'static str,
        reason: &'static str,
    },
}
