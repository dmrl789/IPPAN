//! Mining manager for IPPAN
//! 
//! Orchestrates the entire mining process including block creation,
//! validation, transaction selection, and propagation.

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealHashFunctions, RealEd25519, RealTransactionSigner};
use crate::database::{DatabaseManager, StoredTransaction, StoredBlock};
use crate::consensus::bft_engine::{BFTBlock, BFTBlockHeader, BFTTransaction};
use crate::network::real_p2p::RealP2PNetwork;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

use super::{
    BlockCreator, BlockValidator, TransactionSelector, BlockPropagator,
    BlockCreationConfig, BlockValidationConfig, TransactionSelectionConfig, BlockPropagationConfig,
    BlockCreationRequest, BlockCreationResult, BlockValidationResult, TransactionSelectionResult, BlockPropagationResult,
    MiningConfig, MiningStats,
};

/// Mining manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningManagerConfig {
    /// Enable mining
    pub enable_mining: bool,
    /// Mining interval in seconds
    pub mining_interval_seconds: u64,
    /// Enable automatic block creation
    pub enable_automatic_block_creation: bool,
    /// Enable block validation
    pub enable_block_validation: bool,
    /// Enable transaction selection
    pub enable_transaction_selection: bool,
    /// Enable block propagation
    pub enable_block_propagation: bool,
    /// Maximum mining attempts per interval
    pub max_mining_attempts_per_interval: u32,
    /// Mining timeout in seconds
    pub mining_timeout_seconds: u64,
    /// Enable mining statistics
    pub enable_mining_statistics: bool,
    /// Enable mining performance monitoring
    pub enable_performance_monitoring: bool,
}

impl Default for MiningManagerConfig {
    fn default() -> Self {
        Self {
            enable_mining: true,
            mining_interval_seconds: 10,
            enable_automatic_block_creation: true,
            enable_block_validation: true,
            enable_transaction_selection: true,
            enable_block_propagation: true,
            max_mining_attempts_per_interval: 3,
            mining_timeout_seconds: 30,
            enable_mining_statistics: true,
            enable_performance_monitoring: true,
        }
    }
}

/// Mining operation result
#[derive(Debug, Clone)]
pub struct MiningOperationResult {
    /// Operation success
    pub success: bool,
    /// Created block
    pub created_block: Option<BFTBlock>,
    /// Validation result
    pub validation_result: Option<BlockValidationResult>,
    /// Selection result
    pub selection_result: Option<TransactionSelectionResult>,
    /// Propagation result
    pub propagation_result: Option<BlockPropagationResult>,
    /// Total operation time in milliseconds
    pub total_operation_time_ms: u64,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Mining statistics
    pub mining_stats: MiningStats,
}

/// Mining manager
pub struct MiningManager {
    /// Configuration
    config: MiningManagerConfig,
    /// Block creator
    block_creator: Arc<BlockCreator>,
    /// Block validator
    block_validator: Arc<BlockValidator>,
    /// Transaction selector
    transaction_selector: Arc<TransactionSelector>,
    /// Block propagator
    block_propagator: Arc<BlockPropagator>,
    /// Database manager
    database: Arc<DatabaseManager>,
    /// Network manager
    network: Arc<RealP2PNetwork>,
    /// Statistics
    stats: Arc<RwLock<MiningStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
    /// Current block number
    current_block_number: Arc<RwLock<u64>>,
    /// Current view number
    current_view_number: Arc<RwLock<u64>>,
    /// Current sequence number
    current_sequence_number: Arc<RwLock<u64>>,
}

