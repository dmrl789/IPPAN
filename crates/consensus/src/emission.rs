//! DAG-Fair Emission Module
//!
//! Deterministic round-based reward emission for IPPAN BlockDAG.
//!
//! ## Design Principles
//!
//! IPPAN produces thousands of micro-blocks per second in parallel rounds.
//! Instead of per-block rewards (which would inflate supply), emission is tied
//! to **rounds** — deterministic time windows (≈100–250 ms) that aggregate many blocks.
//!
//! Each round issues a **fixed reward** subdivided fairly among all participating
//! validators based on their contribution (blocks proposed/verified) and reputation.
//!
//! ## Key Features
//!
//! - **Total supply cap:** 21,000,000 IPN (hard cap)
//! - **Round-based emission:** Rewards per round, not per block
//! - **Halving schedule:** Every ~2 years (≈630M rounds at 100ms/round)
//! - **Fair distribution:** Proportional to participation, uptime, and reputation
//! - **Deterministic:** All nodes calculate identical rewards from HashTimer data
//! - **Auditable:** Full emission history trackable via round finalization records
//!
//! ## Emission Formula
//!
//! ```text
//! R(t) = R₀ / 2^⌊t / T_h⌋
//! ```
//!
//! Where:
//! - R₀ = 0.0001 IPN = 10,000 µIPN (initial reward per round)
//! - T_h = halving interval ≈ 630,000,000 rounds (2 years at 100ms rounds)
//! - t = current round number
//!
//! ## Distribution Within a Round
//!
//! Total round reward R(t) is distributed as:
//!
//! 1. **60%** - Base emission distributed by participation
//! 2. **25%** - Transaction fee rewards (from block fees)
//! 3. **10%** - AI micro-service commissions
//! 4. **5%** - Network reward pool dividend (weekly redistribution)
//!
//! Among validators in each round:
//! - Block proposers: higher weight (configurable, default 1.2×)
//! - Block verifiers: standard weight (1.0×)
//! - Weighted by reputation score (AI-based or stake-based)
//! - Adjusted for uptime and participation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Global emission parameters for IPPAN DAG-Fair model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionParams {
    /// Initial reward per round (in µIPN — micro-IPN, where 1 IPN = 10^8 µIPN)
    /// Default: 10,000 µIPN = 0.0001 IPN per round
    pub r0: u128,

    /// Number of rounds between halvings
    /// Default: 630,000,000 rounds ≈ 2 years at 100ms/round
    pub halving_rounds: u64,

    /// Supply cap (e.g. 21 M IPN = 21,000,000 × 10^8 µIPN)
    pub supply_cap: u128,

    /// Proposer reward weight multiplier (basis points; 12000 = 1.2×)
    pub proposer_weight_bps: u16,

    /// Verifier reward weight multiplier (basis points; 10000 = 1.0×)
    pub verifier_weight_bps: u16,

    /// Base emission share (basis points; 6000 = 60%)
    pub base_emission_bps: u16,

    /// Transaction fee share (basis points; 2500 = 25%)
    pub fee_share_bps: u16,

    /// AI service commission share (basis points; 1000 = 10%)
    pub ai_commission_bps: u16,

    /// Network reward pool dividend share (basis points; 500 = 5%)
    pub network_pool_bps: u16,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            // 0.0001 IPN per round = 10,000 µIPN
            // At 10 rounds/sec (100ms each), this gives:
            // - 10 rounds/sec × 10,000 µIPN = 100,000 µIPN/sec = 0.001 IPN/sec
            // - 0.001 IPN/sec × 86,400 sec/day ≈ 86.4 IPN/day
            // - 86.4 IPN/day × 365 days ≈ 31,536 IPN/year initially
            r0: 10_000,

            // Halving every ~2 years
            // At 100ms rounds: 10 rounds/sec × 60 sec × 60 min × 24 hr × 365 days × 2 years
            // = 10 × 31,536,000 × 2 = 630,720,000 rounds
            halving_rounds: 630_720_000,

            // 21 million IPN cap (in µIPN: 21M × 10^8)
            supply_cap: 21_000_000_00000000,

            // Proposer gets 1.2× weight (20% bonus)
            proposer_weight_bps: 12000,

            // Verifier gets 1.0× weight (baseline)
            verifier_weight_bps: 10000,

            // Distribution shares (must sum to 10000 = 100%)
            base_emission_bps: 6000,  // 60%
            fee_share_bps: 2500,      // 25%
            ai_commission_bps: 1000,  // 10%
            network_pool_bps: 500,    // 5%
        }
    }
}

