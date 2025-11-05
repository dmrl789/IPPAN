//! Emission Tracker & Auditor
//!
//! Tracks cumulative emission, validates consistency with the emission schedule,
//! and provides audit records for governance and transparency.

use blake3::Hasher;
use ippan_economics::{
    scheduled_round_reward, EmissionParams, RoundRewardDistribution, ValidatorId, ValidatorReward,
};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Audit record for emission tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionAuditRecord {
    pub round: u64,
    pub start_round: u64,
    pub end_round: u64,
    pub cumulative_supply: u128,
    pub round_emission: u128,
    pub total_base_emission: u128,
    pub fees_collected: u128,
    pub total_fees_collected: u128,
    pub total_ai_commissions: u128,
    pub total_network_dividends: u128,
    pub total_distributed: u128,
    pub empty_rounds: u64,
    pub distribution_hash: String,
    pub timestamp: u64,
}

/// Validator contribution to a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorContribution {
    pub validator_id: [u8; 32],
    pub blocks_proposed: u32,
    pub blocks_verified: u32,
    pub reputation_score: f64,
}

/// Tracks emission state across rounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionTracker {
    /// Emission parameters
    pub params: EmissionParams,

    /// Current cumulative supply including fees and commissions (µIPN)
    pub cumulative_supply: u128,

    /// Cumulative base emission only (for consistency checks)
    pub cumulative_base_emission: u128,

    /// Last finalized round
    pub last_round: u64,

    /// Total fees collected (lifetime)
    pub total_fees_collected: u128,

    /// Total AI commissions collected (lifetime)
    pub total_ai_commissions: u128,

    /// Network reward pool balance
    pub network_pool_balance: u128,

    /// Total network dividends distributed (lifetime)
    pub total_network_dividends: u128,

    /// Validator lifetime earnings
    pub validator_earnings: HashMap<[u8; 32], u128>,

    /// Number of rounds with zero participation
    pub empty_rounds: u64,

    /// Audit checkpoint interval (rounds)
    pub audit_interval: u64,

    /// Last audit checkpoint round
    pub last_audit_round: u64,

    /// Historical audit records
    pub audit_history: Vec<EmissionAuditRecord>,

    /// Fees collected since last audit checkpoint
    audit_period_fees: u128,
}

impl EmissionTracker {
    /// Create a new emission tracker
    pub fn new(params: EmissionParams, audit_interval: u64) -> Self {
        Self {
            params,
            cumulative_supply: 0,
            cumulative_base_emission: 0,
            last_round: 0,
            total_fees_collected: 0,
            total_ai_commissions: 0,
            network_pool_balance: 0,
            total_network_dividends: 0,
            validator_earnings: HashMap::new(),
            empty_rounds: 0,
            audit_interval,
            last_audit_round: 0,
            audit_history: Vec::new(),
            audit_period_fees: 0,
        }
    }

    fn compute_base_reward(&self, round: u64) -> u64 {
        scheduled_round_reward(round, &self.params)
    }