impl MiningManager {
    /// Create a new mining manager
    pub fn new(
        config: MiningManagerConfig,
        database: Arc<DatabaseManager>,
        network: Arc<RealP2PNetwork>,
    ) -> Self {
        // Create mining components
        let block_creator = Arc::new(BlockCreator::new(
            BlockCreationConfig::default(),
            database.clone(),
        ));
        
        let block_validator = Arc::new(BlockValidator::new(
            BlockValidationConfig::default(),
            database.clone(),
        ));
        
        let transaction_selector = Arc::new(TransactionSelector::new(
            TransactionSelectionConfig::default(),
            database.clone(),
        ));
        
        let block_propagator = Arc::new(BlockPropagator::new(
            BlockPropagationConfig::default(),
            network.clone(),
        ));
        
        let stats = MiningStats {
            blocks_created: 0,
            blocks_validated: 0,
            blocks_propagated: 0,
            average_block_creation_time_ms: 0.0,
            average_block_validation_time_ms: 0.0,
            average_transactions_per_block: 0.0,
            average_block_size_bytes: 0.0,
            block_creation_success_rate: 0.0,
            block_validation_success_rate: 0.0,
            uptime_seconds: 0,
            last_block_creation: None,
            last_block_validation: None,
        };
        
        Self {
            config,
            block_creator,
            block_validator,
            transaction_selector,
            block_propagator,
            database,
            network,
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
            current_block_number: Arc::new(RwLock::new(0)),
            current_view_number: Arc::new(RwLock::new(1)),
            current_sequence_number: Arc::new(RwLock::new(1)),
        }
    }
    
    /// Start the mining manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting mining manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Start mining components
        self.block_creator.start().await?;
        self.block_validator.start().await?;
        self.transaction_selector.start().await?;
        self.block_propagator.start().await?;
        
        // Start mining loop
        let config = self.config.clone();
        let block_creator = self.block_creator.clone();
        let block_validator = self.block_validator.clone();
        let transaction_selector = self.transaction_selector.clone();
        let block_propagator = self.block_propagator.clone();
        let database = self.database.clone();
        let network = self.network.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        let current_block_number = self.current_block_number.clone();
        let current_view_number = self.current_view_number.clone();
        let current_sequence_number = self.current_sequence_number.clone();
        
        tokio::spawn(async move {
            Self::mining_loop(
                config,
                block_creator,
                block_validator,
                transaction_selector,
                block_propagator,
                database,
                network,
                stats,
                is_running,
                start_time,
                current_block_number,
                current_view_number,
                current_sequence_number,
            ).await;
        });
        
