//! Round manager for zk-STARK integration
//!
//! Aggregates blocks per round, sorts via HashTimer, and builds Merkle trees of transactions.

use crate::{
    consensus::{blockdag::Block, hashtimer::HashTimer},
    Result,
};
use super::{RoundHeader, MerkleTree, RoundAggregation, RoundStats};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Round manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundManagerConfig {
    /// Target round duration in milliseconds (100-250ms)
    pub round_duration_ms: u64,
    /// Maximum number of blocks per round
    pub max_blocks_per_round: usize,
    /// Maximum number of transactions per round
    pub max_transactions_per_round: usize,
    /// Minimum number of blocks required to start aggregation
    pub min_blocks_for_aggregation: usize,
    /// Enable HashTimer-based sorting
    pub enable_hashtimer_sorting: bool,
}

impl Default for RoundManagerConfig {
    fn default() -> Self {
        Self {
            round_duration_ms: 200, // 200ms rounds for sub-second finality
            max_blocks_per_round: 1000,
            max_transactions_per_round: 100_000,
            min_blocks_for_aggregation: 10,
            enable_hashtimer_sorting: true,
        }
    }
}

/// Block aggregation for a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundBlock {
    /// Block hash
    pub block_hash: [u8; 32],
    /// Block data
    pub block: Block,
    /// HashTimer timestamp for sorting
    pub hashtimer_timestamp: u64,
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Block index in the round
    pub index: usize,
}

/// Round aggregation state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoundState {
    /// Collecting blocks for the round
    Collecting,
    /// Aggregating blocks and generating zk-STARK proof
    Aggregating,
    /// Round completed with proof
    Completed,
    /// Round failed
    Failed,
}

/// Round manager for zk-STARK integration
#[derive(Debug)]
pub struct ZkRoundManager {
    /// Configuration
    config: RoundManagerConfig,
    /// Current round number
    current_round: u64,
    /// Current round state
    round_state: RoundState,
    /// Blocks collected for current round
    round_blocks: Arc<RwLock<Vec<RoundBlock>>>,
    /// Round start timestamp
    round_start_timestamp: u64,
    /// Round statistics
    round_stats: Arc<RwLock<HashMap<u64, RoundStats>>>,
    /// Validators for current round
    validators: Vec<[u8; 32]>,
}

impl ZkRoundManager {
    /// Create a new zk-STARK round manager
    pub fn new(config: RoundManagerConfig) -> Self {
        Self {
            config,
            current_round: 0,
            round_state: RoundState::Collecting,
            round_blocks: Arc::new(RwLock::new(Vec::new())),
            round_start_timestamp: 0,
            round_stats: Arc::new(RwLock::new(HashMap::new())),
            validators: Vec::new(),
        }
    }

    /// Start a new round
    pub async fn start_round(&mut self, round_number: u64, validators: Vec<[u8; 32]>) -> Result<()> {
        if round_number <= self.current_round {
            return Err(crate::IppanError::Validation(
                format!("Invalid round number: {}", round_number)
            ));
        }

        info!("Starting zk-STARK round {}", round_number);
        
        self.current_round = round_number;
        self.round_state = RoundState::Collecting;
        self.validators = validators;
        self.round_start_timestamp = Self::get_current_timestamp();
        
        // Clear previous round blocks
        {
            let mut blocks = self.round_blocks.write().await;
            blocks.clear();
        }
        
        debug!("Round {} started with {} validators", round_number, self.validators.len());
        Ok(())
    }

    /// Add a block to the current round
    pub async fn add_block(&self, block: Block) -> Result<bool> {
        if self.round_state != RoundState::Collecting {
            return Err(crate::IppanError::Validation(
                "Cannot add block: round not in collecting state".to_string()
            ));
        }

        // Check if block belongs to current round
        if block.header.round != self.current_round {
            return Ok(false);
        }

        // Check if validator is authorized
        if !self.validators.contains(&block.header.validator_id) {
            warn!("Block from unauthorized validator: {:?}", block.header.validator_id);
            return Ok(false);
        }

        // Check round limits
        {
            let blocks = self.round_blocks.read().await;
            if blocks.len() >= self.config.max_blocks_per_round {
                warn!("Round {}: maximum blocks reached", self.current_round);
                return Ok(false);
            }
        }

        // Create round block
        let round_block = RoundBlock {
            block_hash: block.header.hash,
            block: block.clone(),
            hashtimer_timestamp: block.header.hashtimer.timestamp_ns,
            validator_id: block.header.validator_id,
            index: 0, // Will be set during aggregation
        };

        // Add to round blocks
        {
            let mut blocks = self.round_blocks.write().await;
            blocks.push(round_block);
        }

        debug!(
            "Added block {} to round {} (total: {})",
            hex::encode(&block.header.hash),
            self.current_round,
            self.round_blocks.read().await.len()
        );

        Ok(true)
    }

