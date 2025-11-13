//! Verifier set selection and block validation for DLC consensus
//!
//! This module handles selecting verifiers for each consensus round
//! and validating blocks produced by the primary verifier.

use crate::dag::Block;
use crate::dgbdt::{FairnessModel, ValidatorMetrics};
use crate::error::{DlcError, Result};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

/// A set of verifiers for a consensus round
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifierSet {
    /// Primary block proposer
    pub primary: String,
    /// Shadow verifiers (backups and validators)
    pub shadows: Vec<String>,
    /// Round number
    pub round: u64,
    /// Random seed used for selection
    pub seed: String,
}

impl VerifierSet {
    /// Select verifiers for a consensus round using fairness model
    ///
    /// `max_set_size` caps the number of validators chosen (including the primary).
    pub fn select(
        model: &FairnessModel,
        validators: &HashMap<String, ValidatorMetrics>,
        seed: impl Into<String>,
        round: u64,
        max_set_size: usize,
    ) -> Result<Self> {
        if validators.is_empty() {
            return Err(DlcError::InvalidVerifierSet(
                "No validators available".to_string(),
            ));
        }

        let seed_string = seed.into();

        // Score all validators deterministically
        let mut scored: Vec<(String, i64)> = validators
            .iter()
            .map(|(id, metrics)| (id.clone(), model.score_deterministic(metrics)))
            .collect();

        scored.sort_by(
            |a, b| match b.1.cmp(&a.1) {
                Ordering::Equal => Self::compare_with_entropy(&seed_string, round, &a.0, &b.0),
                other => other,
            },
        );

        let selection_count = max_set_size.max(1).min(scored.len());

        let primary = scored
            .first()
            .map(|(id, _)| id.clone())
            .ok_or_else(|| DlcError::InvalidVerifierSet("No primary validator".to_string()))?;

        let rest: Vec<String> = scored.iter().skip(1).map(|(id, _)| id.clone()).collect();

        let mut shadows = Vec::new();
        if selection_count > 1 && !rest.is_empty() {
            let rotation_offset = Self::selection_offset(&seed_string, round, rest.len());
            for idx in 0..(selection_count - 1) {
                let candidate_index = (rotation_offset + idx) % rest.len();
                let candidate = rest[candidate_index].clone();
                if !shadows.contains(&candidate) {
                    shadows.push(candidate);
                }
            }
        }

        Ok(Self {
            primary,
            shadows,
            round,
            seed: seed_string,
        })
    }

    fn compare_with_entropy(seed: &str, round: u64, a: &str, b: &str) -> Ordering {
        let entropy_a = Self::id_entropy(seed, round, a);
        let entropy_b = Self::id_entropy(seed, round, b);

        entropy_b.cmp(&entropy_a).then_with(|| a.cmp(b))
    }

    fn selection_offset(seed: &str, round: u64, len: usize) -> usize {
        if len == 0 {
            return 0;
        }

        (Self::id_entropy(seed, round, "rotation") % len as u64) as usize
    }

    fn id_entropy(seed: &str, round: u64, id: &str) -> u64 {
        let mut hasher = Hasher::new();
        hasher.update(seed.as_bytes());
        hasher.update(&round.to_le_bytes());
        hasher.update(id.as_bytes());
        let hash = hasher.finalize();

        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&hash.as_bytes()[..8]);
        u64::from_le_bytes(bytes)
    }

    /// Collect and filter blocks from pool for this verifier set
    pub fn collect_blocks(&self, pool: Vec<Block>) -> Vec<Block> {
        // Filter blocks that should be processed in this round
        pool.into_iter()
            .filter(|block| {
                // Check if block is from current or previous round
                block.timestamp.round <= self.round
                    // Check if proposer is in verifier set
                    && (block.proposer == self.primary || self.shadows.contains(&block.proposer))
            })
            .collect()
    }

    /// Validate a block
    pub fn validate(&self, block: &Block) -> Result<()> {
        // 1. Verify block structure
        if block.id.is_empty() {
            return Err(DlcError::BlockValidation("Block ID is empty".to_string()));
        }

        // 2. Verify proposer is in verifier set
        if block.proposer != self.primary && !self.shadows.contains(&block.proposer) {
            return Err(DlcError::BlockValidation(format!(
                "Block proposer {} not in verifier set",
                block.proposer
            )));
        }

        // 3. Verify block round
        if block.timestamp.round > self.round {
            return Err(DlcError::BlockValidation(format!(
                "Block round {} exceeds current round {}",
                block.timestamp.round, self.round
            )));
        }

        // 4. Verify block signature
        if !block.verify_signature() && !block.is_genesis() {
            return Err(DlcError::BlockValidation(
                "Block signature verification failed".to_string(),
            ));
        }

        // 5. Verify merkle root
        self.verify_merkle_root(block)?;

        Ok(())
    }

    /// Verify block's merkle root matches its data
    fn verify_merkle_root(&self, block: &Block) -> Result<()> {
        let mut hasher = Hasher::new();
        hasher.update(&block.data);
        let computed_root = hasher.finalize().to_hex().to_string();

        if computed_root != block.merkle_root {
            return Err(DlcError::BlockValidation(
                "Merkle root mismatch".to_string(),
            ));
        }

        Ok(())
    }

    /// Get all verifiers (primary + shadows)
    pub fn all_verifiers(&self) -> Vec<String> {
        let mut all = vec![self.primary.clone()];
        all.extend(self.shadows.clone());
        all
    }

    /// Check if a validator is in this verifier set
    pub fn contains(&self, validator_id: &str) -> bool {
        self.primary == validator_id || self.shadows.iter().any(|id| id == validator_id)
    }

    /// Get verifier set size
    pub fn size(&self) -> usize {
        1 + self.shadows.len()
    }
}

