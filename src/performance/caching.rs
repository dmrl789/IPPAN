//! High-performance caching system for IPPAN
//! 
//! This module provides advanced caching strategies for optimizing
//! frequently accessed data and reducing computational overhead.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// High-performance cache with LRU eviction
pub struct HighPerformanceCache<K, V> {
    data: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    access_order: Arc<RwLock<VecDeque<K>>>,
    max_size: usize,
    hit_count: AtomicU64,
    miss_count: AtomicU64,
    eviction_count: AtomicU64,
}

struct CacheEntry<V> {
    value: V,
    last_access: Instant,
    access_count: u64,
}

impl<K, V> HighPerformanceCache<K, V>
where
    K: Clone + Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new high-performance cache
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            hit_count: AtomicU64::new(0),
            miss_count: AtomicU64::new(0),
            eviction_count: AtomicU64::new(0),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut data = self.data.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        if let Some(entry) = data.get_mut(key) {
            // Update access information
            entry.last_access = Instant::now();
            entry.access_count += 1;
            
            // Update access order
            access_order.retain(|k| k != key);
            access_order.push_back(key.clone());
            
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            Some(entry.value.clone())
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        let mut data = self.data.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        // Check if we need to evict
        if data.len() >= self.max_size && !data.contains_key(&key) {
            self.evict_lru(&mut data, &mut access_order);
        }
        
        // Insert the new entry
        let entry = CacheEntry {
            value: value.clone(),
            last_access: Instant::now(),
            access_count: 1,
        };
        
        data.insert(key.clone(), entry);
        access_order.push_back(key);
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut data = self.data.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        if let Some(entry) = data.remove(key) {
            access_order.retain(|k| k != key);
            Some(entry.value)
        } else {
            None
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        let mut access_order = self.access_order.write().unwrap();
        
        data.clear();
        access_order.clear();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let data = self.data.read().unwrap();
        let hit_count = self.hit_count.load(Ordering::Relaxed);
        let miss_count = self.miss_count.load(Ordering::Relaxed);
        let eviction_count = self.eviction_count.load(Ordering::Relaxed);
        
        let total_requests = hit_count + miss_count;
        let hit_rate = if total_requests > 0 {
            (hit_count as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        CacheStats {
            size: data.len(),
            max_size: self.max_size,
            hit_count,
            miss_count,
            eviction_count,
            hit_rate,
        }
    }

    /// Evict least recently used item
    fn evict_lru(&self, data: &mut HashMap<K, CacheEntry<V>>, access_order: &mut VecDeque<K>) {
        if let Some(key) = access_order.pop_front() {
            data.remove(&key);
            self.eviction_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub hit_rate: f64,
}

/// Time-based cache with TTL
pub struct TimeBasedCache<K, V> {
    cache: HighPerformanceCache<K, CacheValue<V>>,
    default_ttl: Duration,
}

#[derive(Clone)]
struct CacheValue<V> {
    value: V,
    expires_at: Instant,
}

impl<K, V> TimeBasedCache<K, V>
where
    K: Clone + Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new time-based cache
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: HighPerformanceCache::new(max_size),
            default_ttl,
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(cache_value) = self.cache.get(key) {
            if cache_value.expires_at > Instant::now() {
                Some(cache_value.value)
            } else {
                // Expired, remove from cache
                self.cache.remove(key);
                None
            }
        } else {
            None
        }
    }

    /// Insert a value with default TTL
    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    /// Insert a value with custom TTL
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let cache_value = CacheValue {
            value,
            expires_at: Instant::now() + ttl,
        };
        self.cache.insert(key, cache_value);
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        // This is a simplified implementation
        // In practice, you'd want to implement a more efficient cleanup strategy
        let now = Instant::now();
        let data = self.cache.data.read().unwrap();
        let expired_keys: Vec<K> = data.iter()
            .filter(|(_, entry)| entry.value.expires_at <= now)
            .map(|(key, _)| key.clone())
            .collect();
        drop(data);
        
        for key in expired_keys {
            self.cache.remove(&key);
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.cache.get_stats()
    }
}

/// Multi-level cache for different access patterns
pub struct MultiLevelCache<K, V> {
    l1_cache: HighPerformanceCache<K, V>,
    l2_cache: HighPerformanceCache<K, V>,
    l1_size: usize,
    l2_size: usize,
}

impl<K, V> MultiLevelCache<K, V>
where
    K: Clone + Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new multi-level cache
    pub fn new(l1_size: usize, l2_size: usize) -> Self {
        Self {
            l1_cache: HighPerformanceCache::new(l1_size),
            l2_cache: HighPerformanceCache::new(l2_size),
            l1_size,
            l2_size,
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        // Try L1 cache first
        if let Some(value) = self.l1_cache.get(key) {
            return Some(value);
        }
        
        // Try L2 cache
        if let Some(value) = self.l2_cache.get(key) {
            // Promote to L1 cache
            self.l1_cache.insert(key.clone(), value.clone());
            return Some(value);
        }
        
        None
    }

    /// Insert a value into the cache
    pub fn insert(&self, key: K, value: V) {
        // Insert into both caches
        self.l1_cache.insert(key.clone(), value.clone());
        self.l2_cache.insert(key, value);
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let l1_result = self.l1_cache.remove(key);
        let l2_result = self.l2_cache.remove(key);
        
        l1_result.or(l2_result)
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.l1_cache.clear();
        self.l2_cache.clear();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> MultiLevelCacheStats {
        let l1_stats = self.l1_cache.get_stats();
        let l2_stats = self.l2_cache.get_stats();
        
        MultiLevelCacheStats {
            l1_stats,
            l2_stats,
            total_hit_count: l1_stats.hit_count + l2_stats.hit_count,
            total_miss_count: l1_stats.miss_count + l2_stats.miss_count,
        }
    }
}

/// Multi-level cache statistics
#[derive(Debug, Clone)]
pub struct MultiLevelCacheStats {
    pub l1_stats: CacheStats,
    pub l2_stats: CacheStats,
    pub total_hit_count: u64,
    pub total_miss_count: u64,
}

/// Cache manager for different types of data
pub struct CacheManager {
    block_cache: HighPerformanceCache<[u8; 32], BlockData>,
    transaction_cache: HighPerformanceCache<[u8; 32], TransactionData>,
    validator_cache: TimeBasedCache<[u8; 32], ValidatorData>,
    config_cache: MultiLevelCache<String, ConfigData>,
}

#[derive(Clone)]
struct BlockData {
    block: Vec<u8>,
    timestamp: u64,
}

#[derive(Clone)]
struct TransactionData {
    transaction: Vec<u8>,
    timestamp: u64,
}

#[derive(Clone)]
struct ValidatorData {
    validator: Vec<u8>,
    stake: u64,
}

#[derive(Clone)]
struct ConfigData {
    config: Vec<u8>,
    version: u32,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            block_cache: HighPerformanceCache::new(10000),
            transaction_cache: HighPerformanceCache::new(100000),
            validator_cache: TimeBasedCache::new(1000, Duration::from_secs(300)),
            config_cache: MultiLevelCache::new(100, 1000),
        }
    }

    /// Get block data
    pub fn get_block(&self, hash: &[u8; 32]) -> Option<BlockData> {
        self.block_cache.get(hash)
    }

    /// Cache block data
    pub fn cache_block(&self, hash: [u8; 32], block: BlockData) {
        self.block_cache.insert(hash, block);
    }

    /// Get transaction data
    pub fn get_transaction(&self, hash: &[u8; 32]) -> Option<TransactionData> {
        self.transaction_cache.get(hash)
    }

    /// Cache transaction data
    pub fn cache_transaction(&self, hash: [u8; 32], transaction: TransactionData) {
        self.transaction_cache.insert(hash, transaction);
    }

    /// Get validator data
    pub fn get_validator(&self, id: &[u8; 32]) -> Option<ValidatorData> {
        self.validator_cache.get(id)
    }

    /// Cache validator data
    pub fn cache_validator(&self, id: [u8; 32], validator: ValidatorData) {
        self.validator_cache.insert(id, validator);
    }

    /// Get config data
    pub fn get_config(&self, key: &str) -> Option<ConfigData> {
        self.config_cache.get(&key.to_string())
    }

    /// Cache config data
    pub fn cache_config(&self, key: String, config: ConfigData) {
        self.config_cache.insert(key, config);
    }

    /// Get overall cache statistics
    pub fn get_stats(&self) -> CacheManagerStats {
        CacheManagerStats {
            block_stats: self.block_cache.get_stats(),
            transaction_stats: self.transaction_cache.get_stats(),
            validator_stats: self.validator_cache.get_stats(),
            config_stats: self.config_cache.get_stats(),
        }
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        self.validator_cache.cleanup_expired();
    }
}

/// Cache manager statistics
#[derive(Debug, Clone)]
pub struct CacheManagerStats {
    pub block_stats: CacheStats,
    pub transaction_stats: CacheStats,
    pub validator_stats: CacheStats,
    pub config_stats: MultiLevelCacheStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_performance_cache() {
        let cache = HighPerformanceCache::new(3);
        
        // Insert items
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        cache.insert("key3", "value3");
        
        // Test retrieval
        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));
        assert_eq!(cache.get(&"key3"), Some("value3"));
        
        // Test LRU eviction
        cache.insert("key4", "value4");
        assert_eq!(cache.get(&"key1"), None); // Should be evicted
        assert_eq!(cache.get(&"key4"), Some("value4"));
        
        // Test statistics
        let stats = cache.get_stats();
        assert!(stats.hit_count > 0);
    }

    #[test]
    fn test_time_based_cache() {
        let cache = TimeBasedCache::new(10, Duration::from_millis(100));
        
        // Insert item
        cache.insert("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_multi_level_cache() {
        let cache = MultiLevelCache::new(2, 4);
        
        // Insert items
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        cache.insert("key3", "value3");
        
        // Test retrieval
        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));
        assert_eq!(cache.get(&"key3"), Some("value3"));
        
        // Test promotion from L2 to L1
        cache.insert("key4", "value4");
        assert_eq!(cache.get(&"key1"), Some("value1")); // Should be promoted to L1
    }

    #[test]
    fn test_cache_manager() {
        let manager = CacheManager::new();
        
        // Test block caching
        let block_data = BlockData {
            block: vec![1, 2, 3],
            timestamp: 1234567890,
        };
        let block_hash = [1u8; 32];
        manager.cache_block(block_hash, block_data.clone());
        assert_eq!(manager.get_block(&block_hash), Some(block_data));
        
        // Test transaction caching
        let tx_data = TransactionData {
            transaction: vec![4, 5, 6],
            timestamp: 1234567890,
        };
        let tx_hash = [2u8; 32];
        manager.cache_transaction(tx_hash, tx_data.clone());
        assert_eq!(manager.get_transaction(&tx_hash), Some(tx_data));
        
        // Test validator caching
        let validator_data = ValidatorData {
            validator: vec![7, 8, 9],
            stake: 1000,
        };
        let validator_id = [3u8; 32];
        manager.cache_validator(validator_id, validator_data.clone());
        assert_eq!(manager.get_validator(&validator_id), Some(validator_data));
        
        // Test config caching
        let config_data = ConfigData {
            config: vec![10, 11, 12],
            version: 1,
        };
        manager.cache_config("test_config".to_string(), config_data.clone());
        assert_eq!(manager.get_config("test_config"), Some(config_data));
    }
}
