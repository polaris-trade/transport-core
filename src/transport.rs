//! Recv seam plus the `AsPayload` shape protocol crates consume. Recv yields
//! owned `RecvFrame` handles that carry their own bytes (pool slab, buffer-ring
//! id, UMEM descriptor, mbuf) and return them to their pool on `Drop`. Recv is
//! sync and batch-first so every backend shares one zero-cost-capable path;
//! `AsyncReady` is an optional readiness adapter so the sync core never carries
//! a waker. Protocols stay generic over `DatagramSource`/`StreamSource`.

use crate::error::TransportError;
use core::future::Future;
use core::mem::MaybeUninit;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Bytes-plus-metadata shape protocol code reads from a received frame.
/// Backend frames implement it directly; protocol frames re-implement it after
/// wire parsing sets `sequence` + `stream_id`.
pub trait AsPayload {
    fn payload(&self) -> &[u8];
    fn sequence(&self) -> u64;
    fn stream_id(&self) -> u8;
}

/// Owned received frame. `payload()` borrows the handle (`&self`), not the
/// transport, so a frame outlives the recv call, moves across threads, and
/// carries its own bytes for zero-copy handoff. Blanket-implemented for any
/// `AsPayload + Send + 'static`; a backend just makes its frame type own its
/// pool slab.
pub trait RecvFrame: AsPayload + Send + 'static {}

impl<T: AsPayload + Send + 'static> RecvFrame for T {}

/// Reusable burst container. Preallocate once with `with_capacity`, pass by
/// `&mut` into `recv_burst`, then `drain` the filled frames. `drain` retains the
/// backing `Vec` allocation, so steady-state burst recv adds no per-call heap
/// allocation.
pub struct FrameBatch<F> {
    frames: Vec<F>,
}

impl<F> FrameBatch<F> {
    /// Allocate a batch that holds `cap` frames before the backing `Vec` grows.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            frames: Vec::with_capacity(cap),
        }
    }

    /// Slots free before the backing `Vec` must reallocate. A backend caps its
    /// reap at `min(max, batch.spare())` to stay allocation-free.
    pub fn spare(&self) -> usize {
        self.frames.capacity() - self.frames.len()
    }

    /// Frames currently held (filled by a burst, not yet drained).
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Append a reaped frame. Backends call this from `recv_burst`.
    pub fn push(&mut self, frame: F) {
        self.frames.push(frame);
    }

    /// Drain the filled frames, keeping the backing allocation for the next
    /// burst. Each yielded `F` is owned by the caller.
    pub fn drain(&mut self) -> impl Iterator<Item = F> + '_ {
        self.frames.drain(..)
    }
}

impl<F> Default for FrameBatch<F> {
    fn default() -> Self {
        Self { frames: Vec::new() }
    }
}

/// Common base every backend implements: a stable name plus the low-rate async
/// `send`. Recv lives in the `DatagramSource`/`StreamSource` extensions. `send`
/// stays async because it is off the hot path (gap re-requests, heartbeats) and
/// never needs the sync busy-poll core.
pub trait TransportCore {
    fn name(&self) -> &'static str;

    fn send(&mut self, buf: &[u8]) -> impl Future<Output = Result<(), TransportError>> + Send;
}

/// Discrete-datagram recv.
///
/// `recv_burst` reaps up to `max` datagrams into the caller's `FrameBatch`, each
/// frame owning a pool slab (zero-copy), and returns the count reaped. `Ok(0)`
/// means nothing was ready (retry). `Err(TransportError::PoolExhausted)` means
/// backpressure: stop reaping and let the kernel drop.
///
/// # Example: an owned frame crosses a thread boundary
/// ```
/// use transport_core::{
///     AsPayload, DatagramSource, FrameBatch, TransportCore, TransportError,
/// };
///
/// struct Frame(Vec<u8>);
/// impl AsPayload for Frame {
///     fn payload(&self) -> &[u8] { &self.0 }
///     fn sequence(&self) -> u64 { 0 }
///     fn stream_id(&self) -> u8 { 0 }
/// }
///
/// struct Mock { pending: Vec<Vec<u8>> }
/// impl TransportCore for Mock {
///     fn name(&self) -> &'static str { "mock" }
///     async fn send(&mut self, _buf: &[u8]) -> Result<(), TransportError> { Ok(()) }
/// }
/// impl DatagramSource for Mock {
///     type Frame = Frame;
///     fn recv_burst(
///         &mut self,
///         out: &mut FrameBatch<Frame>,
///         max: usize,
///     ) -> Result<usize, TransportError> {
///         let mut n = 0;
///         while n < max {
///             match self.pending.pop() {
///                 Some(bytes) => { out.push(Frame(bytes)); n += 1; }
///                 None => break,
///             }
///         }
///         Ok(n)
///     }
/// }
///
/// let mut mock = Mock { pending: vec![vec![1, 2, 3]] };
/// let mut batch = FrameBatch::with_capacity(8);
/// assert_eq!(mock.recv_burst(&mut batch, 8).unwrap(), 1);
///
/// let frame = batch.drain().next().unwrap();
/// // Owned + Send + 'static: hand the frame to another thread.
/// let handle = std::thread::spawn(move || frame.payload().to_vec());
/// assert_eq!(handle.join().unwrap(), vec![1, 2, 3]);
/// ```
pub trait DatagramSource: TransportCore {
    type Frame: RecvFrame;

    fn recv_burst(
        &mut self,
        out: &mut FrameBatch<Self::Frame>,
        max: usize,
    ) -> Result<usize, TransportError>;
}

/// Byte-stream recv. `recv_into` lands bytes once into caller-owned `dst`
/// (typically the uninitialised spare capacity of a decode buffer) and returns
/// the count written. `Ok(0)` means nothing was ready. The caller marks exactly
/// `n` returned bytes initialised.
pub trait StreamSource: TransportCore {
    fn recv_into(&mut self, dst: &mut [MaybeUninit<u8>]) -> Result<usize, TransportError>;
}

/// Optional readiness adapter for `.await`-driven callers. `ready()` resolves
/// when the next sync `recv_burst`/`recv_into` can make progress. A busy-poll
/// backend omits this, so the sync core never carries a waker.
pub trait AsyncReady: TransportCore {
    fn ready(&mut self) -> impl Future<Output = Result<(), TransportError>> + Send;
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

/// Datagram-only extension: multicast join + addressed send. TCP-only backends
/// skip it. Re-based on `TransportCore` alongside the recv split.
pub trait UdpTransport: TransportCore {
    fn join_multicast(
        &mut self,
        group: IpAddr,
        interface: MulticastInterface,
    ) -> impl Future<Output = Result<(), TransportError>> + Send;

    fn send_to(
        &mut self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> impl Future<Output = Result<(), TransportError>> + Send;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MulticastInterface {
    pub v4: Option<Ipv4Addr>,
    pub v6_scope_id: Option<u32>,
}
