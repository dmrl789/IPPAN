//! Advanced caching system for IPPAN
//! 
//! Provides high-performance caching, optimization strategies, and performance monitoring

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub value: T,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub access_count: u64,
    pub size_bytes: usize,
    pub ttl_seconds: Option<u64>,
    pub tags: Vec<String>,
    pub priority: CachePriority,
}

/// Cache priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CachePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: u64,
    pub total_size_bytes: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub hit_rate: f64,
    pub average_access_time_ms: f64,
    pub memory_usage_percent: f64,
    pub entries_by_priority: HashMap<CachePriority, u64>,
    pub entries_by_tag: HashMap<String, u64>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_size_bytes: u64,
    pub max_entries: u64,
    pub default_ttl_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub eviction_policy: EvictionPolicy,
    pub enable_compression: bool,
    pub enable_encryption: bool,
    pub enable_statistics: bool,
    pub enable_metrics: bool,
}

/// Eviction policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    LRU,      // Least Recently Used
    LFU,      // Least Frequently Used
    FIFO,     // First In First Out
    TTL,      // Time To Live
    Random,   // Random eviction
    Priority, // Priority-based eviction
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub operation_count: u64,
    pub average_operation_time_ms: f64,
    pub slow_operations: u64,
    pub memory_pressure_events: u64,
    pub compression_ratio: f64,
    pub cache_efficiency: f64,
}

/// Advanced cache system
pub struct AdvancedCacheSystem {
    cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<u8>>>>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
    metrics: Arc<RwLock<CacheMetrics>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    priority_queues: Arc<RwLock<HashMap<CachePriority, Vec<String>>>>,
    tag_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    compression_enabled: bool,
    encryption_enabled: bool,
}

