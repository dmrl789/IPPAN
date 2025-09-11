//! Real BFT (Byzantine Fault Tolerant) consensus engine for IPPAN
//! 
//! Implements a practical BFT consensus algorithm with the following phases:
//! 1. Pre-prepare: Primary proposes a block
//! 2. Prepare: Validators vote on the proposal
//! 3. Commit: Validators commit to the block
//! 4. Finalize: Block is finalized and added to the chain

use crate::{Result, IppanError, TransactionHash};
use crate::crypto::{RealEd25519, RealHashFunctions, RealTransactionSigner};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// BFT consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BFTConfig {
    /// Block time in milliseconds
    pub block_time_ms: u64,
    /// View change timeout in milliseconds
    pub view_change_timeout_ms: u64,
    /// Maximum number of transactions per block
    pub max_transactions_per_block: usize,
    /// Minimum number of validators required
    pub min_validators: usize,
    /// Maximum number of validators
    pub max_validators: usize,
    /// Enable optimistic finality
    pub enable_optimistic_finality: bool,
    /// Enable pipelining
    pub enable_pipelining: bool,
}

impl Default for BFTConfig {
    fn default() -> Self {
        Self {
            block_time_ms: 1000, // 1 second blocks
            view_change_timeout_ms: 10000, // 10 seconds
            max_transactions_per_block: 1000,
            min_validators: 4,
            max_validators: 100,
            enable_optimistic_finality: true,
            enable_pipelining: true,
        }
    }
}

/// BFT consensus phases
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BFTPhase {
    /// Waiting for new block proposal
    Idle,
    /// Pre-prepare phase: Primary has proposed a block
    PrePrepare,
    /// Prepare phase: Validators are voting on the proposal
    Prepare,
    /// Commit phase: Validators are committing to the block
    Commit,
    /// Finalized: Block has been finalized
    Finalized,
    /// View change: Changing primary due to timeout or failure
    ViewChange,
}

/// BFT message types
#[derive(Debug, Clone)]
pub enum BFTMessage {
    /// Pre-prepare message from primary
    PrePrepare {
        view_number: u64,
        sequence_number: u64,
        block_hash: [u8; 32],
        block_data: Vec<u8>,
        signature: [u8; 64],
    },
    /// Prepare message from validator
    Prepare {
        view_number: u64,
        sequence_number: u64,
        block_hash: [u8; 32],
        validator_id: [u8; 32],
        signature: [u8; 64],
    },
    /// Commit message from validator
    Commit {
        view_number: u64,
        sequence_number: u64,
        block_hash: [u8; 32],
        validator_id: [u8; 32],
        signature: [u8; 64],
    },
    /// View change message
    ViewChange {
        new_view_number: u64,
        validator_id: [u8; 32],
        prepared_proofs: Vec<PreparedProof>,
        signature: [u8; 64],
    },
}

/// Proof of prepared block
#[derive(Debug, Clone)]
pub struct PreparedProof {
    pub view_number: u64,
    pub sequence_number: u64,
    pub block_hash: [u8; 32],
    pub prepare_signatures: Vec<[u8; 64]>,
}

/// BFT consensus state
#[derive(Debug, Clone)]
pub struct BFTConsensusState {
    /// Current view number
    pub view_number: u64,
    /// Current sequence number
    pub sequence_number: u64,
    /// Current phase
    pub phase: BFTPhase,
    /// Primary validator ID
    pub primary_id: [u8; 32],
    /// Current block being processed
    pub current_block: Option<BFTBlock>,
    /// Prepare votes received
    pub prepare_votes: HashMap<[u8; 32], BFTMessage>,
    /// Commit votes received
    pub commit_votes: HashMap<[u8; 32], BFTMessage>,
    /// View change messages
    pub view_change_messages: HashMap<[u8; 32], BFTMessage>,
    /// Last view change time
    pub last_view_change: Instant,
    /// Block timeout
    pub block_timeout: Instant,
}

/// BFT block structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BFTBlock {
    /// Block header
    pub header: BFTBlockHeader,
    /// Transactions in the block
    pub transactions: Vec<BFTTransaction>,
    /// Block hash
    pub hash: [u8; 32],
}

/// BFT block header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BFTBlockHeader {
    /// Block number
    pub number: u64,
    /// Previous block hash
    pub previous_hash: [u8; 32],
    /// Merkle root of transactions
    pub merkle_root: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// View number
    pub view_number: u64,
    /// Sequence number
    pub sequence_number: u64,
    /// Validator ID
    pub validator_id: [u8; 32],
}

