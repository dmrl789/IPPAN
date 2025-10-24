use anyhow::Result;
use std::collections::HashMap;
use rand::Rng;

#[cfg(feature = "ai_l1")]
use ippan_ai_core::{features, gbdt::GbdtEvaluator, model::Model};

// -----------------------------------------------------------------------------
// ✅ Fallback definitions when AI feature is disabled
// -----------------------------------------------------------------------------
#[cfg(not(feature = "ai_l1"))]
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorTelemetry {
    pub validator_id: [u8; 32],
    pub block_production_rate: f64,
    pub avg_block_size: f64,
    pub uptime: f64,
    pub network_latency: f64,
    pub validation_accuracy: f64,
    pub stake: u64,
    pub slashing_events: u32,
    pub last_activity: u64,
    pub custom_metrics: HashMap<String, f64>,
}

#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone)]
pub struct Node {
    pub feature_index: usize,
    pub threshold: f64,
    pub left: usize,
    pub right: usize,
    pub value: Option<f64>,
}

#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone)]
pub struct Model {
    pub version: u32,
    pub feature_count: usize,
    pub tree_count: usize,
    pub max_depth: usize,
    pub trees: Vec<Tree>,
}

#[cfg(not(feature = "ai_l1"))]
impl Model {
    pub fn new(version: u32, feature_count: usize, tree_count: usize, max_depth: usize, trees: Vec<Tree>) -> Self {
        Self { version, feature_count, tree_count, max_depth, trees }
    }

    pub fn validate(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "ai_l1"))]
pub struct GbdtEvaluator {
    model: Model,
}

#[cfg(not(feature = "ai_l1"))]
impl GbdtEvaluator {
    pub fn new(model: Model) -> Result<Self> {
        Ok(Self { model })
    }

    pub fn evaluate(&self, _features: &[f64]) -> Result<i32> {
        Ok(5000) // Default score
    }
}

#[cfg(not(feature = "ai_l1"))]
pub mod features {
    use super::ValidatorTelemetry;
    use anyhow::Result;

    pub fn from_telemetry(telemetry: &ValidatorTelemetry) -> Result<Vec<f64>> {
        Ok(vec![
            telemetry.block_production_rate,
            telemetry.avg_block_size,
            telemetry.uptime,
            telemetry.network_latency,
            telemetry.validation_accuracy,
            telemetry.stake as f64,
            telemetry.slashing_events as f64,
            telemetry.last_activity as f64,
        ])
    }
}

// -----------------------------------------------------------------------------
// ✅ RoundConsensus implementation
// -----------------------------------------------------------------------------

/// Round-based consensus with AI reputation scoring
pub struct RoundConsensus {
    current_round: u64,
    active_model: Option<Model>,
    validator_telemetry: HashMap<[u8; 32], ValidatorTelemetry>,
    reputation_scores: HashMap<[u8; 32], i32>,
}

/// Validator selection result
#[derive(Debug, Clone)]
pub struct ValidatorSelection {
    pub proposer: [u8; 32],
    pub verifiers: Vec<[u8; 32]>,
    pub reputation_scores: HashMap<[u8; 32], i32>,
    pub selection_weights: HashMap<[u8; 32], f64>,
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

    pub fn set_active_model(&mut self, model: Model) -> Result<()> {
        model.validate()?;
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

        let model = match self.active_model.as_ref() {
            Some(m) => m,
            None => return Ok(5000),
        };

        let features = features::from_telemetry(telemetry)?;
        let evaluator = GbdtEvaluator::new(model.clone())?;
        evaluator.evaluate(&features)
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

            let stake_weight = stake_weights.get(validator).copied().unwrap_or(0) as f64;
            let reputation_weight = (reputation as f64) / 10_000.0;
            let combined_weight = stake_weight * 0.7 + reputation_weight * 1_000_000.0 * 0.3;
            selection_weights.insert(*validator, combined_weight);
        }

        let proposer = self.weighted_random_selection(validators, &selection_weights)?;
        let verifier_candidates: Vec<[u8; 32]> = validators
            .iter()
            .filter(|&&v| v != proposer)
            .copied()
            .collect();

        let verifier_weights: HashMap<[u8; 32], f64> = verifier_candidates
            .iter()
            .filter_map(|v| selection_weights.get(v).map(|&w| (*v, w)))
            .collect();

