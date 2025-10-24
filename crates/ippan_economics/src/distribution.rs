<<<<<<< HEAD
//! Deterministic round reward distribution for IPPAN BlockDAG.
//! Ensures fair proportional allocation under the DAG-Fair model,
//! using weighted participation and fee caps defined in `EconomicsParams`.

use crate::{EcoError, EconomicsParams};
use crate::types::{MicroIPN, ParticipationSet, Payouts, Role};

/// Distribute a round’s total emission (μIPN) and fees across validators proportionally,
/// according to their recorded participation (`blocks`) and role (proposer/verifier).
///
/// - `emission_micro`: total emission allocated for the round (after halving and cap)
/// - `fees_micro`: total transaction fees collected during the round
/// - `parts`: per-validator participation map
///
/// Returns `(payouts_map, emission_paid, fees_paid)`.
///
/// If `parts` is empty, this returns an empty set with `(0, 0)`.
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    parts: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, MicroIPN), EcoError> {
    // --- Fee cap enforcement ---
    let (numer, denom) = params.fee_cap_fraction();
    let fee_cap = emission_micro
        .saturating_mul(numer as u128)
        / (denom as u128).max(1);
    if fees_micro > fee_cap {
        return Err(EcoError::FeeCapExceeded {
            fees: fees_micro,
            cap: fee_cap,
        });
    }

    // --- Weighted total calculation ---
    let mut total_weighted: u128 = 0;
    for p in parts.values() {
        let weight = role_weight(params, p.role) as u128;
        total_weighted = total_weighted.saturating_add(weight.saturating_mul(p.blocks as u128));
    }

    // --- No participation → no payout ---
    if total_weighted == 0 {
        return Ok((Payouts::default(), 0, 0));
    }

    // --- Pool to distribute ---
    let pool = emission_micro.saturating_add(fees_micro);

    // --- Proportional payout allocation ---
    let mut payouts = Payouts::default();
    let mut distributed: u128 = 0;

    for (vid, p) in parts.iter() {
        let weight = role_weight(params, p.role) as u128;
        let share = weight.saturating_mul(p.blocks as u128);
        let amt = pool.saturating_mul(share) / total_weighted;

        if amt > 0 {
            payouts.insert(vid.clone(), amt);
            distributed = distributed.saturating_add(amt);
        }
    }

    // Slight rounding remainder (due to integer division) is ignored or can be auto-burned.
    Ok((payouts, emission_micro, fees_micro))
}