impl EmissionParams {
    /// Validate that the emission parameters are consistent
    pub fn validate(&self) -> Result<(), String> {
        // Check that distribution shares sum to 100%
        let total_bps = self.base_emission_bps
            + self.fee_share_bps
            + self.ai_commission_bps
            + self.network_pool_bps;

        if total_bps != 10000 {
            return Err(format!(
                "Distribution shares must sum to 10000 basis points (100%), got {}",
                total_bps
            ));
        }

        if self.r0 == 0 {
            return Err("Initial reward (r0) must be positive".to_string());
        }

        if self.halving_rounds == 0 {
            return Err("Halving rounds must be positive".to_string());
        }

        if self.supply_cap == 0 {
            return Err("Supply cap must be positive".to_string());
        }

        Ok(())
    }

    /// Calculate the expected total emission over a given period
    pub fn expected_annual_emission(&self, rounds_per_year: u64) -> u128 {
        projected_supply(rounds_per_year, self)
    }
}

/// Compute per-round reward using halving schedule
///
/// # Formula
///
/// ```text
/// R(t) = R₀ / 2^halvings
/// ```
///
/// where `halvings = ⌊t / halving_rounds⌋`
///
/// # Arguments
///
/// * `round` - Current round number (0-based)
/// * `params` - Emission parameters
///
/// # Returns
///
/// Total reward for the round in µIPN
pub fn round_reward(round: u64, params: &EmissionParams) -> u128 {
    if round == 0 {
        return 0;
    }

    let halvings = (round / params.halving_rounds) as u32;

    // After 64 halvings, reward becomes negligible (effectively 0)
    if halvings >= 64 {
        return 0;
    }

    // Right-shift is equivalent to division by 2^halvings
    params.r0 >> halvings
}

/// Validator role in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorRole {
    /// Validator proposed one or more blocks
    Proposer,
    /// Validator verified blocks
    Verifier,
}

/// Contribution of a single validator in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorContribution {
    /// Validator's public key / address
    pub validator_id: [u8; 32],

    /// Number of blocks proposed by this validator
    pub blocks_proposed: usize,

    /// Number of blocks verified by this validator
    pub blocks_verified: usize,

    /// Reputation score (0-10000, where 10000 = 100% reputation)
    pub reputation_score: u32,

    /// Uptime factor (0-10000, where 10000 = 100% uptime)
    pub uptime_factor: u32,
}

impl ValidatorContribution {
    /// Calculate the weighted contribution score
    ///
    /// Score = (proposed × proposer_weight + verified × verifier_weight)
    ///       × (reputation / 10000) × (uptime / 10000)
    pub fn weighted_score(&self, params: &EmissionParams) -> u128 {
        let proposed_weight = (self.blocks_proposed as u128)
            .saturating_mul(params.proposer_weight_bps as u128);

        let verified_weight = (self.blocks_verified as u128)
            .saturating_mul(params.verifier_weight_bps as u128);

        let raw_score = proposed_weight.saturating_add(verified_weight);

        // Apply reputation multiplier (scaled by 10000)
        let rep_adjusted = raw_score
            .saturating_mul(self.reputation_score as u128)
            .saturating_div(10000);

        // Apply uptime multiplier (scaled by 10000)
        rep_adjusted
            .saturating_mul(self.uptime_factor as u128)
            .saturating_div(10000)
    }
}

/// Distribution of rewards for a single round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    /// Round number
    pub round: u64,

    /// Total base emission for this round (µIPN)
    pub total_base_emission: u128,

    /// Transaction fees collected this round (µIPN)
    pub transaction_fees: u128,

    /// AI service commissions collected this round (µIPN)
    pub ai_commissions: u128,

    /// Network pool dividend for this round (µIPN)
    pub network_dividend: u128,

    /// Total reward distributed (sum of all components)
    pub total_distributed: u128,

    /// Number of blocks in this round
    pub block_count: usize,

    /// Number of unique validators who participated
    pub validator_count: usize,

    /// Per-validator rewards
    pub validator_rewards: HashMap<[u8; 32], u128>,

    /// Total weighted score across all validators
    pub total_weighted_score: u128,
}

