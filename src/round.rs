use crate::block::Block;
use crate::crypto::{self, Hash, KeyPair, SignatureBytes};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub const ROUND_CADENCE_MS: u64 = 200; // 200ms default
pub const MIN_ROUND_CADENCE_MS: u64 = 100;
pub const MAX_ROUND_CADENCE_MS: u64 = 250;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundBlock {
    pub block: Block,
    pub signatures: HashMap<Hash, SignatureBytes>, // verifier_id -> signature
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub round_id: u64,
    pub start_time_us: u64,
    pub end_time_us: u64,
    pub blocks: Vec<RoundBlock>,
    pub verifier_set: Vec<Hash>, // List of verifier public key hashes
    pub finality_threshold: usize, // 2f+1 signatures required
    pub finalized: bool,
    pub finality_signatures: HashMap<Hash, SignatureBytes>, // verifier_id -> finality signature
}

impl Round {
    pub fn new(
        round_id: u64,
        start_time_us: u64,
        verifier_set: Vec<Hash>,
    ) -> Self {
        let finality_threshold = (verifier_set.len() * 2 / 3) + 1; // 2f+1
        
        Self {
            round_id,
            start_time_us,
            end_time_us: 0,
            blocks: Vec::new(),
            verifier_set,
            finality_threshold,
            finalized: false,
            finality_signatures: HashMap::new(),
        }
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), Error> {
        // Verify block belongs to this round
        if block.header.round_id != self.round_id {
            return Err(Error::Round(format!(
                "Block round_id {} doesn't match round {}",
                block.header.round_id, self.round_id
            )));
        }

        // Verify block
        block.verify()?;

        let round_block = RoundBlock {
            block,
            signatures: HashMap::new(),
        };

        self.blocks.push(round_block);
        Ok(())
    }

    pub fn add_block_signature(
        &mut self,
        block_id: &Hash,
        verifier_id: Hash,
        signature: SignatureBytes,
    ) -> Result<(), Error> {
        // Find the block
        for round_block in &mut self.blocks {
            if round_block.block.compute_id()? == *block_id {
                round_block.signatures.insert(verifier_id, signature);
                return Ok(());
            }
        }

        Err(Error::Round("Block not found in round".to_string()))
    }

