//! `BufferPool` contract check: acquire returns `None` once saturated,
//! drop returns the slot to the free list. Lock the owned-handle
//! ergonomics protocol code depends on.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use transport_core::BufferPool;

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

#[test]
fn acquire_returns_none_when_saturated() {
    let pool = NoopPool::new(2);
    let a = pool.acquire(64).expect("first slot free");
    let b = pool.acquire(64).expect("second slot free");
    assert_eq!(pool.in_use(), 2);
    assert!(pool.acquire(64).is_none(), "pool must be saturated");
    drop(a);
    assert_eq!(pool.in_use(), 1);
    let c = pool.acquire(64).expect("slot returned after drop");
    assert_eq!(pool.in_use(), 2);
    drop(b);
    drop(c);
    assert_eq!(pool.in_use(), 0);
}

#[test]
fn slab_exposes_bytes_via_as_ref() {
    let pool = NoopPool::new(1);
    let slab = pool.acquire(4).expect("slot free");
    assert_eq!(slab.as_ref().len(), 4);
    assert_eq!(slab.as_ref(), &[0, 0, 0, 0]);
}