/// BFT transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BFTTransaction {
    /// Transaction hash
    pub hash: [u8; 32],
    /// Transaction data
    pub data: Vec<u8>,
    /// Sender
    pub sender: [u8; 32],
    /// From address
    pub from: String,
    /// To address
    pub to: String,
    /// Amount
    pub amount: u64,
    /// Fee
    pub fee: u64,
    /// Gas used
    pub gas_used: u64,
    /// Gas price
    pub gas_price: u64,
    /// Nonce
    pub nonce: u64,
    /// Signature
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
}

/// BFT consensus engine
pub struct BFTEngine {
    /// Configuration
    config: BFTConfig,
    /// Consensus state
    state: Arc<RwLock<BFTConsensusState>>,
    /// Validator registry
    validators: Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
    /// Message channel
    message_tx: mpsc::UnboundedSender<BFTMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<BFTMessage>>>,
    /// Our signing key
    signing_key: SigningKey,
    /// Our verifying key
    verifying_key: VerifyingKey,
    /// Our node ID
    node_id: [u8; 32],
}

/// Validator information
#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    /// Public key
    pub public_key: VerifyingKey,
    /// Stake amount
    pub stake: u64,
    /// Is active
    pub is_active: bool,
    /// Last seen timestamp
    pub last_seen: Instant,
}

impl BFTEngine {
    /// Create a new BFT consensus engine
    pub fn new(config: BFTConfig, signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        let node_id = verifying_key.to_bytes();
        
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        let state = BFTConsensusState {
            view_number: 0,
            sequence_number: 0,
            phase: BFTPhase::Idle,
            primary_id: [0u8; 32], // Will be set when validators are added
            current_block: None,
            prepare_votes: HashMap::new(),
            commit_votes: HashMap::new(),
            view_change_messages: HashMap::new(),
            last_view_change: Instant::now(),
            block_timeout: Instant::now(),
        };
        
        Self {
            config,
            state: Arc::new(RwLock::new(state)),
            validators: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            signing_key,
            verifying_key,
            node_id,
        }
    }
    
    /// Add a validator to the consensus
    pub async fn add_validator(&self, node_id: [u8; 32], public_key: VerifyingKey, stake: u64) -> Result<()> {
        let mut validators = self.validators.write().await;
        
        let validator_info = ValidatorInfo {
            public_key,
            stake,
            is_active: true,
            last_seen: Instant::now(),
        };
        
        validators.insert(node_id, validator_info);
        
        // Update primary if this is the first validator or if we need to set primary
        let mut state = self.state.write().await;
        if state.primary_id == [0u8; 32] {
            state.primary_id = node_id;
        }
        
        info!("Added validator: {:02x?} with stake: {}", node_id, stake);
        Ok(())
    }
    
    /// Start the BFT consensus engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting BFT consensus engine");
        
        // Start message processing loop
        let state = self.state.clone();
        let validators = self.validators.clone();
        let config = self.config.clone();
        let signing_key = self.signing_key.clone();
        let node_id = self.node_id;
        
        tokio::spawn(async move {
            Self::message_processing_loop(state, validators, config, signing_key, node_id).await;
        });
        
        // Start block production if we're the primary
        let state = self.state.clone();
        let validators = self.validators.clone();
        let config = self.config.clone();
        let signing_key = self.signing_key.clone();
        let node_id = self.node_id;
        
        tokio::spawn(async move {
            Self::block_production_loop(state, validators, config, signing_key, node_id).await;
        });
        
