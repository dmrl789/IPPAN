//! Consensus module for IPPAN
//! 
//! Handles block creation, validation, and consensus mechanisms

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod blockdag;
pub mod hashtimer;
pub mod ippan_time;
pub mod randomness;
pub mod round;

use blockdag::{Block, BlockDAG};
use hashtimer::HashTimer;
use ippan_time::IppanTimeManager;
use randomness::RandomnessManager;
use round::RoundManager;

/// Custom serialization for byte arrays
mod byte_array_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Option<[u8; 64]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(b) => b.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u8; 64]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Option<Vec<u8>> = Option::deserialize(deserializer)?;
        match bytes {
            Some(b) => {
                if b.len() != 64 {
                    return Err(serde::de::Error::custom("Invalid signature length"));
                }
                let mut signature = [0u8; 64];
                signature.copy_from_slice(&b);
                Ok(Some(signature))
            }
            None => Ok(None),
        }
    }
}

/// Consensus engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Maximum number of validators per round
    pub max_validators: usize,
    /// Minimum stake required for validators
    pub min_stake: u64,
    /// Block time in seconds
    pub block_time: u64,
    /// Maximum time drift allowed in seconds
    pub max_time_drift: u64,
    /// Minimum nodes required for IPPAN Time
    pub min_nodes_for_time: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            max_validators: 21,
            min_stake: 10,
            block_time: 10,
            max_time_drift: 30,
            min_nodes_for_time: 3,
        }
    }
}

/// Consensus engine for IPPAN blockchain
#[derive(Debug)]
pub struct ConsensusEngine {
    /// BlockDAG for managing the blockchain
    blockdag: BlockDAG,
    /// Round manager for consensus rounds
    round_manager: RoundManager,
    /// Randomness manager for validator selection
    randomness_manager: RandomnessManager,
    /// IPPAN Time manager for median time calculation
    time_manager: IppanTimeManager,
    /// Configuration
    config: ConsensusConfig,
    /// Current validators and their stakes
    validators: HashMap<[u8; 32], u64>,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        let time_manager = IppanTimeManager::new(
            config.min_nodes_for_time,
            config.max_time_drift,
        );
        
