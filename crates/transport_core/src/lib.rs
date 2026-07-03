//! Transport trait + BufferPool contract for implementing backends.

pub mod config;
pub mod error;

pub use config::{
    AffinityConfig, BatchConfig, BindConfig, HugepageSize, RecvBufConfig, RingConfig,
};
pub use error::TransportError;
