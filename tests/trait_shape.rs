//! Compile-check that `Transport` + `AsPayload` + `UdpTransport` resolve
//! against a stand-in impl. Locks the trait signatures so downstream
//! backend crates don't drift.

use std::net::{IpAddr, SocketAddr};
use std::task::{Context, Poll};
use transport_core::{AsPayload, MulticastInterface, Transport, TransportError, UdpTransport};

struct NoopTransport;

struct NoopFrame<'a> {
    bytes: &'a [u8],
    sequence: u64,
    stream_id: u8,
}

impl<'a> AsPayload for NoopFrame<'a> {
    fn payload(&self) -> &[u8] {
        self.bytes
    }
    fn sequence(&self) -> u64 {
        self.sequence
    }
    fn stream_id(&self) -> u8 {
        self.stream_id
    }
}

impl Transport for NoopTransport {
    type Frame<'a> = NoopFrame<'a>;
    type Event = ();

    fn poll_event(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), TransportError>> {
        Poll::Ready(Ok(()))
    }

    fn next_frame(&self) -> Option<NoopFrame<'_>> {
        None
    }

    fn name(&self) -> &'static str {
        "noop"
    }

    async fn send(&mut self, _buf: &[u8]) -> Result<(), TransportError> {
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

fn takes_transport<T: Transport>(_t: &T) {}
fn takes_udp<T: UdpTransport>(_t: &T) {}

#[test]
fn noop_transport_resolves_frame_lifetime() {
    let t = NoopTransport;
    takes_transport(&t);
    takes_udp(&t);
    assert_eq!(t.name(), "noop");
}

#[test]
fn frame_payload_shape() {
    let f = NoopFrame {
        bytes: b"hello",
        sequence: 42,
        stream_id: 1,
    };
    assert_eq!(f.payload(), b"hello");
    assert_eq!(f.sequence(), 42);
    assert_eq!(f.stream_id(), 1);
}