/// A block that has been verified by the verifier set
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiedBlock {
    /// The verified block
    pub block: Block,
    /// Verifiers who validated this block
    pub verified_by: Vec<String>,
    /// Timestamp of verification
    pub verified_at: chrono::DateTime<chrono::Utc>,
}

impl VerifiedBlock {
    /// Create a new verified block
    pub fn new(block: Block, verified_by: Vec<String>) -> Self {
        Self {
            block,
            verified_by,
            verified_at: chrono::Utc::now(),
        }
    }

    /// Check if block has been verified by required number of validators
    pub fn has_quorum(&self, required: usize) -> bool {
        self.verified_by.len() >= required
    }

    /// Check if a specific validator verified this block
    pub fn verified_by_validator(&self, validator_id: &str) -> bool {
        self.verified_by.contains(&validator_id.to_string())
    }
}

/// Validator set manager
#[derive(Debug)]
pub struct ValidatorSetManager {
    /// All registered validators
    validators: HashMap<String, ValidatorMetrics>,
    /// Current verifier set
    current_verifiers: Option<VerifierSet>,
    /// Fairness model for selection
    model: FairnessModel,
    /// Maximum number of verifiers to select each round
    max_set_size: usize,
}

impl ValidatorSetManager {
    /// Create a new validator set manager
    pub fn new(model: FairnessModel, max_set_size: usize) -> Self {
        Self {
            validators: HashMap::new(),
            current_verifiers: None,
            model,
            max_set_size: max_set_size.max(1),
        }
    }

    /// Register a new validator
    pub fn register_validator(
        &mut self,
        validator_id: String,
        metrics: ValidatorMetrics,
    ) -> Result<()> {
        if self.validators.contains_key(&validator_id) {
            return Err(DlcError::InvalidVerifierSet(format!(
                "Validator {} already registered",
                validator_id
            )));
        }

        self.validators.insert(validator_id, metrics);
        Ok(())
    }

    /// Update validator metrics
    pub fn update_validator(
        &mut self,
        validator_id: &str,
        metrics: ValidatorMetrics,
    ) -> Result<()> {
        self.validators
            .insert(validator_id.to_string(), metrics)
            .ok_or_else(|| {
                DlcError::ValidatorNotFound(format!("Validator {} not found", validator_id))
            })?;
        Ok(())
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, validator_id: &str) -> Result<()> {
        self.validators.remove(validator_id).ok_or_else(|| {
            DlcError::ValidatorNotFound(format!("Validator {} not found", validator_id))
        })?;
        Ok(())
    }

    /// Get validator metrics
    pub fn get_validator(&self, validator_id: &str) -> Option<&ValidatorMetrics> {
        self.validators.get(validator_id)
    }

    /// Select verifiers for a new round
    pub fn select_for_round(&mut self, seed: String, round: u64) -> Result<&VerifierSet> {
        let verifier_set = VerifierSet::select(
            &self.model,
            &self.validators,
            seed,
            round,
            self.max_set_size,
        )?;
        self.current_verifiers = Some(verifier_set);

        Ok(self.current_verifiers.as_ref().unwrap())
    }

