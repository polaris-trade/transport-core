# transport-core

Core crate holding the `Transport` trait, `BufferPool` contract, shared `TransportError`, and config primitives. Every backend (`transport-tokio`, `transport-mio`, and future `transport-*`) and every protocol client (`client-moldudp`, `client-soupbintcp`) depends on this crate only; no I/O syscalls happen here.

## Scope

- `TransportError` — shared error type; backends map internal failures here, protocol crates wrap via `#[from]`.
- Config primitives — `BindConfig`, `RecvBufConfig`, `RingConfig`, `BatchConfig`, `AffinityConfig`, `HugepageSize`. Serde-derived, format-agnostic. Callers pick JSON, YAML, or anything else at load time.
- `Transport` trait + `BufferPool` contract.

## Usage

```rust
use transport_core::{BindConfig, RecvBufConfig, RingConfig, TransportError};

fn build_bind() -> BindConfig {
    BindConfig {
        addr: "0.0.0.0:4242".parse().unwrap(),
        reuse_addr: true,
        reuse_port: true,
    }
}
```

## Dev commands

```bash
cargo nextest run --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
lat check
```

MSRV: `1.96.0` (pinned in `rust-toolchain.toml`).

## Docs

- Architecture concepts → [`lat.md/lat.md`](lat.md/lat.md)
- Active spec → [`specs/`](specs/)

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
