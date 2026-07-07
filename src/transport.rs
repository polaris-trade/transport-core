//! Core `Transport` trait plus the `AsPayload` shape that protocol crates
//! consume. Backends define their own per-call `Frame<'a>` borrowed from
//! `&self`; protocols stay generic over `T: Transport`.

use crate::error::TransportError;
use core::task::{Context, Poll};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub trait Transport {
    type Frame<'a>: AsPayload
    where
        Self: 'a;
    type Event;

    fn poll_event(&mut self, cx: &mut Context<'_>) -> Poll<Result<Self::Event, TransportError>>;
    fn next_frame(&self) -> Option<Self::Frame<'_>>;
    fn name(&self) -> &'static str;

    fn send(
        &mut self,
        buf: &[u8],
    ) -> impl core::future::Future<Output = Result<(), TransportError>> + Send;
}

pub trait AsPayload {
    fn payload(&self) -> &[u8];
    fn sequence(&self) -> u64;
    fn stream_id(&self) -> u8;
}

/// Frames from timestamping-capable backends expose the recv timestamp.
/// Kept separate from [`AsPayload`] so the common shape stays lean; protocol
/// code that needs timestamps bounds `T::Frame: TimestampedPayload`.
pub trait TimestampedPayload: AsPayload {
    fn timestamp(&self) -> Option<Timestamp>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp {
    pub nanos: u64,
    pub source: TimestampSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimestampSource {
    #[default]
    KernelSw,
    HardwareRx,
}

pub trait UdpTransport: Transport {
    fn join_multicast(
        &mut self,
        group: IpAddr,
        interface: MulticastInterface,
    ) -> impl core::future::Future<Output = Result<(), TransportError>> + Send;

    fn send_to(
        &mut self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> impl core::future::Future<Output = Result<(), TransportError>> + Send;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulticastInterface {
    pub v4: Option<Ipv4Addr>,
    pub v6_scope_id: Option<u32>,
}
