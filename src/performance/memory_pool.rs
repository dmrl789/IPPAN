//! Memory pool for zero-copy operations and reduced allocations
//! 
//! This module provides memory pools for efficient memory management
//! in high-throughput scenarios.

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;
use std::alloc::{alloc, dealloc, Layout};
use std::sync::Arc;

/// Memory pool for efficient allocation and deallocation
pub struct MemoryPool<T> {
    blocks: Vec<AtomicPtr<PoolBlock<T>>>,
    free_list: AtomicPtr<PoolBlock<T>>,
    block_size: usize,
    total_blocks: AtomicUsize,
    allocated_blocks: AtomicUsize,
}

struct PoolBlock<T> {
    data: T,
    next: AtomicPtr<PoolBlock<T>>,
    is_allocated: AtomicUsize,
}

impl<T> MemoryPool<T> {
    /// Create a new memory pool
    pub fn new(initial_capacity: usize) -> Self {
        let mut blocks = Vec::with_capacity(initial_capacity);
        let mut free_list = ptr::null_mut();
        
        // Pre-allocate blocks
        for _ in 0..initial_capacity {
            let block = Box::into_raw(Box::new(PoolBlock {
                data: unsafe { std::mem::MaybeUninit::<T>::uninit().assume_init() },
                next: AtomicPtr::new(ptr::null_mut()),
                is_allocated: AtomicUsize::new(0),
            }));
            
            unsafe {
                (*block).next.store(free_list, Ordering::Relaxed);
            }
            free_list = block;
            blocks.push(AtomicPtr::new(block));
        }
        
        Self {
            blocks,
            free_list: AtomicPtr::new(free_list),
            block_size: std::mem::size_of::<T>(),
            total_blocks: AtomicUsize::new(initial_capacity),
            allocated_blocks: AtomicUsize::new(0),
        }
    }

    /// Allocate a block from the pool
    pub fn allocate(&self) -> Option<PooledItem<T>> {
        loop {
            let current_free = self.free_list.load(Ordering::Acquire);
            if current_free.is_null() {
                // No free blocks available
                return None;
            }

            unsafe {
                let next_free = (*current_free).next.load(Ordering::Acquire);
                
                // Try to claim this block
                if (*current_free).is_allocated.compare_exchange_weak(
                    0,
                    1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ).is_ok() {
                    // Successfully claimed the block
                    self.free_list.store(next_free, Ordering::Release);
                    self.allocated_blocks.fetch_add(1, Ordering::Relaxed);
                    
                    return Some(PooledItem {
                        block: current_free,
                        pool: self,
                    });
                } else {
                    // Block was already allocated, try next
                    self.free_list.store(next_free, Ordering::Release);
                    continue;
                }
            }
        }
    }

    /// Allocate multiple blocks at once
    pub fn allocate_batch(&self, count: usize) -> Result<Vec<PooledItem<T>>, String> {
        let mut items = Vec::with_capacity(count);
        
        for _ in 0..count {
            if let Some(item) = self.allocate() {
                items.push(item);
            } else {
                // Not enough free blocks, return error
                return Err(format!("Not enough free blocks in pool. Requested: {}, Available: {}", 
                    count, self.available_blocks()));
            }
        }
        
        Ok(items)
    }

