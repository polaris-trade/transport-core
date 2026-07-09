//! Config primitives shared across every backend. Serde-first so app configs
//! ship as JSON or TOML.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BindConfig {
    pub addr: SocketAddr,
    pub reuse_addr: bool,
    pub reuse_port: bool,
}

impl BindConfig {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            reuse_addr: false,
            reuse_port: false,
        }
    }
}

impl Default for BindConfig {
    fn default() -> Self {
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            reuse_addr: false,
            reuse_port: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecvBufConfig {
    pub so_rcvbuf: Option<u32>,
    pub so_rxq_ovfl: bool,
    #[serde(default)]
    pub so_timestamping: TimestampMode,
    #[serde(default)]
    pub so_busy_poll_us: Option<u32>,
    /// Max bytes a stream backend lands per `recv_into` call. `None` = the
    /// backend's own default. Bounds one TCP landing; streams have no
    /// `recvmmsg`-style batch, so this replaces `BatchConfig` on the read path.
    #[serde(default)]
    pub read_chunk: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SendBufConfig {
    pub so_sndbuf: Option<u32>,
}

/// Requested recv timestamping mode. Backends without support log a warn
/// and fall through to `None` semantics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimestampMode {
    #[default]
    None,
    KernelSw,
    HardwareRx,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RingConfig {
    /// Buffer count. Default stays 1024. A client needing a larger reorder
    /// window (see client-moldudp) derives its own `slab_count` from that
    /// window and asserts pool capacity at construction, rather than inflating
    /// this global default for every backend (a TCP pool never draws slabs).
    pub slab_count: usize,
    pub slab_size: usize,
    pub sqpoll: bool,
    pub hugepages: bool,
    pub hugepage_size: HugepageSize,
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            slab_count: 1024,
            slab_size: 2048,
            sqpoll: false,
            hugepages: false,
            hugepage_size: HugepageSize::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HugepageSize {
    #[default]
    None,
    TwoMB,
    GigaByte,
}

/// `recv_size` gates `recvmmsg` batch depth; `send_size` gates `sendmmsg`
/// batch depth. Zero on either means "single-msg path".
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BatchConfig {
    pub recv_size: u32,
    pub send_size: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AffinityConfig {
    pub io_cpu: Option<usize>,
    pub sqpoll_cpu: Option<usize>,
}
