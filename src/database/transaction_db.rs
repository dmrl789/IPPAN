//! Transaction database for IPPAN
//! 
//! Manages persistent storage of transactions including:
//! - Transaction storage and retrieval
//! - Transaction indexing
//! - Transaction status tracking
//! - Transaction history and analytics

use crate::{Result, IppanError, TransactionHash};
use crate::database::real_database::RealDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Transaction database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDatabaseConfig {
    /// Transaction cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    /// Maximum transactions to keep in memory
    pub max_memory_transactions: usize,
    /// Enable transaction indexing
    pub enable_transaction_indexing: bool,
    /// Index update interval in seconds
    pub index_update_interval_seconds: u64,
    /// Enable transaction analytics
    pub enable_transaction_analytics: bool,
    /// Analytics update interval in seconds
    pub analytics_update_interval_seconds: u64,
    /// Transaction retention period in seconds
    pub transaction_retention_seconds: u64,
}

impl Default for TransactionDatabaseConfig {
    fn default() -> Self {
        Self {
            cleanup_interval_seconds: 3600, // 1 hour
            max_memory_transactions: 10000,
            enable_transaction_indexing: true,
            index_update_interval_seconds: 60, // 1 minute
            enable_transaction_analytics: true,
            analytics_update_interval_seconds: 300, // 5 minutes
            transaction_retention_seconds: 86400 * 30, // 30 days
        }
    }
}

/// Stored transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTransaction {
    /// Transaction hash
    pub hash: TransactionHash,
    /// Block hash (if included in a block)
    pub block_hash: Option<[u8; 32]>,
    /// From address
    pub from_address: String,
    /// To address
    pub to_address: String,
    /// Amount
    pub amount: u64,
    /// Fee
    pub fee: u64,
    /// Nonce
    pub nonce: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Transaction data
    pub data: Option<Vec<u8>>,
    /// Signature
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Status
    pub status: TransactionStatus,
    /// Created timestamp
    pub created_at: u64,
    /// Confirmed timestamp (if confirmed)
    pub confirmed_at: Option<u64>,
    /// Block number (if confirmed)
    pub block_number: Option<u64>,
    /// Gas used (if applicable)
    pub gas_used: Option<u64>,
    /// Transaction type
    pub transaction_type: String,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Pending
    Pending,
    /// Confirmed
    Confirmed,
    /// Failed
    Failed {
        error_message: String,
    },
    /// Cancelled
    Cancelled,
}

/// Transaction index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionIndexEntry {
    /// Transaction hash
    pub hash: TransactionHash,
    /// From address
    pub from_address: String,
    /// To address
    pub to_address: String,
    /// Amount
    pub amount: u64,
    /// Status
    pub status: TransactionStatus,
    /// Created timestamp
    pub created_at: u64,
    /// Block number (if confirmed)
    pub block_number: Option<u64>,
}

/// Transaction analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAnalytics {
    /// Total transactions
    pub total_transactions: u64,
    /// Pending transactions
    pub pending_transactions: u64,
    /// Confirmed transactions
    pub confirmed_transactions: u64,
    /// Failed transactions
    pub failed_transactions: u64,
    /// Total volume (sum of all amounts)
    pub total_volume: u64,
    /// Total fees collected
    pub total_fees: u64,
    /// Average transaction size
    pub average_transaction_size: f64,
    /// Average confirmation time in seconds
    pub average_confirmation_time: f64,
    /// Transactions per second
    pub transactions_per_second: f64,
    /// Top senders by volume
    pub top_senders: Vec<(String, u64)>,
    /// Top recipients by volume
    pub top_recipients: Vec<(String, u64)>,
    /// Transaction type distribution
    pub transaction_type_distribution: HashMap<String, u64>,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Transaction database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    /// Total transactions stored
    pub total_transactions: u64,
    /// Pending transactions
    pub pending_transactions: u64,
    /// Confirmed transactions
    pub confirmed_transactions: u64,
    /// Failed transactions
    pub failed_transactions: u64,
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

