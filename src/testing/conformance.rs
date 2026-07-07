//! Runs the shared conformance suite against a backend. Each case has a
//! stable name so failures line up across backends in CI dashboards. TCP
//! peer is spun up on `127.0.0.1:0` for the duration of the suite so
//! `connect_tcp` has something to talk to.

use crate::config::{
    AffinityConfig, BatchConfig, BindConfig, RecvBufConfig, RingConfig, SendBufConfig,
};
use crate::error::TransportError;
use crate::ext::{PoolAccess, TransportBind};
use crate::pool::BufferPool;
use crate::transport::{DatagramSource, FrameBatch};

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
        AffinityConfig::default(),
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
        AffinityConfig::default(),
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string());

    let _ = accept_task.await;
    res
}

/// Stable case labels for the `DatagramSource` recv conformance suite. Names
/// line up across backends in CI dashboards, same as [`ConformanceCase`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatagramCase {
    BurstLeqMax,
    SingleIsBurstOfOne,
    DrainsToOkZero,
    PoolReclaimAfterForeignDrop,
    PoolExhaustedWhenEmpty,
}

impl DatagramCase {
    pub fn label(self) -> &'static str {
        match self {
            Self::BurstLeqMax => "burst_leq_max",
            Self::SingleIsBurstOfOne => "single_is_burst_of_one",
            Self::DrainsToOkZero => "drains_to_ok_zero",
            Self::PoolReclaimAfterForeignDrop => "pool_reclaim_after_foreign_drop",
            Self::PoolExhaustedWhenEmpty => "pool_exhausted_when_empty",
        }
    }
}

#[derive(Debug, Default)]
pub struct DatagramConformanceReport {
    pub passed: Vec<&'static str>,
    pub failed: Vec<(&'static str, String)>,
}

impl DatagramConformanceReport {
    pub fn all_passed(&self) -> bool {
        self.failed.is_empty()
    }

