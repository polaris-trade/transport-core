//! Runs the shared conformance suite against a backend. Each case has a
//! stable name so failures line up across backends in CI dashboards.

use crate::config::{BatchConfig, BindConfig, RecvBufConfig, RingConfig};
use crate::ext::TransportBind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceCase {
    BindUdp,
    ConnectTcp,
    NameNonEmpty,
}

impl ConformanceCase {
    pub fn label(self) -> &'static str {
        match self {
            Self::BindUdp => "bind_udp",
            Self::ConnectTcp => "connect_tcp",
            Self::NameNonEmpty => "name_non_empty",
        }
    }
}

#[derive(Debug, Default)]
pub struct ConformanceReport {
    pub passed: Vec<&'static str>,
    pub failed: Vec<(&'static str, String)>,
}

impl ConformanceReport {
    pub fn all_passed(&self) -> bool {
        self.failed.is_empty()
    }

    fn record<E: std::fmt::Display>(&mut self, case: ConformanceCase, res: Result<(), E>) {
        match res {
            Ok(()) => self.passed.push(case.label()),
            Err(e) => self.failed.push((case.label(), e.to_string())),
        }
    }
}

pub async fn run_conformance_suite<T: TransportBind>() -> ConformanceReport {
    let mut report = ConformanceReport::default();

    let bind_res = T::bind_udp(
        BindConfig::default(),
        RecvBufConfig::default(),
        RingConfig::default(),
        BatchConfig::default(),
    )
    .await;
    match &bind_res {
        Ok(t) => {
            let name_ok: Result<(), String> = if t.name().is_empty() {
                Err("empty transport name".to_string())
            } else {
                Ok(())
            };
            report.record(ConformanceCase::NameNonEmpty, name_ok);
        }
        Err(_) => {
            let stub: Result<(), &'static str> = Err("bind_udp failed, name unchecked");
            report.record(ConformanceCase::NameNonEmpty, stub);
        }
    }
    report.record(ConformanceCase::BindUdp, bind_res.map(|_| ()));

    let tcp_res = T::connect_tcp(BindConfig::default(), RingConfig::default()).await;
    report.record(ConformanceCase::ConnectTcp, tcp_res.map(|_| ()));

    report
}
