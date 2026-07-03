//! Test harness shared across every backend implementation. Backends
//! import [`run_conformance_suite`] and pair it with a [`MockPeer`] to
//! verify their `Transport` impl behaves uniformly.
//!
//! Enable via the `testing` feature.

pub mod conformance;
pub mod mock_peer;

pub use conformance::{ConformanceCase, ConformanceReport, run_conformance_suite};
pub use mock_peer::{MockAction, MockKind, MockPeer, MockRunReport};
