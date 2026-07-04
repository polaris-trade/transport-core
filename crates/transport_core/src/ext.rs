//! Extension traits splitting orthogonal backend concerns off the core
//! `Transport` shape: `PoolAccess` exposes the backend's `BufferPool`;
//! `TransportBind` supplies the async constructors receivers call.

use crate::config::{BatchConfig, BindConfig, RecvBufConfig, RingConfig, SendBufConfig};
use crate::error::TransportError;
use crate::pool::BufferPool;
use crate::transport::Transport;

pub trait PoolAccess {
    type Pool: BufferPool;

    fn pool(&self) -> &Self::Pool;
}

pub trait TransportBind: Transport {
    fn bind_udp(
        bind: BindConfig,
        rx: RecvBufConfig,
        tx: SendBufConfig,
        ring: RingConfig,
        batch: BatchConfig,
    ) -> impl core::future::Future<Output = Result<Self, TransportError>> + Send
    where
        Self: Sized;

    fn connect_tcp(
        bind: BindConfig,
        rx: RecvBufConfig,
        tx: SendBufConfig,
        ring: RingConfig,
    ) -> impl core::future::Future<Output = Result<Self, TransportError>> + Send
    where
        Self: Sized;
}
