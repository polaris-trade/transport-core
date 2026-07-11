//! Transport trait + BufferPool contract for implementing backends.

pub mod config;
pub mod error;
pub mod ext;
pub mod pool;
pub mod transport;

#[cfg(feature = "testing")]
pub mod testing;

// Shared recv-counter seam + gate. Backends reach observability-core through this
// one re-export so metric names and the gate path live in one crate.
#[cfg(feature = "observability")]
pub mod telemetry;
pub use config::{
    AffinityConfig, BatchConfig, BindConfig, HugepageSize, RecvBufConfig, RingConfig,
    SendBufConfig, TimestampMode,
};
pub use error::TransportError;
pub use ext::{PoolAccess, TransportBind};
#[cfg(feature = "observability")]
pub use observability_core;
pub use pool::{BufferPool, SharedPool};
pub use transport::{
    AsPayload, AsyncReady, DatagramSource, FrameBatch, MulticastInterface, RecvFrame, StreamSource,
    Timestamp, TimestampSource, TimestampedPayload, TransportCore, UdpTransport,
};