        info!("Mining manager started successfully");
        Ok(())
    }
    
    /// Stop the mining manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping mining manager");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // Stop mining components
        self.block_creator.stop().await?;
        self.block_validator.stop().await?;
        self.transaction_selector.stop().await?;
        self.block_propagator.stop().await?;
        
        info!("Mining manager stopped");
        Ok(())
    }
    
    /// Perform a mining operation
    pub async fn mine_block(&self) -> Result<MiningOperationResult> {
        let start_time = Instant::now();
        
        info!("Starting mining operation");
        
        if !self.config.enable_mining {
            return Ok(MiningOperationResult {
                success: false,
                created_block: None,
                validation_result: None,
                selection_result: None,
                propagation_result: None,
                total_operation_time_ms: start_time.elapsed().as_millis() as u64,
                error_message: Some("Mining disabled".to_string()),
                mining_stats: self.get_stats().await?,
            });
        }
        
        let mut created_block = None;
        let mut validation_result = None;
        let mut selection_result = None;
        let mut propagation_result = None;
        let mut error_message = None;
        
        // Step 1: Select transactions
        if self.config.enable_transaction_selection {
            match self.transaction_selector.select_transactions().await {
                Ok(result) => {
                    selection_result = Some(result);
                    debug!("Transaction selection completed");
                }
                Err(e) => {
                    error!("Transaction selection failed: {}", e);
                    error_message = Some(format!("Transaction selection failed: {}", e));
                }
            }
        }
        
        // Step 2: Create block
        if self.config.enable_automatic_block_creation && error_message.is_none() {
            let block_number = {
                let mut current = self.current_block_number.write().await;
                *current += 1;
                *current
            };
            
            let view_number = {
                let current = self.current_view_number.read().await;
                *current
            };
            
            let sequence_number = {
                let mut current = self.current_sequence_number.write().await;
                *current += 1;
                *current
            };
            
            let request = BlockCreationRequest {
                previous_block_hash: [0u8; 32], // TODO: Get from latest block
                block_number,
                validator_id: [1u8; 32], // TODO: Get from node ID
                view_number,
                sequence_number,
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                gas_limit: 10000000,
                gas_used: 0,
                difficulty: 1000,
            };
            
            match self.block_creator.create_block(request).await {
                Ok(result) => {
                    created_block = Some(result.block);
                    debug!("Block creation completed");
                }
                Err(e) => {
                    error!("Block creation failed: {}", e);
                    error_message = Some(format!("Block creation failed: {}", e));
                }
            }
        }
        
        // Step 3: Validate block
        if self.config.enable_block_validation && created_block.is_some() && error_message.is_none() {
            let block = created_block.as_ref().unwrap();
            match self.block_validator.validate_block(block).await {
                Ok(result) => {
                    let is_valid = result.is_valid;
                    validation_result = Some(result);
                    if !is_valid {
                        error_message = Some("Block validation failed".to_string());
                    }
                    debug!("Block validation completed");
                }
                Err(e) => {
                    error!("Block validation failed: {}", e);
                    error_message = Some(format!("Block validation failed: {}", e));
                }
            }
        }
        
        // Step 4: Propagate block
        if self.config.enable_block_propagation && created_block.is_some() && error_message.is_none() {
            let block = created_block.as_ref().unwrap();
            let propagation_request = super::BlockPropagationRequest {
                block: block.clone(),
                target_peers: vec![], // Broadcast to all peers
                priority: super::PropagationPriority::High,
                retry_attempts: 0,
                request_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            };
            
            match self.block_propagator.propagate_block(propagation_request).await {
                Ok(result) => {
                    propagation_result = Some(result);
                    debug!("Block propagation completed");
                }
                Err(e) => {
                    error!("Block propagation failed: {}", e);
                    error_message = Some(format!("Block propagation failed: {}", e));
                }
            }
        }
        
        let total_operation_time = start_time.elapsed().as_millis() as u64;
        let success = error_message.is_none();
        
        // Update statistics
        self.update_stats(
            created_block.is_some(),
            validation_result.as_ref().map(|r| r.is_valid).unwrap_or(false),
            propagation_result.as_ref().map(|r| r.success).unwrap_or(false),
            total_operation_time,
        ).await;
        
        let result = MiningOperationResult {
            success,
            created_block,
            validation_result,
            selection_result,
            propagation_result,
            total_operation_time_ms: total_operation_time,
            error_message,
            mining_stats: self.get_stats().await?,
        };
        
        if success {
            info!("Mining operation completed successfully in {}ms", total_operation_time);
        } else {
            warn!("Mining operation failed in {}ms", total_operation_time);
        }
        
        Ok(result)
    }
    
    /// Mining loop
    async fn mining_loop(
        config: MiningManagerConfig,
        block_creator: Arc<BlockCreator>,
        block_validator: Arc<BlockValidator>,
        transaction_selector: Arc<TransactionSelector>,
        block_propagator: Arc<BlockPropagator>,
        database: Arc<DatabaseManager>,
        network: Arc<RealP2PNetwork>,
        stats: Arc<RwLock<MiningStats>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
        current_block_number: Arc<RwLock<u64>>,
        current_view_number: Arc<RwLock<u64>>,
        current_sequence_number: Arc<RwLock<u64>>,
    ) {
        while *is_running.read().await {
            if config.enable_mining {
                // Perform mining operation
                debug!("Performing mining operation");
                
                // Update statistics
                let mut stats = stats.write().await;
                stats.uptime_seconds = start_time.elapsed().as_secs();
            }
            
            // Sleep for mining interval
            tokio::time::sleep(Duration::from_secs(config.mining_interval_seconds)).await;
        }
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        block_created: bool,
        block_validated: bool,
        block_propagated: bool,
        operation_time_ms: u64,
    ) {
        let mut stats = self.stats.write().await;
        
        if block_created {
            stats.blocks_created += 1;
            stats.last_block_creation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        }
        
        if block_validated {
            stats.blocks_validated += 1;
            stats.last_block_validation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        }
        
        if block_propagated {
            stats.blocks_propagated += 1;
        }
        
        // Update averages
        let total_operations = stats.blocks_created + stats.blocks_validated + stats.blocks_propagated;
        if total_operations > 0 {
            stats.average_block_creation_time_ms = 
                (stats.average_block_creation_time_ms * (total_operations - 1) as f64 + operation_time_ms as f64) / total_operations as f64;
        }
        
        // Update success rates
        if stats.blocks_created > 0 {
            stats.block_creation_success_rate = stats.blocks_created as f64 / (stats.blocks_created + stats.blocks_validated) as f64;
        }
        
        if stats.blocks_validated > 0 {
            stats.block_validation_success_rate = stats.blocks_validated as f64 / (stats.blocks_created + stats.blocks_validated) as f64;
        }
        
        // Update uptime
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get mining statistics
    pub async fn get_stats(&self) -> Result<MiningStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Get block creator statistics
    pub async fn get_block_creator_stats(&self) -> Result<super::BlockCreationStats> {
        self.block_creator.get_stats().await
    }
    
    /// Get block validator statistics
    pub async fn get_block_validator_stats(&self) -> Result<super::BlockValidationStats> {
        self.block_validator.get_stats().await
    }
    
    /// Get transaction selector statistics
    pub async fn get_transaction_selector_stats(&self) -> Result<super::TransactionSelectionStats> {
        self.transaction_selector.get_stats().await
    }
    
    /// Get block propagator statistics
    pub async fn get_block_propagator_stats(&self) -> Result<super::BlockPropagationStats> {
        self.block_propagator.get_stats().await
    }
    
    /// Clear all caches
    pub async fn clear_caches(&self) -> Result<()> {
        self.block_validator.clear_cache().await?;
        self.block_propagator.clear_queue().await?;
        info!("All mining caches cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mining_manager_config() {
        let config = MiningManagerConfig::default();
        assert!(config.enable_mining);
        assert_eq!(config.mining_interval_seconds, 10);
        assert!(config.enable_automatic_block_creation);
        assert!(config.enable_block_validation);
        assert!(config.enable_transaction_selection);
        assert!(config.enable_block_propagation);
    }
    
    #[tokio::test]
    async fn test_mining_operation_result() {
        let result = MiningOperationResult {
            success: true,
            created_block: None,
            validation_result: None,
            selection_result: None,
            propagation_result: None,
            total_operation_time_ms: 100,
            error_message: None,
            mining_stats: MiningStats {
                blocks_created: 1,
                blocks_validated: 1,
                blocks_propagated: 1,
                average_block_creation_time_ms: 50.0,
                average_block_validation_time_ms: 25.0,
                average_transactions_per_block: 10.0,
                average_block_size_bytes: 1024.0,
                block_creation_success_rate: 1.0,
                block_validation_success_rate: 1.0,
                uptime_seconds: 3600,
                last_block_creation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
                last_block_validation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            },
        };
        
        assert!(result.success);
        assert_eq!(result.total_operation_time_ms, 100);
        assert!(result.error_message.is_none());
        assert_eq!(result.mining_stats.blocks_created, 1);
    }
    
    #[tokio::test]
    async fn test_mining_stats() {
        let stats = MiningStats {
            blocks_created: 100,
            blocks_validated: 95,
            blocks_propagated: 90,
            average_block_creation_time_ms: 50.0,
            average_block_validation_time_ms: 25.0,
            average_transactions_per_block: 15.5,
            average_block_size_bytes: 2048.0,
            block_creation_success_rate: 0.95,
            block_validation_success_rate: 0.90,
            uptime_seconds: 7200,
            last_block_creation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            last_block_validation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.blocks_created, 100);
        assert_eq!(stats.blocks_validated, 95);
        assert_eq!(stats.blocks_propagated, 90);
        assert_eq!(stats.block_creation_success_rate, 0.95);
        assert_eq!(stats.block_validation_success_rate, 0.90);
    }
}
