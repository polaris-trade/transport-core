//! Shared recv telemetry seam. Backends route recv-burst counters here so metric
//! names + the gate live in one place, not duplicated per backend. Gated twice:
//! the `observability` cargo feature compiles it in, and observability-core's
//! runtime gate makes each call a no-op when metrics are off. see observability-core.

/// Metric names emitted by the recv path. One definition point for every backend,
/// so the five backends never drift. Prometheus normalizes `.` to `_` at scrape.
pub mod metric {
    /// Frames reaped, `backend` label. Monotonic counter.
    pub const RECV_PACKETS: &str = "transport.recv.packets";
    /// Bytes reaped, `backend` label. Monotonic counter.
    pub const RECV_BYTES: &str = "transport.recv.bytes";
    // Backend-specific recv-error metric names stay in the backend that owns them,
    // passed to record_recv_event. This crate defines only the universal counters.
}

/// Record one recv burst: `packets` frames totalling `bytes`, tagged `backend`.
/// No-op when the gate is off: one thread-local gate read, no atomic, no alloc,
/// so a batched recv path stays allocation-free off-gate.
#[inline]
pub fn record_recv_burst(backend: &'static str, packets: u64, bytes: u64) {
    if observability_core::metrics_enabled() {
        metrics::counter!(metric::RECV_PACKETS, "backend" => backend).increment(packets);
        metrics::counter!(metric::RECV_BYTES, "backend" => backend).increment(bytes);
    }
}

/// Increment one recv-side event counter by one, tagged `backend`. `name` is a
/// backend-owned static metric name (the backend defines its own error taxonomy). Gated.
#[inline]
pub fn record_recv_event(name: &'static str, backend: &'static str) {
    if observability_core::metrics_enabled() {
        metrics::counter!(name, "backend" => backend).increment(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gate-off calls are safe no-ops: no recorder installed, no panic. Guards the
    /// invariant that a backend may call the seam before any binary installs a recorder.
    #[test]
    fn record_recv_is_noop_when_gate_off() {
        observability_core::set_metrics_enabled(false);
        observability_core::refresh_thread_gate();
        record_recv_burst("test-backend", 7, 448);
        record_recv_event("transport.recv.dropped", "test-backend");
    }
}
