//! Index database for IPPAN
//! 
//! Manages persistent storage of indexes including:
//! - Transaction indexes
//! - Account indexes
//! - Block indexes
//! - Search indexes

use crate::{Result, IppanError, TransactionHash};
use crate::database::real_database::RealDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Index database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDatabaseConfig {
    /// Index update interval in seconds
    pub index_update_interval_seconds: u64,
    /// Maximum indexes to keep in memory
    pub max_memory_indexes: usize,
    /// Enable index optimization
    pub enable_index_optimization: bool,
    /// Optimization interval in seconds
    pub optimization_interval_seconds: u64,
    /// Enable index compression
    pub enable_index_compression: bool,
    /// Index retention period in seconds
    pub index_retention_seconds: u64,
}

impl Default for IndexDatabaseConfig {
    fn default() -> Self {
        Self {
            index_update_interval_seconds: 60, // 1 minute
            max_memory_indexes: 10000,
            enable_index_optimization: true,
            optimization_interval_seconds: 3600, // 1 hour
            enable_index_compression: true,
            index_retention_seconds: 86400 * 30, // 30 days
        }
    }
}

/// Index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Index key
    pub key: String,
    /// Index value
    pub value: Vec<u8>,
    /// Index type
    pub index_type: IndexType,
    /// Created timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub last_updated: u64,
    /// Access count
    pub access_count: u64,
    /// Last accessed timestamp
    pub last_accessed: u64,
}

/// Index type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    /// Transaction index
    Transaction {
        transaction_hash: TransactionHash,
        from_address: String,
        to_address: String,
    },
    /// Account index
    Account {
        address: String,
        public_key: [u8; 32],
    },
    /// Block index
    Block {
        block_hash: [u8; 32],
        block_number: u64,
        producer: String,
    },
    /// Search index
    Search {
        search_term: String,
        result_type: String,
    },
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total indexes
    pub total_indexes: u64,
    /// Transaction indexes
    pub transaction_indexes: u64,
    /// Account indexes
    pub account_indexes: u64,
    /// Block indexes
    pub block_indexes: u64,
    /// Search indexes
    pub search_indexes: u64,
    /// Total operations performed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average operation time in milliseconds
    pub average_operation_time_ms: f64,
    /// Index optimizations performed
    pub index_optimizations: u64,
    /// Index compressions performed
    pub index_compressions: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last optimization timestamp
    pub last_optimization: Option<u64>,
    /// Last compression timestamp
    pub last_compression: Option<u64>,
}

