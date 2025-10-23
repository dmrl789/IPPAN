//! DAG-Fair Emission Module
//!
//! Deterministic round-based reward emission for IPPAN.
//! Includes halving logic, proposer/verifier reward split,
//! total supply projection, and fee recycling.
//!
//! All amounts use atomic IPN units with 24 decimal precision.

use ippan_types::{Amount, SUPPLY_CAP};
use serde::{Deserialize, Serialize};

/// Global emission parameters for IPPAN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionParams {
    /// Initial reward per round (in atomic IPN units with 24 decimal precision)
    pub r0: Amount,
    /// Number of rounds between halvings
    pub halving_rounds: u64,
    /// Supply cap (21 M IPN in atomic units)
    pub supply_cap: Amount,
    /// Proposer reward percentage (basis points; 20 % = 2000)
    pub proposer_bps: u16,
    /// Verifier reward percentage (basis points; 80 % = 8000)
    pub verifier_bps: u16,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            // ~50 IPN/day at 100 ms rounds with finalization
            r0: Amount::from_micro_ipn(10_000),
            // Halving ≈ every 2 years at 200 ms rounds (~315 M rounds)
            halving_rounds: 315_000_000,
            // 21 million IPN in atomic units
            supply_cap: Amount(SUPPLY_CAP),
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}

/// Compute per-round reward using a halving schedule
pub fn round_reward(round: u64, params: &EmissionParams) -> Amount {
    if round == 0 {
        return Amount::zero();
    }
    let halvings = (round / params.halving_rounds) as u32;
    if halvings >= 64 {
        return Amount::zero();
    }
    Amount(params.r0.atomic() >> halvings)
}

/// Distribution of rewards for a single round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    pub total: Amount,
    pub proposer_reward: Amount,
    pub verifier_pool: Amount,
    pub verifier_count: usize,
    pub per_verifier: Amount,
}

/// Compute reward distribution for a round
pub fn distribute_round_reward(
    round: u64,
    params: &EmissionParams,
    block_count: usize,
    verifier_count: usize,
) -> RoundRewardDistribution {
    let total = round_reward(round, params);
    if total.is_zero() || block_count == 0 {
        return RoundRewardDistribution {
            total: Amount::zero(),
            proposer_reward: Amount::zero(),
            verifier_pool: Amount::zero(),
            verifier_count: 0,
            per_verifier: Amount::zero(),
        };
    }

    let proposer_reward = total.percentage(params.proposer_bps);
    let verifier_pool = total.saturating_sub(proposer_reward);
    let per_verifier = if verifier_count > 0 {
        verifier_pool / verifier_count as u128
    } else {
        Amount::zero()
    };

    RoundRewardDistribution {
        total,
        proposer_reward,
        verifier_pool,
        verifier_count,
        per_verifier,
    }
}

/// Project total supply emitted after given number of rounds
pub fn projected_supply(rounds: u64, params: &EmissionParams) -> Amount {
    if rounds == 0 {
        return Amount::zero();
    }

    let mut total = Amount::zero();
    let mut halvings = 0u32;

    loop {
        let reward = if halvings >= 64 { 
            Amount::zero() 
        } else { 
            Amount(params.r0.atomic() >> halvings) 
        };
        if reward.is_zero() {
            break;
        }

        let start_round = (halvings as u64) * params.halving_rounds + 1;
        let end_round = ((halvings + 1) as u64) * params.halving_rounds;
        if start_round > rounds {
            break;
        }

        let effective_end = end_round.min(rounds);
        let count = (effective_end - start_round + 1) as u128;
        total = Amount(total.atomic().saturating_add(reward.atomic().saturating_mul(count)));

        halvings += 1;
    }

    if total > params.supply_cap {
        params.supply_cap
    } else {
        total
    }
}

/// Weekly fee-recycling parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingParams {
    /// Rounds per week (≈ 3 024 000 at 200 ms rounds)
    pub rounds_per_week: u64,
    /// Percentage of collected fees to recycle (basis points)
    pub recycle_bps: u16,
}

