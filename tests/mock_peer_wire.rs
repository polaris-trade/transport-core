//! Locks `MockPeer` wire format. Backends depending on MockPeer for
//! conformance need stable bytes for mold session id, sequence encoding,
//! and message-block framing.

#![cfg(feature = "testing")]

use transport_core::testing::{MockAction, MockKind, MockPeer};

#[tokio::test]
async fn mock_peer_udp_roundtrip() {
    let peer_sock = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind client");
    let client_addr = peer_sock.local_addr().expect("addr");

    let listener = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind peer");
    let peer_bind = listener.local_addr().expect("addr");
    drop(listener);

    let peer =
        MockPeer::new(MockKind::Udp { bind: peer_bind }, Some(client_addr)).with_script(vec![
            MockAction::SendMoldHeartbeat,
            MockAction::SendMoldPacket {
                seq: 42,
                payload: b"hello".to_vec(),
            },
        ]);

    let peer_task = tokio::spawn(peer.run());

    let mut buf = vec![0u8; 512];
    let (n, _) = peer_sock.recv_from(&mut buf).await.expect("recv heartbeat");
    assert_eq!(n, 20, "mold heartbeat wire length");

    let (n, _) = peer_sock.recv_from(&mut buf).await.expect("recv data");
    assert_eq!(&buf[..10], b"MOCKPEER01");
    assert_eq!(u64::from_be_bytes(buf[10..18].try_into().unwrap()), 42);
    assert_eq!(u16::from_be_bytes(buf[18..20].try_into().unwrap()), 1);
    assert_eq!(u16::from_be_bytes(buf[20..22].try_into().unwrap()), 5);
    assert_eq!(&buf[22..n], b"hello");

    let report = peer_task.await.expect("join").expect("run ok");
    assert_eq!(report.actions_completed, 2);
    assert!(report.bytes_sent > 0);
}
