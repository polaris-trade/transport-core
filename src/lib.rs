//! Transport trait + BufferPool contract for implementing backends.

pub mod config;
pub mod error;
pub mod ext;
pub mod pool;
pub mod transport;

#[cfg(feature = "testing")]
pub mod testing;

pub use config::{
    AffinityConfig, BatchConfig, BindConfig, HugepageSize, RecvBufConfig, RingConfig,
    SendBufConfig, TimestampMode,
};
pub use error::TransportError;
pub use ext::{PoolAccess, TransportBind};
pub use pool::{BufferPool, SharedPool};
pub use transport::{
    AsPayload, AsyncReady, DatagramSource, FrameBatch, MulticastInterface, RecvFrame, StreamSource,
    Timestamp, TimestampSource, TimestampedPayload, TransportCore, UdpTransport,
};
