//! BlockDAG implementation for IPPAN consensus
//!
//! Only blocks are part of the DAG. Rounds are a logical/consensus concept and are not DAG nodes.
//!
//! Provides Directed Acyclic Graph structure for blocks with deterministic ordering

use crate::{
    consensus::{hashtimer::HashTimer, ippan_time::IppanTimeManager},
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
    /// Block creation timestamp in nanoseconds
    pub timestamp_ns: u64,
}

/// Block containing transactions and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Transactions in this block
    pub transactions: Vec<Transaction>,
    /// Block signature (optional, for future use)
    #[serde(with = "byte_array_serde")]
    pub signature: Option<[u8; 64]>,
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
    pub proof_type: Option<crate::crosschain::external_anchor::ProofType>,
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
        // Check basic structure
        if block.transactions.is_empty() {
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
        }

        Ok(true)
    }

    /// Calculate block score for ordering
    fn calculate_block_score(&self, block: &Block) -> u64 {
        // Score based on HashTimer precision and IPPAN Time
        let hashtimer_score = block.header.hashtimer.ippan_time_micros();
        let transaction_count = block.transactions.len() as u64;
        
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
        for tx in &block.transactions {
            hasher.update(&tx.hash);
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
            .map(|node| node.block.transactions.len())
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
    /// Create a new block
    pub fn new(
        round: u64,
        transactions: Vec<Transaction>,
        validator_id: NodeId,
        hashtimer: HashTimer,
    ) -> Self {
        let parent_hashes = vec![]; // Will be set by caller
        let height = 0; // Will be calculated by caller
        let timestamp_ns = hashtimer.timestamp_ns;
        
        let header = BlockHeader {
            hash: [0u8; 32], // Will be calculated
            round,
            height,
            validator_id,
            hashtimer,
            parent_hashes,
            timestamp_ns,
        };

        let mut block = Self {
            header,
            transactions,
            signature: None,
        };

        // Calculate hash
        block.header.hash = Self::calculate_block_hash(&block);
        
        block
    }

    /// Calculate block hash
    fn calculate_block_hash(block: &Block) -> BlockHash {
        let mut hasher = Sha256::new();
        
        hasher.update(&block.header.round.to_be_bytes());
        hasher.update(&block.header.validator_id);
        for parent_hash in &block.header.parent_hashes {
            hasher.update(parent_hash);
        }
        for tx in &block.transactions {
            hasher.update(&tx.hash);
        }
        hasher.update(&block.header.hashtimer.ippan_time_ns().to_be_bytes());
        
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
        proof_type: Option<crate::crosschain::external_anchor::ProofType>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ippan_time::IppanTimeManager;

    #[tokio::test]
    async fn test_blockdag_creation() {
        let time_manager = Arc::new(IppanTimeManager::new(3, 30));
        let blockdag = BlockDAG::new(time_manager);
        
        assert_eq!(blockdag.get_block_count().await, 0);
        assert_eq!(blockdag.get_tip_count().await, 0);
    }

    #[tokio::test]
    async fn test_block_creation() {
        let time_manager = Arc::new(IppanTimeManager::new(3, 30));
        let blockdag = BlockDAG::new(time_manager);
        
        let validator_id = [1u8; 32];
        let hashtimer = HashTimer::new("test_node", 1, 1);
        let transactions = vec![];
        
        let block = Block::new(1, transactions, validator_id, hashtimer);
        
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
            Some(crate::crosschain::external_anchor::ProofType::Signature),
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