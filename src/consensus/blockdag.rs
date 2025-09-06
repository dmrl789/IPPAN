//! BlockDAG implementation for IPPAN consensus
//!
//! Only blocks are part of the DAG. Rounds are a logical/consensus concept and are not DAG nodes.
//!
//! Provides Directed Acyclic Graph structure for blocks with deterministic ordering

use crate::{
    consensus::{hashtimer::{HashTimer, IppanTimeManager}, limits::MAX_BLOCK_SIZE_BYTES},
    error::IppanError,
    NodeId, BlockHash, TransactionHash,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
// TODO: Fix logging imports
// use crate::utils::logging::{log_block, log_transaction};

/// Block-related errors
#[derive(thiserror::Error, Debug)]
pub enum BlockError {
    #[error("block too large: {size} bytes (max {max})")]
    TooLarge { size: usize, max: usize },
    #[error("invalid block structure: {reason}")]
    InvalidStructure { reason: String },
}

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

/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block hash
    pub hash: BlockHash,
    /// Round number
    pub round: u64,
    /// Block height (number of blocks from genesis)
    pub height: u64,
    /// Validator ID that created this block
    pub validator_id: NodeId,
    /// HashTimer for precise timing
    pub hashtimer: HashTimer,
    /// Parent block hashes (for DAG structure)
    pub parent_hashes: Vec<BlockHash>,
    /// Parent round numbers (corresponding to parent_hashes)
    pub parent_rounds: Vec<u64>,
    /// Block creation timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Block size in bytes (populated by Block::new)
    pub block_size_bytes: u32,
    /// Transaction count
    pub tx_count: u32,
    /// Merkle root of transactions
    pub merkle_root: [u8; 32],
}

impl BlockHeader {
    /// Estimate serialized size of the header
    pub fn estimate_size_bytes(&self) -> usize {
        // Sum of fields: 32+8+8+32+32+8+4+4+32 = 160 bytes
        // Plus variable size for parent_hashes and parent_rounds
        let parent_size = self.parent_hashes.len() * 32 + self.parent_rounds.len() * 8;
        160 + parent_size
    }
}

/// Block containing transaction hashes and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Transaction hashes in this block (references only, no inlined payload)
    pub tx_hashes: Vec<TransactionHash>,
    /// Block signature (optional, for future use)
    #[serde(with = "byte_array_serde")]
    pub signature: Option<[u8; 64]>,
}

impl Block {
    /// Estimate serialized size: header + tx refs (no payloads inlined)
    pub fn estimate_size_bytes(&self) -> usize {
        let header_bytes = self.header.estimate_size_bytes();
        // 32 bytes per tx hash; add small vec overhead if needed
        header_bytes + (self.tx_hashes.len() * 32)
    }

    /// Calculate merkle root from transaction hashes
    fn calculate_merkle_root(tx_hashes: &[TransactionHash]) -> [u8; 32] {
        if tx_hashes.is_empty() {
            // Empty merkle root
            [0u8; 32]
        } else if tx_hashes.len() == 1 {
            // Single transaction, hash it
            let mut hasher = Sha256::new();
            hasher.update(&tx_hashes[0]);
            let result = hasher.finalize();
            let mut root = [0u8; 32];
            root.copy_from_slice(&result);
            root
        } else {
            // Multiple transactions, build merkle tree
            let mut current_level = tx_hashes.to_vec();
            while current_level.len() > 1 {
                let mut next_level = Vec::new();
                for chunk in current_level.chunks(2) {
                    let mut hasher = Sha256::new();
                    hasher.update(&chunk[0]);
                    if chunk.len() == 2 {
                        hasher.update(&chunk[1]);
                    } else {
                        // Odd number, duplicate the last element
                        hasher.update(&chunk[0]);
                    }
                    let result = hasher.finalize();
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&result);
                    next_level.push(hash);
                }
                current_level = next_level;
            }
            current_level[0]
        }
    }
}

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Payment transaction
    Payment(PaymentData),
    /// Anchor transaction from external chains
    Anchor(AnchorData),
    /// Staking transaction
    Staking(StakingData),
    /// Storage transaction
    Storage(StorageData),
    /// DNS zone update transaction
    DnsZoneUpdate(DnsZoneUpdateData),
    /// Program call transaction (smart contracts)
    #[cfg(feature = "contracts")]
    ProgramCall(ProgramCallData),
    /// L2 commit transaction
    L2Commit(L2CommitData),
    /// L2 exit transaction
    L2Exit(L2ExitData),
}