    fn record(&mut self, case: DatagramCase, res: Result<(), String>) {
        match res {
            Ok(()) => self.passed.push(case.label()),
            Err(e) => self.failed.push((case.label(), e)),
        }
    }
}

/// Drive a backend's [`DatagramSource`] through the recv contract, naming no
/// backend crate. `build(count)` returns a source pre-loaded with `count`
/// reap-able datagrams sharing one pool; a real backend implements it by
/// binding and having a peer send, an in-process test by canned frames. The
/// pool must have room for at least `count` slabs plus a few for the exhaustion
/// case.
///
/// Asserts: `recv_burst` count `<= max`; single recv is a burst of 1; a drained
/// source returns `Ok(0)`; a reaped frame is `Send + 'static` and its pool slab
/// is reclaimed even when the frame is dropped on another thread (the
/// single-producer ring backends satisfy this via their internal return queue,
/// drained on the next `recv_burst`); an empty pool with data pending yields
/// `PoolExhausted`, not `Ok(0)`.
pub fn run_datagram_source<T, B>(build: B) -> DatagramConformanceReport
where
    T: DatagramSource + PoolAccess,
    B: Fn(usize) -> T,
{
    let mut report = DatagramConformanceReport::default();
    report.record(DatagramCase::BurstLeqMax, case_burst_leq_max(&build));
    report.record(DatagramCase::SingleIsBurstOfOne, case_single_burst(&build));
    report.record(DatagramCase::DrainsToOkZero, case_drains_to_ok_zero(&build));
    report.record(
        DatagramCase::PoolReclaimAfterForeignDrop,
        case_pool_reclaim(&build),
    );
    report.record(
        DatagramCase::PoolExhaustedWhenEmpty,
        case_pool_exhausted(&build),
    );
    report
}

fn case_burst_leq_max<T, B>(build: &B) -> Result<(), String>
where
    T: DatagramSource + PoolAccess,
    B: Fn(usize) -> T,
{
    let mut src = build(5);
    let mut batch = FrameBatch::with_capacity(64);
    let n = src.recv_burst(&mut batch, 3).map_err(|e| e.to_string())?;
    if n > 3 {
        return Err(format!("recv_burst(max=3) returned {n} > 3"));
    }
    if batch.len() != n {
        return Err(format!(
            "batch holds {} frames, recv_burst said {n}",
            batch.len()
        ));
    }
    Ok(())
}

fn case_single_burst<T, B>(build: &B) -> Result<(), String>
where
    T: DatagramSource + PoolAccess,
    B: Fn(usize) -> T,
{
    let mut src = build(3);
    let mut batch = FrameBatch::with_capacity(64);
    let n = src.recv_burst(&mut batch, 1).map_err(|e| e.to_string())?;
    if n > 1 {
        return Err(format!(
            "single recv (max=1) returned {n}, not a burst of 1"
        ));
    }
    Ok(())
}

fn case_drains_to_ok_zero<T, B>(build: &B) -> Result<(), String>
where
    T: DatagramSource + PoolAccess,
    B: Fn(usize) -> T,
{
    let mut src = build(2);
    let mut batch = FrameBatch::with_capacity(64);
    let mut reaped = 0;
    // Drain everything, then a further reap must report nothing ready.
    for _ in 0..1024 {
        let n = src.recv_burst(&mut batch, 8).map_err(|e| e.to_string())?;
        if n > 8 {
            return Err(format!("recv_burst(max=8) returned {n} > 8"));
        }
        reaped += n;
        for _ in batch.drain() {}
        if n == 0 {
            break;
        }
    }
    if reaped < 2 {
        return Err(format!("expected to reap 2 datagrams, got {reaped}"));
    }
    let after = src.recv_burst(&mut batch, 8).map_err(|e| e.to_string())?;
    if after != 0 {
        return Err(format!("drained source returned {after}, expected Ok(0)"));
    }
    Ok(())
}

fn case_pool_reclaim<T, B>(build: &B) -> Result<(), String>
where
    T: DatagramSource + PoolAccess,
    B: Fn(usize) -> T,
{
    let mut src = build(2);
    let mut batch = FrameBatch::with_capacity(64);
    let mut frames = Vec::new();
    for _ in 0..1024 {
        let n = src.recv_burst(&mut batch, 8).map_err(|e| e.to_string())?;
        frames.extend(batch.drain());
        if frames.len() >= 2 || n == 0 {
            break;
        }
    }
    if frames.len() < 2 {
        return Err(format!("expected 2 frames, reaped {}", frames.len()));
    }
    if src.pool().in_use() == 0 {
        return Err("pool in_use stayed 0 while frames were held".to_string());
    }
    // Drop the owned frames on another thread; Send + 'static must hold.
    let handle = std::thread::spawn(move || drop(frames));
    handle
        .join()
        .map_err(|_| "drop thread panicked".to_string())?;
    // Let a deferred return queue (single-producer ring backends) drain.
    let _ = src.recv_burst(&mut batch, 8).map_err(|e| e.to_string())?;
    for _ in batch.drain() {}
    if src.pool().in_use() != 0 {
        return Err(format!(
            "pool in_use = {} after frames dropped, expected 0",
            src.pool().in_use()
        ));
    }
    Ok(())
}

fn case_pool_exhausted<T, B>(build: &B) -> Result<(), String>
where
    T: DatagramSource + PoolAccess,
    B: Fn(usize) -> T,
{
    let mut src = build(2);
    // Exhaust the pool by holding every slab, so recv has data but no landing.
    let mut held = Vec::new();
    while let Some(slab) = src.pool().acquire(1) {
        held.push(slab);
        if held.len() > src.pool().capacity() {
            return Err("pool acquire never returned None at capacity".to_string());
        }
    }
    let mut batch = FrameBatch::with_capacity(64);
    match src.recv_burst(&mut batch, 8) {
        Err(TransportError::PoolExhausted { .. }) => {
            drop(held);
            Ok(())
        }
        Err(other) => Err(format!("expected PoolExhausted, got {other}")),
        Ok(n) => Err(format!("expected PoolExhausted, got Ok({n})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{AsPayload, TransportCore};
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockPool {
        capacity: usize,
        in_use: Arc<AtomicUsize>,
    }

    struct MockSlab {
        counter: Arc<AtomicUsize>,
    }

    impl AsRef<[u8]> for MockSlab {
        // Content lives on the frame; the slab is the pool-accounting handle.
        fn as_ref(&self) -> &[u8] {
            &[]
        }
    }

    impl Drop for MockSlab {
        fn drop(&mut self) {
            self.counter.fetch_sub(1, Ordering::AcqRel);
        }
    }

    impl BufferPool for MockPool {
        type Slab = MockSlab;

        fn acquire(&self, _len: usize) -> Option<MockSlab> {
            let prev = self.in_use.fetch_add(1, Ordering::AcqRel);
            if prev >= self.capacity {
                self.in_use.fetch_sub(1, Ordering::AcqRel);
                return None;
            }
            Some(MockSlab {
                counter: self.in_use.clone(),
            })
        }

        fn capacity(&self) -> usize {
            self.capacity
        }

        fn in_use(&self) -> usize {
            self.in_use.load(Ordering::Acquire)
        }
    }

    struct MockFrame {
        _slab: MockSlab,
        bytes: Vec<u8>,
    }

    impl AsPayload for MockFrame {
        fn payload(&self) -> &[u8] {
            &self.bytes
        }
        fn sequence(&self) -> u64 {
            0
        }
        fn stream_id(&self) -> u8 {
            0
        }
    }

    struct MockSource {
        pending: VecDeque<Vec<u8>>,
        pool: MockPool,
    }

    impl TransportCore for MockSource {
        fn name(&self) -> &'static str {
            "mock-datagram-source"
        }
        async fn send(&mut self, _buf: &[u8]) -> Result<(), TransportError> {
            Ok(())
        }
    }

    impl PoolAccess for MockSource {
        type Pool = MockPool;
        fn pool(&self) -> &MockPool {
            &self.pool
        }
    }

    impl DatagramSource for MockSource {
        type Frame = MockFrame;

        fn recv_burst(
            &mut self,
            out: &mut FrameBatch<Self::Frame>,
            max: usize,
        ) -> Result<usize, TransportError> {
            let cap = max.min(out.spare());
            let mut n = 0;
            while n < cap {
                if self.pending.is_empty() {
                    break;
                }
                match self.pool.acquire(1) {
                    Some(slab) => {
                        let bytes = self.pending.pop_front().unwrap();
                        out.push(MockFrame { _slab: slab, bytes });
                        n += 1;
                    }
                    None => {
                        if n == 0 {
                            return Err(TransportError::PoolExhausted {
                                in_use: self.pool.in_use(),
                                capacity: self.pool.capacity(),
                            });
                        }
                        break;
                    }
                }
            }
            Ok(n)
        }
    }

    fn build(count: usize) -> MockSource {
        MockSource {
            pending: (0..count).map(|i| vec![i as u8; 4]).collect(),
            pool: MockPool {
                capacity: 8,
                in_use: Arc::new(AtomicUsize::new(0)),
            },
        }
    }

    #[test]
    fn mock_datagram_source_passes_conformance() {
        let report = run_datagram_source(build);
        assert!(
            report.all_passed(),
            "conformance failures: {:?}",
            report.failed
        );
        assert_eq!(report.passed.len(), 5);
    }
}