    /// Check if round should be aggregated
    pub async fn should_aggregate_round(&self) -> bool {
        let current_time = Self::get_current_timestamp();
        let round_duration = current_time - self.round_start_timestamp;
        
        // Check time-based aggregation
        if round_duration >= self.config.round_duration_ms * 1_000_000 {
            return true;
        }
        
        // Check block count-based aggregation
        let block_count = self.round_blocks.read().await.len();
        if block_count >= self.config.min_blocks_for_aggregation {
            return true;
        }
        
        false
    }

    /// Aggregate the current round and generate zk-STARK proof
    pub async fn aggregate_round(&mut self) -> Result<RoundAggregation> {
        if self.round_state != RoundState::Collecting {
            return Err(crate::IppanError::Validation(
                "Cannot aggregate: round not in collecting state".to_string()
            ));
        }

        info!("Aggregating round {} with zk-STARK proof", self.current_round);
        self.round_state = RoundState::Aggregating;

        // Get and sort blocks by HashTimer
        let mut blocks = self.round_blocks.read().await.clone();
        if blocks.is_empty() {
            self.round_state = RoundState::Failed;
            return Err(crate::IppanError::Validation(
                "No blocks to aggregate".to_string()
            ));
        }

        // Sort blocks by HashTimer timestamp
        if self.config.enable_hashtimer_sorting {
            blocks.sort_by_key(|b| b.hashtimer_timestamp);
        }

        // Set block indices
        for (i, block) in blocks.iter_mut().enumerate() {
            block.index = i;
        }

        // Extract all transactions
        let mut all_transactions = Vec::new();
        let mut transaction_hashes = Vec::new();
        
        for block in &blocks {
            for tx in &block.block.transactions {
                all_transactions.push(tx.clone());
                transaction_hashes.push(tx.hash);
            }
        }

        // Build Merkle tree from transaction hashes
        let merkle_tree = MerkleTree::new(transaction_hashes.clone());

        // Calculate state root (simplified for now)
        let state_root = self.calculate_state_root(&all_transactions);

        // Create round header
        let header = RoundHeader::new(
            self.current_round,
            merkle_tree.root,
            state_root,
            blocks[0].hashtimer_timestamp, // Use first block's timestamp
            self.validators[0], // Use first validator for now
        );

        // Generate zk-STARK proof (placeholder for now)
        let zk_proof = self.generate_zk_proof(&header, &all_transactions).await?;

        // Create round aggregation
        let aggregation = RoundAggregation {
            header,
            zk_proof,
            transaction_hashes,
            merkle_tree,
        };

        // Update round state
        self.round_state = RoundState::Completed;

        // Record statistics
        self.record_round_stats(&aggregation).await;

        info!(
            "Round {} aggregated: {} blocks, {} transactions, proof size: {} bytes",
            self.current_round,
            blocks.len(),
            all_transactions.len(),
            aggregation.zk_proof.proof_size
        );

        Ok(aggregation)
    }

    /// Generate zk-STARK proof for the round
    async fn generate_zk_proof(
        &self,
        header: &RoundHeader,
        transactions: &[crate::consensus::blockdag::Transaction],
    ) -> Result<super::ZkStarkProof> {
        let start_time = std::time::Instant::now();
        
        // TODO: Integrate with actual zk-STARK prover (Winterfell or custom)
        // For now, create a placeholder proof
        
        let proof_data = self.create_placeholder_proof(header, transactions);
        let proving_time = start_time.elapsed().as_millis() as u64;
        let proof_size = proof_data.len();
        
        Ok(super::ZkStarkProof {
            proof_data,
            proof_size,
            proving_time_ms: proving_time,
            verification_time_ms: 10, // Placeholder
            round_number: header.round_number,
            transaction_count: transactions.len() as u32,
        })
    }

    /// Create placeholder zk-STARK proof
    fn create_placeholder_proof(
        &self,
        header: &RoundHeader,
        transactions: &[crate::consensus::blockdag::Transaction],
    ) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        
        // Create a simple hash-based "proof" for now
        let mut hasher = Sha256::new();
        hasher.update(header.round_number.to_le_bytes());
        hasher.update(header.merkle_root);
        hasher.update(header.state_root);
        hasher.update(header.hashtimer_timestamp.to_le_bytes());
        hasher.update(header.validator_id);
        
