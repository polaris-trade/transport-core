# transport-core

Core crate holding the recv seam traits, `BufferPool` contract, shared error type, and config primitives. No I/O syscalls happen here; every backend and every protocol client depends on this crate only.

## Testing harness (feature-gated)

Feature `testing` exposes two suites plus [[src/testing/mock_peer.rs#MockPeer]]: [[src/testing/conformance.rs#run_conformance_suite]] (construction) and [[src/testing/conformance.rs#run_datagram_source]] (recv contract). Every backend runs the same suites so failures line up 1:1 across CI dashboards.

The construction suite auto-spins a `127.0.0.1:0` TCP listener via [[src/testing/conformance.rs#spin_tcp_peer_and_connect]] before calling `T::connect_tcp` so backends do not need a running peer of their own.

[[src/testing/conformance.rs#ConformanceReport]] holds `passed` + `failed` case labels. [[src/testing/conformance.rs#ConformanceCase]] enumerates the stable case names.

[[src/testing/conformance.rs#run_datagram_source]] is the recv-contract suite, generic over a backend's `DatagramSource` + `PoolAccess` and driven by a `build(count)` factory (real peer or in-process mock). It asserts burst count `<= max`, single is a burst of exactly 1 (bounded 8-iteration retry to tolerate a backend whose reclaim/readiness lags one tick, not to mask a broken one), a drained source returns `Ok(0)`, pool slab reclaim even when a frame is dropped on another thread (single-producer ring backends drain their return queue on the next `recv_burst`), and `PoolExhausted` when the pool is empty with data pending. [[src/testing/conformance.rs#DatagramConformanceReport]] + [[src/testing/conformance.rs#DatagramCase]] mirror the construction suite's report shape.

[[src/testing/mock_peer.rs#MockPeer]] binds a real `127.0.0.1:0` socket (kind picked by [[src/testing/mock_peer.rs#MockKind]]) and drives a scripted [[src/testing/mock_peer.rs#MockAction]] list: send mock MoldUDP data/heartbeat, send SoupBinTCP frame, read + assert client-written bytes, sleep. `drop_rate` + `jitter` fields inject synthetic loss/latency.

[[src/testing/mock_peer.rs#MockRunReport]] returns `actions_completed`, `bytes_sent`, `bytes_dropped_synthetic` counters. [[src/testing/mock_peer.rs#MockPeerError]] carries structured failures: bind, I/O, missing UDP target, unmet expect assertions.

## Recv telemetry seam (feature-gated)

Feature `observability` adds a shared recv-counter seam so the five backends emit identical metric names from one place, not hand-rolled per backend. Off by default, keeping the lean seam for consumers that do not instrument.

[[src/telemetry.rs#record_recv_burst]] takes a `backend` label plus `packets`/`bytes` and increments the two monotonic counters once per burst, guarded by `observability_core::metrics_enabled` so an off-gate call is one thread-local read with no atomic or alloc. [[src/telemetry.rs#record_recv_event]] increments a single named backend-owned event counter the same way.

The universal metric names live in the `metric` submodule as consts (`transport.recv.packets`, `.bytes`), the one definition point every backend shares; backend-specific error counters are named by the backend that owns them and passed to `record_recv_event`. The crate re-exports `observability_core` under the same feature so backends reach the runtime gate through this one path rather than each pinning observability-core themselves.

## Transport trait

[[src/transport.rs#TransportCore]] is the common base every backend implements: `name()` plus an async `send`. Recv splits into two sync extensions so datagram and stream shapes stay honest, with an optional async adapter on top.

[[src/transport.rs#DatagramSource]] is the discrete-datagram recv: `recv_burst(out, max)` reaps up to `max` owned frames into a caller-preallocated [[src/transport.rs#FrameBatch]] and returns the count. `Ok(0)` = nothing ready; `PoolExhausted` = backpressure. Single recv is a burst of 1.

[[tests/trait_shape.rs#framebatch_reuse_zero_alloc]] proves the zero-alloc reuse claim with `allocation-counter`: an in-process mock clones a preallocated `Arc` payload per frame (refcount bump, no heap traffic) across several `recv_burst`/`drain` cycles over one `FrameBatch`, first cycle as warmup, and asserts 0 allocations over the rest.

[[src/transport.rs#StreamSource]] is the byte-stream recv: `recv_into(dst)` lands bytes once into caller-owned `MaybeUninit` spare capacity and returns the count written.

[[src/transport.rs#AsyncReady]] is the optional readiness adapter: `ready().await` resolves when the next sync recv can progress. Busy-poll backends omit it so the sync core never carries a waker.

[[src/transport.rs#RecvFrame]] is the owned-frame marker (`AsPayload + Send + 'static`), blanket-implemented. `payload()` borrows the handle, not the transport, so a frame outlives the recv call and moves across threads.

[[src/transport.rs#AsPayload]] is the shape protocol code consumes from a frame: `payload()`, `sequence()`, `stream_id()`. Backend frames implement it; protocol frames re-implement it after wire parsing sets sequence + stream_id.

[[src/transport.rs#TimestampedPayload]] extends `AsPayload` with `timestamp() -> Option<Timestamp>`. Kept as a separate trait so `AsPayload` stays lean; protocol code that needs recv timestamps bounds `T::Frame: TimestampedPayload`. [[src/transport.rs#Timestamp]] carries `nanos` + [[src/transport.rs#TimestampSource]] (kernel software vs hardware NIC).

[[src/transport.rs#UdpTransport]] extends `TransportCore` with `join_multicast` + `send_to`. TCP-only backends skip it. [[src/transport.rs#MulticastInterface]] unifies IPv4 interface address + IPv6 scope id.

## Extension traits

[[src/ext.rs#PoolAccess]] exposes a backend's `BufferPool` under `type Pool: BufferPool`. Protocol receivers read from `T::pool()` to reserve slabs before recv.

[[src/ext.rs#TransportBind]] holds the async constructors: `bind_udp(bind, rx, tx, ring, batch, affinity)` and `connect_tcp(bind, rx, tx, ring, affinity)`. Split from `TransportCore` because construction is orthogonal to the running transport's recv/send loop; both paths take `RecvBufConfig` + `SendBufConfig` so kernel buffer sizing stays symmetric, plus `AffinityConfig` for core pinning. TCP has no `BatchConfig` (streams have no `recvmmsg` batch); its per-landing bound rides `RecvBufConfig::read_chunk`.

## BufferPool contract

[[src/pool.rs#BufferPool]] is the owned-handle pool trait. `Slab` is `AsRef<[u8]> + Send + 'static` so it crosses `.await` points and lives in reassembler slots. `acquire` returns `None` at saturation for backpressure.

[[src/pool.rs#SharedPool]] is the `Arc<P>` alias for the common receiver pattern where one pool serves multiple transport instances.

## Error primitive

[[src/error.rs#TransportError]] is the shared error type backends map their I/O and pool failures into. Protocol crates wrap it via `#[from]` in their own error enums so callers can match by kind.

Variants: `BindFailed`, `Io` (wraps `std::io::Error`), `PoolExhausted`, `RingFull`, `BackendUnavailable`, `Unsupported`. Display strings are locked as user-facing log lines.

## Config primitives

Serde-first configs shared across every backend so app configs ship as JSON or TOML without per-backend forks. All structs are `#[non_exhaustive]`; construct via `T::default()` then set the fields you care about.

### BindConfig

[[src/config.rs#BindConfig]] captures socket bind target plus `SO_REUSEADDR` / `SO_REUSEPORT` toggles. `Default` binds to `0.0.0.0:0` (kernel-picked port on all interfaces).

### RecvBufConfig

[[src/config.rs#RecvBufConfig]] holds the recv-side socket knobs: `SO_RCVBUF`, `SO_RXQ_OVFL`, [[src/config.rs#TimestampMode]], `SO_BUSY_POLL` microseconds (Linux), and a `read_chunk` bound on stream landings.

`read_chunk` is the max bytes per `recv_into` landing on stream backends (`None` = backend default). Backends log a warn on kernel shortfall or unsupported timestamping mode. All optional fields are `#[serde(default)]` so legacy config decodes unchanged.

### SendBufConfig

[[src/config.rs#SendBufConfig]] holds `SO_SNDBUF` request, symmetric with `RecvBufConfig`. Send-heavy paths (SoupBinTCP session, retransmit requests) throttle without it sized appropriately.

### RingConfig

[[src/config.rs#RingConfig]] parameterizes buffer-ring shape: slab count/size, SQPOLL flag, hugepages toggle, [[src/config.rs#HugepageSize]]. Naive OSS pools honor `slab_count`/`slab_size` only; kernel-bypass backends consume the rest.

### BatchConfig

[[src/config.rs#BatchConfig]] splits `recv_size` (recvmmsg batch) and `send_size` (sendmmsg batch). `Default` is 0 on both, which each backend interprets as its own single-msg path.

### AffinityConfig

[[src/config.rs#AffinityConfig]] pins the driver loop to `io_cpu` and (when SQPOLL enabled) the kernel poller to `sqpoll_cpu`. `None` = no pinning.