    pub fn add_finality_signature(
        &mut self,
        verifier_id: Hash,
        signature: SignatureBytes,
    ) -> Result<bool, Error> {
        // Verify the verifier is in the verifier set
        if !self.verifier_set.contains(&verifier_id) {
            return Err(Error::Round("Verifier not in verifier set".to_string()));
        }

        self.finality_signatures.insert(verifier_id, signature);

        // Check if we have enough signatures for finality
        if self.finality_signatures.len() >= self.finality_threshold {
            self.finalized = true;
            info!("Round {} finalized with {} signatures", self.round_id, self.finality_signatures.len());
            return Ok(true);
        }

        Ok(false)
    }

    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    pub fn get_finalized_blocks(&self) -> Vec<&Block> {
        if self.finalized {
            self.blocks.iter().map(|rb| &rb.block).collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_all_blocks(&self) -> Vec<&Block> {
        self.blocks.iter().map(|rb| &rb.block).collect()
    }

    pub fn get_transaction_count(&self) -> usize {
        self.blocks.iter().map(|rb| rb.block.transaction_count()).sum()
    }

    pub fn end_round(&mut self, end_time_us: u64) {
        self.end_time_us = end_time_us;
    }

    pub fn duration_ms(&self) -> u64 {
        if self.end_time_us > self.start_time_us {
            (self.end_time_us - self.start_time_us) / 1000
        } else {
            0
        }
    }
}

pub struct RoundManager {
    current_round: Arc<RwLock<Option<Round>>>,
    completed_rounds: Arc<RwLock<Vec<Round>>>,
    round_cadence_ms: u64,
    verifier_keypair: KeyPair,
}

impl RoundManager {
    pub fn new(round_cadence_ms: u64, verifier_keypair: KeyPair) -> Self {
        Self {
            current_round: Arc::new(RwLock::new(None)),
            completed_rounds: Arc::new(RwLock::new(Vec::new())),
            round_cadence_ms: round_cadence_ms.clamp(MIN_ROUND_CADENCE_MS, MAX_ROUND_CADENCE_MS),
            verifier_keypair,
        }
    }

    pub async fn start_new_round(&self, round_id: u64, start_time_us: u64, verifier_set: Vec<Hash>) {
        let mut current_round = self.current_round.write().await;
        
        // Move current round to completed if it exists
        if let Some(round) = current_round.take() {
            let mut completed_rounds = self.completed_rounds.write().await;
            completed_rounds.push(round);
        }

        // Create new round
        *current_round = Some(Round::new(round_id, start_time_us, verifier_set));
        
        info!("Started round {} with {} verifiers", round_id, verifier_set.len());
    }

    pub async fn add_block_to_current_round(&self, block: Block) -> Result<(), Error> {
        let mut current_round = self.current_round.write().await;
        
        if let Some(round) = current_round.as_mut() {
            round.add_block(block)
        } else {
            Err(Error::Round("No current round".to_string()))
        }
    }

    pub async fn sign_block(&self, block_id: &Hash) -> Result<SignatureBytes, Error> {
        let verifier_id = crypto::hash(&self.verifier_keypair.public_key);
        
        // Create signature message
        let mut message = Vec::new();
        message.extend_from_slice(&block_id);
        message.extend_from_slice(&verifier_id);
        
        let signature = self.verifier_keypair.sign(&message)?;
        
        // Add signature to current round
        let mut current_round = self.current_round.write().await;
        if let Some(round) = current_round.as_mut() {
            round.add_block_signature(block_id, verifier_id, signature)?;
        }
        
        Ok(signature)
    }

    pub async fn sign_finality(&self) -> Result<Option<SignatureBytes>, Error> {
        let verifier_id = crypto::hash(&self.verifier_keypair.public_key);
        
        let mut current_round = self.current_round.write().await;
        if let Some(round) = current_round.as_mut() {
            // Create finality signature message
            let mut message = Vec::new();
            message.extend_from_slice(&round.round_id.to_le_bytes());
            message.extend_from_slice(&verifier_id);
            
            let signature = self.verifier_keypair.sign(&message)?;
            
            // Add finality signature
            let finalized = round.add_finality_signature(verifier_id, signature)?;
            
            if finalized {
                info!("Round {} finalized by this verifier", round.round_id);
                return Ok(Some(signature));
            }
        }
        
        Ok(None)
    }

    pub async fn add_finality_signature(&self, verifier_id: Hash, signature: SignatureBytes) -> Result<bool, Error> {
        let mut current_round = self.current_round.write().await;
        
        if let Some(round) = current_round.as_mut() {
            round.add_finality_signature(verifier_id, signature)
        } else {
            Err(Error::Round("No current round".to_string()))
        }
    }

    pub async fn get_current_round(&self) -> Option<Round> {
        self.current_round.read().await.clone()
    }

    pub async fn get_completed_rounds(&self) -> Vec<Round> {
        self.completed_rounds.read().await.clone()
    }

    pub async fn get_finalized_rounds(&self) -> Vec<Round> {
        let completed_rounds = self.completed_rounds.read().await;
        completed_rounds.iter().filter(|r| r.is_finalized()).cloned().collect()
    }

    pub fn get_round_cadence_ms(&self) -> u64 {
        self.round_cadence_ms
    }

    pub async fn end_current_round(&self, end_time_us: u64) {
        let mut current_round = self.current_round.write().await;
        
        if let Some(round) = current_round.as_mut() {
            round.end_round(end_time_us);
            info!("Ended round {} (duration: {}ms, finalized: {})", 
                  round.round_id, round.duration_ms(), round.is_finalized());
        }
    }

    pub async fn get_verifier_id(&self) -> Hash {
        crypto::hash(&self.verifier_keypair.public_key)
    }
}

// VRF (Verifiable Random Function) for verifier selection
pub fn select_verifier_set(
    previous_round_hash: &Hash,
    total_validators: &[Hash],
    verifier_count: usize,
) -> Vec<Hash> {
    // Simple deterministic selection based on previous round hash
    // In a real implementation, this would use a proper VRF
    let mut selected = Vec::new();
    let mut hash_bytes = previous_round_hash.to_vec();
    
    for i in 0..verifier_count {
        // Use hash as seed for selection
        let index = (hash_bytes[i % hash_bytes.len()] as usize) % total_validators.len();
        let verifier = total_validators[index];
        
        if !selected.contains(&verifier) {
            selected.push(verifier);
        }
        
        // Rotate hash for next selection
        hash_bytes.rotate_left(1);
    }
    
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::block::Block;
    use crate::transaction::Transaction;
    use crate::time::IppanTime;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_round_creation() {
        let verifier_set = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let round = Round::new(1, 1234567890, verifier_set.clone());
        
        assert_eq!(round.round_id, 1);
        assert_eq!(round.verifier_set, verifier_set);
        assert!(!round.is_finalized());
    }

    #[tokio::test]
    async fn test_round_manager() {
        let keypair = KeyPair::generate();
        let manager = RoundManager::new(200, keypair);
        
        let verifier_set = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        manager.start_new_round(1, 1234567890, verifier_set).await;
        
        let current_round = manager.get_current_round().await;
        assert!(current_round.is_some());
        assert_eq!(current_round.unwrap().round_id, 1);
    }

    #[tokio::test]
    async fn test_verifier_selection() {
        let validators = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];
        let previous_hash = [0u8; 32];
        
        let verifier_set = select_verifier_set(&previous_hash, &validators, 3);
        assert_eq!(verifier_set.len(), 3);
    }

    #[tokio::test]
    async fn test_block_addition_to_round() {
        let verifier_set = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let mut round = Round::new(1, 1234567890, verifier_set);
        
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let block = Block::new(
            vec![],
            1, // round_id
            1234567890,
            [1u8; 32],
            vec![tx],
        ).unwrap();
        
        round.add_block(block).unwrap();
        assert_eq!(round.get_all_blocks().len(), 1);
    }
}