        // Include transaction count
        hasher.update((transactions.len() as u32).to_le_bytes());
        
        // Include first few transaction hashes
        for tx in transactions.iter().take(10) {
            hasher.update(tx.hash);
        }
        
        let result = hasher.finalize();
        result.to_vec()
    }

    /// Calculate state root from transactions
    fn calculate_state_root(&self, transactions: &[crate::consensus::blockdag::Transaction]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update("STATE_ROOT".as_bytes());
        
        for tx in transactions {
            hasher.update(tx.hash);
            match &tx.tx_type {
                crate::consensus::blockdag::TransactionType::Payment(payment) => {
                    hasher.update(payment.from);
                    hasher.update(payment.to);
                    hasher.update(payment.amount.to_le_bytes());
                }
                crate::consensus::blockdag::TransactionType::Anchor(anchor) => {
                    hasher.update(anchor.external_chain_id.as_bytes());
                    hasher.update(anchor.external_state_root.as_bytes());
                }
                crate::consensus::blockdag::TransactionType::Staking(staking) => {
                    hasher.update(staking.staker);
                    hasher.update(staking.validator);
                    hasher.update(staking.amount.to_le_bytes());
                }
                crate::consensus::blockdag::TransactionType::Storage(storage) => {
                    hasher.update(storage.provider);
                    hasher.update(&storage.file_hash);
                    hasher.update(storage.data_size.to_le_bytes());
                }
            }
        }
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Record round statistics
    async fn record_round_stats(&self, aggregation: &RoundAggregation) {
        let stats = RoundStats {
            round_number: self.current_round,
            block_count: self.round_blocks.read().await.len() as u32,
            transaction_count: aggregation.transaction_hashes.len() as u32,
            proof_size: aggregation.zk_proof.proof_size,
            proving_time_ms: aggregation.zk_proof.proving_time_ms,
            verification_time_ms: aggregation.zk_proof.verification_time_ms,
            propagation_latency_ms: 180, // Placeholder for intercontinental latency
        };

        let mut round_stats = self.round_stats.write().await;
        round_stats.insert(self.current_round, stats);
    }

    /// Get current round number
    pub fn current_round(&self) -> u64 {
        self.current_round
    }

    /// Get current round state
    pub fn round_state(&self) -> &RoundState {
        &self.round_state
    }

    /// Get round statistics
    pub async fn get_round_stats(&self, round_number: u64) -> Option<RoundStats> {
        let stats = self.round_stats.read().await;
        stats.get(&round_number).cloned()
    }

    /// Get all round statistics
    pub async fn get_all_round_stats(&self) -> Vec<RoundStats> {
        let stats = self.round_stats.read().await;
        stats.values().cloned().collect()
    }

    /// Get current timestamp in nanoseconds
    fn get_current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::{blockdag::Transaction, hashtimer::HashTimer};

    #[tokio::test]
    async fn test_round_manager_creation() {
        let config = RoundManagerConfig::default();
        let manager = ZkRoundManager::new(config);
        
        assert_eq!(manager.current_round(), 0);
        assert!(matches!(manager.round_state(), RoundState::Collecting));
    }

    #[tokio::test]
    async fn test_round_start() {
        let config = RoundManagerConfig::default();
        let mut manager = ZkRoundManager::new(config);
        
        let validators = vec![[1u8; 32], [2u8; 32]];
        manager.start_round(1, validators).await.unwrap();
        
        assert_eq!(manager.current_round(), 1);
        assert!(matches!(manager.round_state(), RoundState::Collecting));
    }

    #[tokio::test]
    async fn test_block_addition() {
        let config = RoundManagerConfig::default();
        let mut manager = ZkRoundManager::new(config);
        
        let validators = vec![[1u8; 32]];
        manager.start_round(1, validators).await.unwrap();
        
        // Create a test block
        let hashtimer = HashTimer::new([0u8; 32], [1u8; 32]);
        let transaction = Transaction::new(
            [1u8; 32],
            [2u8; 32],
            100,
            1,
            1,
            hashtimer.clone(),
        );
        
        let block = Block::new(1, vec![transaction], [1u8; 32], hashtimer);
        
        let added = manager.add_block(block).await.unwrap();
        assert!(added);
    }
} 