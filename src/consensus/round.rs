//! Round management for consensus
//!
//! Rounds are a logical/consensus concept for validator selection and block production, and are NOT part of the DAG structure.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;


/// Round state enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoundState {
    /// Waiting for round to start
    Waiting,
    /// Producing blocks
    ProducingBlocks,
    /// Validating blocks
    ValidatingBlocks,
    /// Round completed
    Completed,
}

/// Round information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    /// Round number
    pub number: u64,
    
    /// Round state
    pub state: RoundState,
    
    /// Validators for this round
    pub validators: Vec<[u8; 32]>,
    
    /// Blocks produced in this round
    pub blocks: Vec<[u8; 32]>,
}

/// Round manager for consensus rounds
#[derive(Debug)]
pub struct RoundManager {
    /// Current round
    pub current_round: Round,
    /// Validators and their stakes
    validators: HashMap<[u8; 32], u64>,
    // Maximum number of validators per round
    // max_validators: usize, // TODO: Use when implementing validator limits
}

impl RoundManager {
    /// Create a new round manager
    pub fn new(_max_validators: usize) -> Self {
        Self {
            current_round: Round::new(0),
            validators: HashMap::new(),
            // max_validators, // TODO: Use when implementing validator limits
        }
    }

    /// Add a validator with stake
    pub fn add_validator(&mut self, node_id: [u8; 32], stake: u64) {
        self.validators.insert(node_id, stake);
        self.current_round.add_validator(node_id);
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, node_id: &[u8; 32]) {
        self.validators.remove(node_id);
        self.current_round.remove_validator(node_id);
    }

    /// Get current round number
    pub fn current_round(&self) -> u64 {
        self.current_round.number
    }

    /// Check if a validator is authorized for the current round
    pub fn is_validator_authorized(&self, validator: &[u8; 32], round: u64) -> bool {
        if round != self.current_round.number {
            return false;
        }
        self.current_round.has_validator(validator)
    }

    /// Update to a new round
    pub fn update_round(&mut self, round_number: u64) {
        if round_number > self.current_round.number {
            self.current_round = Round::new(round_number);
            // Copy validators from the previous round
            for (validator, _stake) in &self.validators {
                self.current_round.add_validator(*validator);
            }
        }
    }

    /// Get validators for current round
    pub fn get_validators(&self) -> &Vec<[u8; 32]> {
        &self.current_round.validators
    }

    /// Get validator stakes
    pub fn get_validator_stakes(&self) -> &HashMap<[u8; 32], u64> {
        &self.validators
    }

    /// Get current round state
    pub fn get_round_state(&self) -> &RoundState {
        &self.current_round.state
    }

    /// Set round state
    pub fn set_round_state(&mut self, state: RoundState) {
        self.current_round.state = state;
    }
}

/// Round statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundStats {
    /// Round number
    pub round_number: u64,
    /// Current round state
    pub current_round_state: RoundState,
    /// Number of validators
    pub validator_count: usize,
    /// Number of blocks produced
    pub block_count: usize,
}

impl Round {
    /// Create a new round
    pub fn new(number: u64) -> Self {
        Self {
            number,
            state: RoundState::Waiting,
            validators: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Add a validator to the round
    pub fn add_validator(&mut self, validator: [u8; 32]) {
        if !self.validators.contains(&validator) {
            self.validators.push(validator);
        }
    }

    /// Remove a validator from the round
    pub fn remove_validator(&mut self, validator: &[u8; 32]) {
        self.validators.retain(|v| v != validator);
    }

    /// Add a block to the round
    pub fn add_block(&mut self, block_hash: [u8; 32]) {
        if !self.blocks.contains(&block_hash) {
            self.blocks.push(block_hash);
        }
    }

    /// Check if a validator is in this round
    pub fn has_validator(&self, validator: &[u8; 32]) -> bool {
        self.validators.contains(validator)
    }

    /// Get validator count
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Get block count
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        consensus::ippan_time::IppanTimeManager,
        consensus::randomness::RandomnessEngine,
        staking::StakingManager,
    };
    
    #[tokio::test]
    async fn test_round_creation() {
        let node_id = [1u8; 32];
        let time_manager = Arc::new(IppanTimeManager::new(node_id, 1));
        let randomness = Arc::new(RandomnessEngine::new(5, time_manager));
        let staking = Arc::new(StakingManager::new(crate::config::StakingConfig::default(), Arc::new(crate::wallet::WalletManager::new().await.unwrap())));
        
        let manager = RoundManager::new(30);
        
        assert_eq!(manager.current_round(), 0);
    }
    
    #[tokio::test]
    async fn test_round_start() {
        let node_id = [1u8; 32];
        let time_manager = Arc::new(IppanTimeManager::new(node_id, 1));
        let randomness = Arc::new(RandomnessEngine::new(5, time_manager));
        let staking = Arc::new(StakingManager::new(crate::config::StakingConfig::default(), Arc::new(crate::wallet::WalletManager::new().await.unwrap())));
        
        let mut manager = RoundManager::new(30);
        
        manager.add_validator(node_id, 100);
        
        assert_eq!(manager.current_round(), 0);
        assert!(matches!(manager.get_round_state(), &RoundState::Waiting));
    }
}
