//! Block database for IPPAN
//! 
//! Manages persistent storage of blocks including:
//! - Block storage and retrieval
//! - Block indexing
//! - Block validation
//! - Block analytics

use crate::{Result, IppanError, TransactionHash};
use crate::database::real_database::RealDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Block database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDatabaseConfig {
    /// Block cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    /// Maximum blocks to keep in memory
    pub max_memory_blocks: usize,
    /// Enable block indexing
    pub enable_block_indexing: bool,
    /// Index update interval in seconds
    pub index_update_interval_seconds: u64,
    /// Enable block analytics
    pub enable_block_analytics: bool,
    /// Analytics update interval in seconds
    pub analytics_update_interval_seconds: u64,
    /// Block retention period in seconds
    pub block_retention_seconds: u64,
}

impl Default for BlockDatabaseConfig {
    fn default() -> Self {
        Self {
            cleanup_interval_seconds: 3600, // 1 hour
            max_memory_blocks: 1000,
            enable_block_indexing: true,
            index_update_interval_seconds: 60, // 1 minute
            enable_block_analytics: true,
            analytics_update_interval_seconds: 300, // 5 minutes
            block_retention_seconds: 86400 * 365, // 1 year
        }
    }
}

/// Stored block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredBlock {
    /// Block hash
    pub hash: [u8; 32],
    /// Block number
    pub number: u64,
    /// Parent block hash
    pub parent_hash: [u8; 32],
    /// Block timestamp
    pub timestamp: u64,
    /// Block data
    pub data: Vec<u8>,
    /// Block size in bytes
    pub size_bytes: u64,
    /// Transaction count
    pub transaction_count: u64,
    /// Transaction hashes
    pub transaction_hashes: Vec<TransactionHash>,
    /// Block producer
    pub producer: String,
    /// Block signature
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Block status
    pub status: BlockStatus,
    /// Created timestamp
    pub created_at: u64,
    /// Confirmed timestamp (if confirmed)
    pub confirmed_at: Option<u64>,
    /// Block difficulty
    pub difficulty: u64,
    /// Gas used
    pub gas_used: u64,
    /// Gas limit
    pub gas_limit: u64,
}

/// Block status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Pending
    Pending,
    /// Confirmed
    Confirmed,
    /// Finalized
    Finalized,
    /// Orphaned
    Orphaned,
}

/// Block index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIndexEntry {
    /// Block hash
    pub hash: [u8; 32],
    /// Block number
    pub number: u64,
    /// Parent hash
    pub parent_hash: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// Transaction count
    pub transaction_count: u64,
    /// Producer
    pub producer: String,
    /// Status
    pub status: BlockStatus,
}

/// Block analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAnalytics {
    /// Total blocks
    pub total_blocks: u64,
    /// Confirmed blocks
    pub confirmed_blocks: u64,
    /// Finalized blocks
    pub finalized_blocks: u64,
    /// Orphaned blocks
    pub orphaned_blocks: u64,
    /// Average block size
    pub average_block_size: f64,
    /// Average transaction count per block
    pub average_transaction_count: f64,
    /// Average block time in seconds
    pub average_block_time: f64,
    /// Blocks per hour
    pub blocks_per_hour: f64,
    /// Top producers by block count
    pub top_producers: Vec<(String, u64)>,
    /// Block size distribution
    pub block_size_distribution: HashMap<String, u64>,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Block database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStats {
    /// Total blocks stored
    pub total_blocks: u64,
    /// Confirmed blocks
    pub confirmed_blocks: u64,
    /// Finalized blocks
    pub finalized_blocks: u64,
    /// Orphaned blocks
    pub orphaned_blocks: u64,
    /// Total operations performed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average operation time in milliseconds
    pub average_operation_time_ms: f64,
    /// Index entries
    pub index_entries: u64,
    /// Analytics updates
    pub analytics_updates: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last cleanup timestamp
    pub last_cleanup: Option<u64>,
    /// Last analytics update
    pub last_analytics_update: Option<u64>,
}

