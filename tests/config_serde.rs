//! JSON roundtrip per config primitive. Guards on-disk schema stability
//! across releases. Caller picks the wire format (YAML, JSON, whatever)
//! at load time; transport-core only derives serde traits.
//!
//! Structs are `#[non_exhaustive]`; construction pattern is
//! `let mut cfg = T::default(); cfg.field = ...;`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use transport_core::{
    AffinityConfig, BatchConfig, BindConfig, HugepageSize, RecvBufConfig, RingConfig,
    SendBufConfig, TimestampMode,
};

fn json_roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize json");
    let back: T = serde_json::from_str(&json).expect("deserialize json");
    assert_eq!(value, &back);
}

#[test]
fn bind_config_json_roundtrip() {
    let mut cfg = BindConfig::default();
    cfg.addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 4242);
    cfg.reuse_addr = true;
    cfg.reuse_port = true;
    json_roundtrip(&cfg);
}

#[test]
fn recv_buf_config_json_roundtrip() {
    let mut cfg = RecvBufConfig::default();
    cfg.so_rcvbuf = Some(8 * 1024 * 1024);
    cfg.so_rxq_ovfl = true;
    cfg.so_timestamping = TimestampMode::HardwareRx;
    cfg.so_busy_poll_us = Some(50);
    cfg.read_chunk = Some(64 * 1024);
    json_roundtrip(&cfg);
}

#[test]
fn send_buf_config_json_roundtrip() {
    let mut cfg = SendBufConfig::default();
    cfg.so_sndbuf = Some(2 * 1024 * 1024);
    json_roundtrip(&cfg);
}

#[test]
fn timestamp_mode_json_roundtrip_all_variants() {
    for mode in [
        TimestampMode::None,
        TimestampMode::KernelSw,
        TimestampMode::HardwareRx,
    ] {
        json_roundtrip(&mode);
    }
}

#[test]
fn ring_config_json_roundtrip_hugepages_variants() {
    for hp in [
        HugepageSize::None,
        HugepageSize::TwoMB,
        HugepageSize::GigaByte,
    ] {
        let mut cfg = RingConfig::default();
        cfg.slab_count = 2048;
        cfg.slab_size = 4096;
        cfg.sqpoll = true;
        cfg.hugepages = !matches!(hp, HugepageSize::None);
        cfg.hugepage_size = hp;
        json_roundtrip(&cfg);
    }
}

#[test]
fn batch_config_json_roundtrip() {
    let mut cfg = BatchConfig::default();
    cfg.recv_size = 64;
    cfg.send_size = 32;
    json_roundtrip(&cfg);
}

#[test]
fn affinity_config_json_roundtrip() {
    let mut cfg = AffinityConfig::default();
    cfg.io_cpu = Some(3);
    cfg.sqpoll_cpu = Some(4);
    json_roundtrip(&cfg);
}

#[test]
fn recv_buf_config_deserializes_without_new_fields() {
    let legacy = r#"{"so_rcvbuf":1024,"so_rxq_ovfl":false}"#;
    let cfg: RecvBufConfig = serde_json::from_str(legacy).expect("legacy shape decodes");
    assert_eq!(cfg.so_rcvbuf, Some(1024));
    assert_eq!(cfg.so_timestamping, TimestampMode::None);
    assert!(cfg.so_busy_poll_us.is_none());
    assert!(cfg.read_chunk.is_none());
}

#[test]
fn ring_config_default_slab_count_is_1024() {
    // Stays 1024: clients derive a larger reorder window themselves rather than
    // inflating this global default (a TCP pool never draws slabs).
    assert_eq!(RingConfig::default().slab_count, 1024);
}
