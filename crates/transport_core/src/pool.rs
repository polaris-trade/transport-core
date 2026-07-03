//! `BufferPool` contract. Slab is an owned handle so it can cross `.await`
//! points, live in reassembler slots, or sit in per-stream buckets without
//! borrow-checker acrobatics.

use std::sync::Arc;

pub trait BufferPool: Send + Sync {
    type Slab: AsRef<[u8]> + Send + 'static;

    fn acquire(&self, len: usize) -> Option<Self::Slab>;
    fn capacity(&self) -> usize;
    fn in_use(&self) -> usize;
}

pub type SharedPool<P> = Arc<P>;