/// Block database manager
pub struct BlockDatabase {
    /// Database reference
    database: Arc<RealDatabase>,
    /// Configuration
    config: BlockDatabaseConfig,
    /// Block cache
    block_cache: Arc<RwLock<HashMap<[u8; 32], StoredBlock>>>,
    /// Block index
    block_index: Arc<RwLock<HashMap<u64, Vec<BlockIndexEntry>>>>,
    /// Analytics
    analytics: Arc<RwLock<BlockAnalytics>>,
    /// Statistics
    stats: Arc<RwLock<BlockStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl BlockDatabase {
    /// Create a new block database manager
    pub async fn new(database: Arc<RealDatabase>) -> Result<Self> {
        let config = BlockDatabaseConfig::default();
        
        let analytics = BlockAnalytics {
            total_blocks: 0,
            confirmed_blocks: 0,
            finalized_blocks: 0,
            orphaned_blocks: 0,
            average_block_size: 0.0,
            average_transaction_count: 0.0,
            average_block_time: 0.0,
            blocks_per_hour: 0.0,
            top_producers: vec![],
            block_size_distribution: HashMap::new(),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        let stats = BlockStats {
            total_blocks: 0,
            confirmed_blocks: 0,
            finalized_blocks: 0,
            orphaned_blocks: 0,
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            average_operation_time_ms: 0.0,
            index_entries: 0,
            analytics_updates: 0,
            uptime_seconds: 0,
            last_cleanup: None,
            last_analytics_update: None,
        };
        
        Ok(Self {
            database,
            config,
            block_cache: Arc::new(RwLock::new(HashMap::new())),
            block_index: Arc::new(RwLock::new(HashMap::new())),
            analytics: Arc::new(RwLock::new(analytics)),
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }
    
    /// Start the block database manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting block database manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Load blocks from database
        self.load_blocks_from_database().await?;
        
        // Start cleanup loop
        let config = self.config.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::cleanup_loop(config, stats, is_running).await;
        });
        
        // Start indexing loop
        let config = self.config.clone();
        let block_index = self.block_index.clone();
        let block_cache = self.block_cache.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::indexing_loop(
                config,
                block_index,
                block_cache,
                stats,
                is_running,
            ).await;
        });
        
