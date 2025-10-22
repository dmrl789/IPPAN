/// DAG-Fair emission module for round-based rewards
///
/// This module implements a deterministic emission schedule with:
/// - Round-based rewards with halvings
/// - Split distribution between proposers and verifiers
/// - Fee recycling to reward pool
use serde::{Deserialize, Serialize};

/// Emission parameters for the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionParams {
    /// Initial reward per round (in µIPN - micro IPN)
    pub r0: u128,
    /// Number of rounds between halvings
    pub halving_rounds: u64,
    /// Supply cap (21M IPN = 21M * 10^8 µIPN)
    pub supply_cap: u128,
    /// Proposer reward percentage (20% = 2000 basis points)
    pub proposer_bps: u16,
    /// Verifier reward percentage (80% = 8000 basis points)
    pub verifier_bps: u16,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            // Start with 10,000 µIPN per round (~50 IPN/day at 100ms rounds with finalization)
            r0: 10_000,
            // Halving approximately every 2 years (assuming ~200ms per round avg)
            // 2 years ≈ 63M seconds ≈ 315M rounds at 200ms
            halving_rounds: 315_000_000,
            // 21 million IPN with 8 decimals
            supply_cap: 21_000_000_00000000,
            // 20% to proposer
            proposer_bps: 2000,
            // 80% to verifiers
            verifier_bps: 8000,
        }
    }
}

/// Calculate reward for a given round using halving schedule
///
/// # Arguments
/// * `round` - Current round number
/// * `params` - Emission parameters
///
/// # Returns
/// Total reward for the round in µIPN
pub fn round_reward(round: u64, params: &EmissionParams) -> u128 {
    if round == 0 {
        return 0;
    }

    let halvings = (round / params.halving_rounds) as u32;

    // After ~64 halvings (depends on r0), reward becomes 0
    if halvings >= 64 {
        return 0;
    }

    params.r0 >> halvings
}

/// Distribution of rewards for a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    /// Total reward pool for this round
    pub total: u128,
    /// Reward to block proposer
    pub proposer_reward: u128,
    /// Reward pool for verifiers (split equally)
    pub verifier_pool: u128,
    /// Number of verifiers to split the pool
    pub verifier_count: usize,
    /// Reward per verifier
    pub per_verifier: u128,
}

/// Calculate reward distribution for a round
///
/// # Arguments
/// * `round` - Current round number
/// * `params` - Emission parameters
/// * `block_count` - Number of blocks in the round (for multi-block rounds)
/// * `verifier_count` - Number of active verifiers in the round
///
/// # Returns
/// Reward distribution breakdown
pub fn distribute_round_reward(
    round: u64,
    params: &EmissionParams,
    block_count: usize,
    verifier_count: usize,
) -> RoundRewardDistribution {
    let total = round_reward(round, params);

    if total == 0 || block_count == 0 {
        return RoundRewardDistribution {
            total: 0,
            proposer_reward: 0,
            verifier_pool: 0,
            verifier_count: 0,
            per_verifier: 0,
        };
    }

    // Calculate proposer reward (20%)
    let proposer_reward = (total * params.proposer_bps as u128) / 10000;

    // Remaining goes to verifiers (80%)
    let verifier_pool = total - proposer_reward;

    // Split verifier pool equally among active verifiers
    let per_verifier = if verifier_count > 0 {
        verifier_pool / verifier_count as u128
    } else {
        0
    };

    RoundRewardDistribution {
        total,
        proposer_reward,
        verifier_pool,
        verifier_count,
        per_verifier,
    }
}

/// Calculate total supply after a given number of rounds
///
/// This is useful for projection and validation
///
/// # Arguments
/// * `rounds` - Number of rounds elapsed
/// * `params` - Emission parameters
///
/// # Returns
/// Total supply emitted in µIPN
pub fn projected_supply(rounds: u64, params: &EmissionParams) -> u128 {
    if rounds == 0 {
        return 0;
    }

    let mut total = 0u128;
    let mut halvings = 0u32;

    loop {
        let reward_at_level = if halvings >= 64 {
            0
        } else {
            params.r0 >> halvings
        };

        if reward_at_level == 0 {
            break;
        }

        // Calculate range for this halving level
        let start_round = (halvings as u64) * params.halving_rounds + 1;
        let end_round = ((halvings + 1) as u64) * params.halving_rounds;

        if start_round > rounds {
            break;
        }

        let effective_end = end_round.min(rounds);
        let rounds_at_level = (effective_end - start_round + 1) as u128;

        total = total.saturating_add(reward_at_level.saturating_mul(rounds_at_level));

        halvings += 1;
    }

    total.min(params.supply_cap)
}

/// Weekly fee recycling parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingParams {
    /// Rounds per week (assuming ~200ms per round)
    pub rounds_per_week: u64,
    /// Percentage of collected fees to recycle (10000 = 100%)
    pub recycle_bps: u16,
}

