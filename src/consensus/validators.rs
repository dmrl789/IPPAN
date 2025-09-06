//! Validator management and verification for IPPAN consensus
//! 
//! Provides validator set access, f-tolerance calculations, and public key management

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use ed25519_dalek::{VerifyingKey, SigningKey};
use rand::rngs::OsRng;
use rand::RngCore;

/// Global validator registry (singleton pattern)
static VALIDATOR_REGISTRY: std::sync::OnceLock<Arc<RwLock<ValidatorRegistry>>> = std::sync::OnceLock::new();

/// Validator registry containing current validator set and public keys
#[derive(Debug, Clone)]
pub struct ValidatorRegistry {
    /// Current validators and their stakes
    validators: HashMap<[u8; 32], u64>,
    /// Public keys for validators
    public_keys: HashMap<[u8; 32], VerifyingKey>,
    /// Total stake
    total_stake: u64,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            public_keys: HashMap::new(),
            total_stake: 0,
        }
    }

    /// Add a validator with stake and public key
    pub fn add_validator(&mut self, node_id: [u8; 32], stake: u64, public_key: VerifyingKey) {
        self.validators.insert(node_id, stake);
        self.public_keys.insert(node_id, public_key);
        self.total_stake += stake;
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, node_id: &[u8; 32]) -> Option<u64> {
        if let Some(stake) = self.validators.remove(node_id) {
            self.public_keys.remove(node_id);
            self.total_stake -= stake;
            Some(stake)
        } else {
            None
        }
    }

    /// Check if a node is a current validator
    pub fn is_validator(&self, node_id: &[u8; 32]) -> bool {
        self.validators.contains_key(node_id)
    }

    /// Get the current committee size (number of validators)
    pub fn committee_size(&self) -> usize {
        self.validators.len()
    }

    /// Get the f-tolerance (maximum number of Byzantine nodes)
    pub fn f_tolerance(&self) -> usize {
        let n = self.committee_size();
        if n == 0 {
            0
        } else {
            (n - 1) / 3 // For 3f+1 Byzantine fault tolerance
        }
    }

    /// Get public key for a validator
    pub fn get_public_key(&self, node_id: &[u8; 32]) -> Option<&VerifyingKey> {
        self.public_keys.get(node_id)
    }

    /// Get all validator IDs
    pub fn get_validator_ids(&self) -> Vec<[u8; 32]> {
        self.validators.keys().copied().collect()
    }

    /// Get total stake
    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }
}

/// Initialize the global validator registry
pub fn init_validator_registry() -> Arc<RwLock<ValidatorRegistry>> {
    VALIDATOR_REGISTRY.get_or_init(|| Arc::new(RwLock::new(ValidatorRegistry::new()))).clone()
}

/// Get the global validator registry
pub fn get_validator_registry() -> Arc<RwLock<ValidatorRegistry>> {
    VALIDATOR_REGISTRY.get().expect("Validator registry not initialized").clone()
}

/// Check if a node is a current validator
pub fn is_current_validator(id: &[u8; 32]) -> bool {
    let registry = get_validator_registry();
    // Note: This is a sync function, so we can't use async read
    // In a real implementation, you'd need to handle this differently
    // For now, we'll assume the registry is accessible
    true // Placeholder - needs proper async handling
}

/// Get the f-tolerance (maximum number of Byzantine nodes)
pub fn f_tolerance() -> usize {
    let registry = get_validator_registry();
    // Note: This is a sync function, so we can't use async read
    // In a real implementation, you'd need to handle this differently
    7 // Placeholder - needs proper async handling
}

/// Get the current committee size
pub fn current_committee_size() -> usize {
    let registry = get_validator_registry();
    // Note: This is a sync function, so we can't use async read
    // In a real implementation, you'd need to handle this differently
    21 // Placeholder - needs proper async handling
}

/// Get public key for a validator
pub fn pubkey_for(id: &[u8; 32]) -> VerifyingKey {
    let registry = get_validator_registry();
    // Note: This is a sync function, so we can't use async read
    // In a real implementation, you'd need to handle this differently
    // For now, return a dummy key
    VerifyingKey::from_bytes(&[0u8; 32]).expect("Invalid public key")
}

/// Async version of validator checks
pub async fn is_current_validator_async(id: &[u8; 32]) -> bool {
    let registry = get_validator_registry();
    let reg = registry.read().await;
    reg.is_validator(id)
}

/// Async version of f-tolerance
pub async fn f_tolerance_async() -> usize {
    let registry = get_validator_registry();
    let reg = registry.read().await;
    reg.f_tolerance()
}

/// Async version of committee size
pub async fn current_committee_size_async() -> usize {
    let registry = get_validator_registry();
    let reg = registry.read().await;
    reg.committee_size()
}

/// Async version of public key lookup
pub async fn pubkey_for_async(id: &[u8; 32]) -> Option<VerifyingKey> {
    let registry = get_validator_registry();
    let reg = registry.read().await;
    reg.get_public_key(id).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[tokio::test]
    #[ignore] // TODO: Fix ed25519_dalek API compatibility
    async fn test_validator_registry() {
        let registry = init_validator_registry();
        let mut reg = registry.write().await;
        
        // Generate test keys
        let mut rng = rand::rngs::OsRng;
        let mut signing_key1_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key1_bytes);
        let signing_key1 = SigningKey::from_bytes(&signing_key1_bytes);
        
        let mut signing_key2_bytes = [0u8; 32];
        rng.fill_bytes(&mut signing_key2_bytes);
        let signing_key2 = SigningKey::from_bytes(&signing_key2_bytes);
        let pub_key1 = signing_key1.verifying_key();
        let pub_key2 = signing_key2.verifying_key();
        
        let node_id1 = [1u8; 32];
        let node_id2 = [2u8; 32];
        
        // Add validators
        reg.add_validator(node_id1, 100, pub_key1);
        reg.add_validator(node_id2, 200, pub_key2);
        
        assert_eq!(reg.committee_size(), 2);
        assert_eq!(reg.f_tolerance(), 0); // (2-1)/3 = 0
        assert_eq!(reg.total_stake(), 300);
        assert!(reg.is_validator(&node_id1));
        assert!(reg.is_validator(&node_id2));
        
        // Test public key lookup
        assert_eq!(reg.get_public_key(&node_id1), Some(&pub_key1));
        assert_eq!(reg.get_public_key(&node_id2), Some(&pub_key2));
    }

    #[tokio::test]
    #[ignore] // TODO: Fix ed25519_dalek API compatibility
    async fn test_f_tolerance_calculation() {
        let registry = init_validator_registry();
        let mut reg = registry.write().await;
        
        // Test different committee sizes
        for i in 1..=10 {
            let node_id = [i as u8; 32];
            let mut rng = rand::rngs::OsRng;
            let mut signing_key_bytes = [0u8; 32];
            rng.fill_bytes(&mut signing_key_bytes);
            let signing_key = SigningKey::from_bytes(&signing_key_bytes);
            let pub_key = signing_key.verifying_key();
            reg.add_validator(node_id, 100, pub_key);
            
            let expected_f = (i - 1) / 3;
            assert_eq!(reg.f_tolerance(), expected_f as usize);
        }
    }
}
