//! Block creator for IPPAN
//! 
//! Implements actual block creation with transaction selection,
//! header generation, and proper block structure.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use crate::database::{DatabaseManager, StoredTransaction, StoredBlock};
use crate::consensus::bft_engine::{BFTBlock, BFTBlockHeader, BFTTransaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Block creation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCreationConfig {
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Maximum block size in bytes
    pub max_block_size_bytes: usize,
    /// Block time target in seconds
    pub block_time_target_seconds: u64,
    /// Enable transaction prioritization
    pub enable_transaction_prioritization: bool,
    /// Minimum transaction fee
    pub min_transaction_fee: u64,
    /// Enable block compression
    pub enable_block_compression: bool,
    /// Gas limit per block
    pub gas_limit_per_block: u64,
    /// Gas price minimum
    pub gas_price_minimum: u64,
}

impl Default for BlockCreationConfig {
    fn default() -> Self {
        Self {
            max_transactions_per_block: 1000,
            max_block_size_bytes: 1024 * 1024, // 1MB
            block_time_target_seconds: 10,
            enable_transaction_prioritization: true,
            min_transaction_fee: 100,
            enable_block_compression: true,
            gas_limit_per_block: 10000000, // 10M gas
            gas_price_minimum: 1,
        }
    }
}

/// Block creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCreationRequest {
    /// Previous block hash
    pub previous_block_hash: [u8; 32],
    /// Block number
    pub block_number: u64,
    /// Validator ID
    pub validator_id: [u8; 32],
    /// View number
    pub view_number: u64,
    /// Sequence number
    pub sequence_number: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Gas used
    pub gas_used: u64,
    /// Difficulty
    pub difficulty: u64,
}

