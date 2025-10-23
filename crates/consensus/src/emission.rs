//! DAG-Fair Emission Module
//!
//! Deterministic round-based reward emission for IPPAN.
//! Includes halving logic, proposer/verifier reward split,
//! total supply projection, and fee recycling.

use ippan_types::{AtomicIPN, IPNAmount, IPNUnit};
use serde::{Deserialize, Serialize};

/// Global emission parameters for IPPAN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionParams {
    /// Initial reward per round (in atomic units)
    pub r0: AtomicIPN,
    /// Number of rounds between halvings
    pub halving_rounds: u64,
    /// Supply cap (e.g. 21 M IPN = 21 000 000 × 10²⁴ atomic units)
    pub supply_cap: AtomicIPN,
    /// Proposer reward percentage (basis points; 20 % = 2000)
    pub proposer_bps: u16,
    /// Verifier reward percentage (basis points; 80 % = 8000)
    pub verifier_bps: u16,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            // ~50 IPN/day at 100 ms rounds with finalization
            r0: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),
            // Halving ≈ every 2 years at 200 ms rounds (~315 M rounds)
            halving_rounds: 315_000_000,
            // 21 million IPN × 10²⁴ atomic units
            supply_cap: IPNAmount::from_unit(21_000_000, IPNUnit::IPN).atomic(),
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}

/// Compute per-round reward using a halving schedule
pub fn round_reward(round: u64, params: &EmissionParams) -> AtomicIPN {
    if round == 0 {
        return 0;
    }
    let halvings = (round / params.halving_rounds) as u32;
    if halvings >= 64 {
        return 0;
    }
    params.r0 >> halvings
}

/// Distribution of rewards for a single round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    pub total: AtomicIPN,
    pub proposer_reward: AtomicIPN,
    pub verifier_pool: AtomicIPN,
    pub verifier_count: usize,
    pub per_verifier: AtomicIPN,
}

/// Compute reward distribution for a round
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

    let proposer_reward = (total * params.proposer_bps as AtomicIPN) / 10_000;
    let verifier_pool = total.saturating_sub(proposer_reward);
    let per_verifier = if verifier_count > 0 {
        verifier_pool / verifier_count as AtomicIPN
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

/// Project total supply emitted after given number of rounds
pub fn projected_supply(rounds: u64, params: &EmissionParams) -> AtomicIPN {
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
        let count = (effective_end - start_round + 1) as AtomicIPN;
        total = total.saturating_add(reward.saturating_mul(count));

        halvings += 1;
    }

    total.min(params.supply_cap)
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
pub fn calculate_fee_recycling(collected_fees: AtomicIPN, params: &FeeRecyclingParams) -> AtomicIPN {
    (collected_fees * params.recycle_bps as AtomicIPN) / 10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_reward_halving() {
        let params = EmissionParams {
            r0: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),
            halving_rounds: 1000,
            ..Default::default()
        };
        assert_eq!(round_reward(999, &params), IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic());
        assert_eq!(round_reward(1000, &params), IPNAmount::from_unit(5_000, IPNUnit::MicroIPN).atomic());
        assert_eq!(round_reward(2000, &params), IPNAmount::from_unit(2_500, IPNUnit::MicroIPN).atomic());
        assert_eq!(round_reward(3000, &params), IPNAmount::from_unit(1_250, IPNUnit::MicroIPN).atomic());
    }

    #[test]
    fn test_distribution() {
        let params = EmissionParams::default();
        let dist = distribute_round_reward(1, &params, 1, 4);
        let expected_total = IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic();
        assert_eq!(dist.total, expected_total);
        assert_eq!(dist.proposer_reward, expected_total / 5); // 20%
        assert_eq!(dist.verifier_pool, expected_total * 4 / 5); // 80%
        assert_eq!(dist.per_verifier, expected_total * 4 / 5 / 4); // 80% / 4 verifiers
    }

    #[test]
    fn test_projected_supply_growth() {
        let params = EmissionParams {
            r0: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),
            halving_rounds: 1000,
            supply_cap: IPNAmount::from_unit(21_000_000, IPNUnit::IPN).atomic(),
            proposer_bps: 2000,
            verifier_bps: 8000,
        };
        let s1 = projected_supply(1000, &params);
        let s2 = projected_supply(2000, &params);
        let expected_r0 = IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic();
        let expected_r1 = IPNAmount::from_unit(5_000, IPNUnit::MicroIPN).atomic();
        assert_eq!(s1, expected_r0 * 1000);
        assert_eq!(s2, expected_r0 * 1000 + expected_r1 * 1000);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let params = EmissionParams {
            supply_cap: IPNAmount::from_unit(50_000, IPNUnit::MicroIPN).atomic(),
            ..Default::default()
        };
        let supply = projected_supply(10_000_000, &params);
        assert!(supply <= params.supply_cap);
    }

    #[test]
    fn test_fee_recycling() {
        let params = FeeRecyclingParams::default();
        let test_fees = IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic();
        assert_eq!(calculate_fee_recycling(test_fees, &params), test_fees);
        let params_half = FeeRecyclingParams {
            recycle_bps: 5000,
            ..Default::default()
        };
        assert_eq!(calculate_fee_recycling(test_fees, &params_half), test_fees / 2);
    }

    #[test]
    fn test_emission_converges() {
        let params = EmissionParams::default();
        let s10k = projected_supply(10_000, &params);
        let s100k = projected_supply(100_000, &params);
        assert!(s10k < s100k && s100k <= params.supply_cap);
    }
}
