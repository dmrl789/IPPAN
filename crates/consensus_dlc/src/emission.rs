//! Token emission and reward distribution for DLC consensus
//!
//! This module provides a compatibility layer over ippan_economics,
//! adapting it to the DLC consensus engine's needs.

use crate::dgbdt::FairnessModel;
use crate::error::{DlcError, Result};
use crate::fairness_features::features_for_validator;
use crate::reward_weighting::{compute_reward_weights, distribute_by_weights, SCALE};
use ippan_economics::{EmissionEngine, EmissionParams};
use ippan_types::Amount;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base block reward in smallest unit (micro-IPN)
/// This is kept for backwards compatibility but actual emission
/// is now controlled by ippan_economics EmissionEngine
pub const BLOCK_REWARD: u64 = 10_000; // 0.0001 IPN per round (10,000 µIPN)

/// Maximum supply cap: 21 million IPN
pub const SUPPLY_CAP: u64 = 21_000_000_000_000; // 21M * 1M µIPN

/// Emission schedule for token distribution
///
/// This is now a wrapper around ippan_economics::EmissionEngine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionSchedule {
    /// The underlying emission engine from ippan_economics
    #[serde(skip)]
    engine: EmissionEngine,
    /// Emission parameters (serializable)
    params: EmissionParams,
    /// Current round state
    current_round: u64,
    /// Total supply emitted
    total_supply: u64,
    /// Start timestamp
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Last update round
    pub last_update_round: u64,
}

impl Default for EmissionSchedule {
    fn default() -> Self {
        let params = EmissionParams::default();
        Self {
            engine: EmissionEngine::with_params(params.clone()),
            params,
            current_round: 0,
            total_supply: 0,
            start_time: chrono::Utc::now(),
            last_update_round: 0,
        }
    }
}

impl EmissionSchedule {
    /// Create a new emission schedule with custom parameters
    pub fn new_with_params(params: EmissionParams) -> Self {
        Self {
            engine: EmissionEngine::with_params(params.clone()),
            params,
            current_round: 0,
            total_supply: 0,
            start_time: chrono::Utc::now(),
            last_update_round: 0,
        }
    }

    /// Calculate block reward for current round
    pub fn calculate_block_reward(&self, round: u64) -> u64 {
        self.engine.calculate_round_reward(round).unwrap_or(0)
    }

    /// Update emission schedule after a round
    pub fn update(&mut self, round: u64, _blocks_produced: u64) -> Result<()> {
        let _reward = self
            .engine
            .advance_round(round)
            .map_err(|e| DlcError::EmissionCalculation(e.to_string()))?;

        self.current_round = round;
        self.total_supply = self.engine.total_supply();
        self.last_update_round = round;
        Ok(())
    }

    /// Get emission statistics
    pub fn stats(&self) -> EmissionStats {
        let supply_info = self.engine.get_supply_info();

        EmissionStats {
            current_supply: supply_info.total_supply,
            emitted_supply: supply_info.total_supply,
            remaining_supply: supply_info.remaining_supply,
            emission_progress_bps: supply_info
                .emission_percentage
                .checked_mul(Decimal::from(10_000u32))
                .and_then(|value| value.to_u32())
                .unwrap_or(0),
            current_inflation_bps: 0, // Not used in new model
            current_block_reward: self
                .engine
                .calculate_round_reward(self.current_round.max(1))
                .unwrap_or(0),
        }
    }

    /// Get current emission parameters
    pub fn params(&self) -> &EmissionParams {
        &self.params
    }
}

/// Emission statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionStats {
    pub current_supply: u64,
    pub emitted_supply: u64,
    pub remaining_supply: u64,
    /// Emission progress in basis points (0-10000 = 0%-100%)
    pub emission_progress_bps: u32,
    pub current_inflation_bps: u64,
    pub current_block_reward: u64,
}

/// Reward distribution manager
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardDistributor {
    /// Pending rewards for validators
    pending_rewards: HashMap<String, u64>,
    /// Distributed rewards history
    distributed_total: u64,
    /// Distribution splits
    splits: RewardSplits,
}

/// Reward distribution splits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSplits {
    /// Percentage for block proposer (in basis points)
    pub proposer_bps: u64,
    /// Percentage for verifiers (in basis points)
    pub verifiers_bps: u64,
    /// Percentage for treasury/development fund (in basis points)
    pub treasury_bps: u64,
}

impl Default for RewardSplits {
    fn default() -> Self {
        Self {
            proposer_bps: 5000,  // 50%
            verifiers_bps: 4000, // 40%
            treasury_bps: 1000,  // 10%
        }
    }
}

impl RewardDistributor {
    /// Create a new reward distributor
    pub fn new(splits: RewardSplits) -> Self {
        Self {
            pending_rewards: HashMap::new(),
            distributed_total: 0,
            splits,
        }
    }