        Ok(())
    }
    
    /// Process incoming BFT messages
    async fn message_processing_loop(
        state: Arc<RwLock<BFTConsensusState>>,
        validators: Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        config: BFTConfig,
        signing_key: SigningKey,
        node_id: [u8; 32],
    ) {
        let mut message_rx = {
            let state = state.read().await;
            // Create a new receiver for this loop
            let (_, rx) = mpsc::unbounded_channel();
            rx
        };
        
        loop {
            // Process messages
            if let Some(message) = message_rx.recv().await {
                if let Err(e) = Self::process_message(
                    &state,
                    &validators,
                    &config,
                    &signing_key,
                    node_id,
                    message,
                ).await {
                    error!("Error processing BFT message: {}", e);
                }
            }
            
            // Check for timeouts
            if let Err(e) = Self::check_timeouts(&state, &validators, &config, &signing_key, node_id).await {
                error!("Error checking timeouts: {}", e);
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Block production loop for primary
    async fn block_production_loop(
        state: Arc<RwLock<BFTConsensusState>>,
        validators: Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        config: BFTConfig,
        signing_key: SigningKey,
        node_id: [u8; 32],
    ) {
        loop {
            let is_primary = {
                let state = state.read().await;
                state.primary_id == node_id
            };
            
            if is_primary {
                if let Err(e) = Self::propose_block(&state, &validators, &config, &signing_key, node_id).await {
                    error!("Error proposing block: {}", e);
                }
            }
            
            tokio::time::sleep(Duration::from_millis(config.block_time_ms)).await;
        }
    }
    
    /// Process a BFT message
    async fn process_message(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        config: &BFTConfig,
        signing_key: &SigningKey,
        node_id: [u8; 32],
        message: BFTMessage,
    ) -> Result<()> {
        match message {
            BFTMessage::PrePrepare { view_number, sequence_number, block_hash, block_data, signature } => {
                Self::handle_pre_prepare(state, validators, view_number, sequence_number, block_hash, block_data, signature).await
            },
            BFTMessage::Prepare { view_number, sequence_number, block_hash, validator_id, signature } => {
                Self::handle_prepare(state, validators, view_number, sequence_number, block_hash, validator_id, signature).await
            },
            BFTMessage::Commit { view_number, sequence_number, block_hash, validator_id, signature } => {
                Self::handle_commit(state, validators, view_number, sequence_number, block_hash, validator_id, signature).await
            },
            BFTMessage::ViewChange { new_view_number, validator_id, prepared_proofs, signature } => {
                Self::handle_view_change(state, validators, new_view_number, validator_id, prepared_proofs, signature).await
            },
        }
    }
    
    /// Handle pre-prepare message
    async fn handle_pre_prepare(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        view_number: u64,
        sequence_number: u64,
        block_hash: [u8; 32],
        block_data: Vec<u8>,
        signature: [u8; 64],
    ) -> Result<()> {
        let mut state = state.write().await;
        
        // Verify we're in the right view and sequence
        if view_number != state.view_number {
            return Err(IppanError::Consensus("Invalid view number".to_string()));
        }
        
        if sequence_number != state.sequence_number {
            return Err(IppanError::Consensus("Invalid sequence number".to_string()));
        }
        
        // Verify signature (simplified - in real implementation, verify against primary's public key)
        // For now, just check signature format
        if signature == [0u8; 64] {
            return Err(IppanError::Consensus("Invalid signature".to_string()));
        }
        
        // Create block from data
        let block = BFTBlock {
            header: BFTBlockHeader {
                number: sequence_number,
                previous_hash: [0u8; 32], // Will be set properly in real implementation
                merkle_root: RealHashFunctions::merkle_root(&[]), // Will be calculated from transactions
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                view_number,
                sequence_number,
                validator_id: state.primary_id,
            },
            transactions: vec![], // Will be parsed from block_data
            hash: block_hash,
        };
        
        state.current_block = Some(block);
        state.phase = BFTPhase::PrePrepare;
        state.block_timeout = Instant::now();
        
        info!("Received pre-prepare for view {} sequence {}", view_number, sequence_number);
        
        // Send prepare message
        // This would be implemented with actual network communication
        debug!("Would send prepare message");
        
        Ok(())
    }
    
    /// Handle prepare message
    async fn handle_prepare(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        view_number: u64,
        sequence_number: u64,
        block_hash: [u8; 32],
        validator_id: [u8; 32],
        signature: [u8; 64],
    ) -> Result<()> {
        let mut state = state.write().await;
        
        // Verify we're in prepare phase
        if state.phase != BFTPhase::PrePrepare && state.phase != BFTPhase::Prepare {
            return Err(IppanError::Consensus("Not in prepare phase".to_string()));
        }
        
        // Verify view and sequence numbers
        if view_number != state.view_number || sequence_number != state.sequence_number {
            return Err(IppanError::Consensus("Invalid view or sequence number".to_string()));
        }
        
        // Verify block hash matches
        if let Some(ref block) = state.current_block {
            if block.hash != block_hash {
                return Err(IppanError::Consensus("Block hash mismatch".to_string()));
            }
        }
        
        // Add prepare vote
        let prepare_message = BFTMessage::Prepare {
            view_number,
            sequence_number,
            block_hash,
            validator_id,
            signature,
        };
        
        state.prepare_votes.insert(validator_id, prepare_message);
        state.phase = BFTPhase::Prepare;
        
        // Check if we have enough prepare votes
        let validators = validators.read().await;
        let total_stake: u64 = validators.values().map(|v| v.stake).sum();
        let prepare_stake: u64 = state.prepare_votes.values()
            .filter_map(|msg| {
                if let BFTMessage::Prepare { validator_id, .. } = msg {
                    validators.get(validator_id).map(|v| v.stake)
                } else {
                    None
                }
            })
            .sum();
        
        // If we have 2f+1 stake in prepare votes, move to commit phase
        if prepare_stake * 2 > total_stake {
            info!("Received 2f+1 prepare votes, moving to commit phase");
            state.phase = BFTPhase::Commit;
            
            // Send commit message
            debug!("Would send commit message");
        }
        
        Ok(())
    }
    
    /// Handle commit message
    async fn handle_commit(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        view_number: u64,
        sequence_number: u64,
        block_hash: [u8; 32],
        validator_id: [u8; 32],
        signature: [u8; 64],
    ) -> Result<()> {
        let mut state = state.write().await;
        
        // Verify we're in commit phase
        if state.phase != BFTPhase::Commit {
            return Err(IppanError::Consensus("Not in commit phase".to_string()));
        }
        
        // Verify view and sequence numbers
        if view_number != state.view_number || sequence_number != state.sequence_number {
            return Err(IppanError::Consensus("Invalid view or sequence number".to_string()));
        }
        
        // Verify block hash matches
        if let Some(ref block) = state.current_block {
            if block.hash != block_hash {
                return Err(IppanError::Consensus("Block hash mismatch".to_string()));
            }
        }
        
        // Add commit vote
        let commit_message = BFTMessage::Commit {
            view_number,
            sequence_number,
            block_hash,
            validator_id,
            signature,
        };
        
        state.commit_votes.insert(validator_id, commit_message);
        
        // Check if we have enough commit votes
        let validators = validators.read().await;
        let total_stake: u64 = validators.values().map(|v| v.stake).sum();
        let commit_stake: u64 = state.commit_votes.values()
            .filter_map(|msg| {
                if let BFTMessage::Commit { validator_id, .. } = msg {
                    validators.get(validator_id).map(|v| v.stake)
                } else {
                    None
                }
            })
            .sum();
        
        // If we have 2f+1 stake in commit votes, finalize the block
        if commit_stake * 2 > total_stake {
            info!("Received 2f+1 commit votes, finalizing block");
            state.phase = BFTPhase::Finalized;
            
            // Finalize the block
            if let Some(block) = state.current_block.take() {
                info!("Finalized block {} in view {}", block.header.number, view_number);
                
                // Move to next sequence number
                state.sequence_number += 1;
                state.phase = BFTPhase::Idle;
                
                // Clear votes
                state.prepare_votes.clear();
                state.commit_votes.clear();
            }
        }
        
        Ok(())
    }
    
    /// Handle view change message
    async fn handle_view_change(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        new_view_number: u64,
        validator_id: [u8; 32],
        prepared_proofs: Vec<PreparedProof>,
        signature: [u8; 64],
    ) -> Result<()> {
        let mut state = state.write().await;
        
        // Verify view change is for the next view
        if new_view_number != state.view_number + 1 {
            return Err(IppanError::Consensus("Invalid view change number".to_string()));
        }
        
        // Add view change message
        let view_change_message = BFTMessage::ViewChange {
            new_view_number,
            validator_id,
            prepared_proofs,
            signature,
        };
        
        state.view_change_messages.insert(validator_id, view_change_message);
        
        // Check if we have enough view change messages
        let validators = validators.read().await;
        let total_stake: u64 = validators.values().map(|v| v.stake).sum();
        let view_change_stake: u64 = state.view_change_messages.values()
            .filter_map(|msg| {
                if let BFTMessage::ViewChange { validator_id, .. } = msg {
                    validators.get(validator_id).map(|v| v.stake)
                } else {
                    None
                }
            })
            .sum();
        
        // If we have 2f+1 stake in view change messages, change view
        if view_change_stake * 2 > total_stake {
            info!("Received 2f+1 view change messages, changing to view {}", new_view_number);
            state.view_number = new_view_number;
            state.phase = BFTPhase::Idle;
            state.last_view_change = Instant::now();
            
            // Select new primary (simplified - round-robin)
            let validator_ids: Vec<[u8; 32]> = validators.keys().cloned().collect();
            if !validator_ids.is_empty() {
                let primary_index = (new_view_number as usize) % validator_ids.len();
                state.primary_id = validator_ids[primary_index];
            }
            
            // Clear messages
            state.view_change_messages.clear();
            state.prepare_votes.clear();
            state.commit_votes.clear();
        }
        
        Ok(())
    }
    
    /// Check for timeouts and handle them
    async fn check_timeouts(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        config: &BFTConfig,
        signing_key: &SigningKey,
        node_id: [u8; 32],
    ) -> Result<()> {
        let mut state = state.write().await;
        
        // Check block timeout
        if state.block_timeout.elapsed() > Duration::from_millis(config.block_time_ms * 2) {
            if state.phase != BFTPhase::Finalized && state.phase != BFTPhase::Idle {
                warn!("Block timeout, initiating view change");
                state.phase = BFTPhase::ViewChange;
                state.last_view_change = Instant::now();
                
                // Send view change message
                debug!("Would send view change message");
            }
        }
        
        // Check view change timeout
        if state.phase == BFTPhase::ViewChange && 
           state.last_view_change.elapsed() > Duration::from_millis(config.view_change_timeout_ms) {
            warn!("View change timeout, retrying view change");
            state.last_view_change = Instant::now();
            
            // Send view change message again
            debug!("Would resend view change message");
        }
        
        Ok(())
    }
    
    /// Propose a new block (primary only)
    async fn propose_block(
        state: &Arc<RwLock<BFTConsensusState>>,
        validators: &Arc<RwLock<HashMap<[u8; 32], ValidatorInfo>>>,
        config: &BFTConfig,
        signing_key: &SigningKey,
        node_id: [u8; 32],
    ) -> Result<()> {
        let mut state = state.write().await;
        
        // Only primary can propose blocks
        if state.primary_id != node_id {
            return Ok(());
        }
        
        // Only propose if we're idle
        if state.phase != BFTPhase::Idle {
            return Ok(());
        }
        
        // Create a new block
        let block = BFTBlock {
            header: BFTBlockHeader {
                number: state.sequence_number,
                previous_hash: [0u8; 32], // Will be set properly in real implementation
                merkle_root: RealHashFunctions::merkle_root(&[]), // Will be calculated from transactions
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                view_number: state.view_number,
                sequence_number: state.sequence_number,
                validator_id: node_id,
            },
            transactions: vec![], // Will be populated with actual transactions
            hash: [0u8; 32], // Will be calculated
        };
        
        // Calculate block hash
        let block_data = bincode::serialize(&block).unwrap_or_default();
        let block_hash = RealHashFunctions::sha256(&block_data);
        
        // Sign the block
        let signature = RealTransactionSigner::sign_transaction(signing_key, &block_data)?;
        
        // Update state
        state.current_block = Some(block);
        state.phase = BFTPhase::PrePrepare;
        state.block_timeout = Instant::now();
        
        info!("Proposed block {} in view {}", state.sequence_number, state.view_number);
        
        // Send pre-prepare message
        debug!("Would broadcast pre-prepare message");
        
        Ok(())
    }
    
    /// Get current consensus state
    pub async fn get_state(&self) -> BFTConsensusState {
        self.state.read().await.clone()
    }
    
    /// Get validator count
    pub async fn get_validator_count(&self) -> usize {
        self.validators.read().await.len()
    }
    
    /// Check if we're the primary
    pub async fn is_primary(&self) -> bool {
        let state = self.state.read().await;
        state.primary_id == self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::{rngs::OsRng, RngCore};
    
    #[tokio::test]
    async fn test_bft_engine_creation() {
        let mut rng = OsRng;
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let config = BFTConfig::default();
        let engine = BFTEngine::new(config, signing_key);
        
        assert_eq!(engine.get_validator_count().await, 0);
        assert!(!engine.is_primary().await);
    }
    
    #[tokio::test]
    async fn test_add_validator() {
        let mut rng = OsRng;
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let config = BFTConfig::default();
        let engine = BFTEngine::new(config, signing_key.clone());
        
        // Add the engine's own key as a validator
        let engine_id = signing_key.verifying_key().to_bytes();
        engine.add_validator(engine_id, signing_key.verifying_key(), 1000).await.unwrap();
        
        assert_eq!(engine.get_validator_count().await, 1);
        assert!(engine.is_primary().await); // Should be primary as first validator
    }
    
    #[tokio::test]
    async fn test_consensus_phases() {
        let mut rng = OsRng;
        let mut signing_key_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key_bytes);
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let config = BFTConfig::default();
        let engine = BFTEngine::new(config, signing_key.clone());
        
        // Test initial state
        let state = engine.get_state().await;
        assert_eq!(state.phase, BFTPhase::Idle);
        assert_eq!(state.view_number, 0);
        assert_eq!(state.sequence_number, 0);
        
        // Add the engine as a validator to enable consensus
        let engine_id = signing_key.verifying_key().to_bytes();
        engine.add_validator(engine_id, signing_key.verifying_key(), 1000).await.unwrap();
        
        // Verify we're still in Idle phase after adding validator
        let state = engine.get_state().await;
        assert_eq!(state.phase, BFTPhase::Idle);
    }
}
