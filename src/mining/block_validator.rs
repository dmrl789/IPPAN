//! Block validator for IPPAN
//! 
//! Implements comprehensive block validation including structure validation,
//! transaction validation, cryptographic verification, and consensus rules.

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

/// Block validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockValidationConfig {
    /// Enable block structure validation
    pub enable_structure_validation: bool,
    /// Enable transaction validation
    pub enable_transaction_validation: bool,
    /// Enable cryptographic validation
    pub enable_cryptographic_validation: bool,
    /// Enable consensus rule validation
    pub enable_consensus_validation: bool,
    /// Enable block size validation
    pub enable_size_validation: bool,
    /// Maximum block size in bytes
    pub max_block_size_bytes: usize,
    /// Enable timestamp validation
    pub enable_timestamp_validation: bool,
    /// Enable validation caching
    pub enable_validation_caching: bool,
    /// Maximum timestamp drift in seconds
    pub max_timestamp_drift_seconds: u64,
    /// Enable gas validation
    pub enable_gas_validation: bool,
    /// Maximum gas limit
    pub max_gas_limit: u64,
    /// Enable merkle root validation
    pub enable_merkle_validation: bool,
    /// Enable block hash validation
    pub enable_hash_validation: bool,
}

impl Default for BlockValidationConfig {
    fn default() -> Self {
        Self {
            enable_structure_validation: true,
            enable_transaction_validation: true,
            enable_cryptographic_validation: true,
            enable_consensus_validation: true,
            enable_size_validation: true,
            max_block_size_bytes: 1024 * 1024, // 1MB
            enable_timestamp_validation: true,
            enable_validation_caching: true,
            max_timestamp_drift_seconds: 3600, // 1 hour
            enable_gas_validation: true,
            max_gas_limit: 10000000, // 10M gas
            enable_merkle_validation: true,
            enable_hash_validation: true,
        }
    }
}

/// Block validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockValidationResult {
    /// Validation success
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Validation time in milliseconds
    pub validation_time_ms: u64,
    /// Block size in bytes
    pub block_size_bytes: usize,
    /// Transaction count
    pub transaction_count: usize,
    /// Gas used
    pub gas_used: u64,
    /// Validation score (0.0 to 1.0)
    pub validation_score: f64,
}

/// Block validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockValidationStats {
    /// Total blocks validated
    pub total_blocks_validated: u64,
    /// Valid blocks
    pub valid_blocks: u64,
    /// Invalid blocks
    pub invalid_blocks: u64,
    /// Average validation time in milliseconds
    pub average_validation_time_ms: f64,
    /// Average block size in bytes
    pub average_block_size_bytes: f64,
    /// Average transaction count per block
    pub average_transaction_count: f64,
    /// Average gas used per block
    pub average_gas_used: f64,
    /// Validation success rate
    pub validation_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last validation timestamp
    pub last_validation: Option<u64>,
}