    /// Get current verifier set
    pub fn current_verifiers(&self) -> Option<&VerifierSet> {
        self.current_verifiers.as_ref()
    }

    /// Get all validators
    pub fn all_validators(&self) -> &HashMap<String, ValidatorMetrics> {
        &self.validators
    }

    /// Get validator count
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Update fairness model
    pub fn update_model(&mut self, model: FairnessModel) {
        self.model = model;
    }

    /// Update maximum verifier set size
    pub fn set_max_set_size(&mut self, max_set_size: usize) {
        self.max_set_size = max_set_size.max(1);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use crate::hashtimer::HashTimer;
    use ippan_types::Amount;

    fn create_test_validators() -> HashMap<String, ValidatorMetrics> {
        let mut validators = HashMap::new();

        validators.insert(
            "val1".to_string(),
            ValidatorMetrics::from_floats(
                0.99,
                0.05,
                1.0,
                100,
                500,
                Amount::from_micro_ipn(10_000_000),
                100,
            ),
        );
        validators.insert(
            "val2".to_string(),
            ValidatorMetrics::from_floats(
                0.95,
                0.15,
                0.98,
                80,
                400,
                Amount::from_micro_ipn(5_000_000),
                100,
            ),
        );
        validators.insert(
            "val3".to_string(),
            ValidatorMetrics::from_floats(
                0.97,
                0.10,
                0.99,
                90,
                450,
                Amount::from_micro_ipn(8_000_000),
                100,
            ),
        );

        validators
    }

    #[test]
    fn test_verifier_set_selection() {
        let model = FairnessModel::new_production();
        let validators = create_test_validators();

        let verifier_set =
            VerifierSet::select(&model, &validators, "test_seed", 1, validators.len()).unwrap();

        assert!(!verifier_set.primary.is_empty());
        assert!(verifier_set.size() <= 3);
    }

    #[test]
    fn test_deterministic_selection() {
        let model = FairnessModel::new_production();
        let validators = create_test_validators();

        let set1 =
            VerifierSet::select(&model, &validators, "seed123", 1, validators.len()).unwrap();
        let set2 =
            VerifierSet::select(&model, &validators, "seed123", 1, validators.len()).unwrap();

        // Same seed and round should produce same selection
        assert_eq!(set1.primary, set2.primary);
        assert_eq!(set1.shadows, set2.shadows);
    }

    #[test]
    fn test_block_validation() {
        let model = FairnessModel::new_production();
        let validators = create_test_validators();
        let verifier_set =
            VerifierSet::select(&model, &validators, "test", 1, validators.len()).unwrap();

        let mut block = Block::new(
            vec![],
            HashTimer::for_round(1),
            vec![1, 2, 3],
            verifier_set.primary.clone(),
        );
        block.sign(vec![0u8; 64]);

        assert!(verifier_set.validate(&block).is_ok());
    }

    #[test]
    fn test_invalid_proposer() {
        let model = FairnessModel::new_production();
        let validators = create_test_validators();
        let verifier_set =
            VerifierSet::select(&model, &validators, "test", 1, validators.len()).unwrap();

        let mut block = Block::new(
            vec![],
            HashTimer::for_round(1),
            vec![1, 2, 3],
            "invalid_proposer".to_string(),
        );
        block.sign(vec![0u8; 64]);

        assert!(verifier_set.validate(&block).is_err());
    }

    #[test]
    fn test_verified_block() {
        let block = Block::new(vec![], HashTimer::now(), vec![], "test".to_string());

        let verified = VerifiedBlock::new(block, vec!["val1".to_string(), "val2".to_string()]);

        assert!(verified.has_quorum(2));
        assert!(verified.verified_by_validator("val1"));
    }

    #[test]
    fn test_validator_set_manager() {
        let model = FairnessModel::new_production();
        let mut manager = ValidatorSetManager::new(model, 3);

        let metrics = ValidatorMetrics::default();
        assert!(manager
            .register_validator("val1".to_string(), metrics)
            .is_ok());
        assert_eq!(manager.validator_count(), 1);
    }

    #[test]
    fn test_verifier_set_contains() {
        let model = FairnessModel::new_production();
        let validators = create_test_validators();
        let verifier_set =
            VerifierSet::select(&model, &validators, "test", 1, validators.len()).unwrap();

        assert!(verifier_set.contains(&verifier_set.primary));
    }
}
