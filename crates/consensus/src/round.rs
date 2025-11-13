use anyhow::Result;
use blake3::Hasher as Blake3;
use std::collections::HashMap;

#[cfg(feature = "ai_l1")]
pub use ippan_ai_core::features::ValidatorTelemetry;
#[cfg(feature = "ai_l1")]
use ippan_ai_core::{compute_validator_score, gbdt::GBDTModel};

#[cfg(not(feature = "ai_l1"))]
use serde::{Deserialize, Serialize};

// NOTE: This fallback struct is ONLY compiled when feature "ai_l1" is disabled.
// In production, ai_l1 feature is enabled and the integer version from ai_core is used.
#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Fallback only - production uses ai_core::ValidatorTelemetry with integers")]
pub struct ValidatorTelemetry {
    pub validator_id: [u8; 32],
    pub block_production_rate_scaled: i64, // Scaled by 10000
    pub avg_block_size: u64,
    pub uptime_scaled: i64, // Scaled by 10000
    pub network_latency_scaled: i64, // Scaled by 10000  
    pub validation_accuracy_scaled: i64, // Scaled by 10000
    pub stake: u64,
    pub slashing_events: u32,
    pub last_activity: u64,
    pub custom_metrics: HashMap<String, i64>, // All scaled by 10000
}

#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone)]
pub struct GBDTModel {}

#[cfg(not(feature = "ai_l1"))]
pub mod features {
    use super::ValidatorTelemetry;
    use anyhow::Result;

    #[allow(deprecated)]
    pub fn from_telemetry(telemetry: &ValidatorTelemetry) -> Result<Vec<i64>> {
        Ok(vec![
            telemetry.block_production_rate_scaled,
            telemetry.avg_block_size as i64,
            telemetry.uptime_scaled,
            telemetry.network_latency_scaled,
            telemetry.validation_accuracy_scaled,
            telemetry.stake as i64,
            telemetry.slashing_events as i64,
            telemetry.last_activity as i64,
        ])
    }
}

// -----------------------------------------------------------------------------
// ✅ RoundConsensus implementation
// -----------------------------------------------------------------------------

/// Round-based consensus with AI reputation scoring
pub struct RoundConsensus {
    current_round: u64,
    active_model: Option<GBDTModel>,
    validator_telemetry: HashMap<[u8; 32], ValidatorTelemetry>,
    reputation_scores: HashMap<[u8; 32], i32>,
}

/// Validator selection result
#[derive(Debug, Clone)]
pub struct ValidatorSelection {
    pub proposer: [u8; 32],
    pub verifiers: Vec<[u8; 32]>,
    pub reputation_scores: HashMap<[u8; 32], i32>,
    pub selection_weights: HashMap<[u8; 32], i64>, // Scaled by 10000
}

impl RoundConsensus {
    pub fn new() -> Self {
        Self {
            current_round: 0,
            active_model: None,
            validator_telemetry: HashMap::new(),
            reputation_scores: HashMap::new(),
        }
    }

    pub fn set_active_model(&mut self, model: GBDTModel) -> Result<()> {
        self.active_model = Some(model);
        self.reputation_scores.clear();
        Ok(())
    }

    pub fn update_telemetry(&mut self, validator_id: [u8; 32], telemetry: ValidatorTelemetry) {
        self.validator_telemetry.insert(validator_id, telemetry);
        self.reputation_scores.remove(&validator_id);
    }

    pub fn calculate_reputation_score(&self, validator_id: &[u8; 32]) -> Result<i32> {
        if let Some(score) = self.reputation_scores.get(validator_id) {
            return Ok(*score);
        }

        let telemetry = match self.validator_telemetry.get(validator_id) {
            Some(t) => t,
            None => return Ok(5000),
        };

        #[cfg(feature = "ai_l1")]
        {
            let model = match self.active_model.as_ref() {
                Some(m) => m,
                None => return Ok(5000),
            };
            let score = compute_validator_score(telemetry, model);
            Ok(score)
        }

        #[cfg(not(feature = "ai_l1"))]
        {
            Ok(5000)
        }
    }