        let verifiers = self.select_multiple_weighted(&verifier_candidates, &verifier_weights, 3)?;

        Ok(ValidatorSelection {
            proposer,
            verifiers,
            reputation_scores,
            selection_weights,
        })
    }

    fn weighted_random_selection(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], f64>,
    ) -> Result<[u8; 32]> {
        let total_weight: f64 = weights.values().sum();
        if total_weight <= 0.0 {
            return Err(anyhow::anyhow!("Total weight must be positive"));
        }

        let mut rng = rand::thread_rng();
        let random_value: f64 = rng.gen_range(0.0..total_weight);

        let mut cumulative_weight = 0.0;
        for candidate in candidates {
            if let Some(&weight) = weights.get(candidate) {
                cumulative_weight += weight;
                if random_value <= cumulative_weight {
                    return Ok(*candidate);
                }
            }
        }

        Ok(*candidates.last().unwrap())
    }

    fn select_multiple_weighted(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], f64>,
        count: usize,
    ) -> Result<Vec<[u8; 32]>> {
        let mut selected = Vec::new();
        let mut remaining_candidates = candidates.to_vec();
        let mut remaining_weights = weights.clone();

        for _ in 0..count.min(candidates.len()) {
            if remaining_candidates.is_empty() {
                break;
            }

            let selected_item = self.weighted_random_selection(&remaining_candidates, &remaining_weights)?;
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
// ✅ Standalone helper
// -----------------------------------------------------------------------------
pub fn calculate_reputation_score(
    model: &Model,
    telemetry: &ValidatorTelemetry,
) -> Result<i32> {
    let features = features::from_telemetry(telemetry)?;
    let evaluator = GbdtEvaluator::new(model.clone())?;
    evaluator.evaluate(&features)
}

// -----------------------------------------------------------------------------
// ✅ Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ai_l1")]
    fn create_test_model() -> Model {
        Model::new(
            1,
            8,
            1,
            3,
            vec![Tree {
                nodes: vec![
                    Node { feature_index: 0, threshold: 50.0, left: 1, right: 2, value: None },
                    Node { feature_index: 0, threshold: 0.0, left: 0, right: 0, value: Some(1000.0) },
                    Node { feature_index: 0, threshold: 0.0, left: 0, right: 0, value: Some(500.0) },
                ],
            }],
        )
    }

    #[cfg(feature = "ai_l1")]
    fn create_test_telemetry() -> ValidatorTelemetry {
        ValidatorTelemetry {
            validator_id: [1u8; 32],
            block_production_rate: 12.5,
            avg_block_size: 1200.0,
            uptime: 0.98,
            network_latency: 80.0,
            validation_accuracy: 0.99,
            stake: 1_500_000,
            slashing_events: 0,
            last_activity: 300,
            custom_metrics: HashMap::new(),
        }
    }

    #[cfg(feature = "ai_l1")]
    #[test]
    fn test_reputation_score_calculation() {
        let model = create_test_model();
        let telemetry = create_test_telemetry();
        let score = calculate_reputation_score(&model, &telemetry).unwrap();
        assert!(score >= 0 && score <= 10000);
    }

    #[test]
    fn test_validator_selection() {
        let mut consensus = RoundConsensus::new();

        #[cfg(feature = "ai_l1")]
        {
            let model = create_test_model();
            consensus.set_active_model(model).unwrap();

            let validators = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
            let mut stake_weights = HashMap::new();
            stake_weights.insert([1u8; 32], 1000);
            stake_weights.insert([2u8; 32], 2000);
            stake_weights.insert([3u8; 32], 1500);

            for validator in &validators {
                let mut telemetry = create_test_telemetry();
                telemetry.validator_id = *validator;
                consensus.update_telemetry(*validator, telemetry);
            }

            let selection = consensus.select_validators(&validators, &stake_weights).unwrap();
            assert!(validators.contains(&selection.proposer));
            assert_eq!(selection.verifiers.len(), 3);
            assert!(!selection.verifiers.contains(&selection.proposer));
        }
    }

    #[test]
    fn test_round_advancement() {
        let mut consensus = RoundConsensus::new();
        assert_eq!(consensus.current_round(), 0);
        consensus.advance_round();
        assert_eq!(consensus.current_round(), 1);
    }
}
