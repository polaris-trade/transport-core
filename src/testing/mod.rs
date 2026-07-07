//! Test harness shared across every backend implementation. Backends import
//! [`run_conformance_suite`] (construction, paired with a [`MockPeer`]) and
//! [`run_datagram_source`] (recv contract) to verify their impls behave
//! uniformly.
//!
//! Enable via the `testing` feature.

pub mod conformance;
pub mod mock_peer;

pub use conformance::{
    ConformanceCase, ConformanceReport, DatagramCase, DatagramConformanceReport,
    run_conformance_suite, run_datagram_source,
};
pub use mock_peer::{MockAction, MockKind, MockPeer, MockRunReport};