    pub fn select_validators(
        &mut self,
        validators: &[[u8; 32]],
        stake_weights: &HashMap<[u8; 32], u64>,
    ) -> Result<ValidatorSelection> {
        if validators.is_empty() {
            return Err(anyhow::anyhow!("No validators available"));
        }

        let mut reputation_scores = HashMap::new();
        let mut selection_weights = HashMap::new();

        for validator in validators {
            let reputation = self.calculate_reputation_score(validator).unwrap_or(5000);
            reputation_scores.insert(*validator, reputation);

            let stake_weight = stake_weights.get(validator).copied().unwrap_or(0) as i64;
            let reputation_weight = reputation as i64;
            // Combined weight: 70% stake, 30% reputation (scaled integer math)
            let combined_weight = (stake_weight * 7000) / 10000 + (reputation_weight * 3000) / 10000;
            selection_weights.insert(*validator, combined_weight);
        }

        let proposer = self.deterministic_weighted_selection(validators, &selection_weights, 0)?;
        let verifier_candidates: Vec<[u8; 32]> = validators
            .iter()
            .filter(|&&v| v != proposer)
            .copied()
            .collect();

        let verifier_weights: HashMap<[u8; 32], i64> = verifier_candidates
            .iter()
            .filter_map(|v| selection_weights.get(v).map(|&w| (*v, w)))
            .collect();

        let verifiers =
            self.select_multiple_weighted(&verifier_candidates, &verifier_weights, 3)?;

        Ok(ValidatorSelection {
            proposer,
            verifiers,
            reputation_scores,
            selection_weights,
        })
    }

    fn deterministic_weighted_selection(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], i64>,
        salt: u64,
    ) -> Result<[u8; 32]> {
        if candidates.is_empty() {
            return Err(anyhow::anyhow!("No candidates available"));
        }

        let mut ordered: Vec<[u8; 32]> = candidates.to_vec();
        ordered.sort();

        let total_weight: i64 = ordered
            .iter()
            .map(|id| *weights.get(id).unwrap_or(&0))
            .filter(|w| *w > 0)
            .sum();
        if total_weight <= 0 {
            return Err(anyhow::anyhow!("Total weight must be positive"));
        }

        let mut hasher = Blake3::new();
        hasher.update(b"ROUND_CONSENSUS_SELECTION");
        hasher.update(&self.current_round.to_be_bytes());
        hasher.update(&salt.to_be_bytes());
        for id in &ordered {
            hasher.update(id);
            hasher.update(&weights.get(id).unwrap_or(&0).to_be_bytes());
        }
        let selection_hash = hasher.finalize();

        let mut selection_bytes = [0u8; 8];
        selection_bytes.copy_from_slice(&selection_hash.as_bytes()[..8]);
        let mut target = (u64::from_be_bytes(selection_bytes) as i64) % total_weight;

        for candidate in ordered.iter() {
            let weight = *weights.get(candidate).unwrap_or(&0);
            if weight <= 0 {
                continue;
            }
            if target < weight {
                return Ok(*candidate);
            }
            target -= weight;
        }

        ordered
            .into_iter()
            .find(|id| *weights.get(id).unwrap_or(&0) > 0)
            .ok_or_else(|| anyhow::anyhow!("No candidates with positive weight"))
    }

    fn select_multiple_weighted(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], i64>,
        count: usize,
    ) -> Result<Vec<[u8; 32]>> {
        let mut selected = Vec::new();
        let mut remaining_candidates = candidates.to_vec();
        let mut remaining_weights = weights.clone();

        for idx in 0..count.min(candidates.len()) {
            if remaining_candidates.is_empty() {
                break;
            }

            let selected_item = self.deterministic_weighted_selection(
                &remaining_candidates,
                &remaining_weights,
                (idx as u64) + 1,
            )?;
            selected.push(selected_item);
            remaining_candidates.retain(|&x| x != selected_item);
            remaining_weights.remove(&selected_item);
        }

        Ok(selected)
    }

    pub fn current_round(&self) -> u64 {
        self.current_round
    }

    pub fn advance_round(&mut self) {
        self.current_round += 1;
    }

    pub fn get_reputation_scores(&self) -> &HashMap<[u8; 32], i32> {
        &self.reputation_scores
    }

    pub fn get_validator_telemetry(&self) -> &HashMap<[u8; 32], ValidatorTelemetry> {
        &self.validator_telemetry
    }
}