/// Block creation result
#[derive(Debug, Clone)]
pub struct BlockCreationResult {
    /// Created block
    pub block: BFTBlock,
    /// Creation time in milliseconds
    pub creation_time_ms: u64,
    /// Transactions included
    pub transactions_included: usize,
    /// Block size in bytes
    pub block_size_bytes: usize,
    /// Gas used
    pub gas_used: u64,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Block creation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCreationStats {
    /// Total blocks created
    pub total_blocks_created: u64,
    /// Successful block creations
    pub successful_creations: u64,
    /// Failed block creations
    pub failed_creations: u64,
    /// Average creation time in milliseconds
    pub average_creation_time_ms: f64,
    /// Average transactions per block
    pub average_transactions_per_block: f64,
    /// Average block size in bytes
    pub average_block_size_bytes: f64,
    /// Average gas used per block
    pub average_gas_used: f64,
    /// Creation success rate
    pub creation_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last creation timestamp
    pub last_creation: Option<u64>,
}

/// Block creator
pub struct BlockCreator {
    /// Configuration
    config: BlockCreationConfig,
    /// Database manager
    database: Arc<DatabaseManager>,
    /// Statistics
    stats: Arc<RwLock<BlockCreationStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl BlockCreator {
    /// Create a new block creator
    pub fn new(config: BlockCreationConfig, database: Arc<DatabaseManager>) -> Self {
        let stats = BlockCreationStats {
            total_blocks_created: 0,
            successful_creations: 0,
            failed_creations: 0,
            average_creation_time_ms: 0.0,
            average_transactions_per_block: 0.0,
            average_block_size_bytes: 0.0,
            average_gas_used: 0.0,
            creation_success_rate: 0.0,
            uptime_seconds: 0,
            last_creation: None,
        };
        
        Self {
            config,
            database,
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the block creator
    pub async fn start(&self) -> Result<()> {
        info!("Starting block creator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Block creator started successfully");
        Ok(())
    }
    
    /// Stop the block creator
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping block creator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Block creator stopped");
        Ok(())
    }
    
    /// Create a new block
    pub async fn create_block(&self, request: BlockCreationRequest) -> Result<BlockCreationResult> {
        let start_time = Instant::now();
        
        info!("Creating block {} with previous hash {:02x?}", request.block_number, request.previous_block_hash);
        
        // Select transactions for the block
        let transactions = self.select_transactions_for_block().await?;
        
        // Create block header
        let header = self.create_block_header(&request, &transactions).await?;
        
        // Create block
        let mut block = BFTBlock {
            header,
            transactions: transactions.clone(),
            hash: [0u8; 32], // Will be calculated
        };
        
        // Calculate block hash
        let block_data = bincode::serialize(&block)
            .map_err(|e| IppanError::Mining(format!("Failed to serialize block: {}", e)))?;
        block.hash = RealHashFunctions::sha256(&block_data);
        
        // Calculate block size
        let block_size_bytes = block_data.len();
        
        // Calculate gas used
        let gas_used = 21000 * transactions.len() as u64; // TODO: Implement proper gas calculation
        
        let creation_time = start_time.elapsed().as_millis() as u64;
        
        // Update statistics
        self.update_stats(creation_time, transactions.len(), block_size_bytes, gas_used, true).await;
        
        let result = BlockCreationResult {
            block,
            creation_time_ms: creation_time,
            transactions_included: transactions.len(),
            block_size_bytes,
            gas_used,
            success: true,
            error_message: None,
        };
        
        info!("Created block {} with {} transactions in {}ms", 
            request.block_number, transactions.len(), creation_time);
        
        Ok(result)
    }
    
    /// Select transactions for the block
    async fn select_transactions_for_block(&self) -> Result<Vec<BFTTransaction>> {
        debug!("Selecting transactions for block");
        
        // Get pending transactions from database
        let pending_transactions = self.get_pending_transactions().await?;
        
        if pending_transactions.is_empty() {
            return Ok(vec![]);
        }
        
        // Sort transactions by priority (fee, timestamp, etc.)
        let mut sorted_transactions = pending_transactions;
        if self.config.enable_transaction_prioritization {
            sorted_transactions.sort_by(|a, b| {
                // Sort by fee (descending), then by timestamp (ascending)
                b.fee.cmp(&a.fee).then(a.timestamp.cmp(&b.timestamp))
            });
        }
        
        // Select transactions up to the limit
        let mut selected_transactions = Vec::new();
        let mut total_size = 0;
        let mut total_gas = 0;
        
        for transaction in sorted_transactions {
            // Check transaction fee
            if transaction.fee < self.config.min_transaction_fee {
                continue;
            }
            
            // Check block size limit
            let tx_size = 1024; // TODO: Implement proper transaction size calculation
            if total_size + tx_size > self.config.max_block_size_bytes {
                break;
            }
            
            // Check transaction count limit
            if selected_transactions.len() >= self.config.max_transactions_per_block {
                break;
            }
            
            // Check gas limit
            let tx_gas = transaction.gas_used.unwrap_or(21000);
            if total_gas + tx_gas > self.config.gas_limit_per_block {
                break;
            }
            
            // Convert to BFT transaction
            let bft_transaction = BFTTransaction {
                hash: transaction.hash,
                data: transaction.data.unwrap_or_default(),
                sender: [0u8; 32], // Placeholder sender
                from: transaction.from_address,
                to: transaction.to_address,
                amount: transaction.amount,
                fee: transaction.fee,
                gas_used: tx_gas,
                gas_price: transaction.fee / tx_gas.max(1),
                nonce: transaction.nonce,
                signature: transaction.signature,
            };
            
            selected_transactions.push(bft_transaction);
            total_size += tx_size;
            total_gas += tx_gas;
        }
        
        debug!("Selected {} transactions for block", selected_transactions.len());
        Ok(selected_transactions)
    }
    
    /// Create block header
    async fn create_block_header(
        &self,
        request: &BlockCreationRequest,
        transactions: &[BFTTransaction],
    ) -> Result<BFTBlockHeader> {
        // Calculate merkle root
        let transaction_hashes: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.hash).collect();
        let merkle_root = RealHashFunctions::merkle_root(&transaction_hashes);
        
        // Create header
        let header = BFTBlockHeader {
            number: request.block_number,
            previous_hash: request.previous_block_hash,
            merkle_root,
            timestamp: request.timestamp,
            view_number: request.view_number,
            sequence_number: request.sequence_number,
            validator_id: request.validator_id,
        };
        
        Ok(header)
    }
    
    /// Get pending transactions from database
    async fn get_pending_transactions(&self) -> Result<Vec<StoredTransaction>> {
        // In a real implementation, this would query the database for pending transactions
        // For now, return empty vector as placeholder
        debug!("Getting pending transactions from database (placeholder)");
        Ok(vec![])
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        creation_time_ms: u64,
        transaction_count: usize,
        block_size_bytes: usize,
        gas_used: u64,
        success: bool,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_blocks_created += 1;
        if success {
            stats.successful_creations += 1;
        } else {
            stats.failed_creations += 1;
        }
        
        // Update averages
        let total = stats.total_blocks_created as f64;
        stats.average_creation_time_ms = 
            (stats.average_creation_time_ms * (total - 1.0) + creation_time_ms as f64) / total;
        stats.average_transactions_per_block = 
            (stats.average_transactions_per_block * (total - 1.0) + transaction_count as f64) / total;
        stats.average_block_size_bytes = 
            (stats.average_block_size_bytes * (total - 1.0) + block_size_bytes as f64) / total;
        stats.average_gas_used = 
            (stats.average_gas_used * (total - 1.0) + gas_used as f64) / total;
        
        // Update success rate
        stats.creation_success_rate = stats.successful_creations as f64 / total;
        
        // Update timestamps
        stats.last_creation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get block creation statistics
    pub async fn get_stats(&self) -> Result<BlockCreationStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Validate block creation request
    fn validate_request(&self, request: &BlockCreationRequest) -> Result<()> {
        // Validate block number
        if request.block_number == 0 && request.previous_block_hash != [0u8; 32] {
            return Err(IppanError::Mining("Genesis block must have zero previous hash".to_string()));
        }
        
        // Validate timestamp
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if request.timestamp > current_time + 3600 { // Allow 1 hour in future
            return Err(IppanError::Mining("Block timestamp too far in future".to_string()));
        }
        
        // Validate gas limit
        if request.gas_limit == 0 {
            return Err(IppanError::Mining("Gas limit cannot be zero".to_string()));
        }
        
        // Validate gas used
        if request.gas_used > request.gas_limit {
            return Err(IppanError::Mining("Gas used exceeds gas limit".to_string()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_block_creation_config() {
        let config = BlockCreationConfig::default();
        assert_eq!(config.max_transactions_per_block, 1000);
        assert_eq!(config.max_block_size_bytes, 1024 * 1024);
        assert_eq!(config.block_time_target_seconds, 10);
        assert!(config.enable_transaction_prioritization);
    }
    
    #[tokio::test]
    async fn test_block_creation_request() {
        let request = BlockCreationRequest {
            previous_block_hash: [1u8; 32],
            block_number: 1,
            validator_id: [2u8; 32],
            view_number: 1,
            sequence_number: 1,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            gas_limit: 1000000,
            gas_used: 100000,
            difficulty: 1000,
        };
        
        assert_eq!(request.block_number, 1);
        assert_eq!(request.gas_limit, 1000000);
        assert_eq!(request.gas_used, 100000);
    }
    
    #[tokio::test]
    async fn test_block_creation_result() {
        let result = BlockCreationResult {
            block: BFTBlock {
                header: BFTBlockHeader {
                    number: 1,
                    previous_hash: [1u8; 32],
                    merkle_root: [2u8; 32],
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    view_number: 1,
                    sequence_number: 1,
                    validator_id: [3u8; 32],
                },
                transactions: vec![],
                hash: [4u8; 32],
            },
            creation_time_ms: 100,
            transactions_included: 5,
            block_size_bytes: 1024,
            gas_used: 100000,
            success: true,
            error_message: None,
        };
        
        assert!(result.success);
        assert_eq!(result.transactions_included, 5);
        assert_eq!(result.block_size_bytes, 1024);
        assert_eq!(result.gas_used, 100000);
    }
    
    #[tokio::test]
    async fn test_block_creation_stats() {
        let stats = BlockCreationStats {
            total_blocks_created: 100,
            successful_creations: 95,
            failed_creations: 5,
            average_creation_time_ms: 50.0,
            average_transactions_per_block: 10.5,
            average_block_size_bytes: 512.0,
            average_gas_used: 100000.0,
            creation_success_rate: 0.95,
            uptime_seconds: 3600,
            last_creation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_blocks_created, 100);
        assert_eq!(stats.successful_creations, 95);
        assert_eq!(stats.failed_creations, 5);
        assert_eq!(stats.creation_success_rate, 0.95);
    }
}
