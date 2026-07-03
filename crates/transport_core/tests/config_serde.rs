//! JSON roundtrip per config primitive. Guards on-disk schema stability
//! across releases. Caller picks the wire format (YAML, JSON, whatever)
//! at load time; transport-core only derives serde traits.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use transport_core::{
    AffinityConfig, BatchConfig, BindConfig, HugepageSize, RecvBufConfig, RingConfig,
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
    let cfg = BindConfig {
        addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 4242),
        reuse_addr: true,
        reuse_port: true,
    };
    json_roundtrip(&cfg);
}

#[test]
fn recv_buf_config_json_roundtrip() {
    let cfg = RecvBufConfig {
        so_rcvbuf: Some(8 * 1024 * 1024),
        so_rxq_ovfl: true,
    };
    json_roundtrip(&cfg);
}

#[test]
fn ring_config_json_roundtrip_hugepages_variants() {
    for hp in [
        HugepageSize::None,
        HugepageSize::TwoMB,
        HugepageSize::GigaByte,
    ] {
        let cfg = RingConfig {
            slab_count: 2048,
            slab_size: 4096,
            sqpoll: true,
            hugepages: !matches!(hp, HugepageSize::None),
            hugepage_size: hp,
        };
        json_roundtrip(&cfg);
    }
}

#[test]
fn batch_config_json_roundtrip() {
    let cfg = BatchConfig { size: 64 };
    json_roundtrip(&cfg);
}

#[test]
fn affinity_config_json_roundtrip() {
    let cfg = AffinityConfig {
        io_cpu: Some(3),
        sqpoll_cpu: Some(4),
    };
    json_roundtrip(&cfg);
}
