//! Compile-check `PoolAccess` + `TransportBind` resolve against a
//! pooled stand-in transport. Locks the constructor + pool-accessor
//! signatures so backends stay uniform.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use transport_core::{
    AffinityConfig, BatchConfig, BindConfig, BufferPool, PoolAccess, RecvBufConfig, RingConfig,
    SendBufConfig, TransportBind, TransportCore, TransportError,
};

struct NoopPool {
    capacity: usize,
    in_use: Arc<AtomicUsize>,
}

impl NoopPool {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            in_use: Arc::new(AtomicUsize::new(0)),
        }
    }
}

struct NoopSlab {
    buf: Vec<u8>,
    counter: Arc<AtomicUsize>,
}

impl AsRef<[u8]> for NoopSlab {
    fn as_ref(&self) -> &[u8] {
        &self.buf
    }
}

impl Drop for NoopSlab {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

impl BufferPool for NoopPool {
    type Slab = NoopSlab;

    fn acquire(&self, len: usize) -> Option<NoopSlab> {
        let prev = self.in_use.fetch_add(1, Ordering::AcqRel);
        if prev >= self.capacity {
            self.in_use.fetch_sub(1, Ordering::AcqRel);
            return None;
        }
        Some(NoopSlab {
            buf: vec![0; len],
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

struct PooledNoopTransport {
    pool: NoopPool,
}

impl TransportCore for PooledNoopTransport {
    fn name(&self) -> &'static str {
        "pooled-noop"
    }

    async fn send(&mut self, _buf: &[u8]) -> Result<(), TransportError> {
        Ok(())
    }
}

impl PoolAccess for PooledNoopTransport {
    type Pool = NoopPool;

    fn pool(&self) -> &NoopPool {
        &self.pool
    }
}

impl TransportBind for PooledNoopTransport {
    async fn bind_udp(
        _bind: BindConfig,
        _rx: RecvBufConfig,
        _tx: SendBufConfig,
        _ring: RingConfig,
        _batch: BatchConfig,
        _affinity: AffinityConfig,
    ) -> Result<Self, TransportError> {
        Ok(Self {
            pool: NoopPool::new(4),
        })
    }

    async fn connect_tcp(
        _bind: BindConfig,
        _rx: RecvBufConfig,
        _tx: SendBufConfig,
        _ring: RingConfig,
        _affinity: AffinityConfig,
    ) -> Result<Self, TransportError> {
        Ok(Self {
            pool: NoopPool::new(4),
        })
    }
}

fn takes_bind<T: TransportBind>() {}
fn takes_pool_access<T: PoolAccess>(_t: &T) {}

#[tokio::test]
async fn bind_udp_returns_owned_transport() {
    takes_bind::<PooledNoopTransport>();
    let t = PooledNoopTransport::bind_udp(
        BindConfig::default(),
        RecvBufConfig::default(),
        SendBufConfig::default(),
        RingConfig::default(),
        BatchConfig::default(),
        AffinityConfig::default(),
    )
    .await
    .expect("bind ok");
    assert_eq!(t.name(), "pooled-noop");
    takes_pool_access(&t);
    assert_eq!(t.pool().capacity(), 4);
}

#[tokio::test]
async fn connect_tcp_returns_owned_transport() {
    let t = PooledNoopTransport::connect_tcp(
        BindConfig::default(),
        RecvBufConfig::default(),
        SendBufConfig::default(),
        RingConfig::default(),
        AffinityConfig::default(),
    )
    .await
    .expect("connect ok");
    assert_eq!(t.pool().in_use(), 0);
}
