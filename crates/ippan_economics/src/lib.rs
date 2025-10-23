//! IPPAN Economics API facade
//!
//! This crate re-exports emission primitives from `ippan-consensus` and
//! adds thin type aliases used by economic simulations/examples.

use serde::{Deserialize, Serialize};

// Local emission primitives (decoupled from consensus crate)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionParams {
    pub r0: u128,
    pub halving_rounds: u64,
    pub supply_cap: u128,
    pub proposer_bps: u16,
    pub verifier_bps: u16,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            r0: 10_000,
            halving_rounds: 315_000_000,
            supply_cap: 21_000_000_00000000,
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}

pub fn round_reward(round: u64, params: &EmissionParams) -> u128 {
    if round == 0 {
        return 0;
    }
    let halvings = (round / params.halving_rounds) as u32;
    if halvings >= 64 { return 0; }
    params.r0 >> halvings
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    pub total: u128,
    pub proposer_reward: u128,
    pub verifier_pool: u128,
    pub verifier_count: usize,
    pub per_verifier: u128,
}

pub fn distribute_round_reward(
    round: u64,
    params: &EmissionParams,
    block_count: usize,
    verifier_count: usize,
) -> RoundRewardDistribution {
    let total = round_reward(round, params);
    if total == 0 || block_count == 0 {
        return RoundRewardDistribution { total: 0, proposer_reward: 0, verifier_pool: 0, verifier_count: 0, per_verifier: 0 };
    }
    let proposer_reward = (total * params.proposer_bps as u128) / 10_000;
    let verifier_pool = total.saturating_sub(proposer_reward);
    let per_verifier = if verifier_count > 0 { verifier_pool / verifier_count as u128 } else { 0 };
    RoundRewardDistribution { total, proposer_reward, verifier_pool, verifier_count, per_verifier }
}

pub fn projected_supply(rounds: u64, params: &EmissionParams) -> u128 {
    if rounds == 0 { return 0; }
    let mut total = 0u128;
    let mut halvings = 0u32;
    loop {
        let reward = if halvings >= 64 { 0 } else { params.r0 >> halvings };
        if reward == 0 { break; }
        let start_round = (halvings as u64) * params.halving_rounds + 1;
        let end_round = ((halvings + 1) as u64) * params.halving_rounds;
        if start_round > rounds { break; }
        let effective_end = end_round.min(rounds);
        let count = (effective_end - start_round + 1) as u128;
        total = total.saturating_add(reward.saturating_mul(count));
        halvings += 1;
    }
    total.min(params.supply_cap)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingParams {
    pub rounds_per_week: u64,
    pub recycle_bps: u16,
}

impl Default for FeeRecyclingParams {
    fn default() -> Self {
        Self { rounds_per_week: 3_024_000, recycle_bps: 10_000 }
    }
}

pub fn calculate_fee_recycling(collected_fees: u128, params: &FeeRecyclingParams) -> u128 {
    let _ = params.rounds_per_week; // reserved for future scheduling use
    (collected_fees * params.recycle_bps as u128) / 10_000
}

// Economics-specific type aliases (micro-units)
pub type MicroIPN = u128;
pub const MICRO_PER_IPN: MicroIPN = 100_000_000; // 1 IPN = 1e8 µIPN

// Simulation-only structures to match example API
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatorId(pub String);

#[derive(Debug, Clone, Copy)]
pub enum Role { Proposer, Verifier }

#[derive(Debug, Clone, Copy)]
pub struct Participation { pub role: Role, pub blocks: u32 }

use std::collections::HashMap;
pub type ParticipationSet = HashMap<ValidatorId, Participation>;
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

pub fn emission_for_round(round: u64, params: &EconomicsParams) -> MicroIPN {
    // Bridge type: EconomicsParams maps to EmissionParams
    round_reward(round, &EmissionParams {
        r0: params.initial_reward_micro,
        halving_rounds: params.halving_interval_rounds,
        supply_cap: params.supply_cap_micro,
        proposer_bps: params.proposer_bps,
        verifier_bps: params.verifier_bps,
    })
}

pub fn distribute_round(
    emission_micro: MicroIPN,
    _fees_micro: MicroIPN,
    parts: &ParticipationSet,
    params: &EconomicsParams,
) -> Option<(Payouts, MicroIPN, MicroIPN)> {
    if parts.is_empty() { return Some((HashMap::new(), 0, 0)); }

    let verifier_count = parts.values().filter(|p| matches!(p.role, Role::Verifier)).count();
    let block_count: usize = parts.values().map(|p| p.blocks as usize).sum();

    // Use round=1 with custom total emission; emulate split via params
    let fake_params = EmissionParams {
        r0: emission_micro,
        halving_rounds: u64::MAX, // no halving for single step
        supply_cap: u128::MAX,
        proposer_bps: params.proposer_bps,
        verifier_bps: params.verifier_bps,
    };
    let dist = distribute_round_reward(1, &fake_params, block_count.max(1), verifier_count.max(1));

    let mut payouts: Payouts = HashMap::new();

    // Proposer selection: split proposer reward evenly among proposers in this participation
    let proposers: Vec<_> = parts.iter().filter(|(_, p)| matches!(p.role, Role::Proposer)).collect();
    let proposer_split = if proposers.is_empty() { 0 } else { dist.proposer_reward / proposers.len() as u128 };
    for (vid, p) in parts.iter() {
        match p.role {
            Role::Proposer => { payouts.insert(vid.clone(), proposer_split); }
            Role::Verifier => { payouts.insert(vid.clone(), dist.per_verifier); }
        }
    }

    Some((payouts, dist.total, 0))
}

pub fn epoch_auto_burn(emission_micro: MicroIPN, paid_micro: MicroIPN) -> MicroIPN {
    emission_micro.saturating_sub(paid_micro)
}

#[derive(Debug, Clone)]
pub struct EconomicsParams {
    pub initial_reward_micro: MicroIPN,
    pub halving_interval_rounds: u64,
    pub supply_cap_micro: MicroIPN,
    pub proposer_bps: u16,
    pub verifier_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            initial_reward_micro: 10_000,
            halving_interval_rounds: 315_000_000,
            supply_cap_micro: 21_000_000_00000000,
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}
