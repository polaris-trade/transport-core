//! Display strings are user-facing log lines. Lock the format per variant so
//! ops dashboards + log-alert rules keep matching across releases.

use std::io;

use transport_core::TransportError;

#[test]
fn bind_failed_display() {
    let e = TransportError::BindFailed {
        addr: "0.0.0.0:4242".into(),
        reason: "EADDRINUSE".into(),
    };
    assert_eq!(e.to_string(), "bind failed for 0.0.0.0:4242: EADDRINUSE");
}

#[test]
fn io_display_wraps_source() {
    let src = io::Error::new(io::ErrorKind::UnexpectedEof, "peer closed");
    let e: TransportError = src.into();
    assert_eq!(e.to_string(), "I/O error: peer closed");
}

#[test]
fn pool_exhausted_display() {
    let e = TransportError::PoolExhausted {
        in_use: 1024,
        capacity: 1024,
    };
    assert_eq!(
        e.to_string(),
        "buffer pool exhausted (in_use 1024 / capacity 1024)"
    );
}

#[test]
fn ring_full_display() {
    let e = TransportError::RingFull { capacity: 4096 };
    assert_eq!(e.to_string(), "ring full (capacity 4096)");
}

#[test]
fn backend_unavailable_display() {
    let e = TransportError::BackendUnavailable {
        name: "mock-backend",
        reason: "resource unavailable".into(),
    };
    assert_eq!(
        e.to_string(),
        "backend mock-backend unavailable: resource unavailable"
    );
}

#[test]
fn unsupported_display() {
    let e = TransportError::Unsupported {
        name: "mock-backend",
        reason: "multicast join not supported",
    };
    assert_eq!(
        e.to_string(),
        "operation not supported by mock-backend: multicast join not supported"
    );
}