impl AdvancedCacheSystem {
    /// Create a new advanced cache system
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats {
                total_entries: 0,
                total_size_bytes: 0,
                hit_count: 0,
                miss_count: 0,
                eviction_count: 0,
                hit_rate: 0.0,
                average_access_time_ms: 0.0,
                memory_usage_percent: 0.0,
                entries_by_priority: HashMap::new(),
                entries_by_tag: HashMap::new(),
            })),
            metrics: Arc::new(RwLock::new(CacheMetrics {
                operation_count: 0,
                average_operation_time_ms: 0.0,
                slow_operations: 0,
                memory_pressure_events: 0,
                compression_ratio: 1.0,
                cache_efficiency: 0.0,
            })),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            priority_queues: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
            compression_enabled: config.enable_compression,
            encryption_enabled: config.enable_encryption,
        }
    }

    /// Set a value in the cache
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: T,
        ttl_seconds: Option<u64>,
        tags: Vec<String>,
        priority: CachePriority,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Serialize the value
        let serialized = bincode::serialize(&value)?;
        let mut data = serialized;
        
        // Apply compression if enabled
        if self.compression_enabled {
            data = self.compress_data(&data)?;
        }
        
        // Apply encryption if enabled
        if self.encryption_enabled {
            data = self.encrypt_data(&data)?;
        }
        
        let size_bytes = data.len();
        
        // Check if we need to evict entries
        if self.should_evict(size_bytes).await {
            self.evict_entries(size_bytes).await;
        }
        
        let entry = CacheEntry {
            key: key.to_string(),
            value: data,
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
            size_bytes,
            ttl_seconds,
            tags: tags.clone(),
            priority,
        };
        
        // Store the entry
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), entry);
        
        // Update access order
        let mut access_order = self.access_order.write().await;
        access_order.push_back(key.to_string());
        
        // Update priority queues
        let mut priority_queues = self.priority_queues.write().await;
        priority_queues.entry(priority).or_insert_with(Vec::new).push(key.to_string());
        
        // Update tag index
        let mut tag_index = self.tag_index.write().await;
        for tag in tags {
            tag_index.entry(tag).or_insert_with(Vec::new).push(key.to_string());
        }
        
        // Update statistics
        self.update_stats_after_set(size_bytes).await;
        
        // Update metrics
        self.update_metrics(start_time.elapsed()).await;
        
        Ok(())
    }

    /// Get a value from the cache
    pub async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(key) {
            // Check if entry has expired
            if let Some(ttl) = entry.ttl_seconds {
                let age = Utc::now().signed_duration_since(entry.created_at);
                if age.num_seconds() > ttl as i64 {
                    cache.remove(key);
                    self.update_stats_after_miss().await;
                    return Ok(None);
                }
            }
            
            // Update access metadata
            entry.accessed_at = Utc::now();
            entry.access_count += 1;
            
            // Update access order
            let mut access_order = self.access_order.write().await;
            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }
            access_order.push_back(key.to_string());
            
            // Deserialize and return value
            let mut data = entry.value.clone();
            
            // Decrypt if enabled
            if self.encryption_enabled {
                data = self.decrypt_data(&data)?;
            }
            
            // Decompress if enabled
            if self.compression_enabled {
                data = self.decompress_data(&data)?;
            }
            
            let value: T = bincode::deserialize(&data)?;
            
            self.update_stats_after_hit().await;
            self.update_metrics(start_time.elapsed()).await;
            
            Ok(Some(value))
        } else {
            self.update_stats_after_miss().await;
            self.update_metrics(start_time.elapsed()).await;
            Ok(None)
        }
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.remove(key) {
            // Remove from access order
            let mut access_order = self.access_order.write().await;
            if let Some(pos) = access_order.iter().position(|k| k == key) {
                access_order.remove(pos);
            }
            
            // Remove from priority queues
            let mut priority_queues = self.priority_queues.write().await;
            if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                if let Some(pos) = queue.iter().position(|k| k == key) {
                    queue.remove(pos);
                }
            }
            
            // Remove from tag index
            let mut tag_index = self.tag_index.write().await;
            for tag in &entry.tags {
                if let Some(keys) = tag_index.get_mut(tag) {
                    if let Some(pos) = keys.iter().position(|k| k == key) {
                        keys.remove(pos);
                    }
                }
            }
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.total_entries = stats.total_entries.saturating_sub(1);
            stats.total_size_bytes = stats.total_size_bytes.saturating_sub(entry.size_bytes as u64);
            
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut cache = self.cache.write().await;
        cache.clear();
        
        let mut access_order = self.access_order.write().await;
        access_order.clear();
        
        let mut priority_queues = self.priority_queues.write().await;
        priority_queues.clear();
        
        let mut tag_index = self.tag_index.write().await;
        tag_index.clear();
        
        let mut stats = self.stats.write().await;
        stats.total_entries = 0;
        stats.total_size_bytes = 0;
        
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get cache metrics
    pub async fn get_metrics(&self) -> CacheMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Get entries by tag
    pub async fn get_by_tag(&self, tag: &str) -> Vec<String> {
        let tag_index = self.tag_index.read().await;
        tag_index.get(tag).cloned().unwrap_or_default()
    }

    /// Get entries by priority
    pub async fn get_by_priority(&self, priority: &CachePriority) -> Vec<String> {
        let priority_queues = self.priority_queues.read().await;
        priority_queues.get(priority).cloned().unwrap_or_default()
    }

    /// Invalidate entries by tag
    pub async fn invalidate_by_tag(&self, tag: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let keys = self.get_by_tag(tag).await;
        let mut removed_count = 0;
        
        for key in keys {
            if self.remove(&key).await? {
                removed_count += 1;
            }
        }
        
        Ok(removed_count)
    }

    /// Check if cache contains a key
    pub async fn contains(&self, key: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(key)
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Get all cache keys
    pub async fn keys(&self) -> Vec<String> {
        let cache = self.cache.read().await;
        cache.keys().cloned().collect()
    }

    /// Check if we should evict entries
    async fn should_evict(&self, new_entry_size: usize) -> bool {
        let stats = self.stats.read().await;
        let new_total_size = stats.total_size_bytes + new_entry_size as u64;
        
        new_total_size > self.config.max_size_bytes || 
        stats.total_entries >= self.config.max_entries
    }

    /// Evict entries based on policy
    async fn evict_entries(&self, required_space: usize) {
        match self.config.eviction_policy {
            EvictionPolicy::LRU => self.evict_lru(required_space).await,
            EvictionPolicy::LFU => self.evict_lfu(required_space).await,
            EvictionPolicy::FIFO => self.evict_fifo(required_space).await,
            EvictionPolicy::TTL => self.evict_ttl(required_space).await,
            EvictionPolicy::Random => self.evict_random(required_space).await,
            EvictionPolicy::Priority => self.evict_priority(required_space).await,
        }
    }

    /// Evict least recently used entries
    async fn evict_lru(&self, required_space: usize) {
        let mut access_order = self.access_order.write().await;
        let mut cache = self.cache.write().await;
        let mut evicted_size = 0;
        
        while evicted_size < required_space && !access_order.is_empty() {
            if let Some(key) = access_order.pop_front() {
                if let Some(entry) = cache.remove(&key) {
                    evicted_size += entry.size_bytes;
                    
                    // Remove from priority queues
                    let mut priority_queues = self.priority_queues.write().await;
                    if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                        if let Some(pos) = queue.iter().position(|k| k == &key) {
                            queue.remove(pos);
                        }
                    }
                    
                    // Remove from tag index
                    let mut tag_index = self.tag_index.write().await;
                    for tag in &entry.tags {
                        if let Some(keys) = tag_index.get_mut(tag) {
                            if let Some(pos) = keys.iter().position(|k| k == &key) {
                                keys.remove(pos);
                            }
                        }
                    }
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.eviction_count += 1;
    }

    /// Evict least frequently used entries
    async fn evict_lfu(&self, required_space: usize) {
        let mut cache = self.cache.write().await;
        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by_key(|(_, entry)| entry.access_count);
        
        let mut evicted_size = 0;
        let mut keys_to_remove = Vec::new();
        
        for (key, entry) in entries {
            if evicted_size >= required_space {
                break;
            }
            evicted_size += entry.size_bytes;
            keys_to_remove.push(key.clone());
        }
        
        for key in keys_to_remove {
            if let Some(entry) = cache.remove(&key) {
                // Remove from other indexes
                let mut access_order = self.access_order.write().await;
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }
                
                let mut priority_queues = self.priority_queues.write().await;
                if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                    if let Some(pos) = queue.iter().position(|k| k == &key) {
                        queue.remove(pos);
                    }
                }
                
                let mut tag_index = self.tag_index.write().await;
                for tag in &entry.tags {
                    if let Some(keys) = tag_index.get_mut(tag) {
                        if let Some(pos) = keys.iter().position(|k| k == &key) {
                            keys.remove(pos);
                        }
                    }
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.eviction_count += 1;
    }

    /// Evict first in first out entries
    async fn evict_fifo(&self, required_space: usize) {
        let mut cache = self.cache.write().await;
        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by_key(|(_, entry)| entry.created_at);
        
        let mut evicted_size = 0;
        let mut keys_to_remove = Vec::new();
        
        for (key, entry) in entries {
            if evicted_size >= required_space {
                break;
            }
            evicted_size += entry.size_bytes;
            keys_to_remove.push(key.clone());
        }
        
        for key in keys_to_remove {
            if let Some(entry) = cache.remove(&key) {
                // Remove from other indexes
                let mut access_order = self.access_order.write().await;
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }
                
                let mut priority_queues = self.priority_queues.write().await;
                if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                    if let Some(pos) = queue.iter().position(|k| k == &key) {
                        queue.remove(pos);
                    }
                }
                
                let mut tag_index = self.tag_index.write().await;
                for tag in &entry.tags {
                    if let Some(keys) = tag_index.get_mut(tag) {
                        if let Some(pos) = keys.iter().position(|k| k == &key) {
                            keys.remove(pos);
                        }
                    }
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.eviction_count += 1;
    }

    /// Evict time to live expired entries
    async fn evict_ttl(&self, _required_space: usize) {
        let mut cache = self.cache.write().await;
        let now = Utc::now();
        let mut keys_to_remove = Vec::new();
        
        for (key, entry) in cache.iter() {
            if let Some(ttl) = entry.ttl_seconds {
                let age = now.signed_duration_since(entry.created_at);
                if age.num_seconds() > ttl as i64 {
                    keys_to_remove.push(key.clone());
                }
            }
        }
        
        for key in keys_to_remove {
            if let Some(entry) = cache.remove(&key) {
                // Remove from other indexes
                let mut access_order = self.access_order.write().await;
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }
                
                let mut priority_queues = self.priority_queues.write().await;
                if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                    if let Some(pos) = queue.iter().position(|k| k == &key) {
                        queue.remove(pos);
                    }
                }
                
                let mut tag_index = self.tag_index.write().await;
                for tag in &entry.tags {
                    if let Some(keys) = tag_index.get_mut(tag) {
                        if let Some(pos) = keys.iter().position(|k| k == &key) {
                            keys.remove(pos);
                        }
                    }
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.eviction_count += 1;
    }

    /// Evict random entries
    async fn evict_random(&self, required_space: usize) {
        let mut cache = self.cache.write().await;
        let mut keys: Vec<_> = cache.keys().cloned().collect();
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        keys.shuffle(&mut thread_rng());
        
        let mut evicted_size = 0;
        let mut keys_to_remove = Vec::new();
        
        for key in keys {
            if evicted_size >= required_space {
                break;
            }
            if let Some(entry) = cache.get(&key) {
                evicted_size += entry.size_bytes;
                keys_to_remove.push(key);
            }
        }
        
        for key in keys_to_remove {
            if let Some(entry) = cache.remove(&key) {
                // Remove from other indexes
                let mut access_order = self.access_order.write().await;
                if let Some(pos) = access_order.iter().position(|k| k == &key) {
                    access_order.remove(pos);
                }
                
                let mut priority_queues = self.priority_queues.write().await;
                if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                    if let Some(pos) = queue.iter().position(|k| k == &key) {
                        queue.remove(pos);
                    }
                }
                
                let mut tag_index = self.tag_index.write().await;
                for tag in &entry.tags {
                    if let Some(keys) = tag_index.get_mut(tag) {
                        if let Some(pos) = keys.iter().position(|k| k == &key) {
                            keys.remove(pos);
                        }
                    }
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.eviction_count += 1;
    }

    /// Evict by priority (lowest priority first)
    async fn evict_priority(&self, required_space: usize) {
        let priority_order = vec![
            CachePriority::Low,
            CachePriority::Normal,
            CachePriority::High,
            CachePriority::Critical,
        ];
        
        let mut cache = self.cache.write().await;
        let mut evicted_size = 0;
        
        for priority in priority_order {
            if evicted_size >= required_space {
                break;
            }
            
            let mut keys_to_remove = Vec::new();
            for (key, entry) in cache.iter() {
                if entry.priority == priority {
                    keys_to_remove.push(key.clone());
                }
            }
            
            for key in keys_to_remove {
                if evicted_size >= required_space {
                    break;
                }
                if let Some(entry) = cache.remove(&key) {
                    evicted_size += entry.size_bytes;
                    
                    // Remove from other indexes
                    let mut access_order = self.access_order.write().await;
                    if let Some(pos) = access_order.iter().position(|k| k == &key) {
                        access_order.remove(pos);
                    }
                    
                    let mut priority_queues = self.priority_queues.write().await;
                    if let Some(queue) = priority_queues.get_mut(&entry.priority) {
                        if let Some(pos) = queue.iter().position(|k| k == &key) {
                            queue.remove(pos);
                        }
                    }
                    
                    let mut tag_index = self.tag_index.write().await;
                    for tag in &entry.tags {
                        if let Some(keys) = tag_index.get_mut(tag) {
                            if let Some(pos) = keys.iter().position(|k| k == &key) {
                                keys.remove(pos);
                            }
                        }
                    }
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.eviction_count += 1;
    }

    /// Compress data
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        use std::io::Write;
        
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }

    /// Decompress data
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use flate2::read::DeflateDecoder;
        use std::io::Read;
        
        let mut decoder = DeflateDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// Encrypt data
    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Simple XOR encryption for demonstration
        let key = b"ippan_cache_key_2024";
        let mut encrypted = Vec::new();
        
        for (i, &byte) in data.iter().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }
        
        Ok(encrypted)
    }

    /// Decrypt data
    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Simple XOR decryption for demonstration
        let key = b"ippan_cache_key_2024";
        let mut decrypted = Vec::new();
        
        for (i, &byte) in data.iter().enumerate() {
            decrypted.push(byte ^ key[i % key.len()]);
        }
        
        Ok(decrypted)
    }

    /// Update statistics after cache set
    async fn update_stats_after_set(&self, size_bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.total_entries += 1;
        stats.total_size_bytes += size_bytes as u64;
        
        // Update hit rate
        let total_requests = stats.hit_count + stats.miss_count;
        if total_requests > 0 {
            stats.hit_rate = stats.hit_count as f64 / total_requests as f64;
        }
        
        // Update memory usage
        stats.memory_usage_percent = (stats.total_size_bytes as f64 / self.config.max_size_bytes as f64) * 100.0;
    }

    /// Update statistics after cache hit
    async fn update_stats_after_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.hit_count += 1;
        
        // Update hit rate
        let total_requests = stats.hit_count + stats.miss_count;
        if total_requests > 0 {
            stats.hit_rate = stats.hit_count as f64 / total_requests as f64;
        }
    }

    /// Update statistics after cache miss
    async fn update_stats_after_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.miss_count += 1;
        
        // Update hit rate
        let total_requests = stats.hit_count + stats.miss_count;
        if total_requests > 0 {
            stats.hit_rate = stats.hit_count as f64 / total_requests as f64;
        }
    }

    /// Update performance metrics
    async fn update_metrics(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.operation_count += 1;
        
        // Update average operation time
        let total_time = metrics.average_operation_time_ms * (metrics.operation_count - 1) as f64;
        let new_total_time = total_time + duration.as_millis() as f64;
        metrics.average_operation_time_ms = new_total_time / metrics.operation_count as f64;
        
        // Track slow operations
        if duration.as_millis() > 100 {
            metrics.slow_operations += 1;
        }
        
        // Update cache efficiency
        let stats = self.stats.read().await;
        let total_requests = stats.hit_count + stats.miss_count;
        if total_requests > 0 {
            metrics.cache_efficiency = stats.hit_rate;
        }
    }
}

impl Default for AdvancedCacheSystem {
    fn default() -> Self {
        Self::new(CacheConfig {
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            max_entries: 10000,
            default_ttl_seconds: 3600, // 1 hour
            cleanup_interval_seconds: 300, // 5 minutes
            eviction_policy: EvictionPolicy::LRU,
            enable_compression: true,
            enable_encryption: false,
            enable_statistics: true,
            enable_metrics: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig {
            max_size_bytes: 1024 * 1024,
            max_entries: 1000,
            default_ttl_seconds: 3600,
            cleanup_interval_seconds: 300,
            eviction_policy: EvictionPolicy::LRU,
            enable_compression: true,
            enable_encryption: false,
            enable_statistics: true,
            enable_metrics: true,
        };
        
        let cache = AdvancedCacheSystem::new(config);
        assert_eq!(cache.size().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = AdvancedCacheSystem::default();
        
        // Set a value
        cache.set("test_key", "test_value", None, vec![], CachePriority::Normal).await.unwrap();
        
        // Get the value
        let value: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // Check cache size
        assert_eq!(cache.size().await, 1);
        assert!(!cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_remove() {
        let cache = AdvancedCacheSystem::default();
        
        // Set a value
        cache.set("test_key", "test_value", None, vec![], CachePriority::Normal).await.unwrap();
        assert_eq!(cache.size().await, 1);
        
        // Remove the value
        let removed = cache.remove("test_key").await.unwrap();
        assert!(removed);
        assert_eq!(cache.size().await, 0);
        
        // Try to remove non-existent key
        let removed = cache.remove("non_existent").await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = AdvancedCacheSystem::default();
        
        // Set multiple values
        cache.set("key1", "value1", None, vec![], CachePriority::Normal).await.unwrap();
        cache.set("key2", "value2", None, vec![], CachePriority::Normal).await.unwrap();
        assert_eq!(cache.size().await, 2);
        
        // Clear cache
        cache.clear().await.unwrap();
        assert_eq!(cache.size().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = AdvancedCacheSystem::default();
        
        // Set value with short TTL
        cache.set("test_key", "test_value", Some(1), vec![], CachePriority::Normal).await.unwrap();
        
        // Get value immediately
        let value: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Try to get expired value
        let value: Option<String> = cache.get("test_key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_tags() {
        let cache = AdvancedCacheSystem::default();
        
        // Set values with tags
        cache.set("key1", "value1", None, vec!["tag1".to_string()], CachePriority::Normal).await.unwrap();
        cache.set("key2", "value2", None, vec!["tag1".to_string(), "tag2".to_string()], CachePriority::Normal).await.unwrap();
        cache.set("key3", "value3", None, vec!["tag2".to_string()], CachePriority::Normal).await.unwrap();
        
        // Get entries by tag
        let tag1_entries = cache.get_by_tag("tag1").await;
        assert_eq!(tag1_entries.len(), 2);
        assert!(tag1_entries.contains(&"key1".to_string()));
        assert!(tag1_entries.contains(&"key2".to_string()));
        
        let tag2_entries = cache.get_by_tag("tag2").await;
        assert_eq!(tag2_entries.len(), 2);
        assert!(tag2_entries.contains(&"key2".to_string()));
        assert!(tag2_entries.contains(&"key3".to_string()));
    }

    #[tokio::test]
    async fn test_cache_priority() {
        let cache = AdvancedCacheSystem::default();
        
        // Set values with different priorities
        cache.set("key1", "value1", None, vec![], CachePriority::Low).await.unwrap();
        cache.set("key2", "value2", None, vec![], CachePriority::Normal).await.unwrap();
        cache.set("key3", "value3", None, vec![], CachePriority::High).await.unwrap();
        cache.set("key4", "value4", None, vec![], CachePriority::Critical).await.unwrap();
        
        // Get entries by priority
        let low_entries = cache.get_by_priority(&CachePriority::Low).await;
        assert_eq!(low_entries.len(), 1);
        assert!(low_entries.contains(&"key1".to_string()));
        
        let normal_entries = cache.get_by_priority(&CachePriority::Normal).await;
        assert_eq!(normal_entries.len(), 1);
        assert!(normal_entries.contains(&"key2".to_string()));
        
        let high_entries = cache.get_by_priority(&CachePriority::High).await;
        assert_eq!(high_entries.len(), 1);
        assert!(high_entries.contains(&"key3".to_string()));
        
        let critical_entries = cache.get_by_priority(&CachePriority::Critical).await;
        assert_eq!(critical_entries.len(), 1);
        assert!(critical_entries.contains(&"key4".to_string()));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = AdvancedCacheSystem::default();
        
        // Set and get values to generate stats
        cache.set("key1", "value1", None, vec![], CachePriority::Normal).await.unwrap();
        cache.set("key2", "value2", None, vec![], CachePriority::Normal).await.unwrap();
        
        let _: Option<String> = cache.get("key1").await.unwrap();
        let _: Option<String> = cache.get("key2").await.unwrap();
        let _: Option<String> = cache.get("non_existent").await.unwrap();
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.hit_count, 2);
        assert_eq!(stats.miss_count, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[tokio::test]
    async fn test_cache_metrics() {
        let cache = AdvancedCacheSystem::default();
        
        // Perform some operations
        cache.set("key1", "value1", None, vec![], CachePriority::Normal).await.unwrap();
        let _: Option<String> = cache.get("key1").await.unwrap();
        
        let metrics = cache.get_metrics().await;
        assert_eq!(metrics.operation_count, 2);
        assert!(metrics.average_operation_time_ms > 0.0);
    }
} 