    /// Distribute rewards for a block (with fairness-based weighting)
    pub fn distribute_block_reward(
        &mut self,
        block_reward: u64,
        proposer: &str,
        verifiers: &[String],
        model: &FairnessModel,
        validator_metrics: &HashMap<String, crate::dgbdt::ValidatorMetrics>,
    ) -> Result<DistributionResult> {
        if block_reward == 0 {
            return Err(DlcError::EmissionCalculation(
                "Zero block reward".to_string(),
            ));
        }

        // Calculate splits
        let proposer_reward = (block_reward * self.splits.proposer_bps) / 10_000;
        let verifiers_reward = (block_reward * self.splits.verifiers_bps) / 10_000;
        let treasury_reward = block_reward
            .saturating_sub(proposer_reward)
            .saturating_sub(verifiers_reward);

        // Distribute to proposer (unweighted for now, could add weighting later)
        *self
            .pending_rewards
            .entry(proposer.to_string())
            .or_insert(0) += proposer_reward;

        // Distribute to verifiers with fairness-based weights
        if !verifiers.is_empty() && verifiers_reward > 0 {
            // Compute scores for all verifiers
            let max_stake = validator_metrics
                .values()
                .map(|m| m.stake)
                .max()
                .unwrap_or(Amount::from_micro_ipn(1));

            let mut scores = Vec::with_capacity(verifiers.len());
            let mut validator_ids = Vec::with_capacity(verifiers.len());

            for verifier_id in verifiers {
                if let Some(metrics) = validator_metrics.get(verifier_id) {
                    // Extract 7 features and compute score
                    let features = features_for_validator(metrics, max_stake);
                    let raw_model = model.raw_model();
                    let score = raw_model.score(&features);
                    // Clamp score to [0..SCALE]
                    let clamped_score = score.clamp(0, SCALE);
                    scores.push(clamped_score);
                    validator_ids.push(verifier_id.clone());
                } else {
                    // If metrics not found, use neutral score
                    scores.push(SCALE / 2);
                    validator_ids.push(verifier_id.clone());
                }
            }

            // Compute weights
            let weights = compute_reward_weights(&scores, &validator_ids);

            // Distribute verifiers_reward using weights
            let payouts = distribute_by_weights(verifiers_reward, &weights, &validator_ids);

            for (validator_id, amount) in payouts {
                *self.pending_rewards.entry(validator_id).or_insert(0) += amount;
            }
        }

        Ok(DistributionResult {
            total_distributed: block_reward,
            proposer_reward,
            verifier_reward: verifiers_reward,
            treasury_reward,
            verifier_count: verifiers.len(),
        })
    }

    /// Get pending rewards for a validator
    pub fn get_pending(&self, validator_id: &str) -> u64 {
        *self.pending_rewards.get(validator_id).unwrap_or(&0)
    }

    /// Claim rewards for a validator
    pub fn claim_rewards(&mut self, validator_id: &str) -> Result<u64> {
        let amount = self.pending_rewards.remove(validator_id).unwrap_or(0);

        if amount == 0 {
            return Err(DlcError::EmissionCalculation(
                "No pending rewards".to_string(),
            ));
        }

        self.distributed_total = self.distributed_total.saturating_add(amount);

        tracing::debug!("Validator {} claimed {} micro-IPN", validator_id, amount);

        Ok(amount)
    }

    /// Get all pending rewards
    pub fn all_pending(&self) -> &HashMap<String, u64> {
        &self.pending_rewards
    }

    /// Get total distributed rewards
    pub fn total_distributed(&self) -> u64 {
        self.distributed_total
    }

    /// Get distribution statistics
    pub fn stats(&self) -> DistributorStats {
        let total_pending: u64 = self.pending_rewards.values().sum();
        let validator_count = self.pending_rewards.len();

        DistributorStats {
            total_pending,
            total_distributed: self.distributed_total,
            pending_validator_count: validator_count,
        }
    }
}

/// Result of reward distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    pub total_distributed: u64,
    pub proposer_reward: u64,
    pub verifier_reward: u64,
    pub treasury_reward: u64,
    pub verifier_count: usize,
}

