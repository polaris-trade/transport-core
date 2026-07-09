//! Extension traits splitting orthogonal backend concerns off the core
//! `TransportCore` shape: `PoolAccess` exposes the backend's `BufferPool`;
//! `TransportBind` supplies the async constructors receivers call.

use crate::{
    config::{AffinityConfig, BatchConfig, BindConfig, RecvBufConfig, RingConfig, SendBufConfig},
    error::TransportError,
    pool::BufferPool,
    transport::TransportCore,
};

/// Exposes the backend's [`BufferPool`] so a receiver can size its reorder
/// window and read pressure. Recv acquires a slab from this pool; the yielded
/// frame owns that slab and returns it to the pool on `Drop`. Pool exhaustion
/// is the backpressure signal (see `TransportError::PoolExhausted`).
pub trait PoolAccess {
    type Pool: BufferPool;

    fn pool(&self) -> &Self::Pool;
}

pub trait TransportBind: TransportCore {
    /// Bind a datagram socket. `ring` sizes the pool, `batch` the `recvmmsg`
    /// burst depth, `affinity` pins the driver loop (and SQPOLL poller when
    /// enabled). Backends apply what they support and warn on the rest.
    fn bind_udp(
        bind: BindConfig,
        rx: RecvBufConfig,
        tx: SendBufConfig,
        ring: RingConfig,
        batch: BatchConfig,
        affinity: AffinityConfig,
    ) -> impl core::future::Future<Output = Result<Self, TransportError>> + Send
    where
        Self: Sized;

    /// Connect a stream socket. No `BatchConfig` (streams have no `recvmmsg`
    /// batch); the per-landing bound rides `RecvBufConfig::read_chunk`.
    /// `affinity` pins the driver loop.
    fn connect_tcp(
        bind: BindConfig,
        rx: RecvBufConfig,
        tx: SendBufConfig,
        ring: RingConfig,
        affinity: AffinityConfig,
    ) -> impl core::future::Future<Output = Result<Self, TransportError>> + Send
    where
        Self: Sized;
}
