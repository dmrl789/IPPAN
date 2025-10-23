//! Round Executor - Integrates DAG-Fair Emission into Consensus
//!
//! This module provides the finalization logic that triggers emission
//! calculation and reward distribution after each consensus round.

use anyhow::Result;
use ippan_governance::parameters::EconomicsParams;
use ippan_storage::{ChainState, Storage};
use ippan_treasury::reward_pool::{Payouts, RewardSink, ValidatorId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Micro-IPN conversion constant (1 IPN = 10^8 µIPN)
pub const MICRO_PER_IPN: u128 = 100_000_000;

/// Role of a participant in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Proposer,
    Verifier,
}

/// Participation record for a validator in a round
#[derive(Debug, Clone)]
pub struct Participation {
    pub validator_id: ValidatorId,
    pub role: Role,
    pub weight: u64, // Stake or reputation weight
}

/// Set of participants in a round
pub type ParticipationSet = Vec<Participation>;

/// Calculate emission for a given round with hard-cap enforcement
pub fn emission_for_round_capped(
    round: u64,
    current_issued_micro: u128,
    economics: &EconomicsParams,
) -> Result<u128> {
    if round == 0 {
        return Ok(0);
    }

    // Calculate halvings
    let halvings = (round / economics.halving_interval_rounds) as u32;
    if halvings >= 64 {
        return Ok(0); // Emission has ceased
    }

    // Calculate base reward with halving
    let base_reward = economics.initial_round_reward_micro >> halvings;

    // Enforce supply cap
    let remaining = economics
        .supply_cap_micro
        .saturating_sub(current_issued_micro);
    let emission = base_reward.min(remaining);

    Ok(emission)
}

/// Distribute rewards for a round with fee cap enforcement
pub fn distribute_round(
    emission_micro: u128,
    fees_micro: u128,
    participants: &ParticipationSet,
    economics: &EconomicsParams,
) -> Result<(Payouts, u128, u128)> {
    let mut payouts = HashMap::new();

    if emission_micro == 0 && fees_micro == 0 {
        return Ok((payouts, 0, 0));
    }

    // Apply fee cap: fees cannot exceed (fee_cap_numer / fee_cap_denom) * emission
    let max_fees = if economics.fee_cap_denom > 0 {
        (emission_micro * economics.fee_cap_numer as u128) / economics.fee_cap_denom as u128
    } else {
        0
    };
    let capped_fees = fees_micro.min(max_fees);

    let total_pool = emission_micro.saturating_add(capped_fees);
    if total_pool == 0 {
        return Ok((payouts, 0, 0));
    }

    // Separate proposers and verifiers
    let proposers: Vec<_> = participants
        .iter()
        .filter(|p| p.role == Role::Proposer)
        .collect();
    let verifiers: Vec<_> = participants
        .iter()
        .filter(|p| p.role == Role::Verifier)
        .collect();

    // Calculate total weights
    let proposer_total_weight: u64 = proposers.iter().map(|p| p.weight).sum();
    let verifier_total_weight: u64 = verifiers.iter().map(|p| p.weight).sum();

    // Split according to economics parameters
    let proposer_pool = (total_pool * economics.proposer_weight_bps as u128) / 10_000;
    let verifier_pool = total_pool.saturating_sub(proposer_pool);

    // Distribute to proposers
    if proposer_total_weight > 0 {
        for p in proposers {
            let share = (proposer_pool * p.weight as u128) / proposer_total_weight as u128;
            *payouts.entry(p.validator_id).or_insert(0) += share;
        }
    }

    // Distribute to verifiers
    if verifier_total_weight > 0 {
        for v in verifiers {
            let share = (verifier_pool * v.weight as u128) / verifier_total_weight as u128;
            *payouts.entry(v.validator_id).or_insert(0) += share;
        }
    }

    let total_paid: u128 = payouts.values().sum();

    Ok((payouts, emission_micro, capped_fees))
}

/// Called automatically when a round is finalized by the DAG consensus.
pub fn finalize_round(
    round: u64,
    storage: &Arc<dyn Storage + Send + Sync>,
    participants: ParticipationSet,
    fees_micro: u128,
    economics: &EconomicsParams,
    reward_sink: &Arc<RwLock<RewardSink>>,
) -> Result<()> {
    // Get current chain state
    let mut chain_state = storage.get_chain_state()?;
    let issued = chain_state.total_issued_micro();

    // Deterministic per-round emission, hard-cap enforced
    let emission_micro = emission_for_round_capped(round, issued, economics)?;

    // Apply DAG-Fair distribution (enforcing fee cap)
    let (payouts, emission_paid, fees_paid) =
        distribute_round(emission_micro, fees_micro, &participants, economics)?;

    // Record payouts into the treasury module
    reward_sink.write().credit_round_payouts(round, &payouts)?;

    // Update total supply
    chain_state.add_issued_micro(emission_paid);
    chain_state.update_round(round);
    storage.update_chain_state(&chain_state)?;

    info!(
        target: "emission",
        "Round {} → {} μIPN emitted (≈ {:.6} IPN), {} μIPN fees",
        round,
        emission_paid,
        (emission_paid as f64) / (MICRO_PER_IPN as f64),
        fees_paid
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_economics() -> EconomicsParams {
        EconomicsParams::default()
    }

    #[test]
    fn test_emission_halving() {
        let economics = default_economics();
        let round1 = emission_for_round_capped(1, 0, &economics).unwrap();
        let round_halving = economics.halving_interval_rounds;
        let round2 = emission_for_round_capped(round_halving, 0, &economics).unwrap();

        assert_eq!(round1, economics.initial_round_reward_micro);
        assert_eq!(round2, economics.initial_round_reward_micro / 2);
    }

    #[test]
    fn test_supply_cap() {
        let mut economics = default_economics();
        economics.supply_cap_micro = 100_000;

        let emission = emission_for_round_capped(1, 99_000, &economics).unwrap();
        assert_eq!(emission, 1_000); // Capped to remaining supply
    }

    #[test]
    fn test_distribute_round_basic() {
        let economics = default_economics();
        let participants = vec![
            Participation {
                validator_id: [1u8; 32],
                role: Role::Proposer,
                weight: 100,
            },
            Participation {
                validator_id: [2u8; 32],
                role: Role::Verifier,
                weight: 100,
            },
        ];

        let (payouts, _, _) = distribute_round(10_000, 1_000, &participants, &economics).unwrap();

        let total: u128 = payouts.values().sum();
        assert_eq!(total, 11_000); // emission + fees
    }

    #[test]
    fn test_fee_cap() {
        let mut economics = default_economics();
        economics.fee_cap_numer = 1;
        economics.fee_cap_denom = 10; // 10% cap

        let participants = vec![Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            weight: 100,
        }];

        let (payouts, emission, fees) =
            distribute_round(10_000, 5_000, &participants, &economics).unwrap();

        // Fees should be capped to 10% of emission = 1_000
        assert_eq!(fees, 1_000);
        let total: u128 = payouts.values().sum();
        assert_eq!(total, 11_000); // 10_000 emission + 1_000 capped fees
    }
}