impl Default for FeeRecyclingParams {
    fn default() -> Self {
        Self {
            // 1 week ≈ 604800 seconds ≈ 3,024,000 rounds at 200ms
            rounds_per_week: 3_024_000,
            // Recycle 100% of fees
            recycle_bps: 10000,
        }
    }
}

/// Calculate fee recycling amount
///
/// # Arguments
/// * `collected_fees` - Total fees collected in the period
/// * `params` - Fee recycling parameters
///
/// # Returns
/// Amount to recycle to reward pool
pub fn calculate_fee_recycling(collected_fees: u128, params: &FeeRecyclingParams) -> u128 {
    (collected_fees * params.recycle_bps as u128) / 10000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_reward_initial() {
        let params = EmissionParams::default();
        assert_eq!(round_reward(1, &params), 10_000);
        assert_eq!(round_reward(100, &params), 10_000);
    }

    #[test]
    fn test_round_reward_halving() {
        let params = EmissionParams {
            r0: 10_000,
            halving_rounds: 1000,
            ..Default::default()
        };

        // Before first halving
        assert_eq!(round_reward(999, &params), 10_000);

        // After first halving
        assert_eq!(round_reward(1000, &params), 5_000);
        assert_eq!(round_reward(1500, &params), 5_000);

        // After second halving
        assert_eq!(round_reward(2000, &params), 2_500);

        // After third halving
        assert_eq!(round_reward(3000, &params), 1_250);
    }

    #[test]
    fn test_round_reward_genesis() {
        let params = EmissionParams::default();
        assert_eq!(round_reward(0, &params), 0);
    }

    #[test]
    fn test_distribute_round_reward() {
        let params = EmissionParams::default();
        let dist = distribute_round_reward(1, &params, 1, 4);

        assert_eq!(dist.total, 10_000);
        assert_eq!(dist.proposer_reward, 2_000); // 20%
        assert_eq!(dist.verifier_pool, 8_000); // 80%
        assert_eq!(dist.verifier_count, 4);
        assert_eq!(dist.per_verifier, 2_000); // 8000 / 4
    }

    #[test]
    fn test_distribute_no_verifiers() {
        let params = EmissionParams::default();
        let dist = distribute_round_reward(1, &params, 1, 0);

        assert_eq!(dist.total, 10_000);
        assert_eq!(dist.proposer_reward, 2_000);
        assert_eq!(dist.verifier_pool, 8_000);
        assert_eq!(dist.per_verifier, 0);
    }

    #[test]
    fn test_distribute_no_blocks() {
        let params = EmissionParams::default();
        let dist = distribute_round_reward(1, &params, 0, 4);

        assert_eq!(dist.total, 0);
        assert_eq!(dist.proposer_reward, 0);
        assert_eq!(dist.verifier_pool, 0);
    }

    #[test]
    fn test_projected_supply() {
        let params = EmissionParams {
            r0: 10_000,
            halving_rounds: 1000,
            supply_cap: 21_000_000_00000000,
            proposer_bps: 2000,
            verifier_bps: 8000,
        };

        // First 1000 rounds
        let supply_1k = projected_supply(1000, &params);
        assert_eq!(supply_1k, 10_000 * 1000);

        // Include first halving
        let supply_2k = projected_supply(2000, &params);
        assert_eq!(supply_2k, 10_000 * 1000 + 5_000 * 1000);
    }

    #[test]
    fn test_projected_supply_respects_cap() {
        let params = EmissionParams {
            r0: 10_000,
            halving_rounds: 100,
            supply_cap: 50_000, // Very low cap
            proposer_bps: 2000,
            verifier_bps: 8000,
        };

        let supply = projected_supply(1000, &params);
        assert!(supply <= params.supply_cap);
    }

    #[test]
    fn test_fee_recycling() {
        let params = FeeRecyclingParams::default();

        // 100% recycling
        assert_eq!(calculate_fee_recycling(10_000, &params), 10_000);

        // 50% recycling
        let params_50 = FeeRecyclingParams {
            recycle_bps: 5000,
            ..Default::default()
        };
        assert_eq!(calculate_fee_recycling(10_000, &params_50), 5_000);
    }

    #[test]
    fn test_emission_convergence() {
        let params = EmissionParams::default();

        // Calculate supply at various points
        let supply_10k = projected_supply(10_000, &params);
        let supply_100k = projected_supply(100_000, &params);
        let supply_1m = projected_supply(1_000_000, &params);

        // Supply should increase but stay under cap
        assert!(supply_10k < supply_100k);
        assert!(supply_100k < supply_1m);
        assert!(supply_1m <= params.supply_cap);
    }

    #[test]
    fn test_reward_eventually_zero() {
        let params = EmissionParams {
            r0: 1000,
            halving_rounds: 100,
            ..Default::default()
        };

        // After enough halvings, reward should be 0
        let far_future_round = 100 * 65; // 65 halvings
        assert_eq!(round_reward(far_future_round, &params), 0);
    }
}