/// Transaction database manager
pub struct TransactionDatabase {
    /// Database reference
    database: Arc<RealDatabase>,
    /// Configuration
    config: TransactionDatabaseConfig,
    /// Transaction cache
    transaction_cache: Arc<RwLock<HashMap<TransactionHash, StoredTransaction>>>,
    /// Transaction index
    transaction_index: Arc<RwLock<HashMap<String, Vec<TransactionIndexEntry>>>>,
    /// Analytics
    analytics: Arc<RwLock<TransactionAnalytics>>,
    /// Statistics
    stats: Arc<RwLock<TransactionStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl TransactionDatabase {
    /// Create a new transaction database manager
    pub async fn new(database: Arc<RealDatabase>) -> Result<Self> {
        let config = TransactionDatabaseConfig::default();
        
        let analytics = TransactionAnalytics {
            total_transactions: 0,
            pending_transactions: 0,
            confirmed_transactions: 0,
            failed_transactions: 0,
            total_volume: 0,
            total_fees: 0,
            average_transaction_size: 0.0,
            average_confirmation_time: 0.0,
            transactions_per_second: 0.0,
            top_senders: vec![],
            top_recipients: vec![],
            transaction_type_distribution: HashMap::new(),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        let stats = TransactionStats {
            total_transactions: 0,
            pending_transactions: 0,
            confirmed_transactions: 0,
            failed_transactions: 0,
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
            transaction_cache: Arc::new(RwLock::new(HashMap::new())),
            transaction_index: Arc::new(RwLock::new(HashMap::new())),
            analytics: Arc::new(RwLock::new(analytics)),
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        })
    }
    
    /// Start the transaction database manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting transaction database manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Load transactions from database
        self.load_transactions_from_database().await?;
        
        // Start cleanup loop
        let config = self.config.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::cleanup_loop(config, stats, is_running).await;
        });
        
        // Start indexing loop
        let config = self.config.clone();
        let transaction_index = self.transaction_index.clone();
        let transaction_cache = self.transaction_cache.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::indexing_loop(
                config,
                transaction_index,
                transaction_cache,
                stats,
                is_running,
            ).await;
        });
        
        // Start analytics loop
        let config = self.config.clone();
        let analytics = self.analytics.clone();
        let transaction_cache = self.transaction_cache.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            Self::analytics_loop(
                config,
                analytics,
                transaction_cache,
                stats,
                is_running,
            ).await;
        });
        
        // Start statistics update loop
        let stats = self.stats.clone();
        let transaction_cache = self.transaction_cache.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            Self::statistics_update_loop(
                stats,
                transaction_cache,
                is_running,
                start_time,
            ).await;
        });
        
        info!("Transaction database manager started successfully");
        Ok(())
    }
    
    /// Stop the transaction database manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping transaction database manager");
        
        // Save transactions to database
        self.save_transactions_to_database().await?;
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Transaction database manager stopped");
        Ok(())
    }
    
    /// Store a transaction
    pub async fn store_transaction(&self, transaction: StoredTransaction) -> Result<()> {
        let start_time = Instant::now();
        
        // Validate transaction
        if !self.validate_transaction(&transaction).await? {
            return Err(IppanError::Database("Transaction validation failed".to_string()));
        }
        
        // Store in cache
        let mut cache = self.transaction_cache.write().await;
        cache.insert(transaction.hash, transaction.clone());
        
        // Store in database
        // Serialize the transaction properly
        let tx_data = bincode::serialize(&transaction)
            .map_err(|e| IppanError::Serialization(format!("Failed to serialize transaction: {}", e)))?;
        
        self.database.insert("transactions", &format!("{:02x?}", transaction.hash), &tx_data).await?;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.successful_operations += 1;
        stats.total_operations += 1;
        stats.average_operation_time_ms = 
            (stats.average_operation_time_ms * (stats.total_operations - 1) as f64 + 
             start_time.elapsed().as_millis() as f64) / stats.total_operations as f64;
        
        match transaction.status {
            TransactionStatus::Pending => stats.pending_transactions += 1,
            TransactionStatus::Confirmed => stats.confirmed_transactions += 1,
            TransactionStatus::Failed { .. } => stats.failed_transactions += 1,
            TransactionStatus::Cancelled => {},
        }
        
        info!("Stored transaction: {:02x?}", transaction.hash);
        Ok(())
    }
    
    /// Get a transaction by hash
    pub async fn get_transaction(&self, hash: &TransactionHash) -> Result<Option<StoredTransaction>> {
        // Check cache first
        let cache = self.transaction_cache.read().await;
        if let Some(transaction) = cache.get(hash) {
            return Ok(Some(transaction.clone()));
        }
        
        // Load from database
        let tx_data = self.database.query("transactions", &format!("{:02x?}", hash)).await?;
        if let Some(data) = tx_data {
            let transaction: StoredTransaction = bincode::deserialize(&data)
                .map_err(|e| IppanError::Database(format!("Failed to deserialize transaction: {}", e)))?;
            
            // Cache the transaction
            let mut cache = self.transaction_cache.write().await;
            cache.insert(*hash, transaction.clone());
            
            Ok(Some(transaction))
        } else {
            Ok(None)
        }
    }
    
    /// Get transactions by address
    pub async fn get_transactions_by_address(&self, address: &str) -> Result<Vec<StoredTransaction>> {
        let mut transactions = Vec::new();
        
        // Check index first
        let index = self.transaction_index.read().await;
        if let Some(entries) = index.get(address) {
            for entry in entries {
                if let Some(transaction) = self.get_transaction(&entry.hash).await? {
                    transactions.push(transaction);
                }
            }
        }
        
        Ok(transactions)
    }
    
    /// Update transaction status
    pub async fn update_transaction_status(
        &self,
        hash: &TransactionHash,
        new_status: TransactionStatus,
        block_hash: Option<[u8; 32]>,
        block_number: Option<u64>,
    ) -> Result<()> {
        let mut cache = self.transaction_cache.write().await;
        if let Some(transaction) = cache.get_mut(hash) {
            let old_status = std::mem::replace(&mut transaction.status, new_status.clone());
            transaction.block_hash = block_hash;
            transaction.block_number = block_number;
            
            if matches!(new_status, TransactionStatus::Confirmed) {
                transaction.confirmed_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            // Update in database
            // Serialize the transaction properly
            let tx_data = bincode::serialize(&transaction)
                .map_err(|e| IppanError::Serialization(format!("Failed to serialize transaction: {}", e)))?;
            
            self.database.update("transactions", &format!("{:02x?}", hash), &tx_data).await?;
            
            // Update statistics
            let mut stats = self.stats.write().await;
            match old_status {
                TransactionStatus::Pending => stats.pending_transactions = stats.pending_transactions.saturating_sub(1),
                TransactionStatus::Confirmed => stats.confirmed_transactions = stats.confirmed_transactions.saturating_sub(1),
                TransactionStatus::Failed { .. } => stats.failed_transactions = stats.failed_transactions.saturating_sub(1),
                TransactionStatus::Cancelled => {},
            }
            
            match new_status {
                TransactionStatus::Pending => stats.pending_transactions += 1,
                TransactionStatus::Confirmed => stats.confirmed_transactions += 1,
                TransactionStatus::Failed { .. } => stats.failed_transactions += 1,
                TransactionStatus::Cancelled => {},
            }
            
            info!("Updated transaction status: {:02x?} -> {:?}", hash, new_status);
        } else {
            return Err(IppanError::Database("Transaction not found".to_string()));
        }
        
        Ok(())
    }
    
    /// Get transaction analytics
    pub async fn get_analytics(&self) -> Result<TransactionAnalytics> {
        let analytics = self.analytics.read().await;
        Ok(analytics.clone())
    }
    
    /// Get transaction database statistics
    pub async fn get_stats(&self) -> Result<TransactionStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Validate transaction
    async fn validate_transaction(&self, transaction: &StoredTransaction) -> Result<bool> {
        // Validate hash
        if transaction.hash == [0u8; 32] {
            return Err(IppanError::Database("Invalid transaction hash".to_string()));
        }
        
        // Validate addresses
        if transaction.from_address.is_empty() || transaction.to_address.is_empty() {
            return Err(IppanError::Database("Invalid addresses".to_string()));
        }
        
        // Validate amount
        if transaction.amount == 0 {
            return Err(IppanError::Database("Invalid amount".to_string()));
        }
        
        // Validate signature
        if transaction.signature == [0u8; 64] {
            return Err(IppanError::Database("Invalid signature".to_string()));
        }
        
        Ok(true)
    }
    
    /// Load transactions from database
    async fn load_transactions_from_database(&self) -> Result<()> {
        // In a real implementation, this would load all transactions from the database
        debug!("Loading transactions from database (placeholder)");
        Ok(())
    }
    
    /// Save transactions to database
    async fn save_transactions_to_database(&self) -> Result<()> {
        // In a real implementation, this would save all cached transactions
        debug!("Saving transactions to database (placeholder)");
        Ok(())
    }
    
    /// Cleanup loop
    async fn cleanup_loop(
        config: TransactionDatabaseConfig,
        stats: Arc<RwLock<TransactionStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            // In a real implementation, this would clean up old transactions
            debug!("Cleaning up old transactions");
            
            let mut stats = stats.write().await;
            stats.last_cleanup = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            
            tokio::time::sleep(Duration::from_secs(config.cleanup_interval_seconds)).await;
        }
    }
    
    /// Indexing loop
    async fn indexing_loop(
        config: TransactionDatabaseConfig,
        transaction_index: Arc<RwLock<HashMap<String, Vec<TransactionIndexEntry>>>>,
        transaction_cache: Arc<RwLock<HashMap<TransactionHash, StoredTransaction>>>,
        stats: Arc<RwLock<TransactionStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_transaction_indexing {
                // In a real implementation, this would update transaction indexes
                debug!("Updating transaction indexes");
                
                let mut stats = stats.write().await;
                stats.index_entries += 1;
            }
            
            tokio::time::sleep(Duration::from_secs(config.index_update_interval_seconds)).await;
        }
    }
    
    /// Analytics loop
    async fn analytics_loop(
        config: TransactionDatabaseConfig,
        analytics: Arc<RwLock<TransactionAnalytics>>,
        transaction_cache: Arc<RwLock<HashMap<TransactionHash, StoredTransaction>>>,
        stats: Arc<RwLock<TransactionStats>>,
        is_running: Arc<RwLock<bool>>,
    ) {
        while *is_running.read().await {
            if config.enable_transaction_analytics {
                // In a real implementation, this would update analytics
                debug!("Updating transaction analytics");
                
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
        stats: Arc<RwLock<TransactionStats>>,
        transaction_cache: Arc<RwLock<HashMap<TransactionHash, StoredTransaction>>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
    ) {
        while *is_running.read().await {
            let mut stats = stats.write().await;
            let cache = transaction_cache.read().await;
            
            stats.total_transactions = cache.len() as u64;
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
    async fn test_transaction_creation() {
        let transaction = StoredTransaction {
            hash: [1u8; 32],
            block_hash: None,
            from_address: "i1234567890abcdef".to_string(),
            to_address: "i0987654321fedcba".to_string(),
            amount: 1000,
            fee: 100,
            nonce: 1,
            timestamp: 1234567890,
            data: None,
            signature: [2u8; 64],
            status: TransactionStatus::Pending,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            confirmed_at: None,
            block_number: None,
            gas_used: None,
            transaction_type: "transfer".to_string(),
        };
        
        assert_eq!(transaction.amount, 1000);
        assert_eq!(transaction.fee, 100);
        assert_eq!(transaction.from_address, "i1234567890abcdef");
    }
    
    #[tokio::test]
    async fn test_transaction_status() {
        let status = TransactionStatus::Failed {
            error_message: "Insufficient balance".to_string(),
        };
        
        match status {
            TransactionStatus::Failed { error_message } => {
                assert_eq!(error_message, "Insufficient balance");
            }
            _ => panic!("Expected Failed status"),
        }
    }
    
    #[tokio::test]
    async fn test_analytics_structure() {
        let analytics = TransactionAnalytics {
            total_transactions: 1000,
            pending_transactions: 50,
            confirmed_transactions: 900,
            failed_transactions: 50,
            total_volume: 1000000,
            total_fees: 10000,
            average_transaction_size: 256.0,
            average_confirmation_time: 10.5,
            transactions_per_second: 1.5,
            top_senders: vec![("i1234567890abcdef".to_string(), 100000)],
            top_recipients: vec![("i0987654321fedcba".to_string(), 100000)],
            transaction_type_distribution: HashMap::new(),
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        assert_eq!(analytics.total_transactions, 1000);
        assert_eq!(analytics.pending_transactions, 50);
        assert_eq!(analytics.total_volume, 1000000);
    }
}
