# transport-core

Core crate holding the `Transport` trait, `BufferPool` contract, shared `TransportError`, and config primitives. Every backend (`transport-tokio`, `transport-mio`, and future `transport-*`) and every protocol client (`client-moldudp`, `client-soupbintcp`) depends on this crate only; no I/O syscalls happen here.

## Layout

Single-crate Cargo workspace. Room for adjacent helper crates (`crates/transport_core_testing`, etc.) as they land.

```
transport-core/
├── crates/
│   └── transport_core/    # the trait contract crate itself
├── lat.md/                # architecture knowledge graph
├── specs/                 # spec-driven artifacts (migration + design)
└── docs/                  # wire-format specs (MoldUDP64, SoupBinTCP)
```

## Dev commands

```bash
cargo nextest run --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
lat check
```

MSRV: `1.96.0` (pinned in `rust-toolchain.toml`).

## Docs

- Crate-level usage → [`crates/transport_core/README.md`](crates/transport_core/README.md)
- Architecture concepts → [`lat.md/lat.md`](lat.md/lat.md)
- Active spec → [`specs/`](specs/)

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