    /// Return a block to the pool
    fn deallocate(&self, block: *mut PoolBlock<T>) {
        unsafe {
            // Mark block as free
            (*block).is_allocated.store(0, Ordering::Release);
            
            // Add to free list
            loop {
                let current_free = self.free_list.load(Ordering::Acquire);
                (*block).next.store(current_free, Ordering::Relaxed);
                
                match self.free_list.compare_exchange_weak(
                    current_free,
                    block,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            }
        }
        
        self.allocated_blocks.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get the number of available blocks
    pub fn available_blocks(&self) -> usize {
        self.total_blocks.load(Ordering::Relaxed) - self.allocated_blocks.load(Ordering::Relaxed)
    }

    /// Get the number of allocated blocks
    pub fn allocated_blocks(&self) -> usize {
        self.allocated_blocks.load(Ordering::Relaxed)
    }

    /// Get the total number of blocks
    pub fn total_blocks(&self) -> usize {
        self.total_blocks.load(Ordering::Relaxed)
    }
}

/// A pooled item that automatically returns to the pool when dropped
pub struct PooledItem<T> {
    block: *mut PoolBlock<T>,
    pool: *const MemoryPool<T>,
}

unsafe impl<T: Send> Send for PooledItem<T> {}
unsafe impl<T: Send> Sync for PooledItem<T> {}

impl<T> PooledItem<T> {
    /// Get a mutable reference to the data
    pub fn data_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.block).data }
    }

    /// Get a reference to the data
    pub fn data(&self) -> &T {
        unsafe { &(*self.block).data }
    }

    /// Consume the item and return the data
    pub fn into_data(self) -> T {
        let data = unsafe { ptr::read(&(*self.block).data) };
        std::mem::forget(self); // Prevent deallocation
        data
    }
}

impl<T> Drop for PooledItem<T> {
    fn drop(&mut self) {
        unsafe {
            (*self.pool).deallocate(self.block);
        }
    }
}

/// Thread-safe memory pool using Arc
pub type SharedMemoryPool<T> = Arc<MemoryPool<T>>;

/// Memory pool manager for different types
pub struct MemoryPoolManager {
    transaction_pool: SharedMemoryPool<TransactionBlock>,
    block_pool: SharedMemoryPool<BlockBlock>,
    message_pool: SharedMemoryPool<MessageBlock>,
}

struct TransactionBlock {
    data: Vec<u8>,
    timestamp: u64,
    signature: Vec<u8>,
}

struct BlockBlock {
    data: Vec<u8>,
    hash: [u8; 32],
    timestamp: u64,
}

struct MessageBlock {
    data: Vec<u8>,
    message_type: u8,
    timestamp: u64,
}

impl MemoryPoolManager {
    /// Create a new memory pool manager
    pub fn new(transaction_capacity: usize, block_capacity: usize, message_capacity: usize) -> Self {
        Self {
            transaction_pool: Arc::new(MemoryPool::new(transaction_capacity)),
            block_pool: Arc::new(MemoryPool::new(block_capacity)),
            message_pool: Arc::new(MemoryPool::new(message_capacity)),
        }
    }

    /// Get a transaction from the pool
    pub fn get_transaction(&self) -> Option<PooledItem<TransactionBlock>> {
        self.transaction_pool.allocate()
    }

    /// Get a block from the pool
    pub fn get_block(&self) -> Option<PooledItem<BlockBlock>> {
        self.block_pool.allocate()
    }

    /// Get a message from the pool
    pub fn get_message(&self) -> Option<PooledItem<MessageBlock>> {
        self.message_pool.allocate()
    }

    /// Get multiple transactions from the pool
    pub fn get_transactions(&self, count: usize) -> Result<Vec<PooledItem<TransactionBlock>>, String> {
        self.transaction_pool.allocate_batch(count)
    }

    /// Get multiple blocks from the pool
    pub fn get_blocks(&self, count: usize) -> Result<Vec<PooledItem<BlockBlock>>, String> {
        self.block_pool.allocate_batch(count)
    }