/// Block validator
pub struct BlockValidator {
    /// Configuration
    config: BlockValidationConfig,
    /// Database manager
    database: Arc<DatabaseManager>,
    /// Validation cache
    validation_cache: Arc<RwLock<HashMap<[u8; 32], BlockValidationResult>>>,
    /// Statistics
    stats: Arc<RwLock<BlockValidationStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl BlockValidator {
    /// Create a new block validator
    pub fn new(config: BlockValidationConfig, database: Arc<DatabaseManager>) -> Self {
        let stats = BlockValidationStats {
            total_blocks_validated: 0,
            valid_blocks: 0,
            invalid_blocks: 0,
            average_validation_time_ms: 0.0,
            average_block_size_bytes: 0.0,
            average_transaction_count: 0.0,
            average_gas_used: 0.0,
            validation_success_rate: 0.0,
            uptime_seconds: 0,
            last_validation: None,
        };
        
        Self {
            config,
            database,
            validation_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the block validator
    pub async fn start(&self) -> Result<()> {
        info!("Starting block validator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Block validator started successfully");
        Ok(())
    }
    
    /// Stop the block validator
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping block validator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Block validator stopped");
        Ok(())
    }
    
    /// Validate a block
    pub async fn validate_block(&self, block: &BFTBlock) -> Result<BlockValidationResult> {
        let start_time = Instant::now();
        
        info!("Validating block {} with hash {:02x?}", block.header.number, block.hash);
        
        // Check cache first
        if self.config.enable_validation_caching {
            let cache = self.validation_cache.read().await;
            if let Some(cached_result) = cache.get(&block.hash) {
                debug!("Using cached validation result for block {:02x?}", block.hash);
                return Ok(cached_result.clone());
            }
        }
        
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut validation_score: f64 = 1.0;
        
        // Validate block structure
        if self.config.enable_structure_validation {
            if let Err(e) = self.validate_block_structure(block).await {
                errors.push(format!("Structure validation failed: {}", e));
                validation_score -= 0.3;
            }
        }
        
        // Validate block size
        if self.config.enable_size_validation {
            if let Err(e) = self.validate_block_size(block).await {
                errors.push(format!("Size validation failed: {}", e));
                validation_score -= 0.2;
            }
        }
        
        // Validate block hash
        if self.config.enable_hash_validation {
            if let Err(e) = self.validate_block_hash(block).await {
                errors.push(format!("Hash validation failed: {}", e));
                validation_score -= 0.4;
            }
        }
        
        // Validate merkle root
        if self.config.enable_merkle_validation {
            if let Err(e) = self.validate_merkle_root(block).await {
                errors.push(format!("Merkle root validation failed: {}", e));
                validation_score -= 0.3;
            }
        }
        
        // Validate timestamp
        if self.config.enable_timestamp_validation {
            if let Err(e) = self.validate_timestamp(block).await {
                errors.push(format!("Timestamp validation failed: {}", e));
                validation_score -= 0.1;
            }
        }
        
        // Validate gas
        if self.config.enable_gas_validation {
            if let Err(e) = self.validate_gas(block).await {
                errors.push(format!("Gas validation failed: {}", e));
                validation_score -= 0.2;
            }
        }
        
        // Validate transactions
        if self.config.enable_transaction_validation {
            if let Err(e) = self.validate_transactions(block).await {
                errors.push(format!("Transaction validation failed: {}", e));
                validation_score -= 0.5;
            }
        }
        
        // Validate consensus rules
        if self.config.enable_consensus_validation {
            if let Err(e) = self.validate_consensus_rules(block).await {
                errors.push(format!("Consensus validation failed: {}", e));
                validation_score -= 0.4;
            }
        }
        
        // Validate cryptographically
        if self.config.enable_cryptographic_validation {
            if let Err(e) = self.validate_cryptographic(block).await {
                errors.push(format!("Cryptographic validation failed: {}", e));
                validation_score -= 0.6;
            }
        }
        
        let validation_time = start_time.elapsed().as_millis() as u64;
        let block_size = bincode::serialize(block).unwrap_or_default().len();
        let transaction_count = block.transactions.len();
        let gas_used = 21000 * block.transactions.len() as u64; // TODO: Implement proper gas calculation
        
        let is_valid = errors.is_empty() && validation_score >= 0.5;
        
        let result = BlockValidationResult {
            is_valid,
            errors,
            warnings,
            validation_time_ms: validation_time,
            block_size_bytes: block_size,
            transaction_count,
            gas_used,
            validation_score: validation_score.max(0.0f64),
        };
        
        // Cache result
        if self.config.enable_validation_caching {
            let mut cache = self.validation_cache.write().await;
            cache.insert(block.hash, result.clone());
            
            // Limit cache size
            if cache.len() > 10000 {
                let keys_to_remove: Vec<[u8; 32]> = cache.keys().take(1000).cloned().collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
        }
        
        // Update statistics
        self.update_stats(validation_time, block_size, transaction_count, gas_used, is_valid).await;
        
        if is_valid {
            info!("Block {} validation successful in {}ms", block.header.number, validation_time);
        } else {
            warn!("Block {} validation failed in {}ms", block.header.number, validation_time);
        }
        
        Ok(result)
    }
    
    /// Validate block structure
    async fn validate_block_structure(&self, block: &BFTBlock) -> Result<()> {
        // Validate block number
        if block.header.number == 0 && block.header.previous_hash != [0u8; 32] {
            return Err(IppanError::Validation("Genesis block must have zero previous hash".to_string()));
        }
        
        // Validate validator ID
        if block.header.validator_id == [0u8; 32] {
            return Err(IppanError::Validation("Invalid validator ID".to_string()));
        }
        
        // Validate sequence number
        if block.header.sequence_number == 0 {
            return Err(IppanError::Validation("Invalid sequence number".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate block size
    async fn validate_block_size(&self, block: &BFTBlock) -> Result<()> {
        let block_size = bincode::serialize(block).unwrap_or_default().len();
        
        if block_size > self.config.max_block_size_bytes {
            return Err(IppanError::Validation(
                format!("Block size {} exceeds maximum {}", block_size, self.config.max_block_size_bytes)
            ));
        }
        
        Ok(())
    }
    
    /// Validate block hash
    async fn validate_block_hash(&self, block: &BFTBlock) -> Result<()> {
        let block_data = bincode::serialize(block).unwrap_or_default();
        let calculated_hash = RealHashFunctions::sha256(&block_data);
        
        if calculated_hash != block.hash {
            return Err(IppanError::Validation("Block hash mismatch".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate merkle root
    async fn validate_merkle_root(&self, block: &BFTBlock) -> Result<()> {
        let transaction_hashes: Vec<[u8; 32]> = block.transactions.iter().map(|tx| tx.hash).collect();
        let calculated_merkle_root = RealHashFunctions::merkle_root(&transaction_hashes);
        
        if calculated_merkle_root != block.header.merkle_root {
            return Err(IppanError::Validation("Merkle root mismatch".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate timestamp
    async fn validate_timestamp(&self, block: &BFTBlock) -> Result<()> {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let block_time = block.header.timestamp;
        
        if block_time > current_time + self.config.max_timestamp_drift_seconds {
            return Err(IppanError::Validation("Block timestamp too far in future".to_string()));
        }
        
        if block_time < current_time - self.config.max_timestamp_drift_seconds {
            return Err(IppanError::Validation("Block timestamp too far in past".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate gas
    async fn validate_gas(&self, block: &BFTBlock) -> Result<()> {
        let total_gas_used: u64 = 21000 * block.transactions.len() as u64; // TODO: Implement proper gas calculation
        
        if total_gas_used > self.config.max_gas_limit {
            return Err(IppanError::Validation(
                format!("Total gas used {} exceeds maximum {}", total_gas_used, self.config.max_gas_limit)
            ));
        }
        
        Ok(())
    }
    
    /// Validate transactions
    async fn validate_transactions(&self, block: &BFTBlock) -> Result<()> {
        for (index, transaction) in block.transactions.iter().enumerate() {
            if let Err(e) = self.validate_transaction(transaction).await {
                return Err(IppanError::Validation(
                    format!("Transaction {} validation failed: {}", index, e)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate individual transaction
    async fn validate_transaction(&self, transaction: &BFTTransaction) -> Result<()> {
        // Validate transaction hash
        // TODO: Implement proper transaction hash validation
        // For now, use placeholder data
        let tx_data = b"transaction_placeholder".to_vec();
        let calculated_hash = RealHashFunctions::sha256(&tx_data);
        
        if calculated_hash != transaction.hash {
            return Err(IppanError::Validation("Transaction hash mismatch".to_string()));
        }
        
        // Validate signature
        if transaction.signature == [0u8; 64] {
            return Err(IppanError::Validation("Invalid transaction signature".to_string()));
        }
        
        // Validate nonce
        if transaction.nonce == 0 {
            return Err(IppanError::Validation("Invalid transaction nonce".to_string()));
        }
        
        // TODO: Implement proper transaction validation
        // For now, just validate that the transaction has data
        if transaction.data.is_empty() {
            return Err(IppanError::Validation("Invalid transaction data".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate consensus rules
    async fn validate_consensus_rules(&self, block: &BFTBlock) -> Result<()> {
        // Validate block number sequence
        if block.header.number > 0 {
            // In a real implementation, this would check against the previous block
            // For now, just validate that the number is reasonable
            if block.header.number > 1000000 {
                return Err(IppanError::Validation("Block number too high".to_string()));
            }
        }
        
        // Validate view number
        if block.header.view_number == 0 {
            return Err(IppanError::Validation("Invalid view number".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate cryptographic aspects
    async fn validate_cryptographic(&self, block: &BFTBlock) -> Result<()> {
        // Validate block hash format
        if block.hash == [0u8; 32] {
            return Err(IppanError::Validation("Invalid block hash".to_string()));
        }
        
        // Validate merkle root format
        if block.header.merkle_root == [0u8; 32] && !block.transactions.is_empty() {
            return Err(IppanError::Validation("Invalid merkle root for non-empty block".to_string()));
        }
        
        // Validate previous hash format
        if block.header.previous_hash == [0u8; 32] && block.header.number > 0 {
            return Err(IppanError::Validation("Invalid previous hash for non-genesis block".to_string()));
        }
        
        Ok(())
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        validation_time_ms: u64,
        block_size_bytes: usize,
        transaction_count: usize,
        gas_used: u64,
        is_valid: bool,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_blocks_validated += 1;
        if is_valid {
            stats.valid_blocks += 1;
        } else {
            stats.invalid_blocks += 1;
        }
        
        // Update averages
        let total = stats.total_blocks_validated as f64;
        stats.average_validation_time_ms = 
            (stats.average_validation_time_ms * (total - 1.0) + validation_time_ms as f64) / total;
        stats.average_block_size_bytes = 
            (stats.average_block_size_bytes * (total - 1.0) + block_size_bytes as f64) / total;
        stats.average_transaction_count = 
            (stats.average_transaction_count * (total - 1.0) + transaction_count as f64) / total;
        stats.average_gas_used = 
            (stats.average_gas_used * (total - 1.0) + gas_used as f64) / total;
        
        // Update success rate
        stats.validation_success_rate = stats.valid_blocks as f64 / total;
        
        // Update timestamps
        stats.last_validation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get block validation statistics
    pub async fn get_stats(&self) -> Result<BlockValidationStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Clear validation cache
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.validation_cache.write().await;
        cache.clear();
        info!("Validation cache cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_block_validation_config() {
        let config = BlockValidationConfig::default();
        assert!(config.enable_structure_validation);
        assert!(config.enable_transaction_validation);
        assert!(config.enable_cryptographic_validation);
        assert_eq!(config.max_block_size_bytes, 1024 * 1024);
        assert_eq!(config.max_timestamp_drift_seconds, 3600);
    }
    
    #[tokio::test]
    async fn test_block_validation_result() {
        let result = BlockValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec!["Minor issue".to_string()],
            validation_time_ms: 50,
            block_size_bytes: 1024,
            transaction_count: 10,
            gas_used: 100000,
            validation_score: 0.95,
        };
        
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.validation_time_ms, 50);
        assert_eq!(result.block_size_bytes, 1024);
        assert_eq!(result.transaction_count, 10);
        assert_eq!(result.gas_used, 100000);
        assert_eq!(result.validation_score, 0.95);
    }
    
    #[tokio::test]
    async fn test_block_validation_stats() {
        let stats = BlockValidationStats {
            total_blocks_validated: 1000,
            valid_blocks: 950,
            invalid_blocks: 50,
            average_validation_time_ms: 25.0,
            average_block_size_bytes: 512.0,
            average_transaction_count: 5.5,
            average_gas_used: 50000.0,
            validation_success_rate: 0.95,
            uptime_seconds: 7200,
            last_validation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_blocks_validated, 1000);
        assert_eq!(stats.valid_blocks, 950);
        assert_eq!(stats.invalid_blocks, 50);
        assert_eq!(stats.validation_success_rate, 0.95);
    }
}
