# transport-core

Core crate holding the `Transport` trait, `BufferPool` contract, shared error type, and config primitives. No I/O syscalls happen here; every backend and every protocol client depends on this crate only.

## Testing harness (feature-gated)

Feature `testing` exposes [[crates/transport_core/src/testing/conformance.rs#run_conformance_suite]] plus [[crates/transport_core/src/testing/mock_peer.rs#MockPeer]]. Every backend runs the same suite so failures line up 1:1 across CI dashboards.

The suite auto-spins a `127.0.0.1:0` TCP listener via [[crates/transport_core/src/testing/conformance.rs#spin_tcp_peer_and_connect]] before calling `T::connect_tcp` so backends do not need a running peer of their own.

[[crates/transport_core/src/testing/conformance.rs#ConformanceReport]] holds `passed` + `failed` case labels. [[crates/transport_core/src/testing/conformance.rs#ConformanceCase]] enumerates the stable case names.

[[crates/transport_core/src/testing/mock_peer.rs#MockPeer]] binds a real `127.0.0.1:0` socket (kind picked by [[crates/transport_core/src/testing/mock_peer.rs#MockKind]]) and drives a scripted [[crates/transport_core/src/testing/mock_peer.rs#MockAction]] list: send mock MoldUDP data/heartbeat, send SoupBinTCP frame, read + assert client-written bytes, sleep. `drop_rate` + `jitter` fields inject synthetic loss/latency.

[[crates/transport_core/src/testing/mock_peer.rs#MockRunReport]] returns `actions_completed`, `bytes_sent`, `bytes_dropped_synthetic` counters. [[crates/transport_core/src/testing/mock_peer.rs#MockPeerError]] carries structured failures: bind, I/O, missing UDP target, unmet expect assertions.

## Transport trait

[[crates/transport_core/src/transport.rs#Transport]] is the trait every backend implements: `poll_event` returns `Poll<Self::Event>`, `next_frame` yields a borrowed `Self::Frame<'_>` (per-call type borrowed from `&self`), `send` is async. Protocol crates stay generic over `T: Transport`.

[[crates/transport_core/src/transport.rs#AsPayload]] is the shape protocol code consumes from a frame: `payload()`, `sequence()`, `stream_id()`. Backend frames implement it; protocol frames re-implement it after wire parsing sets sequence + stream_id.

[[crates/transport_core/src/transport.rs#TimestampedPayload]] extends `AsPayload` with `timestamp() -> Option<Timestamp>`. Kept as a separate trait so `AsPayload` stays lean; protocol code that needs recv timestamps bounds `T::Frame: TimestampedPayload`. [[crates/transport_core/src/transport.rs#Timestamp]] carries `nanos` + [[crates/transport_core/src/transport.rs#TimestampSource]] (kernel software vs hardware NIC).

[[crates/transport_core/src/transport.rs#UdpTransport]] extends `Transport` with `join_multicast` + `send_to`. TCP-only backends skip it. [[crates/transport_core/src/transport.rs#MulticastInterface]] unifies IPv4 interface address + IPv6 scope id.

## Extension traits

[[crates/transport_core/src/ext.rs#PoolAccess]] exposes a backend's `BufferPool` under `type Pool: BufferPool`. Protocol receivers read from `T::pool()` to reserve slabs before recv.

[[crates/transport_core/src/ext.rs#TransportBind]] holds the async constructors: `bind_udp(bind, rx, tx, ring, batch)` and `connect_tcp(bind, rx, tx, ring)`. Split from `Transport` because construction is orthogonal to the running transport's poll/send loop; both paths take `RecvBufConfig` + `SendBufConfig` so kernel buffer sizing stays symmetric.

## BufferPool contract

[[crates/transport_core/src/pool.rs#BufferPool]] is the owned-handle pool trait. `Slab` is `AsRef<[u8]> + Send + 'static` so it crosses `.await` points and lives in reassembler slots. `acquire` returns `None` at saturation for backpressure.

[[crates/transport_core/src/pool.rs#SharedPool]] is the `Arc<P>` alias for the common receiver pattern where one pool serves multiple transport instances.

## Error primitive

[[crates/transport_core/src/error.rs#TransportError]] is the shared error type backends map their I/O and pool failures into. Protocol crates wrap it via `#[from]` in their own error enums so callers can match by kind.

Variants: `BindFailed`, `Io` (wraps `std::io::Error`), `PoolExhausted`, `RingFull`, `BackendUnavailable`, `Unsupported`. Display strings are locked as user-facing log lines.

## Config primitives

Serde-first configs shared across every backend so app configs ship as JSON or TOML without per-backend forks. All structs are `#[non_exhaustive]`; construct via `T::default()` then set the fields you care about.

### BindConfig

[[crates/transport_core/src/config.rs#BindConfig]] captures socket bind target plus `SO_REUSEADDR` / `SO_REUSEPORT` toggles. `Default` binds to `0.0.0.0:0` (kernel-picked port on all interfaces).

### RecvBufConfig

[[crates/transport_core/src/config.rs#RecvBufConfig]] holds `SO_RCVBUF` request, `SO_RXQ_OVFL` opt-in, [[crates/transport_core/src/config.rs#TimestampMode]] request (`None` / `KernelSw` / `HardwareRx`), and `SO_BUSY_POLL` microseconds (Linux). Backends log a warn on kernel shortfall or unsupported timestamping mode.

### SendBufConfig

[[crates/transport_core/src/config.rs#SendBufConfig]] holds `SO_SNDBUF` request, symmetric with `RecvBufConfig`. Send-heavy paths (SoupBinTCP session, retransmit requests) throttle without it sized appropriately.

### RingConfig

[[crates/transport_core/src/config.rs#RingConfig]] parameterizes buffer-ring shape: slab count/size, SQPOLL flag, hugepages toggle, [[crates/transport_core/src/config.rs#HugepageSize]]. Naive OSS pools honor `slab_count`/`slab_size` only; kernel-bypass backends consume the rest.

### BatchConfig

[[crates/transport_core/src/config.rs#BatchConfig]] splits `recv_size` (recvmmsg batch) and `send_size` (sendmmsg batch). `Default` is 0 on both, which each backend interprets as its own single-msg path.

### AffinityConfig

[[crates/transport_core/src/config.rs#AffinityConfig]] pins the driver loop to `io_cpu` and (when SQPOLL enabled) the kernel poller to `sqpoll_cpu`. `None` = no pinning.