/// Return milli-weight for each role (1000 = 1.0x)
#[inline]
fn role_weight(params: &EconomicsParams, role: Role) -> u32 {
    match role {
        Role::Proposer => params.weight_proposer_milli,
        Role::Verifier => params.weight_verifier_milli,
=======
//! Reward distribution logic for DAG-Fair emission

use crate::types::*;
use crate::errors::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use tracing::{debug, info, warn};

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

        // Calculate total weight for all validators
        let total_weight = self.calculate_total_weight(&participations)?;
        
        if total_weight == Decimal::ZERO {
            warn!("No validators with non-zero weight in round {}", round_index);
            return Ok(self.create_empty_distribution(round_index, round_reward, fees_collected));
        }

        // Apply fee cap to collected fees
        let capped_fees = self.apply_fee_cap(fees_collected, round_reward);
        
        // Create reward composition using actual collected fees instead of minting from emission
        let composition = RewardComposition::new_with_fees(round_reward, capped_fees);
        
        // Distribute rewards proportionally
        let mut validator_rewards = HashMap::new();
        
        for participation in &participations {
            let weight = self.calculate_validator_weight(participation)?;
            let weight_fraction = weight / total_weight;
            
            let validator_reward = ValidatorReward {
                round_emission: self.calculate_component_reward(composition.round_emission, weight_fraction)?,
                transaction_fees: self.calculate_component_reward(composition.transaction_fees, weight_fraction)?,
                ai_commissions: self.calculate_component_reward(composition.ai_commissions, weight_fraction)?,
                network_dividend: self.calculate_component_reward(composition.network_dividend, weight_fraction)?,
                total_reward: 0, // Will be calculated
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

        // Calculate excess to burn (if any)
        let total_distributed: RewardAmount = validator_rewards.values()
            .map(|r| r.total_reward)
            .sum();
        
        // Total available reward is round_reward + capped_fees
        let total_available = round_reward + capped_fees;
        let excess = if total_distributed > total_available {
            total_distributed - total_available
        } else {
            0
        };

        // Calculate excess fees (fees that were capped and not used)
        let excess_fees = fees_collected.saturating_sub(capped_fees);

        info!(
            "Distributed rewards for round {}: {} micro-IPN to {} validators, excess burned: {}",
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

    /// Calculate the weight for a single validator based on their participation
    fn calculate_validator_weight(&self, participation: &ValidatorParticipation) -> Result<Decimal, DistributionError> {
        let role_weight = participation.role.weight_multiplier();
        let uptime_factor = participation.uptime_score;
        let blocks_factor = Decimal::from(participation.blocks_contributed);
        
        // Weight = role_weight * uptime_factor * blocks_factor
        let weight = role_weight
            .checked_mul(uptime_factor)
            .and_then(|w| w.checked_mul(blocks_factor))
            .ok_or(DistributionError::CalculationFailed("Weight calculation overflow".to_string()))?;

        Ok(weight)
    }

    /// Calculate total weight for all validators
    fn calculate_total_weight(&self, participations: &[ValidatorParticipation]) -> Result<Decimal, DistributionError> {
        let mut total = Decimal::ZERO;
        
        for participation in participations {
            let weight = self.calculate_validator_weight(participation)?;
            total = total
                .checked_add(weight)
                .ok_or(DistributionError::CalculationFailed("Total weight calculation overflow".to_string()))?;
        }
        
        Ok(total)
    }

    /// Calculate reward for a specific component based on weight fraction
    fn calculate_component_reward(
        &self,
        component_total: RewardAmount,
        weight_fraction: Decimal,
    ) -> Result<RewardAmount, DistributionError> {
        let component_decimal = Decimal::from(component_total);
        let reward_decimal = component_decimal
            .checked_mul(weight_fraction)
            .ok_or(DistributionError::CalculationFailed("Component reward calculation overflow".to_string()))?;
        
        // Round to nearest micro-IPN
        let reward = reward_decimal.round_dp(0).to_u64()
            .ok_or(DistributionError::CalculationFailed("Component reward conversion failed".to_string()))?;
        
        Ok(reward)
    }

    /// Apply fee cap to prevent economic centralization
    pub fn apply_fee_cap(&self, fees_collected: RewardAmount, round_reward: RewardAmount) -> RewardAmount {
        let max_fees = (Decimal::from(round_reward) * self.params.fee_cap_fraction)
            .round_dp(0)
            .to_u64()
            .unwrap_or(0);
        
        fees_collected.min(max_fees)
    }

    /// Validate validator participations
    fn validate_participations(&self, participations: &[ValidatorParticipation]) -> Result<(), DistributionError> {
        for participation in participations {
            if participation.uptime_score < Decimal::ZERO || participation.uptime_score > Decimal::ONE {
                return Err(DistributionError::InvalidParticipation(format!(
                    "Invalid uptime score for validator {}: {}",
                    participation.validator_id, participation.uptime_score
                )));
            }

            if participation.blocks_contributed == 0 && participation.role != ValidatorRole::Observer {
                return Err(DistributionError::InvalidParticipation(format!(
                    "Non-observer validator {} contributed 0 blocks",
                    participation.validator_id
                )));
            }
        }

        Ok(())
    }

    /// Create an empty distribution when no validators participate
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

    /// Calculate the theoretical maximum reward for a validator in a round
    pub fn calculate_max_validator_reward(
        &self,
        round_reward: RewardAmount,
        blocks_contributed: u32,
    ) -> RewardAmount {
        let composition = RewardComposition::new(round_reward);
        let max_weight = ValidatorRole::Proposer.weight_multiplier() * Decimal::ONE * Decimal::from(blocks_contributed);
        
        // This is a simplified calculation for estimation purposes
        let total_components = Decimal::from(composition.total());
        (total_components * max_weight).round_dp(0).to_u64().unwrap_or(0)
    }

    /// Get the current emission parameters
    pub fn params(&self) -> &EmissionParams {
        &self.params
    }
}

impl Default for RoundRewards {
    fn default() -> Self {
        Self::new(EmissionParams::default())
>>>>>>> 952abe6 (feat: Add ippan_economics crate and DAG-Fair emission framework)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
<<<<<<< HEAD
    use crate::types::{Participation, ValidatorId};

    #[test]
    fn test_distribute_round_proportional() {
        let params = EconomicsParams::default();

        let mut parts = ParticipationSet::new();
        parts.insert(
            ValidatorId("alice".into()),
            Participation { role: Role::Proposer, blocks: 5 },
        );
        parts.insert(
            ValidatorId("bob".into()),
            Participation { role: Role::Verifier, blocks: 10 },
        );

        let emission = 1_000_000u128; // 1 IPN
        let fees = 100_000u128; // 0.1 IPN
        let (payouts, e_paid, f_paid) = distribute_round(emission, fees, &parts, &params).unwrap();

        assert_eq!(e_paid, emission);
        assert_eq!(f_paid, fees);
        assert_eq!(payouts.len(), 2);

        // Proposer gets higher weight → higher reward ratio
        let a = payouts.get(&ValidatorId("alice".into())).copied().unwrap_or(0);
        let b = payouts.get(&ValidatorId("bob".into())).copied().unwrap_or(0);
        assert!(a < b); // 5 vs 10 blocks, even with 1.2x weight
    }

    #[test]
    fn test_fee_cap_enforced() {
        let params = EconomicsParams::default();
        let mut parts = ParticipationSet::new();
        parts.insert(
            ValidatorId("alice".into()),
            Participation { role: Role::Verifier, blocks: 10 },
        );
        let result = distribute_round(1_000_000, 200_000, &parts, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_participation_yields_zero() {
        let params = EconomicsParams::default();
        let parts = ParticipationSet::new();
        let (payouts, e, f) = distribute_round(1000, 100, &parts, &params).unwrap();
        assert!(payouts.is_empty());
        assert_eq!(e, 0);
        assert_eq!(f, 0);
    }
}
=======

    fn create_test_participation(validator_id: &str, role: ValidatorRole, blocks: u32) -> ValidatorParticipation {
        ValidatorParticipation {
            validator_id: validator_id.to_string(),
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

        let distribution = rewards.distribute_round_rewards(1, 10_000, participations, 1_000).unwrap();
        
        assert_eq!(distribution.round_index, 1);
        assert_eq!(distribution.total_reward, 11_000); // 10_000 round reward + 1_000 fees
        assert_eq!(distribution.validator_rewards.len(), 2);
        
        // Proposer should get more reward than verifier
        let proposer_reward = distribution.validator_rewards.get("validator1").unwrap();
        let verifier_reward = distribution.validator_rewards.get("validator2").unwrap();
        
        assert!(proposer_reward.total_reward > verifier_reward.total_reward);
    }

    #[test]
    fn test_fee_cap() {
        let mut params = EmissionParams::default();
        params.fee_cap_fraction = Decimal::new(1, 1); // 10%
        
        let rewards = RoundRewards::new(params);
        let capped_fees = rewards.apply_fee_cap(5_000, 10_000);
        
        assert_eq!(capped_fees, 1_000); // 10% of 10,000
    }

    #[test]
    fn test_empty_participation() {
        let rewards = RoundRewards::new(EmissionParams::default());
        let result = rewards.distribute_round_rewards(1, 10_000, vec![], 0);
        
        assert!(matches!(result, Err(DistributionError::NoValidators(1))));
    }
}
>>>>>>> 952abe6 (feat: Add ippan_economics crate and DAG-Fair emission framework)
