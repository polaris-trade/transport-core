# transport-core

Core crate holding the `Transport` trait, `BufferPool` contract, shared error type, and config primitives. No I/O syscalls happen here; every backend and every protocol client depends on this crate only.

## Transport trait

[[crates/transport_core/src/transport.rs#Transport]] is the trait every backend implements: `poll_event` returns `Poll<Self::Event>`, `next_frame` yields a borrowed `Self::Frame<'_>` (per-call type borrowed from `&self`), `send` is async. Protocol crates stay generic over `T: Transport`.

[[crates/transport_core/src/transport.rs#AsPayload]] is the shape protocol code consumes from a frame: `payload()`, `sequence()`, `stream_id()`. Backend frames implement it; protocol frames re-implement it after wire parsing sets sequence + stream_id.

[[crates/transport_core/src/transport.rs#UdpTransport]] extends `Transport` with `join_multicast` + `send_to`. TCP-only backends skip it. [[crates/transport_core/src/transport.rs#MulticastInterface]] unifies IPv4 interface address + IPv6 scope id.

## Extension traits

[[crates/transport_core/src/ext.rs#PoolAccess]] exposes a backend's `BufferPool` under `type Pool: BufferPool`. Protocol receivers read from `T::pool()` to reserve slabs before recv.

[[crates/transport_core/src/ext.rs#TransportBind]] holds the async constructors: `bind_udp(bind, rx, ring, batch)` and `connect_tcp(bind, ring)`. Split from `Transport` because construction is orthogonal to the running transport's poll/send loop, and TCP-only backends skip UDP-only fields cleanly.

## BufferPool contract

[[crates/transport_core/src/pool.rs#BufferPool]] is the owned-handle pool trait. `Slab` is `AsRef<[u8]> + Send + 'static` so it crosses `.await` points and lives in reassembler slots. `acquire` returns `None` at saturation for backpressure.

[[crates/transport_core/src/pool.rs#SharedPool]] is the `Arc<P>` alias for the common receiver pattern where one pool serves multiple transport instances.

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
