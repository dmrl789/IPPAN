//! DAG-Fair Emission Module
//!
//! Deterministic round-based reward emission for IPPAN.
//! Includes halving logic, proposer/verifier reward split,
//! total supply projection, and fee recycling.
//!
//! All amounts use atomic IPN units with 24 decimal precision.

use ippan_types::{Amount, SUPPLY_CAP};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        verifier_pool / verifier_count as AtomicIPN
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

// ============================================================================
// Advanced DAG-Fair Emission Types and Functions (for integration tests)
// ============================================================================

/// Type alias for micro-IPN (smallest unit: 1 IPN = 10^8 µIPN)
pub type MicroIPN = u128;

/// Constant: micro-IPN per IPN
pub const MICRO_PER_IPN: u128 = 100_000_000;

/// Validator identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

/// Validator role in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Proposer,
    Verifier,
}

/// Participation record for a validator in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u64,
}

/// Set of validator participations for a round
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

/// Enhanced economics parameters with additional controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsParams {
    pub r0: u128,
    pub halving_interval_rounds: u64,
    pub hard_cap_micro: MicroIPN,
    pub proposer_bps: u16,
    pub verifier_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            r0: 10_000,
            halving_interval_rounds: 315_000_000,
            hard_cap_micro: 21_000_000 * MICRO_PER_IPN,
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}

/// Compute emission for a specific round
pub fn emission_for_round(round: u64, params: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }
    let halvings = (round / params.halving_interval_rounds) as u32;
    if halvings >= 64 {
        return 0;
    }
    params.r0 >> halvings
}

/// Compute emission with hard cap enforcement
pub fn emission_for_round_capped(
    round: u64,
    total_issued: MicroIPN,
    params: &EconomicsParams,
) -> Result<MicroIPN, &'static str> {
    if total_issued >= params.hard_cap_micro {
        return Err("hard cap exceeded");
    }
    let base_emission = emission_for_round(round, params);
    let remaining = params.hard_cap_micro.saturating_sub(total_issued);
    Ok(base_emission.min(remaining))
}

/// Distribution result
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// Distribute emission and fees among validators based on participation
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    parts: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, MicroIPN), &'static str> {
    if parts.is_empty() {
        return Ok((HashMap::new(), 0, 0));
    }

    let total_pool = emission_micro.saturating_add(fees_micro);
    if total_pool == 0 {
        return Ok((HashMap::new(), 0, 0));
    }

    // Calculate total blocks by proposers and verifiers
    let mut proposer_blocks = 0u64;
    let mut verifier_blocks = 0u64;

    for participation in parts.values() {
        match participation.role {
            Role::Proposer => proposer_blocks += participation.blocks,
            Role::Verifier => verifier_blocks += participation.blocks,
        }
    }

    let total_blocks = proposer_blocks.saturating_add(verifier_blocks);
    if total_blocks == 0 {
        return Ok((HashMap::new(), 0, 0));
    }

    // Split pool according to proposer/verifier basis points
    let proposer_pool = (total_pool * params.proposer_bps as u128) / 10_000;
    let verifier_pool = total_pool.saturating_sub(proposer_pool);

    let mut payouts = HashMap::new();
    let mut emission_paid = 0u128;
    let mut fees_paid = 0u128;

    // Distribute to each validator proportionally
    for (vid, participation) in parts.iter() {
        let payout = match participation.role {
            Role::Proposer if proposer_blocks > 0 => {
                (proposer_pool * participation.blocks as u128) / proposer_blocks as u128
            }
            Role::Verifier if verifier_blocks > 0 => {
                (verifier_pool * participation.blocks as u128) / verifier_blocks as u128
            }
            _ => 0,
        };

        if payout > 0 {
            payouts.insert(vid.clone(), payout);
            // Track what portion came from emission vs fees
            let emission_portion = (payout * emission_micro) / total_pool;
            let fee_portion = payout.saturating_sub(emission_portion);
            emission_paid = emission_paid.saturating_add(emission_portion);
            fees_paid = fees_paid.saturating_add(fee_portion);
        }
    }

    Ok((payouts, emission_paid, fees_paid))
}

/// Sum emission over a range of rounds
pub fn sum_emission_over_rounds<F>(start: u64, end: u64, mut emission_fn: F) -> MicroIPN
where
    F: FnMut(u64) -> MicroIPN,
{
    let mut total = 0u128;
    for round in start..=end {
        total = total.saturating_add(emission_fn(round));
    }
    total
}

/// Calculate auto-burn amount due to rounding errors
pub fn epoch_auto_burn(expected: MicroIPN, actual: MicroIPN) -> MicroIPN {
    expected.saturating_sub(actual)
>>>>>>> origin/main
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
<<<<<<< HEAD
            r0: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),
            halving_rounds: 1000,
            supply_cap: IPNAmount::from_unit(21_000_000, IPNUnit::IPN).atomic(),
=======
            r0: Amount::from_atomic(10_000),
            halving_rounds: 1000,
            supply_cap: Amount(SUPPLY_CAP),
>>>>>>> origin/main
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