impl RoundRewardDistribution {
    /// Verify that distribution is complete and consistent
    pub fn validate(&self) -> Result<(), String> {
        let sum: u128 = self.validator_rewards.values().sum();

        if sum > self.total_distributed {
            return Err(format!(
                "Sum of validator rewards ({}) exceeds total distributed ({})",
                sum, self.total_distributed
            ));
        }

        // Allow some rounding loss (up to validator count µIPN)
        let max_loss = self.validator_count as u128;
        let diff = self.total_distributed.saturating_sub(sum);

        if diff > max_loss {
            return Err(format!(
                "Distribution loss ({}) exceeds maximum allowed ({})",
                diff, max_loss
            ));
        }

        Ok(())
    }
}

/// Compute reward distribution for a round
///
/// # Arguments
///
/// * `round` - Current round number
/// * `params` - Emission parameters
/// * `contributions` - List of validator contributions
/// * `transaction_fees` - Total transaction fees collected in this round (µIPN)
/// * `ai_commissions` - Total AI service commissions in this round (µIPN)
/// * `network_pool_balance` - Current network reward pool balance (µIPN)
///
/// # Returns
///
/// Complete breakdown of reward distribution
pub fn distribute_round_reward(
    round: u64,
    params: &EmissionParams,
    contributions: &[ValidatorContribution],
    transaction_fees: u128,
    ai_commissions: u128,
    network_pool_balance: u128,
) -> RoundRewardDistribution {
    // Calculate base emission for this round
    let base_emission = round_reward(round, params);

    // Calculate component rewards
    let base_reward = (base_emission * params.base_emission_bps as u128) / 10_000;
    let fee_reward = (transaction_fees * params.fee_share_bps as u128) / 10_000;
    let ai_reward = (ai_commissions * params.ai_commission_bps as u128) / 10_000;
    let network_reward = (network_pool_balance * params.network_pool_bps as u128) / 10_000;

    let total_distributed = base_reward
        .saturating_add(fee_reward)
        .saturating_add(ai_reward)
        .saturating_add(network_reward);

    // If no contributions, return empty distribution with zero rewards
    if contributions.is_empty() {
        return RoundRewardDistribution {
            round,
            total_base_emission: base_emission,
            transaction_fees,
            ai_commissions,
            network_dividend: 0, // No dividend if no contributions
            total_distributed: 0, // No distribution if no work done
            block_count: 0,
            validator_count: 0,
            validator_rewards: HashMap::new(),
            total_weighted_score: 0,
        };
    }
    
    // If total distributed is zero, return empty distribution
    if total_distributed == 0 {
        return RoundRewardDistribution {
            round,
            total_base_emission: base_emission,
            transaction_fees,
            ai_commissions,
            network_dividend: network_reward,
            total_distributed: 0,
            block_count: 0,
            validator_count: 0,
            validator_rewards: HashMap::new(),
            total_weighted_score: 0,
        };
    }

    // Calculate total weighted score
    let total_weighted_score: u128 = contributions
        .iter()
        .map(|c| c.weighted_score(params))
        .sum();

    // Distribute rewards proportionally
    let mut validator_rewards = HashMap::new();

    if total_weighted_score > 0 {
        for contribution in contributions {
            let validator_score = contribution.weighted_score(params);
            let validator_reward = total_distributed
                .saturating_mul(validator_score)
                .saturating_div(total_weighted_score);

            *validator_rewards.entry(contribution.validator_id).or_insert(0) += validator_reward;
        }
    }

    // Count total blocks
    let block_count: usize = contributions
        .iter()
        .map(|c| c.blocks_proposed + c.blocks_verified)
        .sum();

    RoundRewardDistribution {
        round,
        total_base_emission: base_emission,
        transaction_fees,
        ai_commissions,
        network_dividend: network_reward,
        total_distributed,
        block_count,
        validator_count: contributions.len(),
        validator_rewards,
        total_weighted_score,
    }
}

/// Project total supply emitted after given number of rounds
///
/// Uses geometric series to calculate cumulative emission:
///
/// ```text
/// Total = Σ(R₀ / 2^i × rounds_per_halving_period)
/// ```
///
/// # Arguments
///
/// * `rounds` - Number of rounds to project
/// * `params` - Emission parameters
///
/// # Returns
///
/// Projected total supply in µIPN, capped at supply_cap
pub fn projected_supply(rounds: u64, params: &EmissionParams) -> u128 {
    if rounds == 0 {
        return 0;
    }

    let mut total = 0u128;
    let mut halvings = 0u32;

    loop {
        let reward = if halvings >= 64 { 0 } else { params.r0 >> halvings };

        if reward == 0 {
            break;
        }

        let start_round = (halvings as u64) * params.halving_rounds + 1;
        let end_round = ((halvings + 1) as u64) * params.halving_rounds;

        if start_round > rounds {
            break;
        }

        let effective_end = end_round.min(rounds);
        let count = (effective_end - start_round + 1) as u128;
        total = total.saturating_add(reward.saturating_mul(count));

        halvings += 1;
    }

    total.min(params.supply_cap)
}