impl Default for RoundConsensus {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------------------
// ✅ Helper (standalone function)
// -----------------------------------------------------------------------------
#[cfg(feature = "ai_l1")]
pub fn calculate_reputation_score(
    model: &GBDTModel,
    telemetry: &ValidatorTelemetry,
) -> Result<i32> {
    Ok(compute_validator_score(telemetry, model))
}

#[cfg(not(feature = "ai_l1"))]
pub fn calculate_reputation_score(
    _model: &GBDTModel,
    _telemetry: &ValidatorTelemetry,
) -> Result<i32> {
    Ok(5000)
}

// -----------------------------------------------------------------------------
// ✅ Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[cfg(feature = "ai_l1")]
    use ippan_ai_core::gbdt::{Node, Tree};

    #[cfg(feature = "ai_l1")]
    fn create_test_model() -> GBDTModel {
        GBDTModel::new(
            vec![Tree {
                nodes: vec![Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(5000),
                }],
            }],
            0,
            10000,
            1,
        )
        .expect("valid test model")
    }

    #[cfg(feature = "ai_l1")]
    fn create_test_telemetry() -> ValidatorTelemetry {
        ippan_ai_core::features::ValidatorTelemetry {
            blocks_proposed: 1,
            blocks_verified: 1,
            rounds_active: 1,
            avg_latency_us: 1,
            slash_count: 0,
            stake: 1,
            age_rounds: 1,
        }
    }

    #[cfg(feature = "ai_l1")]
    #[test]
    fn test_reputation_score_calculation() {
        let model = create_test_model();
        let telemetry = create_test_telemetry();
        let score = calculate_reputation_score(&model, &telemetry).unwrap();
        assert!((0..=10000).contains(&score));
    }

    #[test]
    fn test_validator_selection() {
        let mut consensus = RoundConsensus::new();

        #[cfg(feature = "ai_l1")]
        {
            let model = create_test_model();
            consensus.set_active_model(model).unwrap();
        }

        let validators = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let mut stake_weights = HashMap::new();
        stake_weights.insert([1u8; 32], 1000);
        stake_weights.insert([2u8; 32], 2000);
        stake_weights.insert([3u8; 32], 1500);

        for (idx, validator) in validators.iter().enumerate() {
            #[cfg(feature = "ai_l1")]
            let telemetry = ValidatorTelemetry {
                blocks_proposed: 10 + idx as u64,
                blocks_verified: 20 + idx as u64,
                rounds_active: 100 + idx as u64,
                avg_latency_us: 150_000 + (idx as u64 * 1_000),
                slash_count: idx as u32,
                stake: 1_000_000 + (idx as u64 * 50_000),
                age_rounds: 1_000 + (idx as u64 * 100),
            };

            #[cfg(not(feature = "ai_l1"))]
            let telemetry = ValidatorTelemetry {
                validator_id: *validator,
                block_production_rate: 1.0 + idx as f64,
                avg_block_size: 2.0 + idx as f64,
                uptime: 99.0 - idx as f64,
                network_latency: 0.2 + idx as f64 * 0.01,
                validation_accuracy: 0.95,
                stake: 1_000 + idx as u64 * 500,
                slashing_events: idx as u32,
                last_activity: 123_456 + idx as u64,
                custom_metrics: HashMap::new(),
            };

            consensus.update_telemetry(*validator, telemetry);
        }

        let selection = consensus
            .select_validators(&validators, &stake_weights)
            .unwrap();
        assert!(validators.contains(&selection.proposer));
        let expected_verifiers = validators.len().saturating_sub(1).min(3);
        assert_eq!(selection.verifiers.len(), expected_verifiers);
        assert!(!selection.verifiers.contains(&selection.proposer));
    }

    #[test]
    fn test_round_advancement() {
        let mut consensus = RoundConsensus::new();
        assert_eq!(consensus.current_round(), 0);
        consensus.advance_round();
        assert_eq!(consensus.current_round(), 1);
    }
}
