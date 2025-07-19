use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::thread;


/// Cache implementation for frequently accessed data
pub struct Cache<K, V> {
    data: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    max_size: usize,
    ttl: Duration,
}

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    access_count: u64,
}

impl<K, V> Cache<K, V>
where
    K: Clone + std::hash::Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Create a new cache with specified size and TTL
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl,
        }
    }

    /// Get a value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut data = self.data.write().unwrap();
        
        if let Some(entry) = data.get_mut(key) {
            if entry.created_at.elapsed() < self.ttl {
                entry.access_count += 1;
                return Some(entry.value.clone());
            } else {
                data.remove(key);
            }
        }
        
        None
    }

    /// Put a value in cache
    pub fn put(&self, key: K, value: V) {
        let mut data = self.data.write().unwrap();
        
        // Evict least recently used entries if cache is full
        if data.len() >= self.max_size {
            self.evict_lru(&mut data);
        }
        
        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            access_count: 1,
        };
        
        data.insert(key, entry);
    }

    /// Remove expired entries
    pub fn cleanup(&self) {
        let mut data = self.data.write().unwrap();
        data.retain(|_, entry| entry.created_at.elapsed() < self.ttl);
    }

    /// Evict least recently used entries
    fn evict_lru(&self, data: &mut HashMap<K, CacheEntry<V>>) {
        // Collect keys to remove
        let mut entries: Vec<_> = data.iter().map(|(k, v)| (k.clone(), v.access_count)).collect();
        entries.sort_by_key(|(_, count)| *count);
        
        // Remove 20% of least used entries
        let to_remove = (entries.len() / 5).max(1);
        for (key, _) in entries.iter().take(to_remove) {
            data.remove(key);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let data = self.data.read().unwrap();
        let total_entries = data.len();
        let total_accesses: u64 = data.values().map(|entry| entry.access_count).sum();
        
        CacheStats {
            total_entries,
            total_accesses,
            hit_rate: if total_accesses > 0 {
                total_accesses as f64 / (total_accesses + total_entries as u64) as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_accesses: u64,
    pub hit_rate: f64,
}

/// Connection pool for network operations
pub struct ConnectionPool {
    connections: Arc<Mutex<HashMap<String, PooledConnection>>>,
    max_connections: usize,
    connection_timeout: Duration,
}

struct PooledConnection {
    connection_id: String,
    created_at: Instant,
    last_used: Instant,
    is_active: bool,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_connections: usize, connection_timeout: Duration) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            max_connections,
            connection_timeout,
        }
    }

    /// Get a connection from the pool
    pub fn get_connection(&self, peer_id: &str) -> Option<String> {
        let mut connections = self.connections.lock().unwrap();
        
        // Clean up expired connections
        self.cleanup_expired(&mut connections);
        
        if let Some(conn) = connections.get_mut(peer_id) {
            if conn.is_active && conn.created_at.elapsed() < self.connection_timeout {
                conn.last_used = Instant::now();
                return Some(conn.connection_id.clone());
            }
        }
        
        None
    }

    /// Add a connection to the pool
    pub fn add_connection(&self, peer_id: String, connection_id: String) {
        let mut connections = self.connections.lock().unwrap();
        
        if connections.len() >= self.max_connections {
            self.evict_oldest(&mut connections);
        }
        
        let pooled_conn = PooledConnection {
            connection_id,
            created_at: Instant::now(),
            last_used: Instant::now(),
            is_active: true,
        };
        
        connections.insert(peer_id, pooled_conn);
    }

    /// Mark a connection as inactive
    pub fn mark_inactive(&self, peer_id: &str) {
        let mut connections = self.connections.lock().unwrap();
        if let Some(conn) = connections.get_mut(peer_id) {
            conn.is_active = false;
        }
    }

    /// Clean up expired connections
    fn cleanup_expired(&self, connections: &mut HashMap<String, PooledConnection>) {
        connections.retain(|_, conn| {
            conn.created_at.elapsed() < self.connection_timeout && conn.is_active
        });
    }

    /// Evict oldest connections
    fn evict_oldest(&self, connections: &mut HashMap<String, PooledConnection>) {
        // Collect keys to remove
        let mut entries: Vec<_> = connections.iter().map(|(k, v)| (k.clone(), v.last_used)).collect();
        entries.sort_by_key(|(_, time)| *time);
        
        // Remove 20% of oldest connections
        let to_remove = (entries.len() / 5).max(1);
        for (key, _) in entries.iter().take(to_remove) {
            connections.remove(key);
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let connections = self.connections.lock().unwrap();
        let total_connections = connections.len();
        let active_connections = connections.values().filter(|conn| conn.is_active).count();
        
        PoolStats {
            total_connections,
            active_connections,
            utilization_rate: if self.max_connections > 0 {
                total_connections as f64 / self.max_connections as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub utilization_rate: f64,
}

/// Batch processor for efficient bulk operations
pub struct BatchProcessor<T> {
    batch_size: usize,
    batch_timeout: Duration,
    processor: Arc<dyn Fn(Vec<T>) + Send + Sync>,
    current_batch: Arc<Mutex<Vec<T>>>,
    last_flush: Arc<Mutex<Instant>>,
}

impl<T> BatchProcessor<T>
where
    T: Send + Sync + 'static,
{
    /// Create a new batch processor
    pub fn new(
        batch_size: usize,
        batch_timeout: Duration,
        processor: impl Fn(Vec<T>) + Send + Sync + 'static,
    ) -> Self {
        Self {
            batch_size,
            batch_timeout,
            processor: Arc::new(processor),
            current_batch: Arc::new(Mutex::new(Vec::new())),
            last_flush: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Add an item to the current batch
    pub fn add(&self, item: T) {
        let mut batch = self.current_batch.lock().unwrap();
        batch.push(item);
        
        // Flush if batch is full
        if batch.len() >= self.batch_size {
            self.flush_batch();
        }
    }

    /// Flush the current batch
    pub fn flush(&self) {
        self.flush_batch();
    }

    /// Flush batch if timeout has elapsed
    pub fn check_timeout(&self) {
        let last_flush = *self.last_flush.lock().unwrap();
        if last_flush.elapsed() >= self.batch_timeout {
            self.flush_batch();
        }
    }

    fn flush_batch(&self) {
        let mut batch = self.current_batch.lock().unwrap();
        if !batch.is_empty() {
            let items = std::mem::replace(&mut *batch, Vec::new());
            let processor = Arc::clone(&self.processor);
            
            // Process in background thread
            thread::spawn(move || {
                processor(items);
            });
            
            *self.last_flush.lock().unwrap() = Instant::now();
        }
    }
}

/// Rate limiter for controlling operation frequency
pub struct RateLimiter {
    max_operations: usize,
    time_window: Duration,
    operations: Arc<Mutex<Vec<Instant>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_operations: usize, time_window: Duration) -> Self {
        Self {
            max_operations,
            time_window,
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if an operation is allowed
    pub fn allow(&self) -> bool {
        let mut operations = self.operations.lock().unwrap();
        
        // Remove expired operations
        let now = Instant::now();
        operations.retain(|&op_time| now.duration_since(op_time) < self.time_window);
        
        if operations.len() < self.max_operations {
            operations.push(now);
            true
        } else {
            false
        }
    }

    /// Wait until an operation is allowed
    pub async fn wait_for_permission(&self) {
        while !self.allow() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Get current rate statistics
    pub fn stats(&self) -> RateLimitStats {
        let operations = self.operations.lock().unwrap();
        let current_operations = operations.len();
        
        RateLimitStats {
            current_operations,
            max_operations: self.max_operations,
            utilization_rate: if self.max_operations > 0 {
                current_operations as f64 / self.max_operations as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub current_operations: usize,
    pub max_operations: usize,
    pub utilization_rate: f64,
}

/// Memory pool for efficient memory allocation
pub struct MemoryPool {
    pool_size: usize,
    chunk_size: usize,
    available_chunks: Arc<Mutex<Vec<Vec<u8>>>>,
    allocated_chunks: Arc<Mutex<HashMap<*const u8, Vec<u8>>>>,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(pool_size: usize, chunk_size: usize) -> Self {
        let mut available_chunks = Vec::new();
        for _ in 0..pool_size {
            available_chunks.push(vec![0u8; chunk_size]);
        }
        
        Self {
            pool_size,
            chunk_size,
            available_chunks: Arc::new(Mutex::new(available_chunks)),
            allocated_chunks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Allocate a chunk from the pool
    pub fn allocate(&self) -> Option<Vec<u8>> {
        let mut available = self.available_chunks.lock().unwrap();
        available.pop()
    }

    /// Return a chunk to the pool
    pub fn deallocate(&self, chunk: Vec<u8>) {
        if chunk.len() == self.chunk_size {
            let mut available = self.available_chunks.lock().unwrap();
            if available.len() < self.pool_size {
                available.push(chunk);
            }
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> MemoryPoolStats {
        let available = self.available_chunks.lock().unwrap();
        let allocated = self.allocated_chunks.lock().unwrap();
        
        MemoryPoolStats {
            total_chunks: self.pool_size,
            available_chunks: available.len(),
            allocated_chunks: allocated.len(),
            utilization_rate: if self.pool_size > 0 {
                allocated.len() as f64 / self.pool_size as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    pub total_chunks: usize,
    pub available_chunks: usize,
    pub allocated_chunks: usize,
    pub utilization_rate: f64,
}

/// Performance optimization manager
pub struct OptimizationManager {
    caches: Arc<Mutex<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
    connection_pools: Arc<Mutex<HashMap<String, ConnectionPool>>>,
    rate_limiters: Arc<Mutex<HashMap<String, RateLimiter>>>,
    memory_pools: Arc<Mutex<HashMap<String, MemoryPool>>>,
}

impl OptimizationManager {
    /// Create a new optimization manager
    pub fn new() -> Self {
        Self {
            caches: Arc::new(Mutex::new(HashMap::new())),
            connection_pools: Arc::new(Mutex::new(HashMap::new())),
            rate_limiters: Arc::new(Mutex::new(HashMap::new())),
            memory_pools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a cache to the manager
    pub fn add_cache<K, V>(&self, name: String, cache: Cache<K, V>)
    where
        K: 'static + Send + Sync,
        V: 'static + Send + Sync,
    {
        let mut caches = self.caches.lock().unwrap();
        caches.insert(name, Box::new(cache));
    }

    /// Add a connection pool to the manager
    pub fn add_connection_pool(&self, name: String, pool: ConnectionPool) {
        let mut pools = self.connection_pools.lock().unwrap();
        pools.insert(name, pool);
    }

    /// Add a rate limiter to the manager
    pub fn add_rate_limiter(&self, name: String, limiter: RateLimiter) {
        let mut limiters = self.rate_limiters.lock().unwrap();
        limiters.insert(name, limiter);
    }

    /// Add a memory pool to the manager
    pub fn add_memory_pool(&self, name: String, pool: MemoryPool) {
        let mut pools = self.memory_pools.lock().unwrap();
        pools.insert(name, pool);
    }

    /// Get optimization statistics
    pub fn get_stats(&self) -> OptimizationStats {
        let caches = self.caches.lock().unwrap();
        let connection_pools = self.connection_pools.lock().unwrap();
        let rate_limiters = self.rate_limiters.lock().unwrap();
        let memory_pools = self.memory_pools.lock().unwrap();
        
        OptimizationStats {
            total_caches: caches.len(),
            total_connection_pools: connection_pools.len(),
            total_rate_limiters: rate_limiters.len(),
            total_memory_pools: memory_pools.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationStats {
    pub total_caches: usize,
    pub total_connection_pools: usize,
    pub total_rate_limiters: usize,
    pub total_memory_pools: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        let cache: Cache<String, String> = Cache::new(100, Duration::from_secs(60));
        
        cache.put("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()), None);
    }

    #[test]
    fn test_connection_pool() {
        let pool = ConnectionPool::new(10, Duration::from_secs(60));
        
        pool.add_connection("peer1".to_string(), "conn1".to_string());
        assert!(pool.get_connection("peer1").is_some());
        assert!(pool.get_connection("peer2").is_none());
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));
        
        for _ in 0..5 {
            assert!(limiter.allow());
        }
        assert!(!limiter.allow());
    }

    #[test]
    fn test_memory_pool() {
        let pool = MemoryPool::new(10, 1024);
        
        let chunk1 = pool.allocate();
        assert!(chunk1.is_some());
        
        let chunk2 = pool.allocate();
        assert!(chunk2.is_some());
        
        if let Some(chunk) = chunk1 {
            pool.deallocate(chunk);
        }
    }
} 