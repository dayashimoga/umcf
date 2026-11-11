//! High-performance page-aligned and cache-line aligned slab buffer pool for UMCF data plane.
//! Eliminates malloc/free churn on tensor activation and shard transfers with automatic return-on-drop.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use bytes::Bytes;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Buffer pool errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BufferPoolError {
    #[error("Requested buffer size {requested} exceeds block size {capacity}")]
    SizeExceedsCapacity { requested: usize, capacity: usize },

    #[error("Buffer pool exhausted: max capacity {0} reached")]
    PoolExhausted(usize),

    #[error("Invalid alignment: {0} must be a power of two")]
    InvalidAlignment(usize),
}

/// Thread-safe slab buffer pool providing page-aligned and cacheline-aligned buffers.
#[derive(Debug, Clone)]
pub struct AlignedSlabBufferPool {
    inner: Arc<PoolInner>,
}

#[derive(Debug)]
struct PoolInner {
    block_size: usize,
    alignment: usize,
    max_buffers: usize,
    available: Mutex<Vec<Vec<u8>>>,
    allocated_count: Mutex<usize>,
}

impl AlignedSlabBufferPool {
    /// Constructs a new slab buffer pool with specified block size, byte alignment, and max buffers.
    pub fn new(block_size: usize, alignment: usize, max_buffers: usize) -> Result<Self, BufferPoolError> {
        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            return Err(BufferPoolError::InvalidAlignment(alignment));
        }

        Ok(Self {
            inner: Arc::new(PoolInner {
                block_size,
                alignment,
                max_buffers,
                available: Mutex::new(Vec::with_capacity(max_buffers)),
                allocated_count: Mutex::new(0),
            }),
        })
    }

    /// Acquires a buffer lease from the pool, reusing an existing buffer or allocating a new one.
    pub fn acquire(&self) -> Result<BufferLease, BufferPoolError> {
        let mut avail = self
            .inner
            .available
            .lock()
            .map_err(|_| BufferPoolError::PoolExhausted(self.inner.max_buffers))?;

        if let Some(buf) = avail.pop() {
            let raw_addr = buf.as_ptr() as usize;
            let offset = (self.inner.alignment - (raw_addr % self.inner.alignment)) % self.inner.alignment;

            return Ok(BufferLease {
                buffer: Some(buf),
                offset,
                block_size: self.inner.block_size,
                pool: Some(self.clone()),
            });
        }

        let mut count = self
            .inner
            .allocated_count
            .lock()
            .map_err(|_| BufferPoolError::PoolExhausted(self.inner.max_buffers))?;

        if *count >= self.inner.max_buffers {
            return Err(BufferPoolError::PoolExhausted(self.inner.max_buffers));
        }

        *count += 1;
        drop(count);
        drop(avail);

        // Allocate aligned buffer without unsafe code by over-allocating by alignment bytes
        let raw = vec![0u8; self.inner.block_size + self.inner.alignment];
        let raw_addr = raw.as_ptr() as usize;
        let offset = (self.inner.alignment - (raw_addr % self.inner.alignment)) % self.inner.alignment;

        Ok(BufferLease {
            buffer: Some(raw),
            offset,
            block_size: self.inner.block_size,
            pool: Some(self.clone()),
        })
    }

    /// Returns the number of buffers currently available in the free slab list.
    pub fn available_count(&self) -> usize {
        self.inner
            .available
            .lock()
            .map(|a| a.len())
            .unwrap_or(0)
    }

    /// Returns the total number of buffers allocated by this pool.
    pub fn total_allocated(&self) -> usize {
        self.inner
            .allocated_count
            .lock()
            .map(|c| *c)
            .unwrap_or(0)
    }

    /// Returns the configured block size.
    pub fn block_size(&self) -> usize {
        self.inner.block_size
    }

    /// Returns the configured alignment requirement.
    pub fn alignment(&self) -> usize {
        self.inner.alignment
    }

    /// Internal method to return a recycled buffer to the slab pool.
    fn return_buffer(&self, buf: Vec<u8>) {
        if buf.capacity() >= self.inner.block_size + self.inner.alignment {
            if let Ok(mut avail) = self.inner.available.lock() {
                avail.push(buf);
            }
        }
    }
}

/// RAII Lease holding a slab buffer that automatically returns to the pool on drop.
#[derive(Debug)]
pub struct BufferLease {
    buffer: Option<Vec<u8>>,
    offset: usize,
    block_size: usize,
    pool: Option<AlignedSlabBufferPool>,
}

impl BufferLease {
    /// Converts the leased buffer into an immutable `Bytes` payload while consuming the lease.
    pub fn into_bytes(mut self) -> Bytes {
        if let Some(buf) = self.buffer.take() {
            let offset = self.offset;
            let size = self.block_size;
            Bytes::from(buf).slice(offset..offset + size)
        } else {
            Bytes::new()
        }
    }

    /// Returns the raw memory address pointer of the aligned buffer slice.
    pub fn as_ptr(&self) -> *const u8 {
        self.deref().as_ptr()
    }

    /// Returns the mutable raw memory address pointer.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.deref_mut().as_mut_ptr()
    }

    /// Verifies if the underlying buffer address satisfies an alignment boundary.
    pub fn is_aligned_to(&self, alignment: usize) -> bool {
        let addr = self.as_ptr() as usize;
        addr.is_multiple_of(alignment)
    }
}

impl Deref for BufferLease {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if let Some(buf) = self.buffer.as_ref() {
            &buf[self.offset..self.offset + self.block_size]
        } else {
            &[]
        }
    }
}

impl DerefMut for BufferLease {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(buf) = self.buffer.as_mut() {
            let offset = self.offset;
            let size = self.block_size;
            &mut buf[offset..offset + size]
        } else {
            &mut []
        }
    }
}

impl Drop for BufferLease {
    fn drop(&mut self) {
        if let Some(buf) = self.buffer.take() {
            if let Some(pool) = self.pool.take() {
                pool.return_buffer(buf);
            }
        }
    }
}
