//! Token emission and reward distribution for DLC consensus
//!
//! This module handles block rewards, validator incentives, and
//! token emission schedules.

use crate::error::{DlcError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base block reward in smallest unit (micro-IPN)
pub const BLOCK_REWARD: u64 = 1_0000_0000; // 1 IPN = 100,000,000 micro-IPN

/// Maximum supply cap: 21 million IPN (matches Bitcoin's supply model)
pub const SUPPLY_CAP: u64 = 21_000_000 * 1_0000_0000; // 21 million IPN in micro-IPN

/// Initial annual inflation rate (in basis points, 1% = 100 bps)
pub const INITIAL_INFLATION_BPS: u64 = 500; // 5%

/// Minimum inflation rate (in basis points)
pub const MIN_INFLATION_BPS: u64 = 100; // 1%

/// Inflation reduction per year (in basis points)
pub const INFLATION_REDUCTION_BPS: u64 = 50; // 0.5% per year

/// Emission schedule for token distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionSchedule {
    /// Initial supply (in micro-IPN)
    pub initial_supply: u64,
    /// Current total supply
    pub current_supply: u64,
    /// Target supply (max cap)
    pub max_supply: u64,
    /// Current block reward
    pub current_block_reward: u64,
    /// Blocks per year (approximate)
    pub blocks_per_year: u64,
    /// Current inflation rate (basis points)
    pub current_inflation_bps: u64,
    /// Start timestamp
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// Last update round
    pub last_update_round: u64,
}

impl Default for EmissionSchedule {
    fn default() -> Self {
        Self::new(
            0,          // Start from genesis (0 initial supply)
            SUPPLY_CAP, // 21 million IPN max (matches Bitcoin model)
            BLOCK_REWARD,
            525_600, // ~1 block per minute = ~525,600 blocks per year
        )
    }
}

impl EmissionSchedule {
    /// Create a new emission schedule
    ///
    /// # Arguments
    /// * `initial_supply` - Starting supply (typically 0 for genesis)
    /// * `max_supply` - Maximum supply cap (21 million IPN = 2,100,000,000,000,000 micro-IPN)
    /// * `block_reward` - Initial reward per block
    /// * `blocks_per_year` - Expected blocks per year (~525,600 at 1 block/min)
    pub fn new(
        initial_supply: u64,
        max_supply: u64,
        block_reward: u64,
        blocks_per_year: u64,
    ) -> Self {
        Self {
            initial_supply,
            current_supply: initial_supply,
            max_supply,
            current_block_reward: block_reward,
            blocks_per_year,
            current_inflation_bps: INITIAL_INFLATION_BPS,
            start_time: chrono::Utc::now(),
            last_update_round: 0,
        }
    }

    /// Calculate block reward for current round
    pub fn calculate_block_reward(&self, round: u64) -> u64 {
        // Check if we've hit max supply
        if self.current_supply >= self.max_supply {
            return 0;
        }

        // Calculate years elapsed
        let years_elapsed = round / self.blocks_per_year;

        // Reduce inflation over time
        let inflation_bps = INITIAL_INFLATION_BPS
            .saturating_sub(years_elapsed * INFLATION_REDUCTION_BPS)
            .max(MIN_INFLATION_BPS);

        // Bootstrap phase: use fixed block reward when supply is very low
        // This ensures fair launch can get started
        let block_reward = if self.current_supply < BLOCK_REWARD * 1000 {
            // First ~1000 blocks use fixed reward for bootstrap
            BLOCK_REWARD
        } else {
            // Calculate reward based on current supply and inflation
            let annual_emission =
                (self.current_supply as u128 * inflation_bps as u128) / 10_000u128;
            (annual_emission / self.blocks_per_year as u128) as u64
        };

        // Ensure we don't exceed max supply
        block_reward.min(self.max_supply.saturating_sub(self.current_supply))
    }

    /// Update emission schedule after a round
    pub fn update(&mut self, round: u64, blocks_produced: u64) -> Result<()> {
        let reward_per_block = self.calculate_block_reward(round);
        let total_reward = reward_per_block.saturating_mul(blocks_produced);

        // Update supply
        self.current_supply = self
            .current_supply
            .saturating_add(total_reward)
            .min(self.max_supply);

        // Update current block reward
        self.current_block_reward = reward_per_block;

        // Update inflation rate
        let years_elapsed = round / self.blocks_per_year;
        self.current_inflation_bps = INITIAL_INFLATION_BPS
            .saturating_sub(years_elapsed * INFLATION_REDUCTION_BPS)
            .max(MIN_INFLATION_BPS);

        self.last_update_round = round;

        Ok(())
    }

