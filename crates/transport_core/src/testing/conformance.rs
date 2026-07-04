//! Runs the shared conformance suite against a backend. Each case has a
//! stable name so failures line up across backends in CI dashboards. TCP
//! peer is spun up on `127.0.0.1:0` for the duration of the suite so
//! `connect_tcp` has something to talk to.

use crate::config::{BatchConfig, BindConfig, RecvBufConfig, RingConfig, SendBufConfig};
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
        SendBufConfig::default(),
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

    let tcp_res = spin_tcp_peer_and_connect::<T>().await;
    report.record(ConformanceCase::ConnectTcp, tcp_res);

    report
}

async fn spin_tcp_peer_and_connect<T: TransportBind>() -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("conformance listener bind failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("conformance listener local_addr failed: {e}"))?;
    let accept_task = tokio::spawn(async move {
        let _ = listener.accept().await;
    });

    let bind_cfg = BindConfig {
        addr,
        ..Default::default()
    };
    let res = T::connect_tcp(
        bind_cfg,
        RecvBufConfig::default(),
        SendBufConfig::default(),
        RingConfig::default(),
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string());

    let _ = accept_task.await;
    res
}