/// Index database manager
pub struct IndexDatabase {
    /// Database reference
    database: Arc<RealDatabase>,
    /// Configuration
    config: IndexDatabaseConfig,
    /// Index cache
    index_cache: Arc<RwLock<HashMap<String, IndexEntry>>>,
    /// Statistics
    stats: Arc<RwLock<IndexStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl IndexDatabase {
    /// Create a new index database manager
    pub async fn new(database: Arc<RealDatabase>) -> Result<Self> {
        let config = IndexDatabaseConfig::default();
        
        let stats = IndexStats {
            total_indexes: 0,
            transaction_indexes: 0,
            account_indexes: 0,
            block_indexes: 0,
            search_indexes: 0,
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            average_operation_time_ms: 0.0,
            index_optimizations: 0,
            index_compressions: 0,
            uptime_seconds: 0,
            last_optimization: None,
            last_compression: None,
        };
        
        Ok(Self {
            database,
            config,
            index_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }
    
    /// Start the index database manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting index database manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Load indexes from database
        self.load_indexes_from_database().await?;
        
        // Start index update loop
        let config = self.config.clone();
        let index_cache = self.index_cache.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::index_update_loop(
                config,
                index_cache,
                stats,
                is_running,
            ).await;
        });
        
        // Start optimization loop
        let config = self.config.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::optimization_loop(config, stats, is_running).await;
        });
        
        // Start statistics update loop
        let stats = self.stats.clone();
        let index_cache = self.index_cache.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            Self::statistics_update_loop(
                stats,
                index_cache,
                is_running,
                start_time,
            ).await;
        });
        
        info!("Index database manager started successfully");
        Ok(())
    }
    
    /// Stop the index database manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping index database manager");
        
        // Save indexes to database
        self.save_indexes_to_database().await?;
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Index database manager stopped");
        Ok(())
    }
    
    /// Create an index entry
    pub async fn create_index(&self, key: String, value: Vec<u8>, index_type: IndexType) -> Result<()> {
        let start_time = Instant::now();
        
        let entry = IndexEntry {
            key: key.clone(),
            value: value.clone(),
            index_type: index_type.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 0,
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        // Store in cache
        let mut cache = self.index_cache.write().await;
        cache.insert(key.clone(), entry.clone());
        
        // Store in database
        let index_data = bincode::serialize(&entry)
            .map_err(|e| IppanError::Database(format!("Failed to serialize index: {}", e)))?;
        
        self.database.insert("indexes", &key, &index_data).await?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.successful_operations += 1;
        stats.total_operations += 1;
        stats.average_operation_time_ms = 
            (stats.average_operation_time_ms * (stats.total_operations - 1) as f64 + 
             start_time.elapsed().as_millis() as f64) / stats.total_operations as f64;
        
        match index_type {
            IndexType::Transaction { .. } => stats.transaction_indexes += 1,
            IndexType::Account { .. } => stats.account_indexes += 1,
            IndexType::Block { .. } => stats.block_indexes += 1,
            IndexType::Search { .. } => stats.search_indexes += 1,
        }
        
        info!("Created index: {}", key);
        Ok(())
    }
    
    /// Get an index entry
    pub async fn get_index(&self, key: &str) -> Result<Option<IndexEntry>> {
        // Check cache first
        let mut cache = self.index_cache.write().await;
        if let Some(entry) = cache.get_mut(key) {
            // Update access statistics
            entry.access_count += 1;
            entry.last_accessed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            
            return Ok(Some(entry.clone()));
        }
        
        // Load from database
        let index_data = self.database.query("indexes", key).await?;
        if let Some(data) = index_data {
            let entry: IndexEntry = bincode::deserialize(&data)
                .map_err(|e| IppanError::Database(format!("Failed to deserialize index: {}", e)))?;
            
            // Cache the entry
            cache.insert(key.to_string(), entry.clone());
            
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }
    
    /// Update an index entry
    pub async fn update_index(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let mut cache = self.index_cache.write().await;
        if let Some(entry) = cache.get_mut(key) {
            entry.value = value;
            entry.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            
            // Update in database
            let index_data = bincode::serialize(&*entry)
                .map_err(|e| IppanError::Database(format!("Failed to serialize index: {}", e)))?;
            
            self.database.update("indexes", key, &index_data).await?;
            
            info!("Updated index: {}", key);
        } else {
            return Err(IppanError::Database("Index not found".to_string()));
        }
        
        Ok(())
    }
    
    /// Delete an index entry
    pub async fn delete_index(&self, key: &str) -> Result<()> {
        // Remove from cache
        let mut cache = self.index_cache.write().await;
        if let Some(entry) = cache.remove(key) {
            // Update statistics
            let mut stats = self.stats.write().await;
            match entry.index_type {
                IndexType::Transaction { .. } => stats.transaction_indexes = stats.transaction_indexes.saturating_sub(1),
                IndexType::Account { .. } => stats.account_indexes = stats.account_indexes.saturating_sub(1),
                IndexType::Block { .. } => stats.block_indexes = stats.block_indexes.saturating_sub(1),
                IndexType::Search { .. } => stats.search_indexes = stats.search_indexes.saturating_sub(1),
            }
        }
        
        // Remove from database
        self.database.delete("indexes", key).await?;
        
        info!("Deleted index: {}", key);
        Ok(())
    }
    
    /// Search indexes by type
    pub async fn search_indexes_by_type(&self, index_type: &IndexType) -> Result<Vec<IndexEntry>> {
        let mut results = Vec::new();
        let cache = self.index_cache.read().await;
        
        for entry in cache.values() {
            match (&entry.index_type, index_type) {
                (IndexType::Transaction { .. }, IndexType::Transaction { .. }) => {
                    results.push(entry.clone());
                }
                (IndexType::Account { .. }, IndexType::Account { .. }) => {
                    results.push(entry.clone());
                }
                (IndexType::Block { .. }, IndexType::Block { .. }) => {
                    results.push(entry.clone());
                }
                (IndexType::Search { .. }, IndexType::Search { .. }) => {
                    results.push(entry.clone());
                }
                _ => {}
            }
        }
        
        Ok(results)
    }
    
    /// Get index database statistics
    pub async fn get_stats(&self) -> Result<IndexStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Load indexes from database
    async fn load_indexes_from_database(&self) -> Result<()> {
        // In a real implementation, this would load all indexes from the database
        debug!("Loading indexes from database (placeholder)");
        Ok(())
    }
    
    /// Save indexes to database
    async fn save_indexes_to_database(&self) -> Result<()> {
        // In a real implementation, this would save all cached indexes
        debug!("Saving indexes to database (placeholder)");
        Ok(())
    }
    
    /// Index update loop
    async fn index_update_loop(
        config: IndexDatabaseConfig,
        index_cache: Arc<RwLock<HashMap<String, IndexEntry>>>,
        stats: Arc<RwLock<IndexStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            // In a real implementation, this would update indexes
            debug!("Updating indexes");
            
            tokio::time::sleep(Duration::from_secs(config.index_update_interval_seconds)).await;
        }
    }
    
    /// Optimization loop
    async fn optimization_loop(
        config: IndexDatabaseConfig,
        stats: Arc<RwLock<IndexStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_index_optimization {
                // In a real implementation, this would optimize indexes
                debug!("Optimizing indexes");
                
                let mut stats = stats.write().await;
                stats.index_optimizations += 1;
                stats.last_optimization = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            if config.enable_index_compression {
                // In a real implementation, this would compress indexes
                debug!("Compressing indexes");
                
                let mut stats = stats.write().await;
                stats.index_compressions += 1;
                stats.last_compression = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            tokio::time::sleep(Duration::from_secs(config.optimization_interval_seconds)).await;
        }
    }
    
    /// Statistics update loop
    async fn statistics_update_loop(
        stats: Arc<RwLock<IndexStats>>,
        index_cache: Arc<RwLock<HashMap<String, IndexEntry>>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
    ) {
        while *is_running.read().await {
            let mut stats = stats.write().await;
            let cache = index_cache.read().await;
            
            stats.total_indexes = cache.len() as u64;
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
    async fn test_index_creation() {
        let index_type = IndexType::Transaction {
            transaction_hash: [1u8; 32],
            from_address: "i1234567890abcdef".to_string(),
            to_address: "i0987654321fedcba".to_string(),
        };
        
        let entry = IndexEntry {
            key: "test_key".to_string(),
            value: vec![1, 2, 3, 4],
            index_type: index_type.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 0,
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.value, vec![1, 2, 3, 4]);
        
        match entry.index_type {
            IndexType::Transaction { from_address, to_address, .. } => {
                assert_eq!(from_address, "i1234567890abcdef");
                assert_eq!(to_address, "i0987654321fedcba");
            }
            _ => panic!("Expected Transaction index type"),
        }
    }
    
    #[tokio::test]
    async fn test_index_types() {
        let transaction_index = IndexType::Transaction {
            transaction_hash: [1u8; 32],
            from_address: "i1234567890abcdef".to_string(),
            to_address: "i0987654321fedcba".to_string(),
        };
        
        let account_index = IndexType::Account {
            address: "i1234567890abcdef".to_string(),
            public_key: [2u8; 32],
        };
        
        let block_index = IndexType::Block {
            block_hash: [3u8; 32],
            block_number: 1,
            producer: "i1234567890abcdef".to_string(),
        };
        
        let search_index = IndexType::Search {
            search_term: "test".to_string(),
            result_type: "transaction".to_string(),
        };
        
        // Test all index types are created correctly
        match transaction_index {
            IndexType::Transaction { .. } => {},
            _ => panic!("Expected Transaction index type"),
        }
        
        match account_index {
            IndexType::Account { .. } => {},
            _ => panic!("Expected Account index type"),
        }
        
        match block_index {
            IndexType::Block { .. } => {},
            _ => panic!("Expected Block index type"),
        }
        
        match search_index {
            IndexType::Search { .. } => {},
            _ => panic!("Expected Search index type"),
        }
    }
    
    #[tokio::test]
    async fn test_statistics_structure() {
        let stats = IndexStats {
            total_indexes: 1000,
            transaction_indexes: 400,
            account_indexes: 300,
            block_indexes: 200,
            search_indexes: 100,
            total_operations: 5000,
            successful_operations: 4800,
            failed_operations: 200,
            average_operation_time_ms: 5.5,
            index_optimizations: 10,
            index_compressions: 5,
            uptime_seconds: 3600,
            last_optimization: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            last_compression: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_indexes, 1000);
        assert_eq!(stats.transaction_indexes, 400);
        assert_eq!(stats.account_indexes, 300);
        assert_eq!(stats.block_indexes, 200);
        assert_eq!(stats.search_indexes, 100);
        assert_eq!(stats.total_operations, 5000);
        assert_eq!(stats.index_optimizations, 10);
    }
}