        // Start analytics loop
        let config = self.config.clone();
        let analytics = self.analytics.clone();
        let block_cache = self.block_cache.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::analytics_loop(
                config,
                analytics,
                block_cache,
                stats,
                is_running,
            ).await;
        });
        
        // Start statistics update loop
        let stats = self.stats.clone();
        let block_cache = self.block_cache.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            Self::statistics_update_loop(
                stats,
                block_cache,
                is_running,
                start_time,
            ).await;
        });
        
        info!("Block database manager started successfully");
        Ok(())
    }
    
    /// Stop the block database manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping block database manager");
        
        // Save blocks to database
        self.save_blocks_to_database().await?;
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Block database manager stopped");
        Ok(())
    }
    
    /// Store a block
    pub async fn store_block(&self, block: StoredBlock) -> Result<()> {
        let start_time = Instant::now();
        
        // Validate block
        if !self.validate_block(&block).await? {
            return Err(IppanError::Database("Block validation failed".to_string()));
        }
        
        // Store in cache
        let mut cache = self.block_cache.write().await;
        cache.insert(block.hash, block.clone());
        
        // Store in database
        let block_data = bincode::serialize(&block)
            .map_err(|e| IppanError::Database(format!("Failed to serialize block: {}", e)))?;
        
        self.database.insert("blocks", &format!("{:02x?}", block.hash), &block_data).await?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.successful_operations += 1;
        stats.total_operations += 1;
        stats.average_operation_time_ms = 
            (stats.average_operation_time_ms * (stats.total_operations - 1) as f64 + 
             start_time.elapsed().as_millis() as f64) / stats.total_operations as f64;
        
        match block.status {
            BlockStatus::Confirmed => stats.confirmed_blocks += 1,
            BlockStatus::Finalized => stats.finalized_blocks += 1,
            BlockStatus::Orphaned => stats.orphaned_blocks += 1,
            BlockStatus::Pending => {},
        }
        
        info!("Stored block: {} (height: {})", format!("{:02x?}", block.hash), block.number);
        Ok(())
    }
    
    /// Get a block by hash
    pub async fn get_block(&self, hash: &[u8; 32]) -> Result<Option<StoredBlock>> {
        // Check cache first
        let cache = self.block_cache.read().await;
        if let Some(block) = cache.get(hash) {
            return Ok(Some(block.clone()));
        }
        
        // Load from database
        let block_data = self.database.query("blocks", &format!("{:02x?}", hash)).await?;
        if let Some(data) = block_data {
            let block: StoredBlock = bincode::deserialize(&data)
                .map_err(|e| IppanError::Database(format!("Failed to deserialize block: {}", e)))?;
            
            // Cache the block
            let mut cache = self.block_cache.write().await;
            cache.insert(*hash, block.clone());
            
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }
    
    /// Get a block by number
    pub async fn get_block_by_number(&self, number: u64) -> Result<Option<StoredBlock>> {
        // Check index first
        let index = self.block_index.read().await;
        if let Some(entries) = index.get(&number) {
            for entry in entries {
                if let Some(block) = self.get_block(&entry.hash).await? {
                    return Ok(Some(block));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get blocks by producer
    pub async fn get_blocks_by_producer(&self, producer: &str) -> Result<Vec<StoredBlock>> {
        let mut blocks = Vec::new();
        let cache = self.block_cache.read().await;
        
        for block in cache.values() {
            if block.producer == producer {
                blocks.push(block.clone());
            }
        }
        
        Ok(blocks)
    }
    
    /// Update block status
    pub async fn update_block_status(
        &self,
        hash: &[u8; 32],
        new_status: BlockStatus,
    ) -> Result<()> {
        let mut cache = self.block_cache.write().await;
        if let Some(block) = cache.get_mut(hash) {
            let old_status = std::mem::replace(&mut block.status, new_status.clone());
            
            if matches!(new_status, BlockStatus::Confirmed | BlockStatus::Finalized) {
                block.confirmed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            // Update in database
            let block_data = bincode::serialize(&*block)
                .map_err(|e| IppanError::Database(format!("Failed to serialize block: {}", e)))?;
            
            self.database.update("blocks", &format!("{:02x?}", hash), &block_data).await?;
            
            // Update statistics
            let mut stats = self.stats.write().await;
            match old_status {
                BlockStatus::Confirmed => stats.confirmed_blocks = stats.confirmed_blocks.saturating_sub(1),
                BlockStatus::Finalized => stats.finalized_blocks = stats.finalized_blocks.saturating_sub(1),
                BlockStatus::Orphaned => stats.orphaned_blocks = stats.orphaned_blocks.saturating_sub(1),
                BlockStatus::Pending => {},
            }
            
            match new_status {
                BlockStatus::Confirmed => stats.confirmed_blocks += 1,
                BlockStatus::Finalized => stats.finalized_blocks += 1,
                BlockStatus::Orphaned => stats.orphaned_blocks += 1,
                BlockStatus::Pending => {},
            }
            
            info!("Updated block status: {:02x?} -> {:?}", hash, new_status);
        } else {
            return Err(IppanError::Database("Block not found".to_string()));
        }
        
        Ok(())
    }
    
    /// Get block analytics
    pub async fn get_analytics(&self) -> Result<BlockAnalytics> {
        let analytics = self.analytics.read().await;
        Ok(analytics.clone())
    }
    
    /// Get block database statistics
    pub async fn get_stats(&self) -> Result<BlockStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Validate block
    async fn validate_block(&self, block: &StoredBlock) -> Result<bool> {
        // Validate hash
        if block.hash == [0u8; 32] {
            return Err(IppanError::Database("Invalid block hash".to_string()));
        }
        
        // Validate block number
        if block.number == 0 && block.parent_hash != [0u8; 32] {
            return Err(IppanError::Database("Invalid genesis block".to_string()));
        }
        
        // Validate timestamp
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if block.timestamp > current_time + 3600 { // Allow 1 hour in future
            return Err(IppanError::Database("Block timestamp too far in future".to_string()));
        }
        
        // Validate producer
        if block.producer.is_empty() {
            return Err(IppanError::Database("Invalid block producer".to_string()));
        }
        
        // Validate signature
        if block.signature == [0u8; 64] {
            return Err(IppanError::Database("Invalid block signature".to_string()));
        }
        
        Ok(true)
    }
    
    /// Load blocks from database
    async fn load_blocks_from_database(&self) -> Result<()> {
        // In a real implementation, this would load all blocks from the database
        debug!("Loading blocks from database (placeholder)");
        Ok(())
    }
    
    /// Save blocks to database
    async fn save_blocks_to_database(&self) -> Result<()> {
        // In a real implementation, this would save all cached blocks
        debug!("Saving blocks to database (placeholder)");
        Ok(())
    }
    
    /// Cleanup loop
    async fn cleanup_loop(
        config: BlockDatabaseConfig,
        stats: Arc<RwLock<BlockStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            // In a real implementation, this would clean up old blocks
            debug!("Cleaning up old blocks");
            
            let mut stats = stats.write().await;
            stats.last_cleanup = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            
            tokio::time::sleep(Duration::from_secs(config.cleanup_interval_seconds)).await;
        }
    }
    
    /// Indexing loop
    async fn indexing_loop(
        config: BlockDatabaseConfig,
        block_index: Arc<RwLock<HashMap<u64, Vec<BlockIndexEntry>>>>,
        block_cache: Arc<RwLock<HashMap<[u8; 32], StoredBlock>>>,
        stats: Arc<RwLock<BlockStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_block_indexing {
                // In a real implementation, this would update block indexes
                debug!("Updating block indexes");
                
                let mut stats = stats.write().await;
                stats.index_entries += 1;
            }
            
            tokio::time::sleep(Duration::from_secs(config.index_update_interval_seconds)).await;
        }
    }
    
    /// Analytics loop
    async fn analytics_loop(
        config: BlockDatabaseConfig,
        analytics: Arc<RwLock<BlockAnalytics>>,
        block_cache: Arc<RwLock<HashMap<[u8; 32], StoredBlock>>>,
        stats: Arc<RwLock<BlockStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_block_analytics {
                // In a real implementation, this would update analytics
                debug!("Updating block analytics");
                
                let mut analytics = analytics.write().await;
                analytics.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                
                let mut stats = stats.write().await;
                stats.analytics_updates += 1;
                stats.last_analytics_update = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            tokio::time::sleep(Duration::from_secs(config.analytics_update_interval_seconds)).await;
        }
    }
    
    /// Statistics update loop
    async fn statistics_update_loop(
        stats: Arc<RwLock<BlockStats>>,
        block_cache: Arc<RwLock<HashMap<[u8; 32], StoredBlock>>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
    ) {
        while *is_running.read().await {
            let mut stats = stats.write().await;
            let cache = block_cache.read().await;
            
            stats.total_blocks = cache.len() as u64;
            stats.uptime_seconds = start_time.elapsed().as_secs();
            
            drop(stats);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_block_creation() {
        let block = StoredBlock {
            hash: [1u8; 32],
            number: 1,
            parent_hash: [0u8; 32],
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            data: vec![1, 2, 3, 4],
            size_bytes: 4,
            transaction_count: 0,
            transaction_hashes: vec![],
            producer: "i1234567890abcdef".to_string(),
            signature: [2u8; 64],
            status: BlockStatus::Pending,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            confirmed_at: None,
            difficulty: 1000,
            gas_used: 0,
            gas_limit: 1000000,
        };
        
        assert_eq!(block.number, 1);
        assert_eq!(block.size_bytes, 4);
        assert_eq!(block.producer, "i1234567890abcdef");
    }
    
    #[tokio::test]
    async fn test_block_status() {
        let status = BlockStatus::Confirmed;
        
        match status {
            BlockStatus::Confirmed => {
                // Test passed
            }
            _ => panic!("Expected Confirmed status"),
        }
    }
    
    #[tokio::test]
    async fn test_analytics_structure() {
        let analytics = BlockAnalytics {
            total_blocks: 1000,
            confirmed_blocks: 900,
            finalized_blocks: 800,
            orphaned_blocks: 10,
            average_block_size: 1024.0,
            average_transaction_count: 10.5,
            average_block_time: 10.0,
            blocks_per_hour: 360.0,
            top_producers: vec![("i1234567890abcdef".to_string(), 100)],
            block_size_distribution: HashMap::new(),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        assert_eq!(analytics.total_blocks, 1000);
        assert_eq!(analytics.confirmed_blocks, 900);
        assert_eq!(analytics.average_block_size, 1024.0);
    }
}