/// Payment transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentData {
    /// Sender address
    pub from: NodeId,
    /// Recipient address
    pub to: NodeId,
    /// Amount in smallest units
    pub amount: u64,
}

/// Anchor transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorData {
    /// External chain identifier
    pub external_chain_id: String,
    /// External state root
    pub external_state_root: String,
    /// Proof type
    #[cfg(feature = "crosschain")]
    pub proof_type: Option<crate::crosschain::types::ProofType>,
    #[cfg(not(feature = "crosschain"))]
    pub proof_type: Option<String>,
    /// Proof data
    pub proof_data: Vec<u8>,
}

/// Staking transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingData {
    /// Staker address
    pub staker: NodeId,
    /// Validator address
    pub validator: NodeId,
    /// Stake amount
    pub amount: u64,
    /// Staking action
    pub action: StakingAction,
}

/// Staking actions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StakingAction {
    /// Stake tokens
    Stake,
    /// Unstake tokens
    Unstake,
    /// Claim rewards
    ClaimRewards,
}

/// Storage transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageData {
    /// Storage provider
    pub provider: NodeId,
    /// File hash
    pub file_hash: [u8; 32],
    /// Storage action
    pub action: StorageAction,
    /// Data size
    pub data_size: u64,
}

/// Storage actions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StorageAction {
    /// Store data
    Store,
    /// Retrieve data
    Retrieve,
    /// Delete data
    Delete,
}

/// DNS zone update transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneUpdateData {
    /// Domain name
    pub domain: String,
    /// Zone update operations
    pub ops: Vec<crate::dns::apply::ZoneOp>,
    /// Update timestamp in microseconds
    pub updated_at_us: u64,
}

/// Program call transaction data
#[cfg(feature = "contracts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramCallData {
    /// Program ID
    pub program_id: [u8; 32],
    /// Entry point name
    pub entrypoint: String,
    /// Call data (opaque, bounded)
    pub calldata: Vec<u8>,
    /// Capability references granted by the signer
    pub caps: Vec<crate::blockchain::smart_contract_system::CapabilityRef>,
    /// Gas or syscall budget
    pub budget: u64,
}

/// L2 commit transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CommitData {
    /// L2 identifier
    pub l2_id: String,
    /// L2 epoch/batch number
    pub epoch: u64,
    /// State root after applying batch
    pub state_root: [u8; 32],
    /// Data availability hash
    pub da_hash: [u8; 32],
    /// Proof type
    pub proof_type: String,
    /// Proof bytes
    pub proof: Vec<u8>,
    /// Optional inline data
    pub inline_data: Option<Vec<u8>>,
}

/// L2 exit transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2ExitData {
    /// L2 identifier
    pub l2_id: String,
    /// L2 epoch this exit is based on
    pub epoch: u64,
    /// Proof of inclusion
    pub proof_of_inclusion: Vec<u8>,
    /// Recipient account on L1
    pub account: [u8; 32],
    /// Amount or asset payload
    pub amount: u128,
    /// Nonce to prevent replay attacks
    pub nonce: u64,
}

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction hash
    pub hash: TransactionHash,
    /// Transaction type
    pub tx_type: TransactionType,
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

/// BlockDAG node representing a block in the DAG
#[derive(Debug, Clone)]
pub struct BlockDAGNode {
    /// The block
    pub block: Block,
    /// Children blocks (blocks that have this block as parent)
    pub children: HashSet<BlockHash>,
    /// Whether this block is finalized
    pub finalized: bool,
    /// Block score for ordering (based on HashTimer)
    pub score: u64,
}

/// BlockDAG for managing the blockchain as a Directed Acyclic Graph
#[derive(Debug)]
pub struct BlockDAG {
    /// All blocks in the DAG
    blocks: Arc<RwLock<HashMap<BlockHash, BlockDAGNode>>>,
    /// Genesis block hash
    genesis_hash: BlockHash,
    /// Current tips (blocks with no children)
    tips: Arc<RwLock<HashSet<BlockHash>>>,
    /// Finalized blocks
    finalized_blocks: Arc<RwLock<HashSet<BlockHash>>>,
    /// IPPAN Time manager for timing validation
    // time_manager: Arc<IppanTimeManager>, // TODO: Use when implementing time validation
    /// Maximum number of tips to maintain
    max_tips: usize,
}