    /// Get multiple messages from the pool
    pub fn get_messages(&self, count: usize) -> Result<Vec<PooledItem<MessageBlock>>, String> {
        self.message_pool.allocate_batch(count)
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        PoolStats {
            transaction_pool: PoolTypeStats {
                total: self.transaction_pool.total_blocks(),
                allocated: self.transaction_pool.allocated_blocks(),
                available: self.transaction_pool.available_blocks(),
            },
            block_pool: PoolTypeStats {
                total: self.block_pool.total_blocks(),
                allocated: self.block_pool.allocated_blocks(),
                available: self.block_pool.available_blocks(),
            },
            message_pool: PoolTypeStats {
                total: self.message_pool.total_blocks(),
                allocated: self.message_pool.allocated_blocks(),
                available: self.message_pool.available_blocks(),
            },
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub transaction_pool: PoolTypeStats,
    pub block_pool: PoolTypeStats,
    pub message_pool: PoolTypeStats,
}

/// Statistics for a specific pool type
#[derive(Debug, Clone)]
pub struct PoolTypeStats {
    pub total: usize,
    pub allocated: usize,
    pub available: usize,
}

/// Zero-copy buffer for efficient data handling
pub struct ZeroCopyBuffer {
    data: Vec<u8>,
    position: usize,
    capacity: usize,
}

impl ZeroCopyBuffer {
    /// Create a new zero-copy buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0; capacity],
            position: 0,
            capacity,
        }
    }

    /// Write data to the buffer
    pub fn write(&mut self, data: &[u8]) -> Result<(), String> {
        if self.position + data.len() > self.capacity {
            return Err("Buffer overflow".to_string());
        }
        
        self.data[self.position..self.position + data.len()].copy_from_slice(data);
        self.position += data.len();
        Ok(())
    }

    /// Read data from the buffer
    pub fn read(&mut self, len: usize) -> Result<&[u8], String> {
        if self.position + len > self.capacity {
            return Err("Buffer underflow".to_string());
        }
        
        let start = self.position;
        self.position += len;
        Ok(&self.data[start..start + len])
    }

    /// Get the current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity - self.position
    }

    /// Reset the buffer position
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Get the underlying data as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.position]
    }

    /// Get the underlying data as a mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.position]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool() {
        let pool = MemoryPool::new(10);
        
        assert_eq!(pool.total_blocks(), 10);
        assert_eq!(pool.allocated_blocks(), 0);
        assert_eq!(pool.available_blocks(), 10);
        
        // Allocate a block
        let mut item = pool.allocate().unwrap();
        *item.data_mut() = 42;
        
        assert_eq!(pool.allocated_blocks(), 1);
        assert_eq!(pool.available_blocks(), 9);
        assert_eq!(*item.data(), 42);
        
        // Drop the item
        drop(item);
        
        assert_eq!(pool.allocated_blocks(), 0);
        assert_eq!(pool.available_blocks(), 10);
    }

    #[test]
    fn test_memory_pool_batch() {
        let pool: MemoryPool<u8> = MemoryPool::new(5);
        
        let items = pool.allocate_batch(3).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(pool.allocated_blocks(), 3);
        
        drop(items);
        assert_eq!(pool.allocated_blocks(), 0);
    }

    #[test]
    fn test_zero_copy_buffer() {
        let mut buffer = ZeroCopyBuffer::new(100);
        
        // Write data
        buffer.write(b"hello").unwrap();
        buffer.write(b"world").unwrap();
        
        assert_eq!(buffer.position(), 10);
        assert_eq!(buffer.remaining(), 90);
        
        // Reset and read
        buffer.reset();
        let data1 = buffer.read(5).unwrap().to_vec();
        let data2 = buffer.read(5).unwrap().to_vec();
        
        assert_eq!(data1, b"hello");
        assert_eq!(data2, b"world");
    }

    #[test]
    fn test_memory_pool_manager() {
        let manager = MemoryPoolManager::new(100, 50, 200);
        
        let stats = manager.get_stats();
        assert_eq!(stats.transaction_pool.total, 100);
        assert_eq!(stats.block_pool.total, 50);
        assert_eq!(stats.message_pool.total, 200);
        
        // Get items from different pools
        let transaction = manager.get_transaction().unwrap();
        let block = manager.get_block().unwrap();
        let message = manager.get_message().unwrap();
        
        let stats_after = manager.get_stats();
        assert_eq!(stats_after.transaction_pool.allocated, 1);
        assert_eq!(stats_after.block_pool.allocated, 1);
        assert_eq!(stats_after.message_pool.allocated, 1);
    }
}