impl Default for FeeRecyclingParams {
    fn default() -> Self {
        Self {
            rounds_per_week: 3_024_000,
            recycle_bps: 10_000, // 100 %
        }
    }
}

/// Compute amount of fees to recycle back into reward pool
pub fn calculate_fee_recycling(collected_fees: Amount, params: &FeeRecyclingParams) -> Amount {
    collected_fees.percentage(params.recycle_bps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_reward_halving() {
        let params = EmissionParams {
            r0: Amount::from_atomic(10_000),
            halving_rounds: 1000,
            ..Default::default()
        };
        assert_eq!(round_reward(999, &params).atomic(), 10_000);
        assert_eq!(round_reward(1000, &params).atomic(), 5_000);
        assert_eq!(round_reward(2000, &params).atomic(), 2_500);
        assert_eq!(round_reward(3000, &params).atomic(), 1_250);
    }

    #[test]
    fn test_distribution() {
        let params = EmissionParams::default();
        let dist = distribute_round_reward(1, &params, 1, 4);
        assert_eq!(dist.total, Amount::from_micro_ipn(10_000));
        assert_eq!(dist.proposer_reward, Amount::from_micro_ipn(2_000));
        assert_eq!(dist.verifier_pool, Amount::from_micro_ipn(8_000));
        assert_eq!(dist.per_verifier, Amount::from_micro_ipn(2_000));
    }

    #[test]
    fn test_projected_supply_growth() {
        let params = EmissionParams {
            r0: Amount::from_atomic(10_000),
            halving_rounds: 1000,
            supply_cap: Amount(SUPPLY_CAP),
            proposer_bps: 2000,
            verifier_bps: 8000,
        };
        let s1 = projected_supply(1000, &params);
        let s2 = projected_supply(2000, &params);
        assert_eq!(s1.atomic(), 10_000 * 1000);
        assert_eq!(s2.atomic(), 10_000 * 1000 + 5_000 * 1000);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let params = EmissionParams {
            supply_cap: Amount::from_atomic(50_000),
            ..Default::default()
        };
        let supply = projected_supply(10_000_000, &params);
        assert!(supply <= params.supply_cap);
    }

    #[test]
    fn test_fee_recycling() {
        let params = FeeRecyclingParams::default();
        assert_eq!(calculate_fee_recycling(Amount::from_atomic(10_000), &params).atomic(), 10_000);
        let params_half = FeeRecyclingParams {
            recycle_bps: 5000,
            ..Default::default()
        };
        assert_eq!(calculate_fee_recycling(Amount::from_atomic(10_000), &params_half).atomic(), 5_000);
    }

    #[test]
    fn test_emission_converges() {
        let params = EmissionParams::default();
        let s10k = projected_supply(10_000, &params);
        let s100k = projected_supply(100_000, &params);
        assert!(s10k < s100k && s100k <= params.supply_cap);
    }

    #[test]
    fn test_ultra_fine_precision() {
        // Test yocto-IPN precision in reward distribution
        let micro_reward = Amount::from_str_ipn("0.0001").unwrap();
        let (per_block, remainder) = micro_reward.split(1000);
        
        // Each block should get exactly 0.0000001 IPN (100 nanoIPN)
        assert_eq!(per_block.atomic(), 100_000_000_000_000_000);
        assert_eq!(remainder.atomic(), 0);
        
        // Verify no loss in splitting
        let reconstructed = per_block * 1000 + remainder;
        assert_eq!(reconstructed, micro_reward);
    }

    #[test]
    fn test_atomic_precision_no_loss() {
        // Distribute a very small amount among many recipients
        let total = Amount::from_atomic(999_999_999_999_999_999_999_999);
        let (per_unit, remainder) = total.split(1_000_000);
        
        // Verify no rounding loss
        let reconstructed = per_unit * 1_000_000 + remainder;
        assert_eq!(reconstructed, total);
    }
}