/// Distributor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributorStats {
    pub total_pending: u64,
    pub total_distributed: u64,
    pub pending_validator_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_schedule_creation() {
        let schedule = EmissionSchedule::default();
        let stats = schedule.stats();
        assert_eq!(stats.current_supply, 0);
        assert!(stats.current_supply < SUPPLY_CAP);
    }

    #[test]
    fn test_block_reward_calculation() {
        let schedule = EmissionSchedule::default();
        let reward = schedule.calculate_block_reward(1);
        // First round should have reward equal to initial_round_reward
        assert!(reward > 0);
        assert_eq!(reward, BLOCK_REWARD);
    }

    #[test]
    fn test_emission_update() {
        let mut schedule = EmissionSchedule::default();
        let initial_stats = schedule.stats();
        assert_eq!(initial_stats.current_supply, 0); // Starts from genesis

        schedule.update(1, 1).unwrap();

        // After processing 1 round, supply should increase
        let final_stats = schedule.stats();
        assert!(final_stats.current_supply >= initial_stats.current_supply);
    }

    #[test]
    fn test_max_supply_cap() {
        // Create a custom params with low supply cap for testing
        let params = EmissionParams {
            initial_round_reward_micro: 1000,
            halving_interval_rounds: 10,
            max_supply_micro: 1100,
            ..Default::default()
        };
        let mut schedule = EmissionSchedule::new_with_params(params);

        // Emit beyond max supply
        for round in 1..=20 {
            let _ = schedule.update(round, 10);
        }

        let stats = schedule.stats();
        assert!(stats.current_supply <= SUPPLY_CAP);
    }

    #[test]
    fn test_reward_distribution() {
        use crate::dgbdt::FairnessModel;
        let mut distributor = RewardDistributor::default();
        let model = FairnessModel::testing_stub();
        let mut metrics = std::collections::HashMap::new();
        metrics.insert(
            "proposer1".to_string(),
            crate::dgbdt::ValidatorMetrics::default(),
        );
        metrics.insert("v1".to_string(), crate::dgbdt::ValidatorMetrics::default());
        metrics.insert("v2".to_string(), crate::dgbdt::ValidatorMetrics::default());

        let result = distributor
            .distribute_block_reward(
                BLOCK_REWARD,
                "proposer1",
                &["v1".to_string(), "v2".to_string()],
                &model,
                &metrics,
            )
            .unwrap();

        assert_eq!(result.total_distributed, BLOCK_REWARD);
        assert!(result.proposer_reward > 0);
        assert!(result.verifier_reward > 0);
    }

    #[test]
    fn test_pending_rewards() {
        use crate::dgbdt::FairnessModel;
        let mut distributor = RewardDistributor::default();
        let model = FairnessModel::testing_stub();
        let mut metrics = std::collections::HashMap::new();
        metrics.insert(
            "proposer1".to_string(),
            crate::dgbdt::ValidatorMetrics::default(),
        );
        metrics.insert("v1".to_string(), crate::dgbdt::ValidatorMetrics::default());

        distributor
            .distribute_block_reward(
                BLOCK_REWARD,
                "proposer1",
                &["v1".to_string()],
                &model,
                &metrics,
            )
            .unwrap();

        assert!(distributor.get_pending("proposer1") > 0);
        assert!(distributor.get_pending("v1") > 0);
    }

    #[test]
    fn test_claim_rewards() {
        use crate::dgbdt::FairnessModel;
        let mut distributor = RewardDistributor::default();
        let model = FairnessModel::testing_stub();
        let mut metrics = std::collections::HashMap::new();
        metrics.insert(
            "proposer1".to_string(),
            crate::dgbdt::ValidatorMetrics::default(),
        );

        distributor
            .distribute_block_reward(BLOCK_REWARD, "proposer1", &[], &model, &metrics)
            .unwrap();

        let pending = distributor.get_pending("proposer1");
        let claimed = distributor.claim_rewards("proposer1").unwrap();

        assert_eq!(pending, claimed);
        assert_eq!(distributor.get_pending("proposer1"), 0);
    }

    #[test]
    fn test_reward_splits() {
        use crate::dgbdt::FairnessModel;
        let splits = RewardSplits {
            proposer_bps: 6000,
            verifiers_bps: 3000,
            treasury_bps: 1000,
        };

        let mut distributor = RewardDistributor::new(splits);
        let model = FairnessModel::testing_stub();
        let mut metrics = std::collections::HashMap::new();
        metrics.insert(
            "proposer".to_string(),
            crate::dgbdt::ValidatorMetrics::default(),
        );
        metrics.insert("v1".to_string(), crate::dgbdt::ValidatorMetrics::default());

        let result = distributor
            .distribute_block_reward(10_000, "proposer", &["v1".to_string()], &model, &metrics)
            .unwrap();

        assert_eq!(result.proposer_reward, 6000);
        assert_eq!(result.verifier_reward, 3000);
    }

    #[test]
    fn test_emission_stats() {
        let schedule = EmissionSchedule::default();
        let stats = schedule.stats();

        assert!(stats.emission_progress_bps <= 10000); // Max 100%
        assert!(stats.remaining_supply > 0);
    }

    #[test]
    fn test_distributor_stats() {
        use crate::dgbdt::FairnessModel;
        let mut distributor = RewardDistributor::default();
        let model = FairnessModel::testing_stub();
        let mut metrics = std::collections::HashMap::new();
        metrics.insert(
            "proposer1".to_string(),
            crate::dgbdt::ValidatorMetrics::default(),
        );
        metrics.insert("v1".to_string(), crate::dgbdt::ValidatorMetrics::default());

        distributor
            .distribute_block_reward(
                BLOCK_REWARD,
                "proposer1",
                &["v1".to_string()],
                &model,
                &metrics,
            )
            .unwrap();

        let stats = distributor.stats();
        assert!(stats.total_pending > 0);
        assert_eq!(stats.pending_validator_count, 2);
    }

    #[test]
    fn test_halving_schedule() {
        let mut schedule = EmissionSchedule::default();
        let initial_reward = schedule.calculate_block_reward(1);

        // Advance to first halving
        let halving_round = schedule.params().halving_interval_rounds + 1;
        schedule.update(halving_round, 1).unwrap();

        let halved_reward = schedule.calculate_block_reward(halving_round);
        assert_eq!(halved_reward, initial_reward / 2);
    }
}
