//! Shadow Verifier System for DLC
//!
//! Implements redundant verification by 3-5 shadow verifiers
//! to catch inconsistencies and ensure network integrity.

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use ippan_crypto::{validate_confidential_block, validate_confidential_transaction};
use ippan_types::{Block, BlockId, ValidatorId};

/// Result of shadow verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub verifier_id: ValidatorId,
    pub block_id: BlockId,
    pub is_valid: bool,
    pub verification_time_us: u64,
    pub error: Option<String>,
}

/// Shadow verifier instance
#[derive(Debug)]
pub struct ShadowVerifier {
    pub validator_id: ValidatorId,
    pub verification_count: Arc<RwLock<u64>>,
    pub inconsistency_count: Arc<RwLock<u64>>,
}

impl ShadowVerifier {
    pub fn new(validator_id: ValidatorId) -> Self {
        Self {
            validator_id,
            verification_count: Arc::new(RwLock::new(0)),
            inconsistency_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Verify a block independently
    pub async fn verify_block(&self, block: &Block) -> VerificationResult {
        let start = std::time::Instant::now();
        let block_id = block.hash();

        // Increment verification count
        *self.verification_count.write() += 1;

        // Perform independent verification
        let is_valid = match self.verify_block_internal(block) {
            Ok(valid) => valid,
            Err(e) => {
                return VerificationResult {
                    verifier_id: self.validator_id,
                    block_id,
                    is_valid: false,
                    verification_time_us: start.elapsed().as_micros() as u64,
                    error: Some(e.to_string()),
                };
            }
        };

        VerificationResult {
            verifier_id: self.validator_id,
            block_id,
            is_valid,
            verification_time_us: start.elapsed().as_micros() as u64,
            error: None,
        }
    }

    fn verify_block_internal(&self, block: &Block) -> Result<bool> {
        // Structural validation
        if !block.is_valid() {
            return Ok(false);
        }

        // Validate confidential block
        if let Err(e) = validate_confidential_block(&block.transactions) {
            warn!(
                "Shadow verifier {} found confidential block invalid: {}",
                hex::encode(self.validator_id),
                e
            );
            return Ok(false);
        }

        // Validate each transaction
        for (idx, tx) in block.transactions.iter().enumerate() {
            if !tx.is_valid() {
                warn!(
                    "Shadow verifier {} found transaction #{} invalid",
                    hex::encode(self.validator_id),
                    idx
                );
                return Ok(false);
            }

            if let Err(e) = validate_confidential_transaction(tx) {
                warn!(
                    "Shadow verifier {} found transaction #{} confidential validation failed: {}",
                    hex::encode(self.validator_id),
                    idx,
                    e
                );
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Record an inconsistency detected
    pub fn record_inconsistency(&self) {
        *self.inconsistency_count.write() += 1;
    }

    pub fn get_stats(&self) -> (u64, u64) {
        (
            *self.verification_count.read(),
            *self.inconsistency_count.read(),
        )
    }
}

/// Manages a set of shadow verifiers
pub struct ShadowVerifierSet {
    verifiers: Arc<RwLock<HashMap<ValidatorId, Arc<ShadowVerifier>>>>,
    max_verifiers: usize,
}

impl ShadowVerifierSet {
    pub fn new(max_verifiers: usize) -> Self {
        Self {
            verifiers: Arc::new(RwLock::new(HashMap::new())),
            max_verifiers: max_verifiers.clamp(3, 5),
        }
    }

    /// Add a shadow verifier to the set
    pub fn add_verifier(&self, validator_id: ValidatorId) {
        let mut verifiers = self.verifiers.write();
        if verifiers.len() < self.max_verifiers && !verifiers.contains_key(&validator_id) {
            verifiers.insert(validator_id, Arc::new(ShadowVerifier::new(validator_id)));
            info!("Added shadow verifier: {}", hex::encode(validator_id));
        }
    }

    /// Remove a shadow verifier
    pub fn remove_verifier(&self, validator_id: &ValidatorId) {
        let mut verifiers = self.verifiers.write();
        if verifiers.remove(validator_id).is_some() {
            info!("Removed shadow verifier: {}", hex::encode(validator_id));
        }
    }

    /// Update the active shadow verifier set
    pub fn update_set(&self, new_verifiers: &[ValidatorId]) {
        let mut verifiers = self.verifiers.write();
        verifiers.clear();

        for &validator_id in new_verifiers.iter().take(self.max_verifiers) {
            verifiers.insert(validator_id, Arc::new(ShadowVerifier::new(validator_id)));
        }

        info!("Updated shadow verifier set: {} verifiers", verifiers.len());
    }

    /// Verify a block using all shadow verifiers in parallel
    pub async fn verify_block(
        &mut self,
        block: &Block,
        expected_verifiers: &[ValidatorId],
    ) -> Result<Vec<VerificationResult>> {
        // Update verifier set if needed
        self.update_set(expected_verifiers);

        // Clone verifiers before spawning tasks to avoid holding lock across await
        let verifier_clones: Vec<Arc<ShadowVerifier>> = {
            let verifiers = self.verifiers.read();
            if verifiers.is_empty() {
                return Ok(Vec::new());
            }
            verifiers.values().map(Arc::clone).collect()
        };

        // Spawn parallel verification tasks
        let mut handles: Vec<JoinHandle<VerificationResult>> = Vec::new();

        for verifier in verifier_clones {
            let verifier_clone = Arc::clone(&verifier);
            let block_clone = block.clone();

            let handle =
                tokio::spawn(async move { verifier_clone.verify_block(&block_clone).await });

            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => warn!("Shadow verifier task failed: {}", e),
            }
        }

        // Check for inconsistencies
        if results.len() >= 2 {
            let first_result = results[0].is_valid;
            for result in &results[1..] {
                if result.is_valid != first_result {
                    warn!(
                        "Shadow verifier inconsistency detected! Block: {}, Verifier: {}",
                        hex::encode(result.block_id),
                        hex::encode(result.verifier_id)
                    );

                    // Record inconsistency - reacquire lock
                    let verifiers_lock = self.verifiers.read();
                    if let Some(verifier) = verifiers_lock.get(&result.verifier_id) {
                        verifier.record_inconsistency();
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get statistics for all verifiers
    pub fn get_stats(&self) -> HashMap<ValidatorId, (u64, u64)> {
        let verifiers = self.verifiers.read();
        verifiers
            .iter()
            .map(|(&id, verifier)| (id, verifier.get_stats()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::{Block, Transaction};

    fn create_test_block() -> Block {
        let transactions = vec![];
        Block::new(vec![], transactions, 1, [1u8; 32])
    }

    #[tokio::test]
    async fn test_shadow_verifier_creation() {
        let validator_id = [1u8; 32];
        let verifier = ShadowVerifier::new(validator_id);

        assert_eq!(verifier.validator_id, validator_id);
        let (count, inconsistencies) = verifier.get_stats();
        assert_eq!(count, 0);
        assert_eq!(inconsistencies, 0);
    }

    #[tokio::test]
    async fn test_shadow_verifier_set() {
        let set = ShadowVerifierSet::new(3);

        let validator1 = [1u8; 32];
        let validator2 = [2u8; 32];
        let validator3 = [3u8; 32];

        set.add_verifier(validator1);
        set.add_verifier(validator2);
        set.add_verifier(validator3);

        let stats = set.get_stats();
        assert_eq!(stats.len(), 3);
    }

    #[tokio::test]
    async fn test_parallel_verification() {
        let mut set = ShadowVerifierSet::new(3);
        let validators = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

        let block = create_test_block();
        let results = set.verify_block(&block, &validators).await.unwrap();

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.verification_time_us > 0);
        }
    }
}
