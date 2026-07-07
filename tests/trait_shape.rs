//! Compile-check that the recv seam resolves against a stand-in impl:
//! `TransportCore` + `AsPayload`/`RecvFrame` + `DatagramSource` +
//! `StreamSource` + `AsyncReady` + `UdpTransport`. Locks the trait signatures
//! so downstream backend crates don't drift.

use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr};
use transport_core::{
    AsPayload, AsyncReady, DatagramSource, FrameBatch, MulticastInterface, RecvFrame, StreamSource,
    TransportCore, TransportError, UdpTransport,
};

struct NoopTransport {
    pending: Vec<Vec<u8>>,
}

/// Owned frame: carries its own bytes, so it is `Send + 'static` and satisfies
/// the `RecvFrame` blanket impl.
struct OwnedFrame {
    bytes: Vec<u8>,
    sequence: u64,
    stream_id: u8,
}

impl AsPayload for OwnedFrame {
    fn payload(&self) -> &[u8] {
        &self.bytes
    }
    fn sequence(&self) -> u64 {
        self.sequence
    }
    fn stream_id(&self) -> u8 {
        self.stream_id
    }
}

impl TransportCore for NoopTransport {
    fn name(&self) -> &'static str {
        "noop"
    }

    async fn send(&mut self, _buf: &[u8]) -> Result<(), TransportError> {
        Ok(())
    }
}

impl DatagramSource for NoopTransport {
    type Frame = OwnedFrame;

    fn recv_burst(
        &mut self,
        out: &mut FrameBatch<OwnedFrame>,
        max: usize,
    ) -> Result<usize, TransportError> {
        let mut n = 0;
        while n < max {
            match self.pending.pop() {
                Some(bytes) => {
                    out.push(OwnedFrame {
                        bytes,
                        sequence: 0,
                        stream_id: 0,
                    });
                    n += 1;
                }
                None => break,
            }
        }
        Ok(n)
    }
}

impl StreamSource for NoopTransport {
    fn recv_into(&mut self, _dst: &mut [MaybeUninit<u8>]) -> Result<usize, TransportError> {
        Ok(0)
    }
}

impl AsyncReady for NoopTransport {
    async fn ready(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}

impl UdpTransport for NoopTransport {
    async fn join_multicast(
        &mut self,
        _group: IpAddr,
        _interface: MulticastInterface,
    ) -> Result<(), TransportError> {
        Ok(())
    }

    async fn send_to(&mut self, _buf: &[u8], _addr: SocketAddr) -> Result<(), TransportError> {
        Ok(())
    }
}

fn takes_datagram_source<T: DatagramSource>(_t: &T) {}
fn takes_stream_source<T: StreamSource>(_t: &T) {}
fn takes_async_ready<T: AsyncReady>(_t: &T) {}
fn takes_udp<T: UdpTransport>(_t: &T) {}

/// Frame is `Send + 'static` via the blanket `RecvFrame` impl.
fn assert_recv_frame<F: RecvFrame>() {}

#[test]
fn noop_transport_resolves_recv_seam() {
    let t = NoopTransport { pending: vec![] };
    takes_datagram_source(&t);
    takes_stream_source(&t);
    takes_async_ready(&t);
    takes_udp(&t);
    assert_recv_frame::<OwnedFrame>();
    assert_eq!(t.name(), "noop");
}

#[test]
fn recv_burst_fills_batch_and_caps_at_max() {
    let mut t = NoopTransport {
        pending: vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
    };
    let mut batch = FrameBatch::with_capacity(8);
    let n = t.recv_burst(&mut batch, 2).expect("recv ok");
    assert_eq!(n, 2);
    assert_eq!(batch.len(), 2);

    // Drain retains the backing allocation; frames are owned by the caller.
    let drained: Vec<_> = batch.drain().map(|f| f.payload().to_vec()).collect();
    assert_eq!(drained.len(), 2);
    assert!(batch.is_empty());
}

#[test]
fn owned_frame_crosses_thread_boundary() {
    let mut t = NoopTransport {
        pending: vec![b"hello".to_vec()],
    };
    let mut batch = FrameBatch::with_capacity(4);
    assert_eq!(t.recv_burst(&mut batch, 4).expect("recv ok"), 1);

    let frame = batch.drain().next().expect("one frame");
    let handle = std::thread::spawn(move || frame.payload().to_vec());
    assert_eq!(handle.join().unwrap(), b"hello".to_vec());
}

#[test]
fn frame_payload_shape() {
    let f = OwnedFrame {
        bytes: b"hello".to_vec(),
        sequence: 42,
        stream_id: 1,
    };
    assert_eq!(f.payload(), b"hello");
    assert_eq!(f.sequence(), 42);
    assert_eq!(f.stream_id(), 1);
}
