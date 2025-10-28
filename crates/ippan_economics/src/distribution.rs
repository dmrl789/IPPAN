//! Reward distribution logic for DAG-Fair emission
//!
//! Distributes round-based emissions and collected fees deterministically among validators,
//! based on uptime, role, and contribution weight. Implements the DAG-Fair model for IPPAN.

use crate::types::*;
use crate::errors::*;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, warn};

/// Round reward distribution engine
#[derive(Debug, Clone)]
pub struct RoundRewards {
    /// Current emission parameters
    params: EmissionParams,
}

impl RoundRewards {
    /// Create a new round rewards distributor
    pub fn new(params: EmissionParams) -> Self {
        Self { params }
    }

    /// Distribute rewards for a round based on validator participation
    pub fn distribute_round_rewards(
        &self,
        round_index: RoundIndex,
        round_reward: RewardAmount,
        participations: Vec<ValidatorParticipation>,
        fees_collected: RewardAmount,
    ) -> Result<RoundRewardDistribution, DistributionError> {
        if participations.is_empty() {
            return Err(DistributionError::NoValidators(round_index));
        }

        // Validate participations
        self.validate_participations(&participations)?;

        // Calculate total weight
        let total_weight = self.calculate_total_weight(&participations)?;
        if total_weight == Decimal::ZERO {
            warn!("No validators with non-zero weight in round {}", round_index);
            return Ok(self.create_empty_distribution(round_index, round_reward, fees_collected));
        }

        // Apply fee cap to collected fees
        let capped_fees = self.apply_fee_cap(fees_collected, round_reward);

        // Build composition
        let composition = RewardComposition::new_with_fees(round_reward, capped_fees);

        // Proportional distribution
        let mut validator_rewards = HashMap::new();
        for participation in &participations {
            let weight = self.calculate_validator_weight(participation)?;
            let weight_fraction = weight / total_weight;

            let validator_reward = ValidatorReward {
                round_emission: self.calculate_component_reward(composition.round_emission, weight_fraction)?,
                transaction_fees: self.calculate_component_reward(composition.transaction_fees, weight_fraction)?,
                ai_commissions: self.calculate_component_reward(composition.ai_commissions, weight_fraction)?,
                network_dividend: self.calculate_component_reward(composition.network_dividend, weight_fraction)?,
                total_reward: 0,
                weight_factor: weight,
            };

            let total_reward = validator_reward.round_emission
                + validator_reward.transaction_fees
                + validator_reward.ai_commissions
                + validator_reward.network_dividend;

            let mut final_reward = validator_reward;
            final_reward.total_reward = total_reward;
            validator_rewards.insert(participation.validator_id.clone(), final_reward);
        }

        // Compute totals
        let total_distributed: RewardAmount = validator_rewards.values().map(|r| r.total_reward).sum();
        let total_available = round_reward + capped_fees;
        let excess = total_distributed.saturating_sub(total_available);
        let excess_fees = fees_collected.saturating_sub(capped_fees);

        info!(
            "Distributed rewards for round {}: {} µIPN to {} validators, excess burned: {}",
            round_index,
            total_distributed,
            validator_rewards.len(),
            excess + excess_fees
        );

        Ok(RoundRewardDistribution {
            round_index,
            total_reward: total_available,
            blocks_in_round: participations.iter().map(|p| p.blocks_contributed).sum(),
            validator_rewards,
            fees_collected: capped_fees,
            excess_burned: excess + excess_fees,
        })
    }

    /// Calculate the weight for a single validator
    fn calculate_validator_weight(&self, participation: &ValidatorParticipation) -> Result<Decimal, DistributionError> {
        let role_weight = participation.role.weight_multiplier();
        let uptime_factor = participation.uptime_score;
        let blocks_factor = Decimal::from(participation.blocks_contributed);

        let weight = role_weight
            .checked_mul(uptime_factor)
            .and_then(|w| w.checked_mul(blocks_factor))
            .ok_or_else(|| DistributionError::CalculationFailed("Weight calculation overflow".to_string()))?;

        Ok(weight)
    }

    /// Sum all validator weights
    fn calculate_total_weight(&self, participations: &[ValidatorParticipation]) -> Result<Decimal, DistributionError> {
        let mut total = Decimal::ZERO;
        for p in participations {
            let weight = self.calculate_validator_weight(p)?;
            total = total
                .checked_add(weight)
                .ok_or_else(|| DistributionError::CalculationFailed("Total weight overflow".to_string()))?;
        }
        Ok(total)
    }

