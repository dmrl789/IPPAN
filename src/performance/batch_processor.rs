//! Batch processing for high-throughput operations
//! 
//! This module provides batch processing capabilities for handling
//! large volumes of transactions and blocks efficiently.

use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, mpsc};
use tokio::task;
use crate::performance::memory_pool::PooledItem;

/// Batch processor for high-throughput operations
pub struct BatchProcessor {
    max_batch_size: usize,
    max_concurrent_batches: usize,
    semaphore: Semaphore,
    processed_batches: AtomicU64,
    total_processed_items: AtomicU64,
    total_processing_time: AtomicU64,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(max_batch_size: usize, max_concurrent_batches: usize) -> Self {
        Self {
            max_batch_size,
            max_concurrent_batches,
            semaphore: Semaphore::new(max_concurrent_batches),
            processed_batches: AtomicU64::new(0),
            total_processed_items: AtomicU64::new(0),
            total_processing_time: AtomicU64::new(0),
        }
    }

    /// Process a batch of items
    pub async fn process_batch<T, F, R>(&self, items: Vec<PooledItem<T>>, processor: F) -> Result<Vec<R>, String>
    where
        F: Fn(Vec<PooledItem<T>>) -> Result<Vec<R>, String> + Send + Sync + 'static,
        T: Send + 'static,
        R: Send + 'static,
    {
        let start_time = Instant::now();
        
        // Acquire semaphore to limit concurrent batches
        let _permit = self.semaphore.acquire().await.map_err(|e| format!("Semaphore error: {}", e))?;
        
        // Split items into chunks if they exceed max batch size
        let chunks = self.split_into_chunks(items);
        
        // Process chunks sequentially for now to avoid lifetime issues
        let mut results = Vec::new();
        for chunk in chunks {
            let chunk_results = processor(chunk)?;
            results.extend(chunk_results);
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.processed_batches.fetch_add(1, Ordering::Relaxed);
        self.total_processed_items.fetch_add(results.len() as u64, Ordering::Relaxed);
        self.total_processing_time.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        
        Ok(results)
    }

    /// Split items into chunks based on max batch size
    fn split_into_chunks<T>(&self, items: Vec<PooledItem<T>>) -> Vec<Vec<PooledItem<T>>> {
        if items.len() <= self.max_batch_size {
            return vec![items];
        }
        
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        
        for item in items {
            if current_chunk.len() >= self.max_batch_size {
                chunks.push(current_chunk);
                current_chunk = Vec::new();
            }
            current_chunk.push(item);
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }
        
        chunks
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> BatchProcessorStats {
        BatchProcessorStats {
            processed_batches: self.processed_batches.load(Ordering::Relaxed),
            total_processed_items: self.total_processed_items.load(Ordering::Relaxed),
            total_processing_time: self.total_processing_time.load(Ordering::Relaxed),
            max_batch_size: self.max_batch_size,
            max_concurrent_batches: self.max_concurrent_batches,
        }
    }
}

/// Statistics for batch processor
#[derive(Debug, Clone)]
pub struct BatchProcessorStats {
    pub processed_batches: u64,
    pub total_processed_items: u64,
    pub total_processing_time: u64,
    pub max_batch_size: usize,
    pub max_concurrent_batches: usize,
}

/// Parallel batch processor for CPU-intensive operations
pub struct ParallelBatchProcessor {
    batch_processor: BatchProcessor,
    thread_pool_size: usize,
}

impl ParallelBatchProcessor {
    /// Create a new parallel batch processor
    pub fn new(max_batch_size: usize, max_concurrent_batches: usize, thread_pool_size: usize) -> Self {
        Self {
            batch_processor: BatchProcessor::new(max_batch_size, max_concurrent_batches),
            thread_pool_size,
        }
    }

    /// Process a batch using parallel processing
    pub async fn process_batch_parallel<T, F, R>(&self, items: Vec<PooledItem<T>>, processor: F) -> Result<Vec<R>, String>
    where
        F: Fn(Vec<PooledItem<T>>) -> Result<Vec<R>, String> + Send + Sync + 'static,
        T: Send + 'static,
        R: Send + 'static,
    {
        // Use the underlying batch processor with sequential execution for now
        self.batch_processor.process_batch(items, processor).await
    }

    /// Process a chunk in parallel (simplified for now)
    fn process_chunk_parallel<T, F, R>(&self, chunk: Vec<PooledItem<T>>, processor: &F) -> Result<Vec<R>, String>
    where
        F: Fn(Vec<PooledItem<T>>) -> Result<Vec<R>, String> + Send + Sync,
        T: Send,
        R: Send,
    {
        // For now, just process sequentially to avoid lifetime issues
        processor(chunk)
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> BatchProcessorStats {
        self.batch_processor.get_stats()
    }
}

/// Streaming batch processor for continuous processing
pub struct StreamingBatchProcessor {
    batch_processor: BatchProcessor,
    input_channel: mpsc::UnboundedSender<PooledItem<TransactionBlock>>,
    output_channel: mpsc::UnboundedReceiver<ProcessedTransaction>,
    is_running: std::sync::atomic::AtomicBool,
}

struct TransactionBlock {
    data: Vec<u8>,
    timestamp: u64,
    signature: Vec<u8>,
}

struct ProcessedTransaction {
    transaction: TransactionBlock,
    is_valid: bool,
    processing_time: Instant,
}

impl StreamingBatchProcessor {
    /// Create a new streaming batch processor
    pub fn new(max_batch_size: usize, max_concurrent_batches: usize) -> Self {
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        
        let batch_processor = BatchProcessor::new(max_batch_size, max_concurrent_batches);
        
        // Start the processing loop
        tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut input_rx = input_rx;
            
            while let Some(item) = input_rx.recv().await {
                batch.push(item);
                
                if batch.len() >= max_batch_size {
                    // Process the batch
                    let results = batch_processor.process_batch(batch, |items| {
                        Ok(items.into_iter().map(|item| {
                            ProcessedTransaction {
                                transaction: item.into_data(),
                                is_valid: true, // Placeholder validation
                                processing_time: Instant::now(),
                            }
                        }).collect())
                    }).await;
                    
                    if let Ok(processed) = results {
                        for result in processed {
                            let _ = output_tx.send(result);
                        }
                    }
                    
                    batch = Vec::new();
                }
            }
        });
        
        Self {
            batch_processor: BatchProcessor::new(max_batch_size, max_concurrent_batches),
            input_channel: input_tx,
            output_channel: output_rx,
            is_running: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Send a transaction for processing
    pub fn send_transaction(&self, transaction: PooledItem<TransactionBlock>) -> Result<(), String> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err("Processor is not running".to_string());
        }
        
        self.input_channel.send(transaction).map_err(|_| "Failed to send transaction".to_string())
    }

    /// Receive a processed transaction
    pub async fn receive_processed(&mut self) -> Option<ProcessedTransaction> {
        self.output_channel.recv().await
    }

    /// Stop the processor
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> BatchProcessorStats {
        self.batch_processor.get_stats()
    }
}

/// Batch processing configuration
#[derive(Debug, Clone)]
pub struct BatchProcessingConfig {
    pub max_batch_size: usize,
    pub max_concurrent_batches: usize,
    pub thread_pool_size: usize,
    pub enable_parallel_processing: bool,
    pub enable_streaming: bool,
    pub batch_timeout: Duration,
}

impl Default for BatchProcessingConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10000,
            max_concurrent_batches: 100,
            thread_pool_size: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
            enable_parallel_processing: true,
            enable_streaming: false,
            batch_timeout: Duration::from_millis(100),
        }
    }
}