/// Calculate the round number when supply cap is reached (or projected to be reached)
pub fn rounds_until_cap(params: &EmissionParams) -> u64 {
    // Binary search for the round where supply >= cap
    let mut low = 0u64;
    let mut high = params.halving_rounds * 100; // Max 100 halvings

    while low < high {
        let mid = (low + high) / 2;
        let supply = projected_supply(mid, params);

        if supply >= params.supply_cap {
            high = mid;
        } else {
            low = mid + 1;
        }
    }

    low
}

/// Weekly fee-recycling parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingParams {
    /// Rounds per week (≈ 6,048,000 at 100 ms rounds)
    pub rounds_per_week: u64,

    /// Percentage of collected fees to recycle (basis points; 10000 = 100%)
    pub recycle_bps: u16,
}

impl Default for FeeRecyclingParams {
    fn default() -> Self {
        Self {
            // At 100ms per round: 10 rounds/sec × 60 × 60 × 24 × 7
            rounds_per_week: 6_048_000,

            // Recycle 100% of fees back into reward pool
            recycle_bps: 10_000,
        }
    }
}

/// Compute amount of fees to recycle back into reward pool
pub fn calculate_fee_recycling(collected_fees: u128, params: &FeeRecyclingParams) -> u128 {
    (collected_fees * params.recycle_bps as u128) / 10_000
}

