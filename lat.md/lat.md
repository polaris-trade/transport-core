# transport-core

Zero-I/O leaf crate holding the `Transport` trait, `BufferPool` contract, shared error type, and config primitives. Every backend and every protocol client depends on this crate only.

## Error primitive

[[crates/transport_core/src/error.rs#TransportError]] is the shared error type backends map their I/O and pool failures into. Protocol crates wrap it via `#[from]` in their own error enums so callers can match by kind.

Variants: `BindFailed`, `Io` (wraps `std::io::Error`), `PoolExhausted`, `RingFull`, `BackendUnavailable`, `Unsupported`. Display strings are locked as user-facing log lines.

## Config primitives

Serde-first configs shared across every backend so app configs ship as JSON or TOML without per-backend forks.

### BindConfig

[[crates/transport_core/src/config.rs#BindConfig]] captures socket bind target plus `SO_REUSEADDR` / `SO_REUSEPORT` toggles. `Default` binds to `0.0.0.0:0` (kernel-picked port on all interfaces).

### RecvBufConfig

[[crates/transport_core/src/config.rs#RecvBufConfig]] holds `SO_RCVBUF` request and `SO_RXQ_OVFL` opt-in. Backends log a warn when the kernel grants less than requested rcvbuf.

### RingConfig

[[crates/transport_core/src/config.rs#RingConfig]] parameterizes buffer-ring shape: slab count/size, SQPOLL flag, hugepages toggle, [[crates/transport_core/src/config.rs#HugepageSize]]. Naive OSS pools honor `slab_count`/`slab_size` only; kernel-bypass backends consume the rest.

### BatchConfig

[[crates/transport_core/src/config.rs#BatchConfig]] holds `recvmmsg` batch size. `Default` = 0 which each backend interprets as its own single-recv path.

### AffinityConfig

[[crates/transport_core/src/config.rs#AffinityConfig]] pins the driver loop to `io_cpu` and (when SQPOLL enabled) the kernel poller to `sqpoll_cpu`. `None` = no pinning.