impl BlockDAG {
    /// Create a new BlockDAG
    pub fn new(_time_manager: Arc<IppanTimeManager>) -> Self {
        let genesis_hash = Self::calculate_genesis_hash();
        
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
            genesis_hash,
            tips: Arc::new(RwLock::new(HashSet::new())),
            finalized_blocks: Arc::new(RwLock::new(HashSet::new())),
            // time_manager, // TODO: Use when implementing time validation
            max_tips: 100,
        }
    }

    /// Add a block to the DAG
    pub async fn add_block(&self, block: Block) -> Result<()> {
        let block_hash = block.header.hash;
        
        // Validate the block
        if !self.validate_block(&block).await? {
            return Err(IppanError::Validation("Block validation failed".to_string()));
        }

        // Create DAG node
        let node = BlockDAGNode {
            block: block.clone(),
            children: HashSet::new(),
            finalized: false,
            score: self.calculate_block_score(&block),
        };

        // Add to blocks map
        {
            let mut blocks = self.blocks.write().await;
            
            // Check if block already exists
            if blocks.contains_key(&block_hash) {
                return Err(IppanError::Validation("Block already exists".to_string()));
            }

            // Add the block
            blocks.insert(block_hash, node);
        }

        // Update parent-child relationships
        self.update_parent_child_relationships(&block).await?;

        // Update tips
        self.update_tips(&block_hash).await?;

        // Try to finalize blocks
        self.try_finalize_blocks().await?;

        info!("Added block {} to DAG", hex::encode(&block_hash));
        Ok(())
    }

    /// Validate a block
    pub async fn validate_block(&self, block: &Block) -> Result<bool> {
        // Check basic structure - blocks can be empty (genesis)
        // if block.tx_hashes.is_empty() {
        //     return Ok(false);
        // }

        // Validate block size
        if block.header.block_size_bytes as usize > MAX_BLOCK_SIZE_BYTES {
            tracing::warn!(
                "Block validation failed: size {} bytes exceeds maximum {} bytes",
                block.header.block_size_bytes,
                MAX_BLOCK_SIZE_BYTES
            );
            return Ok(false);
        }

        // Validate HashTimer
        if !self.validate_block_hashtimer(block)? {
            return Ok(false);
        }

        // Validate merkle root
        let expected_merkle_root = Block::calculate_merkle_root(&block.tx_hashes);
        if block.header.merkle_root != expected_merkle_root {
            return Ok(false);
        }

        // Validate parent hashes exist (except for genesis)
        if block.header.hash != self.genesis_hash {
            for parent_hash in &block.header.parent_hashes {
                let blocks = self.blocks.read().await;
                if !blocks.contains_key(parent_hash) {
                    return Ok(false);
                }
            }
        }

        // Validate block hash
        let expected_hash = self.calculate_block_hash(block);
        if block.header.hash != expected_hash {
            return Ok(false);
        }

        Ok(true)
    }

    /// Validate block HashTimer
    fn validate_block_hashtimer(&self, block: &Block) -> Result<bool> {
        // Check if HashTimer matches block content
        if !block.header.hashtimer.matches_content(&block.header.hash) {
            return Ok(false);
        }

        // Validate timing (within acceptable bounds)
        if !block.header.hashtimer.is_valid(30) { // 30 second drift
            return Ok(false);
        }

        Ok(true)
    }

    /// Validate transaction
    fn validate_transaction(&self, tx: &Transaction) -> Result<bool> {
        // Validate HashTimer
        if !tx.hashtimer.matches_content(&tx.hash) {
            return Ok(false);
        }

        // Validate timing
        if !tx.hashtimer.is_valid(30) {
            return Ok(false);
        }

        // Validate transaction type specific data
        match &tx.tx_type {
            TransactionType::Payment(data) => {
                if data.amount == 0 {
                    return Ok(false);
                }
            }
            TransactionType::Anchor(data) => {
                if data.external_chain_id.is_empty() || data.external_state_root.is_empty() {
                    return Ok(false);
                }
            }
            TransactionType::Staking(data) => {
                if data.amount == 0 {
                    return Ok(false);
                }
            }
            TransactionType::Storage(data) => {
                if data.data_size == 0 {
                    return Ok(false);
                }
            }
            TransactionType::DnsZoneUpdate(data) => {
                if data.domain.is_empty() || data.ops.is_empty() {
                    return Ok(false);
                }
            }
            #[cfg(feature = "contracts")]
            TransactionType::ProgramCall(data) => {
                if data.program_id == [0u8; 32] || data.entrypoint.is_empty() || data.calldata.is_empty() {
                    return Ok(false);
                }
                // Additional validation: check budget limits
                if data.budget > 1_000_000 { // 1M gas limit
                    return Ok(false);
                }
                // Check calldata size limit
                if data.calldata.len() > 1024 * 1024 { // 1MB limit
                    return Ok(false);
                }
            }
            TransactionType::L2Commit(data) => {
                if data.l2_id.is_empty() || data.epoch == 0 || data.proof.is_empty() {
                    return Ok(false);
                }
                // Check proof size limit
                if data.proof.len() > 16384 { // 16KB limit
                    return Ok(false);
                }
                // Check inline data size if present
                if let Some(ref inline_data) = data.inline_data {
                    if inline_data.len() > 16384 {
                        return Ok(false);
                    }
                }
            }
            TransactionType::L2Exit(data) => {
                if data.l2_id.is_empty() || data.epoch == 0 || data.proof_of_inclusion.is_empty() || data.amount == 0 {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Calculate block score for ordering
    fn calculate_block_score(&self, block: &Block) -> u64 {
        // Score based on HashTimer precision and IPPAN Time
        let hashtimer_score = block.header.hashtimer.ippan_time_micros();
        let transaction_count = block.tx_hashes.len() as u64;
        
        // Higher score for more precise timing and more transactions
        hashtimer_score + (transaction_count * 1000)
    }

    /// Update parent-child relationships
    async fn update_parent_child_relationships(&self, block: &Block) -> Result<()> {
        let block_hash = block.header.hash;
        
        {
            let mut blocks = self.blocks.write().await;
            
            // Add this block as child to all its parents
            for parent_hash in &block.header.parent_hashes {
                if let Some(parent_node) = blocks.get_mut(parent_hash) {
                    parent_node.children.insert(block_hash);
                }
            }
        }

        Ok(())
    }

    /// Update tips (blocks with no children)
    async fn update_tips(&self, new_block_hash: &BlockHash) -> Result<()> {
        let mut tips = self.tips.write().await;
        let blocks = self.blocks.read().await;

        // Remove parent hashes from tips (they now have children)
        if let Some(block) = blocks.get(new_block_hash) {
            for parent_hash in &block.block.header.parent_hashes {
                tips.remove(parent_hash);
            }
        }

        // Add new block to tips if it has no children
        if let Some(block) = blocks.get(new_block_hash) {
            if block.children.is_empty() {
                tips.insert(*new_block_hash);
            }
        }

        // Limit number of tips
        if tips.len() > self.max_tips {
            // Remove oldest tips (simplified strategy)
            let tips_vec: Vec<BlockHash> = tips.iter().cloned().collect();
            tips.clear();
            for tip in tips_vec.iter().take(self.max_tips) {
                tips.insert(*tip);
            }
        }

        Ok(())
    }

    /// Try to finalize blocks
    async fn try_finalize_blocks(&self) -> Result<()> {
        // Simple finalization: blocks with sufficient depth are finalized
        let blocks = self.blocks.read().await;
        let mut finalized = self.finalized_blocks.write().await;
        
        for (hash, node) in blocks.iter() {
            if !node.finalized && self.is_block_finalizable(node).await {
                finalized.insert(*hash);
                debug!("Finalized block {}", hex::encode(hash));
            }
        }

        Ok(())
    }

    /// Check if a block is finalizable
    async fn is_block_finalizable(&self, node: &BlockDAGNode) -> bool {
        // Simple finalization rule: block is finalizable if it's older than 10 blocks
        // In a real implementation, this would be more sophisticated
        let current_height = self.get_max_height().await;
        node.block.header.height + 10 <= current_height
    }

    /// Get maximum block height
    async fn get_max_height(&self) -> u64 {
        let blocks = self.blocks.read().await;
        blocks.values()
            .map(|node| node.block.header.height)
            .max()
            .unwrap_or(0)
    }

    /// Calculate block hash
    fn calculate_block_hash(&self, block: &Block) -> BlockHash {
        let mut hasher = Sha256::new();
        
        // Include round, validator, and parent hashes
        hasher.update(&block.header.round.to_be_bytes());
        hasher.update(&block.header.validator_id);
        for parent_hash in &block.header.parent_hashes {
            hasher.update(parent_hash);
        }
        
        // Include transaction hashes
        for tx_hash in &block.tx_hashes {
            hasher.update(tx_hash);
        }
        
        // Include HashTimer
        hasher.update(&block.header.hashtimer.ippan_time_ns().to_be_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Calculate genesis block hash
    fn calculate_genesis_hash() -> BlockHash {
        let mut hasher = Sha256::new();
        hasher.update(b"IPPAN_GENESIS");
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get block by hash
    pub async fn get_block(&self, hash: &BlockHash) -> Option<Block> {
        let blocks = self.blocks.read().await;
        blocks.get(hash).map(|node| node.block.clone())
    }

    /// Get all blocks
    pub async fn get_all_blocks(&self) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        blocks.values().map(|node| node.block.clone()).collect()
    }

    /// Get tips (blocks with no children)
    pub async fn get_tips(&self) -> Vec<BlockHash> {
        let tips = self.tips.read().await;
        tips.iter().cloned().collect()
    }

    /// Get finalized blocks
    pub async fn get_finalized_blocks(&self) -> Vec<BlockHash> {
        let finalized = self.finalized_blocks.read().await;
        finalized.iter().cloned().collect()
    }

    /// Get block count
    pub async fn get_block_count(&self) -> usize {
        let blocks = self.blocks.read().await;
        blocks.len()
    }

    /// Get tip count
    pub async fn get_tip_count(&self) -> usize {
        let tips = self.tips.read().await;
        tips.len()
    }

    /// Get finalized block count
    pub async fn get_finalized_count(&self) -> usize {
        let finalized = self.finalized_blocks.read().await;
        finalized.len()
    }

    /// Get blocks at a specific height
    pub async fn get_blocks_at_height(&self, height: u64) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        blocks.values()
            .filter(|node| node.block.header.height == height)
            .map(|node| node.block.clone())
            .collect()
    }

    /// Get blocks by validator
    pub async fn get_blocks_by_validator(&self, validator_id: &NodeId) -> Vec<Block> {
        let blocks = self.blocks.read().await;
        blocks.values()
            .filter(|node| node.block.header.validator_id == *validator_id)
            .map(|node| node.block.clone())
            .collect()
    }

    /// Get block statistics
    pub async fn get_stats(&self) -> BlockDAGStats {
        let blocks = self.blocks.read().await;
        let tips = self.tips.read().await;
        let finalized = self.finalized_blocks.read().await;

        let total_blocks = blocks.len();
        let tip_count = tips.len();
        let finalized_count = finalized.len();
        
        let max_height = blocks.values()
            .map(|node| node.block.header.height)
            .max()
            .unwrap_or(0);

        let total_transactions = blocks.values()
            .map(|node| node.block.tx_hashes.len())
            .sum();

        BlockDAGStats {
            total_blocks,
            tip_count,
            finalized_count,
            max_height,
            total_transactions,
        }
    }
}

/// BlockDAG statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDAGStats {
    pub total_blocks: usize,
    pub tip_count: usize,
    pub finalized_count: usize,
    pub max_height: u64,
    pub total_transactions: usize,
}

impl Block {
    /// Create a new block with size enforcement
    pub fn new(
        round: u64,
        tx_hashes: Vec<TransactionHash>,
        validator_id: NodeId,
        hashtimer: HashTimer,
    ) -> Result<Self> {
        let parent_hashes = vec![]; // Will be set by caller
        let parent_rounds = vec![]; // Will be set by caller
        let height = 0; // Will be calculated by caller
        let timestamp_ns = hashtimer.timestamp_ns;
        
        // Calculate merkle root from transaction hashes
        let merkle_root = Self::calculate_merkle_root(&tx_hashes);
        
        let header = BlockHeader {
            hash: [0u8; 32], // Will be calculated
            round,
            height,
            validator_id,
            hashtimer,
            parent_hashes,
            parent_rounds,
            timestamp_ns,
            block_size_bytes: 0, // Will be calculated
            tx_count: tx_hashes.len() as u32,
            merkle_root,
        };

        let mut block = Self {
            header,
            tx_hashes,
            signature: None,
        };

        // Calculate and enforce size limit
        let estimated_size = block.estimate_size_bytes();
        if estimated_size > MAX_BLOCK_SIZE_BYTES {
            // Log block size violation
            tracing::warn!(
                "Block size violation: {} bytes exceeds maximum {} bytes",
                estimated_size,
                MAX_BLOCK_SIZE_BYTES
            );
            return Err(IppanError::Validation(format!("Block too large: {} bytes (max {})", estimated_size, MAX_BLOCK_SIZE_BYTES)));
        }

        // Set the actual size in the header
        block.header.block_size_bytes = estimated_size as u32;

        // Log block size metrics
        tracing::debug!(
            "Block created: {} bytes, {} transactions, round {}",
            estimated_size,
            block.header.tx_count,
            block.header.round
        );

        // Warn if block is close to size limit
        if estimated_size > (MAX_BLOCK_SIZE_BYTES * 3 / 4) {
            tracing::warn!(
                "Block size warning: {} bytes is within 25% of maximum {} bytes",
                estimated_size,
                MAX_BLOCK_SIZE_BYTES
            );
        }

        // Calculate hash
        block.header.hash = Self::calculate_block_hash(&block);
        
        Ok(block)
    }

    /// Calculate block hash
    fn calculate_block_hash(block: &Block) -> BlockHash {
        let mut hasher = Sha256::new();
        
        hasher.update(&block.header.round.to_be_bytes());
        hasher.update(&block.header.validator_id);
        for parent_hash in &block.header.parent_hashes {
            hasher.update(parent_hash);
        }
        for parent_round in &block.header.parent_rounds {
            hasher.update(&parent_round.to_be_bytes());
        }
        for tx_hash in &block.tx_hashes {
            hasher.update(tx_hash);
        }
        hasher.update(&block.header.hashtimer.ippan_time_ns().to_be_bytes());
        hasher.update(&block.header.merkle_root);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get block hash
    pub fn hash(&self) -> &BlockHash {
        &self.header.hash
    }

    /// Get round number
    pub fn round(&self) -> u64 {
        self.header.round
    }

    /// Get height
    pub fn height(&self) -> u64 {
        self.header.height
    }

    /// Get validator ID
    pub fn validator_id(&self) -> &NodeId {
        &self.header.validator_id
    }

    /// Get HashTimer
    pub fn hashtimer(&self) -> &HashTimer {
        &self.header.hashtimer
    }
}

impl Transaction {
    /// Create a new payment transaction
    pub fn new_payment(
        from: NodeId,
        to: NodeId,
        amount: u64,
        fee: u64,
        nonce: u64,
        hashtimer: HashTimer,
    ) -> Self {
        let payment_data = PaymentData {
            from,
            to,
            amount,
        };
        
        let tx = Self {
            hash: [0u8; 32], // Will be calculated
            tx_type: TransactionType::Payment(payment_data),
            fee,
            nonce,
            hashtimer,
            signature: None,
        };
        
        let hash = Self::calculate_transaction_hash(&tx);
        Self {
            hash,
            ..tx
        }
    }

    /// Create a new anchor transaction
    pub fn new_anchor(
        external_chain_id: String,
        external_state_root: String,
        proof_type: Option<crate::crosschain::types::ProofType>,
        proof_data: Vec<u8>,
        fee: u64,
        nonce: u64,
        hashtimer: HashTimer,
    ) -> Self {
        let anchor_data = AnchorData {
            external_chain_id,
            external_state_root,
            proof_type,
            proof_data,
        };
        
        let tx = Self {
            hash: [0u8; 32], // Will be calculated
            tx_type: TransactionType::Anchor(anchor_data),
            fee,
            nonce,
            hashtimer,
            signature: None,
        };
        
        let hash = Self::calculate_transaction_hash(&tx);
        Self {
            hash,
            ..tx
        }
    }

    /// Create a new staking transaction
    pub fn new_staking(
        staker: NodeId,
        validator: NodeId,
        amount: u64,
        action: StakingAction,
        fee: u64,
        nonce: u64,
        hashtimer: HashTimer,
    ) -> Self {
        let staking_data = StakingData {
            staker,
            validator,
            amount,
            action,
        };
        
        let tx = Self {
            hash: [0u8; 32], // Will be calculated
            tx_type: TransactionType::Staking(staking_data),
            fee,
            nonce,
            hashtimer,
            signature: None,
        };
        
        let hash = Self::calculate_transaction_hash(&tx);
        Self {
            hash,
            ..tx
        }
    }

    /// Create a new storage transaction
    pub fn new_storage(
        provider: NodeId,
        file_hash: [u8; 32],
        action: StorageAction,
        data_size: u64,
        fee: u64,
        nonce: u64,
        hashtimer: HashTimer,
    ) -> Self {
        let storage_data = StorageData {
            provider,
            file_hash,
            action,
            data_size,
        };
        
        let tx = Self {
            hash: [0u8; 32], // Will be calculated
            tx_type: TransactionType::Storage(storage_data),
            fee,
            nonce,
            hashtimer,
            signature: None,
        };
        
        let hash = Self::calculate_transaction_hash(&tx);
        Self {
            hash,
            ..tx
        }
    }

    /// Create a new DNS zone update transaction
    pub fn new_dns_zone_update(
        domain: String,
        ops: Vec<crate::dns::apply::ZoneOp>,
        updated_at_us: u64,
        fee: u64,
        nonce: u64,
        hashtimer: HashTimer,
    ) -> Self {
        let dns_data = DnsZoneUpdateData {
            domain,
            ops,
            updated_at_us,
        };
        
        let tx = Self {
            hash: [0u8; 32], // Will be calculated
            tx_type: TransactionType::DnsZoneUpdate(dns_data),
            fee,
            nonce,
            hashtimer,
            signature: None,
        };
        
        let hash = Self::calculate_transaction_hash(&tx);
        Self {
            hash,
            ..tx
        }
    }

    /// Create a new program call transaction
    #[cfg(feature = "contracts")]
    pub fn new_program_call(
        program_id: [u8; 32],
        entrypoint: String,
        calldata: Vec<u8>,
        caps: Vec<crate::blockchain::smart_contract_system::CapabilityRef>,
        budget: u64,
        fee: u64,
        nonce: u64,
        hashtimer: HashTimer,
    ) -> Self {
        let program_call_data = ProgramCallData {
            program_id,
            entrypoint,
            calldata,
            caps,
            budget,
        };
        
        let tx = Self {
            hash: [0u8; 32], // Will be calculated
            tx_type: TransactionType::ProgramCall(program_call_data),
            fee,
            nonce,
            hashtimer,
            signature: None,
        };
        
        let hash = Self::calculate_transaction_hash(&tx);
        Self {
            hash,
            ..tx
        }
    }

    /// Calculate transaction hash
    fn calculate_transaction_hash(tx: &Transaction) -> TransactionHash {
        let mut hasher = Sha256::new();
        
        // Hash transaction type and data
        match &tx.tx_type {
            TransactionType::Payment(data) => {
                hasher.update(&data.from);
                hasher.update(&data.to);
                hasher.update(&data.amount.to_le_bytes());
            }
            TransactionType::Anchor(data) => {
                hasher.update(data.external_chain_id.as_bytes());
                hasher.update(data.external_state_root.as_bytes());
                hasher.update(&data.proof_data);
            }
            TransactionType::Staking(data) => {
                hasher.update(&data.staker);
                hasher.update(&data.validator);
                hasher.update(&data.amount.to_le_bytes());
                hasher.update(&(data.action as u8).to_le_bytes());
            }
            TransactionType::Storage(data) => {
                hasher.update(&data.provider);
                hasher.update(&data.file_hash);
                hasher.update(&data.data_size.to_le_bytes());
                hasher.update(&(data.action as u8).to_le_bytes());
            }
            TransactionType::DnsZoneUpdate(data) => {
                hasher.update(data.domain.as_bytes());
                hasher.update(&data.updated_at_us.to_le_bytes());
                // Hash the operations
                for op in &data.ops {
                    let op_bytes = serde_json::to_vec(op).unwrap_or_default();
                    hasher.update(&op_bytes);
                }
            }
            #[cfg(feature = "contracts")]
            TransactionType::ProgramCall(data) => {
                hasher.update(&data.program_id);
                hasher.update(data.entrypoint.as_bytes());
                hasher.update(&data.calldata);
                // Serialize capabilities for hashing
                for cap in &data.caps {
                    hasher.update(&cap.kind.to_le_bytes());
                    hasher.update(&cap.target);
                    for perm in &cap.permissions {
                        hasher.update(perm.as_bytes());
                    }
                }
                hasher.update(&data.budget.to_le_bytes());
            }
            TransactionType::L2Commit(data) => {
                hasher.update(data.l2_id.as_bytes());
                hasher.update(&data.epoch.to_le_bytes());
                hasher.update(&data.state_root);
                hasher.update(&data.da_hash);
                hasher.update(data.proof_type.as_bytes());
                hasher.update(&data.proof);
                if let Some(ref inline_data) = data.inline_data {
                    hasher.update(inline_data);
                }
            }
            TransactionType::L2Exit(data) => {
                hasher.update(data.l2_id.as_bytes());
                hasher.update(&data.epoch.to_le_bytes());
                hasher.update(&data.proof_of_inclusion);
                hasher.update(&data.account);
                hasher.update(&data.amount.to_le_bytes());
                hasher.update(&data.nonce.to_le_bytes());
            }
        }
        
        hasher.update(&tx.fee.to_le_bytes());
        hasher.update(&tx.nonce.to_le_bytes());
        hasher.update(&tx.hashtimer.ippan_time_ns().to_be_bytes());
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Get transaction hash
    pub fn hash(&self) -> &TransactionHash {
        &self.hash
    }

    /// Get transaction type
    pub fn tx_type(&self) -> &TransactionType {
        &self.tx_type
    }

    /// Check if transaction is an anchor transaction
    pub fn is_anchor(&self) -> bool {
        matches!(self.tx_type, TransactionType::Anchor(_))
    }

    /// Get anchor data if this is an anchor transaction
    pub fn get_anchor_data(&self) -> Option<&AnchorData> {
        match &self.tx_type {
            TransactionType::Anchor(data) => Some(data),
            _ => None,
        }
    }

    /// Check if transaction is a DNS zone update transaction
    pub fn is_dns_zone_update(&self) -> bool {
        matches!(self.tx_type, TransactionType::DnsZoneUpdate(_))
    }

    /// Get DNS zone update data if this is a DNS zone update transaction
    pub fn get_dns_zone_update_data(&self) -> Option<&DnsZoneUpdateData> {
        match &self.tx_type {
            TransactionType::DnsZoneUpdate(data) => Some(data),
            _ => None,
        }
    }

    /// Check if transaction is a program call transaction
    #[cfg(feature = "contracts")]
    pub fn is_program_call(&self) -> bool {
        matches!(self.tx_type, TransactionType::ProgramCall(_))
    }

    /// Get program call data if this is a program call transaction
    #[cfg(feature = "contracts")]
    pub fn get_program_call_data(&self) -> Option<&ProgramCallData> {
        match &self.tx_type {
            TransactionType::ProgramCall(data) => Some(data),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::hashtimer::IppanTimeManager;

    #[tokio::test]
    async fn test_blockdag_creation() {
        let time_manager = Arc::new(IppanTimeManager::new("test_node", 30));
        let blockdag = BlockDAG::new(time_manager);
        
        assert_eq!(blockdag.get_block_count().await, 0);
        assert_eq!(blockdag.get_tip_count().await, 0);
    }

    #[tokio::test]
    async fn test_block_creation() {
        let time_manager = Arc::new(IppanTimeManager::new("test_node", 30));
        let blockdag = BlockDAG::new(time_manager);
        
        let validator_id = [1u8; 32];
        let hashtimer = HashTimer::new("test_node", 1, 1);
        let tx_hashes = vec![];
        
        let block = Block::new(1, tx_hashes, validator_id, hashtimer).unwrap();
        
        assert_eq!(block.round(), 1);
        assert_eq!(block.validator_id(), &validator_id);
    }

    #[tokio::test]
    async fn test_transaction_creation() {
        let from = [1u8; 32];
        let to = [2u8; 32];
        let hashtimer = HashTimer::new("test_node", 1, 1);
        
        let tx = Transaction::new_payment(from, to, 1000, 10, 1, hashtimer);
        
        match &tx.tx_type {
            TransactionType::Payment(data) => {
                assert_eq!(data.from, from);
                assert_eq!(data.to, to);
                assert_eq!(data.amount, 1000);
            }
            _ => panic!("Expected payment transaction"),
        }
        assert_eq!(tx.fee, 10);
        assert_eq!(tx.nonce, 1);
    }

    #[tokio::test]
    async fn test_anchor_transaction_creation() {
        let hashtimer = HashTimer::new("test_node", 1, 1);
        
        let tx = Transaction::new_anchor(
            "testchain".to_string(),
            "0x1234567890abcdef".to_string(),
            Some(crate::crosschain::types::ProofType::External),
            vec![1; 64],
            10,
            1,
            hashtimer,
        );
        
        assert!(tx.is_anchor());
        if let Some(anchor_data) = tx.get_anchor_data() {
            assert_eq!(anchor_data.external_chain_id, "testchain");
            assert_eq!(anchor_data.external_state_root, "0x1234567890abcdef");
        } else {
            panic!("Expected anchor data");
        }
    }
} 