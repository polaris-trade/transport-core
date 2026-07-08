# transport-core

Backend-agnostic contract for the Polaris networking stack: the recv-seam
traits, the `BufferPool` pool contract, the shared `TransportError`, and the
serde config primitives every backend and protocol client is built on. No
socket, ring, or syscall touches this crate. It is pure types and traits, so a
protocol client compiles against it without pulling in any I/O backend.

## What it defines

### Recv seam

Recv yields **owned frames** that carry their own bytes (a pool slab) and return
them to the pool on `Drop`, so a frame outlives the recv call and moves across
threads for zero-copy handoff. Recv is **sync and batch-first** so every
backend, from an async runtime to a busy-poll kernel-bypass NIC, shares one
path; an optional async adapter layers `.await` on top without a second recv
implementation.

- `TransportCore`: common base, `name()` plus async `send` (the low-rate path).
- `DatagramSource`: `recv_burst(&mut out, max) -> n` reaps up to `max` owned
  frames into a caller-preallocated `FrameBatch`. `Ok(0)` means nothing ready,
  `PoolExhausted` means backpressure. A single recv is a burst of one.
- `StreamSource`: `recv_into(dst: &mut [MaybeUninit<u8>]) -> n` lands bytes once
  into caller-owned spare capacity.
- `AsyncReady`: optional `ready().await` readiness adapter. Busy-poll backends
  omit it so the sync core never carries a waker.
- `AsPayload` / `RecvFrame`: the bytes-plus-metadata shape protocol code reads
  from a frame. `RecvFrame` is the blanket `AsPayload + Send + 'static` marker.
- `UdpTransport`: datagram-only extension, multicast join plus addressed send.

### Pool contract

`BufferPool` is the owned-handle pool: `acquire(len) -> Option<Slab>` returns
`None` at saturation (the backpressure signal), and each `Slab` is
`AsRef<[u8]> + Send + 'static` so it crosses `.await` points and lives in
reassembler slots. `PoolAccess` exposes a backend's pool so a receiver can size
its reorder window and reserve slabs before recv.

### Construction and config

`TransportBind` supplies the async constructors (`bind_udp`, `connect_tcp`).
Config primitives are serde-first so app configs ship as JSON or TOML:
`BindConfig`, `RecvBufConfig`, `SendBufConfig`, `RingConfig`, `BatchConfig`,
`AffinityConfig`. All are `#[non_exhaustive]`; construct with `Default` then set
the fields you need.

### Errors

`TransportError` is the shared error type backends map I/O and pool failures
into; protocol crates wrap it via `#[from]`. Its `Display` strings are locked as
user-facing log lines.

## Usage

A receiver stays generic over the backend. It names a `DatagramSource`, never a
concrete transport, so swapping backends needs no receiver change:

```rust
use transport_core::{AsPayload, DatagramSource, FrameBatch, TransportError};

fn drain<T: DatagramSource>(
    t: &mut T,
    batch: &mut FrameBatch<T::Frame>,
) -> Result<(), TransportError> {
    match t.recv_burst(batch, 64) {
        Ok(0) => {}                       // nothing ready, spin again
        Ok(_n) => {
            for frame in batch.drain() {
                let _bytes: &[u8] = frame.payload();
                // hand the owned frame downstream; it is Send + 'static
            }
        }
        Err(TransportError::PoolExhausted { .. }) => {
            // backpressure: stop reaping, let the kernel drop
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
```

## Backends

Implementations live in their own crates and are selected by the consumer:

- `transport_tokio`: Tokio async runtime backend (UDP + TCP).
- `transport_mio`: runtime-free mio backend.

## Testing harness

Enable the `testing` feature for the shared conformance suites every backend
runs, so failures line up one-to-one across backends:

- `run_conformance_suite::<T>()`: construction (bind, connect, name).
- `run_datagram_source(build)`: the recv contract (burst bound, drain to
  `Ok(0)`, pool reclaim across a thread boundary, `PoolExhausted` backpressure).

```toml
[dev-dependencies]
transport_core = { git = "https://github.com/polaris-trade/transport-core", tag = "transport_core-vX.Y.Z", features = ["testing"] }
```

## Dev commands

```bash
cargo nextest run
cargo clippy --all-targets -- -D warnings
lat check
```

MSRV `1.96.1` (pinned in `rust-toolchain.toml`). Distributed by git tag
(`publish = false`); depend on it with `git = ..., tag = "transport_core-vX.Y.Z"`.

## Docs

Architecture and design rationale live in [`lat.md/lat.md`](lat.md/lat.md).

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