        Self {
            blockdag: BlockDAG::new(),
            round_manager: RoundManager::new(config.max_validators),
            randomness_manager: RandomnessManager::new(),
            time_manager,
            config,
            validators: HashMap::new(),
        }
    }

    /// Add a validator with stake
    pub fn add_validator(&mut self, node_id: [u8; 32], stake: u64) -> Result<()> {
        if stake >= self.config.min_stake {
            self.validators.insert(node_id, stake);
            self.round_manager.add_validator(node_id, stake);
        }
        Ok(())
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, node_id: &[u8; 32]) -> Result<()> {
        self.validators.remove(node_id);
        self.round_manager.remove_validator(node_id);
        Ok(())
    }

    /// Add a time sample from a node
    pub fn add_node_time(&mut self, node_id: [u8; 32], time_ns: u64) {
        self.time_manager.add_node_time(node_id, time_ns);
    }

    /// Create a new block
    pub fn create_block(
        &mut self,
        transactions: Vec<Transaction>,
        validator_id: [u8; 32],
    ) -> Result<Block> {
        let round = self.round_manager.current_round();
        let ippan_time = self.time_manager.median_time_ns();
        
        // Create HashTimer for the block
        let block_hash = self.calculate_block_hash(&transactions, round, validator_id);
        let hashtimer = HashTimer::with_ippan_time(
            block_hash,
            validator_id,
            ippan_time,
        );
        
        let block = Block::new(
            round,
            transactions,
            validator_id,
            hashtimer,
        );
        
        Ok(block)
    }

    /// Validate a block
    pub fn validate_block(&self, block: &Block) -> Result<bool> {
        // Check if validator is authorized for this round
        if !self.round_manager.is_validator_authorized(&block.validator_id, block.round) {
            return Ok(false);
        }

        // Validate HashTimer
        if !self.validate_block_hashtimer(block)? {
            return Ok(false);
        }

        // Validate transactions
        for tx in &block.transactions {
            if !self.validate_transaction(tx)? {
                return Ok(false);
            }
        }

        // Validate block hash
        let expected_hash = self.calculate_block_hash(&block.transactions, block.round, block.validator_id);
        if block.hash != expected_hash {
            return Ok(false);
        }

        Ok(true)
    }

    /// Validate a transaction
    pub fn validate_transaction(&self, tx: &Transaction) -> Result<bool> {
        // Validate HashTimer
        if !self.validate_transaction_hashtimer(tx)? {
            return Ok(false);
        }

        // Basic transaction validation
        if tx.amount == 0 {
            return Ok(false);
        }

        // TODO: Add signature validation
        // TODO: Add balance checks
        // TODO: Add nonce validation

        Ok(true)
    }

    /// Validate block HashTimer
    fn validate_block_hashtimer(&self, block: &Block) -> Result<bool> {
        // Check if HashTimer is within acceptable time bounds
        if !block.hashtimer.is_valid(self.config.max_time_drift) {
            return Ok(false);
        }

        // Check if IPPAN Time is valid
        if !block.hashtimer.is_ippan_time_valid(self.config.max_time_drift) {
            return Ok(false);
        }

        // Check if we have sufficient time samples
        if !self.time_manager.has_sufficient_samples() {
            // Allow blocks if we don't have enough time samples yet
            return Ok(true);
        }

        // Check if the block's IPPAN Time is close to our median
        let drift_ns = self.time_manager.get_time_drift_ns(block.hashtimer.ippan_time_ns);
        let max_drift_ns = self.config.max_time_drift * 1_000_000_000;
        
        Ok(drift_ns.abs() <= max_drift_ns as i64)
    }

    /// Validate transaction HashTimer
    fn validate_transaction_hashtimer(&self, tx: &Transaction) -> Result<bool> {
        // Check if HashTimer is within acceptable time bounds
        if !tx.hashtimer.is_valid(self.config.max_time_drift) {
            return Ok(false);
        }

        // Check if IPPAN Time is valid
        if !tx.hashtimer.is_ippan_time_valid(self.config.max_time_drift) {
            return Ok(false);
        }

        // Check if we have sufficient time samples
        if !self.time_manager.has_sufficient_samples() {
            // Allow transactions if we don't have enough time samples yet
            return Ok(true);
        }

        // Check if the transaction's IPPAN Time is close to our median
        let drift_ns = self.time_manager.get_time_drift_ns(tx.hashtimer.ippan_time_ns);
        let max_drift_ns = self.config.max_time_drift * 1_000_000_000;
        
        Ok(drift_ns.abs() <= max_drift_ns as i64)
    }

    /// Add a block to the consensus engine
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // Validate the block
        if !self.validate_block(&block)? {
            return Err(crate::Error::InvalidBlock);
        }

        // Add to BlockDAG
        self.blockdag.add_block(block)?;

        // Update round if needed
        self.round_manager.update_round(block.round);

        Ok(())
    }

    /// Get current round
    pub fn current_round(&self) -> u64 {
        self.round_manager.current_round()
    }

    /// Get current validators
    pub fn get_validators(&self) -> &HashMap<[u8; 32], u64> {
        &self.validators
    }

    /// Get current IPPAN Time
    pub fn get_ippan_time(&self) -> u64 {
        self.time_manager.median_time_ns()
    }

    /// Get time statistics
    pub fn get_time_stats(&self) -> ippan_time::TimeStats {
        self.time_manager.get_stats()
    }

    /// Calculate block hash
    fn calculate_block_hash(
        &self,
        transactions: &[Transaction],
        round: u64,
        validator_id: [u8; 32],
    ) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(&round.to_be_bytes());
        hasher.update(&validator_id);
        
        for tx in transactions {
            hasher.update(&tx.hash);
        }
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get the BlockDAG
    pub fn blockdag(&self) -> &BlockDAG {
        &self.blockdag
    }

    /// Get the round manager
    pub fn round_manager(&self) -> &RoundManager {
        &self.round_manager
    }
}

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction hash
    pub hash: [u8; 32],
    /// Sender address
    pub from: [u8; 32],
    /// Recipient address
    pub to: [u8; 32],
    /// Amount in smallest units
    pub amount: u64,
    /// Transaction fee
    pub fee: u64,
    /// Nonce to prevent replay attacks
    pub nonce: u64,
    /// HashTimer for precise timing
    pub hashtimer: HashTimer,
    /// Transaction signature
    #[serde(with = "byte_array_serde")]
    pub signature: Option<[u8; 64]>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        from: [u8; 32],
        to: [u8; 32],
        amount: u64,
        fee: u64,
        nonce: u64,
        node_id: [u8; 32],
    ) -> Self {
        let mut tx = Self {
            hash: [0u8; 32],
            from,
            to,
            amount,
            fee,
            nonce,
            hashtimer: HashTimer::new([0u8; 32], node_id),
            signature: None,
        };
        
        tx.hash = tx.calculate_hash();
        tx.hashtimer = HashTimer::new(tx.hash, node_id);
        
        tx
    }

    /// Calculate transaction hash
    fn calculate_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(&self.from);
        hasher.update(&self.to);
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.fee.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get transaction hash
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }
}