    /// Component reward scaled by weight fraction
    fn calculate_component_reward(
        &self,
        component_total: RewardAmount,
        weight_fraction: Decimal,
    ) -> Result<RewardAmount, DistributionError> {
        let reward_decimal = Decimal::from(component_total)
            .checked_mul(weight_fraction)
            .ok_or_else(|| DistributionError::CalculationFailed("Reward fraction overflow".to_string()))?;

        Ok(reward_decimal.round_dp(0).to_u64().unwrap_or(0))
    }

    /// Cap excessive fee revenue
    pub fn apply_fee_cap(&self, fees_collected: RewardAmount, round_reward: RewardAmount) -> RewardAmount {
        let max_fees = (Decimal::from(round_reward) * self.params.fee_cap_fraction)
            .round_dp(0)
            .to_u64()
            .unwrap_or(0);
        fees_collected.min(max_fees)
    }

    /// Sanity check validator participations
    fn validate_participations(&self, participations: &[ValidatorParticipation]) -> Result<(), DistributionError> {
        for p in participations {
            if p.uptime_score < Decimal::ZERO || p.uptime_score > Decimal::ONE {
                return Err(DistributionError::InvalidParticipation(format!(
                    "Invalid uptime score for validator {}: {}",
                    p.validator_id, p.uptime_score
                )));
            }

            if p.blocks_contributed == 0 && p.role != ValidatorRole::Observer {
                return Err(DistributionError::InvalidParticipation(format!(
                    "Non-observer validator {} contributed 0 blocks",
                    p.validator_id
                )));
            }
        }
        Ok(())
    }

    /// Generate empty distribution when no validators contribute
    fn create_empty_distribution(
        &self,
        round_index: RoundIndex,
        round_reward: RewardAmount,
        fees_collected: RewardAmount,
    ) -> RoundRewardDistribution {
        RoundRewardDistribution {
            round_index,
            total_reward: round_reward,
            blocks_in_round: 0,
            validator_rewards: HashMap::new(),
            fees_collected: 0,
            excess_burned: round_reward + fees_collected,
        }
    }

    /// Estimate maximum reward for a validator (for monitoring)
    pub fn calculate_max_validator_reward(
        &self,
        round_reward: RewardAmount,
        blocks_contributed: u32,
    ) -> RewardAmount {
        let composition = RewardComposition::new(round_reward);
        let max_weight =
            ValidatorRole::Proposer.weight_multiplier() * Decimal::ONE * Decimal::from(blocks_contributed);
        let total_components = Decimal::from(composition.total());
        (total_components * max_weight).round_dp(0).to_u64().unwrap_or(0)
    }

    pub fn params(&self) -> &EmissionParams {
        &self.params
    }
}

impl Default for RoundRewards {
    fn default() -> Self {
        Self::new(EmissionParams::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_participation(id: &str, role: ValidatorRole, blocks: u32) -> ValidatorParticipation {
        ValidatorParticipation {
            validator_id: ValidatorId::new(id),
            role,
            blocks_contributed: blocks,
            uptime_score: Decimal::ONE,
        }
    }

    #[test]
    fn test_reward_distribution() {
        let rewards = RoundRewards::new(EmissionParams::default());

        let participations = vec![
            create_test_participation("validator1", ValidatorRole::Proposer, 10),
            create_test_participation("validator2", ValidatorRole::Verifier, 5),
        ];

        let distribution = rewards
            .distribute_round_rewards(1, 10_000, participations, 1_000)
            .unwrap();

        assert_eq!(distribution.round_index, 1);
        assert_eq!(distribution.total_reward, 11_000);
        assert_eq!(distribution.validator_rewards.len(), 2);

        let proposer = distribution
            .validator_rewards
            .get(&ValidatorId::new("validator1"))
            .unwrap();
        let verifier = distribution
            .validator_rewards
            .get(&ValidatorId::new("validator2"))
            .unwrap();
        assert!(proposer.total_reward > verifier.total_reward);
    }

    #[test]
    fn test_fee_cap() {
        let mut params = EmissionParams::default();
        params.fee_cap_fraction = Decimal::new(1, 1); // 10%

        let rewards = RoundRewards::new(params);
        let capped = rewards.apply_fee_cap(5_000, 10_000);
        assert_eq!(capped, 1_000);
    }

    #[test]
    fn test_empty_participation() {
        let rewards = RoundRewards::new(EmissionParams::default());
        let result = rewards.distribute_round_rewards(1, 10_000, vec![], 0);
        assert!(matches!(result, Err(DistributionError::NoValidators(1))));
    }
}