/// Batch processing manager
pub struct BatchProcessingManager {
    config: BatchProcessingConfig,
    parallel_processor: ParallelBatchProcessor,
    streaming_processor: Option<StreamingBatchProcessor>,
}

impl BatchProcessingManager {
    /// Create a new batch processing manager
    pub fn new(config: BatchProcessingConfig) -> Self {
        let parallel_processor = ParallelBatchProcessor::new(
            config.max_batch_size,
            config.max_concurrent_batches,
            config.thread_pool_size,
        );
        
        let streaming_processor = if config.enable_streaming {
            Some(StreamingBatchProcessor::new(
                config.max_batch_size,
                config.max_concurrent_batches,
            ))
        } else {
            None
        };
        
        Self {
            config,
            parallel_processor,
            streaming_processor,
        }
    }

    /// Process a batch of transactions
    pub async fn process_transactions<T, F, R>(&self, transactions: Vec<PooledItem<T>>, processor: F) -> Result<Vec<R>, String>
    where
        F: Fn(Vec<PooledItem<T>>) -> Result<Vec<R>, String> + Send + Sync + 'static,
        T: Send + 'static,
        R: Send + 'static,
    {
        if self.config.enable_parallel_processing {
            self.parallel_processor.process_batch_parallel(transactions, processor).await
        } else {
            // Fallback to sequential processing
            processor(transactions)
        }
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> BatchProcessorStats {
        self.parallel_processor.get_stats()
    }

    /// Get configuration
    pub fn get_config(&self) -> &BatchProcessingConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::memory_pool::MemoryPool;

    #[test]
    fn test_batch_processor() {
        let processor = BatchProcessor::new(10, 5);
        
        let pool = MemoryPool::new(100);
        let items: Vec<_> = (0..25).map(|i| {
            let mut item = pool.allocate().unwrap();
            *item.data_mut() = i;
            item
        }).collect();
        
        let results = tokio::runtime::Runtime::new().unwrap().block_on(async {
            processor.process_batch(items, |chunk| {
                Ok(chunk.into_iter().map(|item| *item.data()).collect())
            }).await
        }).unwrap();
        
        assert_eq!(results.len(), 25);
        assert_eq!(processor.get_stats().processed_batches, 3); // 25 items / 10 batch size = 3 batches
    }

    #[test]
    fn test_parallel_batch_processor() {
        let processor = ParallelBatchProcessor::new(10, 5, 4);
        
        let pool = MemoryPool::new(100);
        let items: Vec<_> = (0..20).map(|i| {
            let mut item = pool.allocate().unwrap();
            *item.data_mut() = i;
            item
        }).collect();
        
        let results = tokio::runtime::Runtime::new().unwrap().block_on(async {
            processor.process_batch_parallel(items, |chunk| {
                Ok(chunk.into_iter().map(|item| *item.data()).collect())
            }).await
        }).unwrap();
        
        assert_eq!(results.len(), 20);
    }

    #[test]
    fn test_batch_processing_manager() {
        let config = BatchProcessingConfig::default();
        let manager = BatchProcessingManager::new(config);
        
        let pool = MemoryPool::new(100);
        let items: Vec<_> = (0..15).map(|i| {
            let mut item = pool.allocate().unwrap();
            *item.data_mut() = i;
            item
        }).collect();
        
        let results = tokio::runtime::Runtime::new().unwrap().block_on(async {
            manager.process_transactions(items, |chunk| {
                Ok(chunk.into_iter().map(|item| *item.data()).collect())
            }).await
        }).unwrap();
        
        assert_eq!(results.len(), 15);
    }
}