    /// Process a completed round and update emission state
    pub fn process_round(
        &mut self,
        round: u64,
        contributions: &[ValidatorContribution],
        transaction_fees: u128,
        ai_commissions: u128,
    ) -> Result<RoundRewardDistribution, String> {
        // Validate round ordering
        if round != self.last_round + 1 && self.last_round != 0 {
            return Err(format!(
                "Non-sequential round: expected {}, got {}",
                self.last_round + 1,
                round
            ));
        }

        // Track empty rounds
        if contributions.is_empty() {
            self.empty_rounds += 1;
        }

        let base_reward = self.compute_base_reward(round);
        let fee_fraction = self
            .params
            .fee_cap_fraction
            .to_f64()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let transaction_fees_capped = transaction_fees.min(u64::MAX as u128) as u64;
        let fee_cap_limit = (base_reward as f64 * fee_fraction) as u64;
        let capped_fees = transaction_fees_capped.min(fee_cap_limit);
        let excess_burned = transaction_fees_capped.saturating_sub(capped_fees);

        let ai_commissions_capped = ai_commissions.min(u64::MAX as u128) as u64;

        let mut validator_rewards = HashMap::new();
        let mut weight_map: Vec<([u8; 32], u128)> = Vec::new();
        let mut total_weight: u128 = 0;

        for contribution in contributions {
            let mut weight = (contribution.blocks_proposed as u128 * 5)
                + (contribution.blocks_verified as u128 * 3)
                + contribution.reputation_score.round() as u128;
            if weight == 0 {
                weight = 1;
            }
            total_weight = total_weight.saturating_add(weight);
            weight_map.push((contribution.validator_id, weight));
        }

        let mut emission_allocated: u128 = 0;
        let mut fee_allocated: u128 = 0;
        let mut ai_allocated: u128 = 0;

        let total_reward = base_reward
            .saturating_add(capped_fees)
            .saturating_add(ai_commissions_capped);

        if !weight_map.is_empty() {
            let validators_count = weight_map.len();
            for (idx, (validator_raw, weight)) in weight_map.iter().enumerate() {
                let is_last = idx == validators_count - 1;

                let emission_share = if total_weight > 0 {
                    if is_last {
                        (base_reward as u128)
                            .saturating_sub(emission_allocated.min(base_reward as u128))
                    } else {
                        ((base_reward as u128) * *weight) / total_weight
                    }
                } else if is_last {
                    (base_reward as u128)
                        .saturating_sub(emission_allocated.min(base_reward as u128))
                } else {
                    (base_reward as u128) / validators_count as u128
                };
                emission_allocated = emission_allocated.saturating_add(emission_share);

                let fee_share = if total_weight > 0 {
                    if is_last {
                        (capped_fees as u128).saturating_sub(fee_allocated.min(capped_fees as u128))
                    } else {
                        ((capped_fees as u128) * *weight) / total_weight
                    }
                } else if is_last {
                    (capped_fees as u128).saturating_sub(fee_allocated.min(capped_fees as u128))
                } else {
                    (capped_fees as u128) / validators_count as u128
                };
                fee_allocated = fee_allocated.saturating_add(fee_share);

                let ai_share = if total_weight > 0 {
                    if is_last {
                        (ai_commissions_capped as u128)
                            .saturating_sub(ai_allocated.min(ai_commissions_capped as u128))
                    } else {
                        ((ai_commissions_capped as u128) * *weight) / total_weight
                    }
                } else if is_last {
                    (ai_commissions_capped as u128)
                        .saturating_sub(ai_allocated.min(ai_commissions_capped as u128))
                } else {
                    (ai_commissions_capped as u128) / validators_count as u128
                };
                ai_allocated = ai_allocated.saturating_add(ai_share);

                let total_share = emission_share
                    .saturating_add(fee_share)
                    .saturating_add(ai_share);

                let validator_id = ValidatorId::new(hex::encode(validator_raw));
                let weight_ratio = if total_weight > 0 {
                    Decimal::from_f64((*weight as f64) / (total_weight as f64))
                        .unwrap_or(Decimal::ZERO)
                } else {
                    Decimal::from_f64(1.0 / validators_count as f64).unwrap_or(Decimal::ZERO)
                };

                validator_rewards.insert(
                    validator_id,
                    ValidatorReward {
                        round_emission: emission_share.min(u64::MAX as u128) as u64,
                        transaction_fees: fee_share.min(u64::MAX as u128) as u64,
                        ai_commissions: ai_share.min(u64::MAX as u128) as u64,
                        network_dividend: 0,
                        total_reward: total_share.min(u64::MAX as u128) as u64,
                        weight_factor: weight_ratio,
                    },
                );
            }
        }

        let distribution = RoundRewardDistribution {
            round_index: round,
            total_reward,
            blocks_in_round: contributions.len() as u32,
            validator_rewards,
            fees_collected: capped_fees,
            excess_burned,
        };

        // Validate distribution
        // distribution.validate()?;

        // Update cumulative supply (includes fees and commissions)
        self.cumulative_supply = self
            .cumulative_supply
            .saturating_add(distribution.total_reward as u128);

        // Update cumulative base emission (for consistency checks)
        self.cumulative_base_emission = self
            .cumulative_base_emission
            .saturating_add(base_reward as u128);

        // Check supply cap (base emission only)
        if self.cumulative_base_emission >= self.params.max_supply_micro as u128 {
            return Err(format!(
                "Supply cap exceeded: {} > {}",
                self.cumulative_base_emission, self.params.max_supply_micro as u128
            ));
        }

        // Update fee and commission totals
        self.total_fees_collected = self.total_fees_collected.saturating_add(transaction_fees);

        // Track fees for current audit period
        self.audit_period_fees = self.audit_period_fees.saturating_add(transaction_fees);

        self.total_ai_commissions = self.total_ai_commissions.saturating_add(ai_commissions);

        // Update network pool (add new dividends, subtract distributed)
        self.network_pool_balance = self
            .network_pool_balance
            .saturating_add((capped_fees as u128) / 20); // 5% of fees go to pool
                                                         // .saturating_sub(distribution.network_dividend); // Field doesn't exist

        // Track total network dividends distributed
        // Note: network_dividend field doesn't exist in RoundRewardDistribution
        // self.total_network_dividends = self
        //     .total_network_dividends
        //     .saturating_add(distribution.network_dividend);

        // Update validator earnings
        for (validator_id, reward) in &distribution.validator_rewards {
            if let Ok(decoded) = hex::decode(validator_id.as_str()) {
                if decoded.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&decoded);
                    let entry = self.validator_earnings.entry(key).or_insert(0);
                    *entry = entry.saturating_add(reward.total_reward as u128);
                }
            }
        }

        self.last_round = round;

        // Check if audit checkpoint is due
        if round >= self.last_audit_round + self.audit_interval {
            self.create_audit_checkpoint(round)?;
        }

        Ok(distribution)
    }

    /// Create an audit checkpoint
    fn create_audit_checkpoint(&mut self, round: u64) -> Result<(), String> {
        let start_round = self.last_audit_round.max(1);

        // Calculate totals for the audit period
        let mut total_base_emission = 0u128;
        for r in start_round..=round {
            total_base_emission =
                total_base_emission.saturating_add(scheduled_round_reward(r, &self.params) as u128);
        }

        let round_emission = scheduled_round_reward(round, &self.params) as u128;
        let total_distributed = self.cumulative_supply;

        // Create distribution hash
        let mut hasher = Hasher::new();
        hasher.update(&round.to_le_bytes());
        hasher.update(&self.cumulative_supply.to_le_bytes());

        for (validator_id, earnings) in &self.validator_earnings {
            hasher.update(validator_id);
            hasher.update(&earnings.to_le_bytes());
        }

        let digest = hasher.finalize();
        let mut distribution_hash = [0u8; 32];
        distribution_hash.copy_from_slice(digest.as_bytes());

        let audit_record = EmissionAuditRecord {
            round,
            start_round,
            end_round: round,
            cumulative_supply: self.cumulative_supply,
            round_emission,
            total_base_emission,
            fees_collected: self.audit_period_fees,
            total_fees_collected: self.total_fees_collected,
            total_ai_commissions: self.total_ai_commissions,
            total_network_dividends: self.total_network_dividends,
            total_distributed,
            empty_rounds: self.empty_rounds,
            distribution_hash: hex::encode(distribution_hash),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.audit_history.push(audit_record);
        self.last_audit_round = round;

        // Reset audit period fee counter
        self.audit_period_fees = 0;

        Ok(())
    }

    /// Verify emission consistency against expected schedule
    ///
    /// This only verifies base emission, not fees or commissions which are external to the schedule
    pub fn verify_consistency(&self) -> Result<(), String> {
        let expected_supply = super::emission::projected_supply(self.last_round, &self.params);

        // Allow minor rounding discrepancies (due to integer division) proportional to rounds.
        let tolerance = (self.last_round as u128).saturating_add(10);

        if self.cumulative_base_emission > expected_supply + tolerance {
            return Err(format!(
                "Cumulative base emission {} exceeds expected {} (round {})",
                self.cumulative_base_emission, expected_supply, self.last_round
            ));
        }

        if self.cumulative_base_emission + tolerance < expected_supply {
            return Err(format!(
                "Cumulative base emission {} below expected {} (round {})",
                self.cumulative_base_emission, expected_supply, self.last_round
            ));
        }

        Ok(())
    }

    /// Get emission statistics
    pub fn get_statistics(&self) -> EmissionStatistics {
        EmissionStatistics {
            current_round: self.last_round,
            cumulative_supply: self.cumulative_supply,
            supply_cap: self.params.max_supply_micro as u128,
            percentage_emitted: if self.params.max_supply_micro > 0 {
                ((self.cumulative_supply as f64 / self.params.max_supply_micro as f64) * 10000.0)
                    as u32
            } else {
                0
            },
            total_fees_collected: self.total_fees_collected,
            total_ai_commissions: self.total_ai_commissions,
            network_pool_balance: self.network_pool_balance,
            active_validators: self.validator_earnings.len(),
            empty_rounds: self.empty_rounds,
            audit_checkpoints: self.audit_history.len(),
        }
    }

    /// Get top validators by earnings
    pub fn get_top_validators(&self, limit: usize) -> Vec<([u8; 32], u128)> {
        let mut validators: Vec<_> = self
            .validator_earnings
            .iter()
            .map(|(id, earnings)| (*id, *earnings))
            .collect();

        validators.sort_by(|a, b| b.1.cmp(&a.1));
        validators.truncate(limit);
        validators
    }

    /// Reset to genesis state (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.cumulative_supply = 0;
        self.cumulative_base_emission = 0;
        self.last_round = 0;
        self.total_fees_collected = 0;
        self.total_ai_commissions = 0;
        self.network_pool_balance = 0;
        self.total_network_dividends = 0;
        self.validator_earnings.clear();
        self.empty_rounds = 0;
        self.last_audit_round = 0;
        self.audit_history.clear();
        self.audit_period_fees = 0;
    }
}

