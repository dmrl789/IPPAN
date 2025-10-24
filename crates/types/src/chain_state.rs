//! Chain state management for IPPAN blockchain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Micro-IPN unit (1 IPN = 10^8 micro-IPN)
pub type MicroIPN = u128;

/// Chain state tracking total issuance and other global state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChainState {
    /// Total micro-IPN issued (for supply cap enforcement)
    pub total_issued_micro: MicroIPN,
    /// Current block height
    pub current_height: u64,
    /// Current round ID
    pub current_round: u64,
    /// State root hash
    pub state_root: [u8; 32],
    /// Timestamp of last update
    pub last_updated: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ChainState {
    /// Create a new chain state
    pub fn new() -> Self {
        Self {
            total_issued_micro: 0,
            current_height: 0,
            current_round: 0,
            state_root: [0u8; 32],
            last_updated: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create with initial values
    pub fn with_initial(
        total_issued_micro: MicroIPN,
        current_height: u64,
        current_round: u64,
    ) -> Self {
        Self {
            total_issued_micro,
            current_height,
            current_round,
            state_root: [0u8; 32],
            last_updated: 0,
            metadata: HashMap::new(),
        }
    }

    /// Get total issued supply in micro-IPN
    pub fn total_issued_micro(&self) -> MicroIPN {
        self.total_issued_micro
    }

    /// Add to total issued supply
    pub fn add_issued_micro(&mut self, amount: MicroIPN) {
        self.total_issued_micro = self.total_issued_micro.saturating_add(amount);
    }

    /// Set total issued supply (for initialization)
    pub fn set_issued_micro(&mut self, amount: MicroIPN) {
        self.total_issued_micro = amount;
    }

    /// Get current block height
    pub fn current_height(&self) -> u64 {
        self.current_height
    }

    /// Increment block height
    pub fn increment_height(&mut self) {
        self.current_height = self.current_height.saturating_add(1);
    }

    /// Set block height
    pub fn set_height(&mut self, height: u64) {
        self.current_height = height;
    }

    /// Get current round
    pub fn current_round(&self) -> u64 {
        self.current_round
    }

    /// Set current round
    pub fn set_round(&mut self, round: u64) {
        self.current_round = round;
    }

    /// Get state root
    pub fn state_root(&self) -> [u8; 32] {
        self.state_root
    }

    /// Set state root
    pub fn set_state_root(&mut self, root: [u8; 32]) {
        self.state_root = root;
    }

    /// Get last updated timestamp
    pub fn last_updated(&self) -> u64 {
        self.last_updated
    }

    /// Set last updated timestamp
    pub fn set_last_updated(&mut self, timestamp: u64) {
        self.last_updated = timestamp;
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Set metadata value
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Remove metadata value
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    /// Get all metadata
    pub fn get_all_metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Update state after a round finalization
    pub fn update_after_round(
        &mut self,
        round: u64,
        emission_micro: MicroIPN,
        state_root: [u8; 32],
        timestamp: u64,
    ) {
        self.set_round(round);
        self.add_issued_micro(emission_micro);
        self.set_state_root(state_root);
        self.set_last_updated(timestamp);
    }

    /// Check if supply cap would be exceeded
    pub fn would_exceed_cap(&self, additional_micro: MicroIPN, cap_micro: MicroIPN) -> bool {
        self.total_issued_micro.saturating_add(additional_micro) > cap_micro
    }

    /// Get remaining supply cap
    pub fn remaining_cap(&self, max_supply_micro: MicroIPN) -> MicroIPN {
        max_supply_micro.saturating_sub(self.total_issued_micro)
    }

    /// Clone with updated values
    pub fn with_updates(
        &self,
        total_issued_micro: Option<MicroIPN>,
        current_height: Option<u64>,
        current_round: Option<u64>,
        state_root: Option<[u8; 32]>,
        last_updated: Option<u64>,
    ) -> Self {
        Self {
            total_issued_micro: total_issued_micro.unwrap_or(self.total_issued_micro),
            current_height: current_height.unwrap_or(self.current_height),
            current_round: current_round.unwrap_or(self.current_round),
            state_root: state_root.unwrap_or(self.state_root),
            last_updated: last_updated.unwrap_or(self.last_updated),
            metadata: self.metadata.clone(),
        }
    }
}

/// Chain state manager for persistent state operations
pub trait ChainStateManager {
    /// Load chain state from storage
    fn load_state(&self) -> Result<ChainState, Box<dyn std::error::Error>>;
    
    /// Save chain state to storage
    fn save_state(&mut self, state: &ChainState) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Update state atomically
    fn update_state<F>(&mut self, updater: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut ChainState) -> Result<(), Box<dyn std::error::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_state_creation() {
        let state = ChainState::new();
        assert_eq!(state.total_issued_micro(), 0);
        assert_eq!(state.current_height(), 0);
        assert_eq!(state.current_round(), 0);
    }

    #[test]
    fn test_chain_state_with_initial() {
        let state = ChainState::with_initial(1000, 5, 10);
        assert_eq!(state.total_issued_micro(), 1000);
        assert_eq!(state.current_height(), 5);
        assert_eq!(state.current_round(), 10);
    }

    #[test]
    fn test_add_issued_micro() {
        let mut state = ChainState::new();
        state.add_issued_micro(1000);
        assert_eq!(state.total_issued_micro(), 1000);
        
        state.add_issued_micro(500);
        assert_eq!(state.total_issued_micro(), 1500);
    }

    #[test]
    fn test_height_operations() {
        let mut state = ChainState::new();
        assert_eq!(state.current_height(), 0);
        
        state.increment_height();
        assert_eq!(state.current_height(), 1);
        
        state.set_height(100);
        assert_eq!(state.current_height(), 100);
    }

    #[test]
    fn test_round_operations() {
        let mut state = ChainState::new();
        assert_eq!(state.current_round(), 0);
        
        state.set_round(50);
        assert_eq!(state.current_round(), 50);
    }

    #[test]
    fn test_metadata_operations() {
        let mut state = ChainState::new();
        
        state.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(state.get_metadata("key1"), Some(&"value1".to_string()));
        
        state.set_metadata("key2".to_string(), "value2".to_string());
        assert_eq!(state.get_metadata("key2"), Some(&"value2".to_string()));
        
        let removed = state.remove_metadata("key1");
        assert_eq!(removed, Some("value1".to_string()));
        assert_eq!(state.get_metadata("key1"), None);
    }

    #[test]
    fn test_update_after_round() {
        let mut state = ChainState::new();
        let state_root = [1u8; 32];
        
        state.update_after_round(10, 1000, state_root, 1234567890);
        
        assert_eq!(state.current_round(), 10);
        assert_eq!(state.total_issued_micro(), 1000);
        assert_eq!(state.state_root(), state_root);
        assert_eq!(state.last_updated(), 1234567890);
    }

    #[test]
    fn test_supply_cap_checks() {
        let state = ChainState::with_initial(1000, 0, 0);
        
        assert!(!state.would_exceed_cap(500, 2000));
        assert!(state.would_exceed_cap(1000, 2000));
        assert_eq!(state.remaining_cap(2000), 1000);
    }

    #[test]
    fn test_with_updates() {
        let original = ChainState::with_initial(1000, 5, 10);
        let new_state_root = [2u8; 32];
        
        let updated = original.with_updates(
            Some(2000),
            Some(15),
            None, // Keep current round
            Some(new_state_root),
            Some(1234567890),
        );
        
        assert_eq!(updated.total_issued_micro(), 2000);
        assert_eq!(updated.current_height(), 15);
        assert_eq!(updated.current_round(), 10); // Unchanged
        assert_eq!(updated.state_root(), new_state_root);
        assert_eq!(updated.last_updated(), 1234567890);
    }
}