/// Emission audit record for transparency and governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionAuditRecord {
    /// Starting round of audit period
    pub start_round: u64,

    /// Ending round of audit period
    pub end_round: u64,

    /// Total base emission during period (µIPN)
    pub total_base_emission: u128,

    /// Total transaction fees collected (µIPN)
    pub total_fees_collected: u128,

    /// Total AI commissions collected (µIPN)
    pub total_ai_commissions: u128,

    /// Total network dividends distributed (µIPN)
    pub total_network_dividends: u128,

    /// Total rewards distributed to validators (µIPN)
    pub total_distributed: u128,

    /// Cumulative supply at end of period (µIPN)
    pub cumulative_supply: u128,

    /// Number of rounds with zero participation
    pub empty_rounds: u64,

    /// Hash of all distribution records (for verification)
    pub distribution_hash: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params_valid() {
        let params = EmissionParams::default();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_round_reward_halving() {
        let params = EmissionParams {
            r0: 10_000,
            halving_rounds: 1000,
            ..Default::default()
        };

        // First epoch
        assert_eq!(round_reward(1, &params), 10_000);
        assert_eq!(round_reward(999, &params), 10_000);

        // First halving
        assert_eq!(round_reward(1000, &params), 5_000);
        assert_eq!(round_reward(1999, &params), 5_000);

        // Second halving
        assert_eq!(round_reward(2000, &params), 2_500);

        // Third halving
        assert_eq!(round_reward(3000, &params), 1_250);
    }

    #[test]
    fn test_distribution_with_contributions() {
        let params = EmissionParams::default();

        let contributions = vec![
            ValidatorContribution {
                validator_id: [1u8; 32],
                blocks_proposed: 5,
                blocks_verified: 10,
                reputation_score: 8000, // 80% reputation
                uptime_factor: 10000,   // 100% uptime
            },
            ValidatorContribution {
                validator_id: [2u8; 32],
                blocks_proposed: 3,
                blocks_verified: 8,
                reputation_score: 9000, // 90% reputation
                uptime_factor: 9500,    // 95% uptime
            },
        ];

        let dist = distribute_round_reward(100, &params, &contributions, 1000, 500, 10_000);

        assert!(dist.total_distributed > 0);
        assert_eq!(dist.validator_count, 2);
        assert!(dist.validator_rewards.contains_key(&[1u8; 32]));
        assert!(dist.validator_rewards.contains_key(&[2u8; 32]));
        assert!(dist.validate().is_ok());
    }

    #[test]
    fn test_distribution_empty() {
        let params = EmissionParams::default();
        let dist = distribute_round_reward(100, &params, &[], 0, 0, 0);

        assert_eq!(dist.total_distributed, 0);
        assert_eq!(dist.validator_count, 0);
        assert!(dist.validator_rewards.is_empty());
    }

    #[test]
    fn test_projected_supply_growth() {
        let params = EmissionParams {
            r0: 10_000,
            halving_rounds: 1000,
            supply_cap: 21_000_000_00000000,
            ..Default::default()
        };

        let s1 = projected_supply(1000, &params);
        let s2 = projected_supply(2000, &params);

        // After first halving period: 1000 rounds × 10,000 µIPN
        assert_eq!(s1, 10_000 * 1000);

        // After second halving period: first period + (1000 rounds × 5,000 µIPN)
        assert_eq!(s2, 10_000 * 1000 + 5_000 * 1000);

        // Verify monotonic increase
        assert!(s2 > s1);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let params = EmissionParams {
            r0: 1_000_000,
            halving_rounds: 100,
            supply_cap: 50_000,
            ..Default::default()
        };

        let supply = projected_supply(10_000_000, &params);
        assert!(supply <= params.supply_cap);
        assert_eq!(supply, params.supply_cap);
    }

    #[test]
    fn test_fee_recycling() {
        let params = FeeRecyclingParams::default();
        assert_eq!(calculate_fee_recycling(10_000, &params), 10_000);

        let params_half = FeeRecyclingParams {
            recycle_bps: 5000,
            ..Default::default()
        };
        assert_eq!(calculate_fee_recycling(10_000, &params_half), 5_000);
    }

    #[test]
    fn test_emission_converges() {
        let params = EmissionParams::default();
        let s10k = projected_supply(10_000, &params);
        let s100k = projected_supply(100_000, &params);
        let s1m = projected_supply(1_000_000, &params);

        assert!(s10k < s100k);
        assert!(s100k < s1m);
        assert!(s1m <= params.supply_cap);
    }

    #[test]
    fn test_weighted_score_calculation() {
        let params = EmissionParams::default();

        let contribution = ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 10,
            blocks_verified: 20,
            reputation_score: 8000,  // 80%
            uptime_factor: 10000,    // 100%
        };

        let score = contribution.weighted_score(&params);

        // Expected: (10 × 12000 + 20 × 10000) × 0.8 × 1.0
        //         = (120000 + 200000) × 0.8
        //         = 320000 × 0.8 = 256000
        assert_eq!(score, 256_000);
    }

    #[test]
    fn test_rounds_until_cap() {
        let params = EmissionParams {
            r0: 10_000,
            halving_rounds: 1000,
            supply_cap: 20_000_000, // Small cap for testing
            ..Default::default()
        };

        let rounds = rounds_until_cap(&params);
        let supply = projected_supply(rounds, &params);

        // The supply should be at or very close to the cap
        // Allow for the fact that we might overshoot slightly on the last round
        assert!(
            supply >= params.supply_cap || (params.supply_cap - supply) <= params.r0,
            "Supply {} should be >= cap {} or within one round reward",
            supply,
            params.supply_cap
        );
        assert!(rounds > 0);
    }

    #[test]
    fn test_default_emission_schedule() {
        let params = EmissionParams::default();

        // Year 1 (assuming 10 rounds/sec, 100ms each)
        let rounds_per_year = 10 * 60 * 60 * 24 * 365; // ~315,360,000
        let year1 = projected_supply(rounds_per_year, &params);

        // Should emit roughly 31,536 IPN in year 1 (before first halving)
        // 10,000 µIPN/round × 315,360,000 rounds ≈ 3,153,600,000,000 µIPN = 31,536 IPN
        assert!(year1 > 30_000_00000000); // > 30,000 IPN
        assert!(year1 < 33_000_00000000); // < 33,000 IPN
    }

    #[test]
    fn test_reputation_impact() {
        let params = EmissionParams::default();

        let high_rep = ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 10,
            blocks_verified: 10,
            reputation_score: 10000, // 100%
            uptime_factor: 10000,
        };

        let low_rep = ValidatorContribution {
            validator_id: [2u8; 32],
            blocks_proposed: 10,
            blocks_verified: 10,
            reputation_score: 5000, // 50%
            uptime_factor: 10000,
        };

        let high_score = high_rep.weighted_score(&params);
        let low_score = low_rep.weighted_score(&params);

        // Low reputation should result in half the score
        assert_eq!(low_score, high_score / 2);
    }
}