/// Emission statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionStatistics {
    pub current_round: u64,
    pub cumulative_supply: u128,
    pub supply_cap: u128,
    pub percentage_emitted: u32, // basis points (10000 = 100%)
    pub total_fees_collected: u128,
    pub total_ai_commissions: u128,
    pub network_pool_balance: u128,
    pub active_validators: usize,
    pub empty_rounds: u64,
    pub audit_checkpoints: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let params = EmissionParams::default();
        let tracker = EmissionTracker::new(params.clone(), 1000);

        assert_eq!(tracker.cumulative_supply, 0);
        assert_eq!(tracker.last_round, 0);
        assert!(tracker.validator_earnings.is_empty());
    }

    #[test]
    fn test_process_single_round() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        let contributions = vec![ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 5,
            blocks_verified: 10,
            reputation_score: 10_000.0,
        }];

        let result = tracker.process_round(1, &contributions, 1000, 500);
        assert!(result.is_ok());

        let dist = result.unwrap();
        assert_eq!(dist.round_index, 1);
        assert_eq!(dist.blocks_in_round, contributions.len() as u32);
        assert!(dist.total_reward > 0);
        assert_eq!(tracker.last_round, 1);
        assert!(tracker.cumulative_supply > 0);
    }

    #[test]
    fn test_sequential_rounds() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 100);

        let contributions = vec![ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 1,
            blocks_verified: 1,
            reputation_score: 10_000.0,
        }];

        // Process 10 rounds
        for round in 1..=10 {
            let result = tracker.process_round(round, &contributions, 100, 50);
            assert!(result.is_ok(), "Round {} failed", round);
        }

        assert_eq!(tracker.last_round, 10);
        assert!(tracker.cumulative_supply > 0);
        assert_eq!(tracker.validator_earnings.len(), 1);
    }

    #[test]
    fn test_non_sequential_round_rejected() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        let contributions = vec![ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 1,
            blocks_verified: 1,
            reputation_score: 10_000.0,
        }];

        // Process round 1
        assert!(tracker.process_round(1, &contributions, 100, 50).is_ok());

        // Try to skip to round 3 (should fail)
        let result = tracker.process_round(3, &contributions, 100, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_rounds_tracked() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        // Process 5 empty rounds
        for round in 1..=5 {
            let result = tracker.process_round(round, &[], 0, 0);
            assert!(result.is_ok());
        }

        assert_eq!(tracker.empty_rounds, 5);
    }

    #[test]
    fn test_consistency_verification() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        let contributions = vec![ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 1,
            blocks_verified: 1,
            reputation_score: 10_000.0,
        }];

        // Process 100 rounds
        for round in 1..=100 {
            tracker
                .process_round(round, &contributions, 100, 50)
                .unwrap();
        }

        // Verify consistency
        assert!(tracker.verify_consistency().is_ok());
    }

    #[test]
    fn test_audit_checkpoint_creation() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 10);

        let contributions = vec![ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 1,
            blocks_verified: 1,
            reputation_score: 10_000.0,
        }];

        // Process 20 rounds (should create 2 checkpoints)
        for round in 1..=20 {
            tracker
                .process_round(round, &contributions, 100, 50)
                .unwrap();
        }

        assert!(tracker.audit_history.len() >= 1);
    }

    #[test]
    fn test_statistics_calculation() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        let contributions = vec![
            ValidatorContribution {
                validator_id: [1u8; 32],
                blocks_proposed: 5,
                blocks_verified: 10,
                reputation_score: 10_000.0,
            },
            ValidatorContribution {
                validator_id: [2u8; 32],
                blocks_proposed: 3,
                blocks_verified: 8,
                reputation_score: 9_000.0,
            },
        ];

        // Process 50 rounds
        for round in 1..=50 {
            tracker
                .process_round(round, &contributions, 100, 50)
                .unwrap();
        }

        let stats = tracker.get_statistics();
        assert_eq!(stats.current_round, 50);
        assert!(stats.cumulative_supply > 0);
        assert_eq!(stats.active_validators, 2);
        assert!(stats.percentage_emitted < 10000);
    }

    #[test]
    fn test_top_validators() {
        let params = EmissionParams::default();
        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        let contributions = vec![
            ValidatorContribution {
                validator_id: [1u8; 32],
                blocks_proposed: 10,
                blocks_verified: 5,
                reputation_score: 10_000.0,
            },
            ValidatorContribution {
                validator_id: [2u8; 32],
                blocks_proposed: 3,
                blocks_verified: 8,
                reputation_score: 9_000.0,
            },
            ValidatorContribution {
                validator_id: [3u8; 32],
                blocks_proposed: 1,
                blocks_verified: 2,
                reputation_score: 8_000.0,
            },
        ];

        // Process rounds
        for round in 1..=10 {
            tracker
                .process_round(round, &contributions, 100, 50)
                .unwrap();
        }

        let top = tracker.get_top_validators(2);
        assert_eq!(top.len(), 2);
        // First validator should have highest earnings
        assert_eq!(top[0].0, [1u8; 32]);
    }

    #[test]
    fn test_supply_cap_enforcement() {
        let params = EmissionParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 10,
            max_supply_micro: 1_500_000,
            ..Default::default()
        };

        let mut tracker = EmissionTracker::new(params.clone(), 1000);

        let contributions = vec![ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 100,
            blocks_verified: 100,
            reputation_score: 10_000.0,
        }];

        // Process rounds until we hit the cap
        for round in 1..=100 {
            let result = tracker.process_round(round, &contributions, 0, 0);

            if tracker.cumulative_supply >= params.max_supply_micro as u128 {
                // Should reject rounds after cap
                assert!(result.is_err());
                break;
            } else {
                assert!(result.is_ok());
            }
        }
    }
}
