# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.3.0](https://github.com/polaris-trade/transport-core/compare/transport_core-v0.2.1...transport_core-v0.3.0) - 2026-07-07

### Features

- *(transport-core)* Implement owned-frame batch recv ([#19](https://github.com/polaris-trade/transport-core/pull/19))


  Replace the Transport trait (borrowed &self frames) with a sync,
batch-first recv API. DatagramSource::recv_burst hands back owned
frames in a reusable FrameBatch; StreamSource::recv_into covers TCP.
Frames own their pool slab and are Send, so they outlive the recv call
and move across threads without a copy. Every backend can hit this
shape, so clients won't need a rewrite as we add more.


## [0.2.1](https://github.com/polaris-trade/transport-core/compare/transport_core-v0.2.0...transport_core-v0.2.1) - 2026-07-07

### Refactor

- *(lib)* Restructure into single-crate layout ([#14](https://github.com/polaris-trade/transport-core/pull/14))


## [0.2.0](https://github.com/polaris-trade/transport-core/compare/transport_core-v0.1.0...transport_core-v0.2.0) - 2026-07-04

### Documentation

- *(changelog)* Manual fix changelog ([#6](https://github.com/polaris-trade/transport-core/pull/6))

- *(changelog)* Manual fix changelog ([#5](https://github.com/polaris-trade/transport-core/pull/5))


### Features

- *(transport-core)* Add perf-config, TimestampedPayload, and TCP conformance peer ([#7](https://github.com/polaris-trade/transport-core/pull/7))

Rework the config layer for UDP + TCP tuning and tighten the conformance suite so backends can be validated end-to-end without external scaffolding.


## [0.1.0](https://github.com/polaris-trade/transport-core/releases/tag/transport_core-v0.1.0) - 2026-07-03

### Features

- *(transport-core)* Core primitives ([#1](https://github.com/polaris-trade/transport-core/pull/1))

  Introduces the shared surface every backend implements.

  **Error model.** `TransportError` enum: `BindFailed`, `Io`, `PoolExhausted`, `RingFull`, `BackendUnavailable`, `Unsupported`. Locked `Display` per variant.

  **Config primitives (serde).** `BindConfig`, `RecvBufConfig`, `RingConfig`, `BatchConfig`, `AffinityConfig`, `HugepageSize`. Format-agnostic.

  **Transport traits.** `Transport` (per-call `Frame<'a>`, `Event` assoc, async `send`), `UdpTransport` (`join_multicast`, `send_to`), `PoolAccess`, `TransportBind`.

  **Buffer pool.** `BufferPool` + owned-handle `Slab` crossing `.await`.

  **Testing harness (feature-gated).** `run_conformance_suite<T: TransportBind>()`, `MockPeer` on `127.0.0.1:0`, scripted actions.
