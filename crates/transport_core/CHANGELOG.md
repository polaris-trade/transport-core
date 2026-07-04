# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0](https://github.com/polaris-trade/transport-core/releases/tag/transport_core-v0.1.0) - 2026-07-03

### Features

- _(transport-core)_ Core primitives ([#1](https://github.com/polaris-trade/transport-core/pull/1))

Introduces the shared surface every backend implements.

**Error model.** `TransportError` enum: `BindFailed`, `Io`, `PoolExhausted`, `RingFull`, `BackendUnavailable`, `Unsupported`. Locked `Display` per variant.

**Config primitives (serde).** `BindConfig`, `RecvBufConfig`, `RingConfig`, `BatchConfig`, `AffinityConfig`, `HugepageSize`. Format-agnostic.

**Transport traits.** `Transport` (per-call `Frame<'a>`, `Event` assoc, async `send`), `UdpTransport` (`join_multicast`, `send_to`), `PoolAccess`, `TransportBind`.

**Buffer pool.** `BufferPool` + owned-handle `Slab` crossing `.await`.

**Testing harness (feature-gated).** `run_conformance_suite<T: TransportBind>()`, `MockPeer` on `127.0.0.1:0`, scripted actions.