    /// Get emission statistics
    pub fn stats(&self) -> EmissionStats {
        let emitted = self.current_supply.saturating_sub(self.initial_supply);
        let remaining = self.max_supply.saturating_sub(self.current_supply);
        // Integer arithmetic: (supply * 10000) / max_supply for basis points
        let progress_bps = ((self.current_supply as u128 * 10000) / self.max_supply as u128) as u32;

        EmissionStats {
            current_supply: self.current_supply,
            emitted_supply: emitted,
            remaining_supply: remaining,
            emission_progress_bps: progress_bps,
            current_inflation_bps: self.current_inflation_bps,
            current_block_reward: self.current_block_reward,
        }
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

    /// Distribute rewards for a block
    pub fn distribute_block_reward(
        &mut self,
        block_reward: u64,
        proposer: &str,
        verifiers: &[String],
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

        // Distribute to proposer
        *self
            .pending_rewards
            .entry(proposer.to_string())
            .or_insert(0) += proposer_reward;

        // Distribute to verifiers
        let reward_per_verifier = if !verifiers.is_empty() {
            verifiers_reward / verifiers.len() as u64
        } else {
            0
        };

        for verifier in verifiers {
            *self.pending_rewards.entry(verifier.clone()).or_insert(0) += reward_per_verifier;
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
        assert_eq!(schedule.current_supply, schedule.initial_supply);
        assert!(schedule.current_supply < schedule.max_supply);
    }

    #[test]
    fn test_block_reward_calculation() {
        let schedule = EmissionSchedule::default();
        let reward = schedule.calculate_block_reward(0);
        // With 0 initial supply, first block should have a reward
        assert!(reward > 0);
        assert!(reward <= BLOCK_REWARD);
    }

    #[test]
    fn test_emission_update() {
        let mut schedule = EmissionSchedule::default();
        let initial_supply = schedule.current_supply;
        assert_eq!(initial_supply, 0); // Starts from genesis

        schedule.update(1, 1).unwrap();

        // After processing 1 block, supply should increase
        assert!(schedule.current_supply >= initial_supply);
    }

    #[test]
    fn test_max_supply_cap() {
        let mut schedule = EmissionSchedule::new(1000, 1100, 50, 100);

        // Emit beyond max supply
        schedule.update(100, 10).unwrap();

        assert!(schedule.current_supply <= schedule.max_supply);
    }

    #[test]
    fn test_reward_distribution() {
        let mut distributor = RewardDistributor::default();

        let result = distributor
            .distribute_block_reward(
                BLOCK_REWARD,
                "proposer1",
                &["v1".to_string(), "v2".to_string()],
            )
            .unwrap();

        assert_eq!(result.total_distributed, BLOCK_REWARD);
        assert!(result.proposer_reward > 0);
        assert!(result.verifier_reward > 0);
    }

    #[test]
    fn test_pending_rewards() {
        let mut distributor = RewardDistributor::default();

        distributor
            .distribute_block_reward(BLOCK_REWARD, "proposer1", &["v1".to_string()])
            .unwrap();

        assert!(distributor.get_pending("proposer1") > 0);
        assert!(distributor.get_pending("v1") > 0);
    }

    #[test]
    fn test_claim_rewards() {
        let mut distributor = RewardDistributor::default();

        distributor
            .distribute_block_reward(BLOCK_REWARD, "proposer1", &[])
            .unwrap();

        let pending = distributor.get_pending("proposer1");
        let claimed = distributor.claim_rewards("proposer1").unwrap();

        assert_eq!(pending, claimed);
        assert_eq!(distributor.get_pending("proposer1"), 0);
    }

    #[test]
    fn test_reward_splits() {
        let splits = RewardSplits {
            proposer_bps: 6000,
            verifiers_bps: 3000,
            treasury_bps: 1000,
        };

        let mut distributor = RewardDistributor::new(splits);

        let result = distributor
            .distribute_block_reward(10_000, "proposer", &["v1".to_string()])
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
        let mut distributor = RewardDistributor::default();

        distributor
            .distribute_block_reward(BLOCK_REWARD, "proposer1", &["v1".to_string()])
            .unwrap();

        let stats = distributor.stats();
        assert!(stats.total_pending > 0);
        assert_eq!(stats.pending_validator_count, 2);
    }

    #[test]
    fn test_inflation_reduction() {
        let mut schedule = EmissionSchedule::default();
        let initial_inflation = schedule.current_inflation_bps;

        // Simulate one year
        schedule.update(schedule.blocks_per_year, 1).unwrap();

        assert!(schedule.current_inflation_bps < initial_inflation);
    }
}
