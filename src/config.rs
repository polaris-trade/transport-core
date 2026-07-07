//! Config primitives shared across every backend. Serde-first so app configs
//! ship as JSON or TOML.

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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
