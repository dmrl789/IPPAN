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

        // Score validators deterministically using the fairness model wrapper
        let mut scored: Vec<(String, i64)> = validators
            .iter()
            .map(|(id, metrics)| (id.clone(), model.score_deterministic(metrics)))
            .collect();

        // Sort by score (descending), then by validator ID for deterministic tie-breaking
        scored.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)) // Stable tie-break: validator ID
        });

        let selection_count = max_set_size.max(1).min(scored.len());

        let primary = Self::select_primary(&scored, &seed_string, round)?;
        let primary_index = scored
            .iter()
            .position(|(id, _)| id == &primary)
            .ok_or_else(|| DlcError::InvalidVerifierSet("No primary validator".to_string()))?;

        // Select shadow verifiers deterministically by score (highest first)
        // Shadows are selected from the remaining validators (excluding primary)
        // in descending score order, with deterministic tie-breaking by validator ID
        let mut shadows = Vec::new();
        if selection_count > 1 {
            for (idx, (id, _)) in scored.iter().enumerate() {
                if idx != primary_index && shadows.len() < (selection_count - 1) {
                    shadows.push(id.clone());
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

    #[allow(dead_code)]
    fn compare_with_entropy(seed: &str, round: u64, a: &str, b: &str) -> Ordering {
        let entropy_a = Self::id_entropy(seed, round, a);
        let entropy_b = Self::id_entropy(seed, round, b);

        entropy_b.cmp(&entropy_a).then_with(|| a.cmp(b))
    }

    #[allow(dead_code)]
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

    fn select_primary(scored: &[(String, i64)], _seed: &str, round: u64) -> Result<String> {
        if scored.is_empty() {
            return Err(DlcError::InvalidVerifierSet(
                "No validators available".to_string(),
            ));
        }

        let rotation = Self::build_primary_rotation(scored);
        if rotation.is_empty() {
            return Ok(scored[0].0.clone());
        }

        let rotation_len = rotation.len();
        let index = ((round as usize).saturating_sub(1)) % rotation_len;
        Ok(rotation[index].clone())
    }

    fn build_primary_rotation(scored: &[(String, i64)]) -> Vec<String> {
        if scored.is_empty() {
            return Vec::new();
        }

        let mut slots = scored.len().saturating_mul(8);
        if slots == 0 {
            slots = 1;
        }
        slots = slots.max(scored.len()).min(4_096);

        let weights: Vec<i64> = scored.iter().map(|(_, score)| (*score).max(0)).collect();
        let total_weight: i64 = weights.iter().sum();
        if total_weight <= 0 {
            // Degenerate case: fall back to deterministic score ordering
            return scored.iter().map(|(id, _)| id.clone()).collect();
        }

        let mut rotation = Vec::with_capacity(slots);
        let mut current: Vec<i128> = vec![0; weights.len()];
        let total_weight_i128 = i128::from(total_weight);

        for _ in 0..slots {
            // Smooth weighted round robin: advance each validator's accumulator
            for (idx, value) in current.iter_mut().enumerate() {
                *value += i128::from(weights[idx]);
            }

            // Pick the validator with the highest accumulator (tie => lexicographic ID)
            let mut best_idx = 0;
            for idx in 1..current.len() {
                let cmp = current[idx].cmp(&current[best_idx]);
                if cmp == Ordering::Greater
                    || (cmp == Ordering::Equal
                        && scored[idx].0.as_str() < scored[best_idx].0.as_str())
                {
                    best_idx = idx;
                }
            }

            rotation.push(scored[best_idx].0.clone());

            // Consume total_weight to maintain smoothness
            current[best_idx] -= total_weight_i128;
        }

        rotation
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
                "Validator {validator_id} already registered"
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
                DlcError::ValidatorNotFound(format!("Validator {validator_id} not found"))
            })?;
        Ok(())
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, validator_id: &str) -> Result<()> {
        self.validators.remove(validator_id).ok_or_else(|| {
            DlcError::ValidatorNotFound(format!("Validator {validator_id} not found"))
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

    /// Get the fairness model (for reward weighting)
    pub fn model(&self) -> &FairnessModel {
        &self.model
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
            ValidatorMetrics::new(
                9900,  // 99% uptime (scaled by 10000)
                500,   // 5% latency (scaled by 10000)
                10000, // 100% honesty (scaled by 10000)
                100,
                500,
                Amount::from_micro_ipn(10_000_000),
                100,
            ),
        );
        validators.insert(
            "val2".to_string(),
            ValidatorMetrics::new(
                9500, // 95% uptime (scaled by 10000)
                1500, // 15% latency (scaled by 10000)
                9800, // 98% honesty (scaled by 10000)
                80,
                400,
                Amount::from_micro_ipn(5_000_000),
                100,
            ),
        );
        validators.insert(
            "val3".to_string(),
            ValidatorMetrics::new(
                9700, // 97% uptime (scaled by 10000)
                1000, // 10% latency (scaled by 10000)
                9900, // 99% honesty (scaled by 10000)
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
        let model = FairnessModel::testing_stub();
        let validators = create_test_validators();

        let verifier_set =
            VerifierSet::select(&model, &validators, "test_seed", 1, validators.len()).unwrap();

        assert!(!verifier_set.primary.is_empty());
        assert!(verifier_set.size() <= 3);
    }

    #[test]
    fn test_deterministic_selection() {
        let model = FairnessModel::testing_stub();
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
        let model = FairnessModel::testing_stub();
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
        let model = FairnessModel::testing_stub();
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
        let model = FairnessModel::testing_stub();
        let mut manager = ValidatorSetManager::new(model, 3);

        let metrics = ValidatorMetrics::default();
        assert!(manager
            .register_validator("val1".to_string(), metrics)
            .is_ok());
        assert_eq!(manager.validator_count(), 1);
    }

    #[test]
    fn test_verifier_set_contains() {
        let model = FairnessModel::testing_stub();
        let validators = create_test_validators();
        let verifier_set =
            VerifierSet::select(&model, &validators, "test", 1, validators.len()).unwrap();

        assert!(verifier_set.contains(&verifier_set.primary));
    }

    #[test]
    fn shadow_verifier_selection_is_deterministic_given_same_inputs() {
        let model = FairnessModel::testing_stub();
        let mut validators = HashMap::new();

        // Create at least 10 validators with varying metrics
        for i in 0..10 {
            let id = format!("val{i}");
            let metrics = ValidatorMetrics::new(
                9000 + (i as i64 * 100), // Varying uptime (90-99%)
                500 + (i as i64 * 50),   // Varying latency
                9500 + (i as i64 * 50),  // Varying honesty
                100 + (i as u64 * 10),   // Varying blocks proposed
                500 + (i as u64 * 20),   // Varying blocks verified
                Amount::from_micro_ipn(1_000_000 + (i as u64 * 100_000)),
                100 + (i as u64 * 5), // Varying rounds active
            );
            validators.insert(id, metrics);
        }

        // Call selection twice for the same round
        let set1 =
            VerifierSet::select(&model, &validators, "deterministic_test_seed", 42, 5).unwrap();
        let set2 =
            VerifierSet::select(&model, &validators, "deterministic_test_seed", 42, 5).unwrap();

        // Assert the resulting lists are identical (same order)
        assert_eq!(
            set1.primary, set2.primary,
            "Primary should be deterministic"
        );
        assert_eq!(
            set1.shadows, set2.shadows,
            "Shadow verifiers should be deterministic and in same order"
        );
        assert_eq!(
            set1.shadows.len(),
            set2.shadows.len(),
            "Shadow count should match"
        );

        // Verify that validators with better metrics rank above worse metrics
        // (This is a sanity check - the model should prefer higher uptime/honesty)
        let mut validator_scores: Vec<(String, i64)> = validators
            .iter()
            .map(|(id, metrics)| (id.clone(), model.score_deterministic(metrics)))
            .collect();

        validator_scores.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        // The primary should be one of the top scorers (allowing for rotation logic)
        // Primary selection uses score-based ranking with rotation, so it should be
        // among the top candidates, not necessarily the absolute top
        let primary_score = validator_scores
            .iter()
            .find(|(id, _)| id == &set1.primary)
            .map(|(_, score)| *score)
            .expect("Primary should be in validator list");

        // Primary should have a positive score
        assert!(primary_score > 0, "Primary should have a positive score");

        // Shadows should also be high scorers (excluding primary)
        let shadow_scores: Vec<i64> = set1
            .shadows
            .iter()
            .filter_map(|id| {
                validator_scores
                    .iter()
                    .find(|(vid, _)| vid == id)
                    .map(|(_, score)| *score)
            })
            .collect();

        // All shadows should have scores >= the lowest score among non-selected validators
        if !shadow_scores.is_empty() && validator_scores.len() > set1.size() {
            let min_shadow_score = *shadow_scores.iter().min().unwrap();
            let non_selected: Vec<i64> = validator_scores
                .iter()
                .skip(set1.size())
                .map(|(_, score)| *score)
                .collect();
            if !non_selected.is_empty() {
                let max_non_selected = *non_selected.iter().max().unwrap();
                assert!(
                    min_shadow_score >= max_non_selected,
                    "Shadow verifiers should have scores >= non-selected validators"
                );
            }
        }
    }

    #[test]
    fn primary_rotation_honors_score_ordering() {
        let mut scored = vec![
            ("validator-high-a".to_string(), 9_800),
            ("validator-high-b".to_string(), 9_800),
            ("validator-medium".to_string(), 5_200),
            ("validator-low".to_string(), 3_300),
            ("validator-adversarial".to_string(), 3_300),
        ];

        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let rotation = VerifierSet::build_primary_rotation(&scored);
        assert!(rotation.len() >= scored.len());

        let count = |id: &str| rotation.iter().filter(|candidate| candidate == &id).count();

        let high_a = count("validator-high-a");
        let high_b = count("validator-high-b");
        let medium = count("validator-medium");
        let low = count("validator-low");
        let adversarial = count("validator-adversarial");

        assert!(
            high_a >= medium && high_b >= medium,
            "high contributors should have at least as many primary slots as medium"
        );
        assert!(
            medium >= low && medium >= adversarial,
            "medium contributors should outrank low/adversarial slots"
        );
    }
}
