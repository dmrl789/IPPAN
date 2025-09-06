//! Performance Integration Tests for IPPAN
//! 
//! Tests the complete performance optimization system integration

use crate::{
    performance::{
        PerformanceManager, PerformanceConfig,
        lockfree::{LockFreeHashMap, LockFreeQueue, LockFreeStack},
        memory_pool::{MemoryPool, PooledItem},
        batch_processor::{BatchProcessor, ParallelBatchProcessor, StreamingBatchProcessor},
        serialization::{HighPerformanceSerializer, BatchSerializer},
        caching::{HighPerformanceCache, TimeBasedCache, MultiLevelCache, CacheManager},
        metrics::PerformanceMetrics,
    },
    consensus::{Block, Transaction},
    storage::StorageOrchestrator,
    network::NetworkManager,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

/// Performance integration test configuration
pub struct PerformanceIntegrationConfig {
    pub performance_config: PerformanceConfig,
    pub test_duration: Duration,
    pub batch_size: usize,
    pub thread_count: usize,
    pub cache_size: usize,
    pub memory_pool_size: usize,
}

impl Default for PerformanceIntegrationConfig {
    fn default() -> Self {
        Self {
            performance_config: PerformanceConfig::default(),
            test_duration: Duration::from_secs(30),
            batch_size: 1000,
            thread_count: 8,
            cache_size: 10000,
            memory_pool_size: 1024 * 1024, // 1MB
        }
    }
}

/// Performance integration test suite
pub struct PerformanceIntegrationTestSuite {
    config: PerformanceIntegrationConfig,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl PerformanceIntegrationTestSuite {
    /// Create a new performance integration test suite
    pub fn new(config: PerformanceIntegrationConfig) -> Self {
        let metrics = Arc::new(RwLock::new(PerformanceMetrics::new()));
        Self { config, metrics }
    }

    /// Run all performance integration tests
    pub async fn run_all_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("🚀 Starting performance integration test suite...");

        // Test lock-free data structures
        self.test_lockfree_structures().await?;
        log::info!("✅ Lock-free data structures tests passed");

        // Test memory pooling
        self.test_memory_pooling().await?;
        log::info!("✅ Memory pooling tests passed");

        // Test batch processing
        self.test_batch_processing().await?;
        log::info!("✅ Batch processing tests passed");

        // Test high-performance serialization
        self.test_serialization().await?;
        log::info!("✅ High-performance serialization tests passed");

        // Test multi-level caching
        self.test_caching().await?;
        log::info!("✅ Multi-level caching tests passed");

        // Test performance manager integration
        self.test_performance_manager().await?;
        log::info!("✅ Performance manager integration tests passed");

        // Test end-to-end performance
        self.test_end_to_end_performance().await?;
        log::info!("✅ End-to-end performance tests passed");

        // Test performance metrics
        self.test_performance_metrics().await?;
        log::info!("✅ Performance metrics tests passed");

        log::info!("🎉 All performance integration tests passed!");
        Ok(())
    }

    /// Test lock-free data structures integration
    async fn test_lockfree_structures(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing lock-free data structures...");

        // Test LockFreeHashMap
        let map = LockFreeHashMap::<String, String>::new();
        
        // Concurrent insertions
        let handles: Vec<_> = (0..self.config.thread_count)
            .map(|i| {
                let map = map.clone();
                tokio::spawn(async move {
                    for j in 0..100 {
                        let key = format!("key_{}_{}", i, j);
                        let value = format!("value_{}_{}", i, j);
                        map.insert(key, value);
                    }
                })
            })
            .collect();

        // Wait for all insertions
        for handle in handles {
            handle.await?;
        }

        // Verify data integrity
        assert_eq!(map.len(), self.config.thread_count * 100);

        // Test LockFreeQueue
        let queue = LockFreeQueue::<i32>::new();
        
        // Concurrent enqueue operations
        let enqueue_handles: Vec<_> = (0..self.config.thread_count)
            .map(|i| {
                let queue = queue.clone();
                tokio::spawn(async move {
                    for j in 0..100 {
                        queue.enqueue(i * 100 + j);
                    }
                })
            })
            .collect();

        // Wait for all enqueue operations
        for handle in enqueue_handles {
            handle.await?;
        }

        // Verify queue size
        assert_eq!(queue.len(), self.config.thread_count * 100);

        // Test LockFreeStack
        let stack = LockFreeStack::<i32>::new();
        
        // Concurrent push operations
        let push_handles: Vec<_> = (0..self.config.thread_count)
            .map(|i| {
                let stack = stack.clone();
                tokio::spawn(async move {
                    for j in 0..100 {
                        stack.push(i * 100 + j);
                    }
                })
            })
            .collect();

        // Wait for all push operations
        for handle in push_handles {
            handle.await?;
        }

        // Verify stack size
        assert_eq!(stack.len(), self.config.thread_count * 100);

        Ok(())
    }

    /// Test memory pooling integration
    async fn test_memory_pooling(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing memory pooling...");

        // Create memory pool
        let pool = MemoryPool::<Vec<u8>>::new(self.config.memory_pool_size, 1000);

        // Test concurrent allocations
        let handles: Vec<_> = (0..self.config.thread_count)
            .map(|i| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let mut items = Vec::new();
                    for j in 0..100 {
                        let data = vec![i as u8; 1024];
                        let item = pool.allocate(data);
                        items.push(item);
                    }
                    items
                })
            })
            .collect();

        // Wait for all allocations
        let mut all_items = Vec::new();
        for handle in handles {
            let items = handle.await?;
            all_items.extend(items);
        }

        // Verify allocations
        assert_eq!(all_items.len(), self.config.thread_count * 100);

        // Test deallocations
        for item in all_items {
            pool.deallocate(item);
        }

        // Verify pool state
        let stats = pool.get_stats();
        assert!(stats.allocated_blocks >= 0);

        Ok(())
    }

    /// Test batch processing integration
    async fn test_batch_processing(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing batch processing...");

        // Test BatchProcessor
        let processor = BatchProcessor::new(self.config.batch_size, self.config.thread_count);
        
        // Create test data
        let items: Vec<i32> = (0..10000).collect();
        
        // Process batch
        let start_time = Instant::now();
        let results = processor.process_batch(items.clone(), |item| {
            item * 2
        }).await?;
        let duration = start_time.elapsed();

        // Verify results
        assert_eq!(results.len(), 10000);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(*result, i as i32 * 2);
        }

        log::info!("Batch processing completed in {:?}", duration);

        // Test ParallelBatchProcessor
        let parallel_processor = ParallelBatchProcessor::new(self.config.batch_size, self.config.thread_count);
        
        let start_time = Instant::now();
        let parallel_results = parallel_processor.process_batch_parallel(items.clone(), |item| {
            item * 3
        }).await?;
        let parallel_duration = start_time.elapsed();

        // Verify parallel results
        assert_eq!(parallel_results.len(), 10000);
        for (i, result) in parallel_results.iter().enumerate() {
            assert_eq!(*result, i as i32 * 3);
        }

        log::info!("Parallel batch processing completed in {:?}", parallel_duration);

        // Test StreamingBatchProcessor
        let streaming_processor = StreamingBatchProcessor::new(self.config.batch_size, self.config.thread_count);
        
        let start_time = Instant::now();
        streaming_processor.start_streaming(|batch| {
            for item in batch {
                let _ = item * 4;
            }
        }).await;
        let streaming_duration = start_time.elapsed();

        log::info!("Streaming batch processing completed in {:?}", streaming_duration);

        Ok(())
    }

    /// Test high-performance serialization integration
    async fn test_serialization(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing high-performance serialization...");

        // Test HighPerformanceSerializer
        let serializer = HighPerformanceSerializer::new();
        
        // Create test data
        let test_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        // Serialize
        let start_time = Instant::now();
        let serialized = serializer.serialize(&test_data)?;
        let serialize_duration = start_time.elapsed();

        // Deserialize
        let start_time = Instant::now();
        let deserialized: Vec<i32> = serializer.deserialize(&serialized)?;
        let deserialize_duration = start_time.elapsed();

        // Verify data integrity
        assert_eq!(test_data, deserialized);

        log::info!("Serialization completed in {:?}", serialize_duration);
        log::info!("Deserialization completed in {:?}", deserialize_duration);

        // Test BatchSerializer
        let batch_serializer = BatchSerializer::new();
        
        // Create batch data
        let batch_data: Vec<Vec<i32>> = (0..1000)
            .map(|i| vec![i, i + 1, i + 2])
            .collect();
        
        // Serialize batch
        let start_time = Instant::now();
        let serialized_batch = batch_serializer.serialize_batch(&batch_data)?;
        let batch_serialize_duration = start_time.elapsed();

        // Deserialize batch
        let start_time = Instant::now();
        let deserialized_batch: Vec<Vec<i32>> = batch_serializer.deserialize_batch(&serialized_batch)?;
        let batch_deserialize_duration = start_time.elapsed();

        // Verify batch data integrity
        assert_eq!(batch_data, deserialized_batch);

        log::info!("Batch serialization completed in {:?}", batch_serialize_duration);
        log::info!("Batch deserialization completed in {:?}", batch_deserialize_duration);

        Ok(())
    }

    /// Test multi-level caching integration
    async fn test_caching(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing multi-level caching...");

        // Test HighPerformanceCache
        let cache = HighPerformanceCache::<String, String>::new(self.config.cache_size);
        
        // Concurrent cache operations
        let handles: Vec<_> = (0..self.config.thread_count)
            .map(|i| {
                let cache = cache.clone();
                tokio::spawn(async move {
                    for j in 0..100 {
                        let key = format!("key_{}_{}", i, j);
                        let value = format!("value_{}_{}", i, j);
                        cache.insert(key.clone(), value.clone());
                        
                        // Verify insertion
                        let retrieved = cache.get(&key);
                        assert_eq!(retrieved, Some(value));
                    }
                })
            })
            .collect();

        // Wait for all cache operations
        for handle in handles {
            handle.await?;
        }

        // Test TimeBasedCache
        let time_cache = TimeBasedCache::<String, String>::new(Duration::from_secs(60));
        
        // Insert with TTL
        time_cache.insert("test_key".to_string(), "test_value".to_string());
        
        // Verify insertion
        let retrieved = time_cache.get("test_key");
        assert_eq!(retrieved, Some("test_value".to_string()));

        // Test MultiLevelCache
        let multi_cache = MultiLevelCache::<String, String>::new(100, 1000);
        
        // Insert into multi-level cache
        multi_cache.insert("multi_key".to_string(), "multi_value".to_string());
        
        // Verify insertion
        let retrieved = multi_cache.get("multi_key");
        assert_eq!(retrieved, Some("multi_value".to_string()));

        // Test CacheManager
        let mut manager = CacheManager::new();
        manager.add_cache("test_cache", cache);
        
        // Test cache manager operations
        let value = manager.get("test_cache", "test_key");
        assert!(value.is_some());

        Ok(())
    }

    /// Test performance manager integration
    async fn test_performance_manager(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing performance manager integration...");

        // Create performance manager
        let mut manager = PerformanceManager::new(self.config.performance_config.clone());

        // Test transaction processing
        let transactions: Vec<Transaction> = (0..1000)
            .map(|i| {
                Transaction::new(
                    [i as u8; 32],
                    1000,
                    [(i + 1) as u8; 32],
                    crate::consensus::HashTimer::with_ippan_time(
                        [i as u8; 32],
                        [(i + 1) as u8; 32],
                        i as u64,
                    ),
                )
            })
            .collect();

        // Process transactions
        let start_time = Instant::now();
        let processed_transactions = manager.process_transactions(transactions).await?;
        let duration = start_time.elapsed();

        // Verify processing
        assert_eq!(processed_transactions.len(), 1000);

        log::info!("Transaction processing completed in {:?}", duration);

        // Test block processing
        let blocks: Vec<Block> = (0..100)
            .map(|i| {
                Block::new(
                    [i as u8; 32],
                    vec![],
                    [i as u8; 32],
                    i as u64,
                )
            })
            .collect();

        // Process blocks
        let start_time = Instant::now();
        let processed_blocks = manager.process_blocks(blocks).await?;
        let duration = start_time.elapsed();

        // Verify processing
        assert_eq!(processed_blocks.len(), 100);

        log::info!("Block processing completed in {:?}", duration);

        // Test performance metrics
        let metrics = manager.get_metrics().await;
        assert!(metrics.transactions_processed > 0);
        assert!(metrics.blocks_processed > 0);

        Ok(())
    }

    /// Test end-to-end performance integration
    async fn test_end_to_end_performance(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing end-to-end performance...");

        // Create performance manager
        let mut manager = PerformanceManager::new(self.config.performance_config.clone());

        // Create test data
        let transactions: Vec<Transaction> = (0..10000)
            .map(|i| {
                Transaction::new(
                    [i as u8; 32],
                    1000,
                    [(i + 1) as u8; 32],
                    crate::consensus::HashTimer::with_ippan_time(
                        [i as u8; 32],
                        [(i + 1) as u8; 32],
                        i as u64,
                    ),
                )
            })
            .collect();

        // Test high-throughput processing
        let start_time = Instant::now();
        let processed_transactions = manager.process_transactions(transactions).await?;
        let duration = start_time.elapsed();

        // Calculate TPS
        let tps = processed_transactions.len() as f64 / duration.as_secs_f64();
        log::info!("Achieved TPS: {:.2}", tps);

        // Verify TPS target (should be > 1000 TPS for basic test)
        assert!(tps > 1000.0, "TPS too low: {:.2}", tps);

        // Test memory efficiency
        let metrics = manager.get_metrics().await;
        assert!(metrics.memory_usage > 0);

        // Test cache performance
        let cache_stats = manager.get_cache_stats().await;
        assert!(cache_stats.hits > 0);

        Ok(())
    }

    /// Test performance metrics integration
    async fn test_performance_metrics(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing performance metrics...");

        let metrics = self.metrics.clone();

        // Record various metrics
        for i in 0..1000 {
            metrics.write().await.record_transaction_processed();
            if i % 100 == 0 {
                metrics.write().await.record_block_processed();
            }
            if i % 200 == 0 {
                metrics.write().await.record_error(crate::performance::metrics::ErrorType::ValidationError);
            }
        }

        // Get metrics
        let stats = metrics.read().await.get_stats();
        
        // Verify metrics
        assert_eq!(stats.transactions_processed, 1000);
        assert_eq!(stats.blocks_processed, 10);
        assert_eq!(stats.errors, 5);

        // Test metrics export
        let exported_metrics = metrics.read().await.export_metrics();
        assert!(!exported_metrics.is_empty());

        Ok(())
    }
}

/// Run performance integration tests
pub async fn run_performance_integration_tests() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🚀 Starting IPPAN performance integration tests...");

    let config = PerformanceIntegrationConfig::default();
    let test_suite = PerformanceIntegrationTestSuite::new(config);
    
    test_suite.run_all_tests().await?;

    log::info!("🎉 All performance integration tests completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_integration_suite() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.run_all_tests().await.unwrap();
    }

    #[tokio::test]
    async fn test_lockfree_structures() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.test_lockfree_structures().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_pooling() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.test_memory_pooling().await.unwrap();
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.test_batch_processing().await.unwrap();
    }

    #[tokio::test]
    async fn test_serialization() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.test_serialization().await.unwrap();
    }

    #[tokio::test]
    async fn test_caching() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.test_caching().await.unwrap();
    }

    #[tokio::test]
    async fn test_end_to_end_performance() {
        let config = PerformanceIntegrationConfig::default();
        let test_suite = PerformanceIntegrationTestSuite::new(config);
        
        test_suite.test_end_to_end_performance().await.unwrap();
    }
}
