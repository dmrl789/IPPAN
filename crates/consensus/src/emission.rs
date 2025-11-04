//! IPPAN DAG-Fair Emission Module
//!
//! Implements deterministic round-based emission for the IPPAN BlockDAG.
//! Each round emits a fixed reward (R₀ / 2^⌊t / Tₕ⌋) subdivided by validator participation,
//! uptime, and AI reputation. Designed for 21 M IPN total supply and ≈2-year halving cycles.

use anyhow::Result;
use ippan_types::{Amount, SUPPLY_CAP};
use serde::{Deserialize, Serialize};

/// Core emission parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGEmissionParams {
    pub r0: Amount,
    pub halving_rounds: u64,
    pub supply_cap: Amount,
    pub round_duration_ms: u64,
    pub fee_cap_bps: u16,
    pub ai_commission_bps: u16,
    pub network_pool_bps: u16,
    pub base_emission_bps: u16,
    pub tx_fee_bps: u16,
    pub proposer_bps: u16,
    pub verifier_bps: u16,
}

impl Default for DAGEmissionParams {
    fn default() -> Self {
        Self {
            r0: Amount::from_micro_ipn(10_000),
            halving_rounds: 315_360_000,
            supply_cap: Amount(SUPPLY_CAP),
            round_duration_ms: 200,
            fee_cap_bps: 1000,
            ai_commission_bps: 1000,
            network_pool_bps: 500,
            base_emission_bps: 6000,
            tx_fee_bps: 2500,
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}

impl DAGEmissionParams {
    pub fn validate(&self) -> Result<()> {
        if self.r0.is_zero() {
            return Err(anyhow::anyhow!("Initial reward must be positive"));
        }
        if self.halving_rounds == 0 {
            return Err(anyhow::anyhow!("Halving rounds must be positive"));
        }
        if self.supply_cap.is_zero() {
            return Err(anyhow::anyhow!("Supply cap must be positive"));
        }
        let total_bps = self.base_emission_bps
            + self.tx_fee_bps
            + self.ai_commission_bps
            + self.network_pool_bps;
        if total_bps != 10_000 {
            return Err(anyhow::anyhow!(
                "Percentages must sum to 10_000 bps, got {}",
                total_bps
            ));
        }
        Ok(())
    }
}

/// Validator role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorRole {
    Proposer,
    Verifier,
    AIService,
}

/// Validator participation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorParticipation {
    pub validator_id: [u8; 32],
    pub role: ValidatorRole,
    pub block_count: usize,
    pub uptime_weight: f64,
    pub reputation_score: u16,
    pub stake_weight: u64,
}

/// Round emission summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundEmission {
    pub round: u64,
    pub total_reward: u128,
    pub base_emission: u128,
    pub tx_fee_portion: u128,
    pub ai_commission_portion: u128,
    pub network_pool_portion: u128,
    pub fee_cap_limit: u128,
    pub halvings_applied: u32,
}

/// Validator reward summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReward {
    pub validator_id: [u8; 32],
    pub total_reward: u128,
    pub base_reward: u128,
    pub tx_fee_reward: u128,
    pub ai_commission_reward: u128,
    pub network_pool_dividend: u128,
    pub role_multiplier: f64,
    pub participation_score: f64,
}

/// Compute per-round reward using halving schedule.
pub fn calculate_round_reward(round: u64, params: &DAGEmissionParams) -> Amount {
    if round == 0 {
        return Amount::zero();
    }
    let halvings = (round / params.halving_rounds) as u32;
    if halvings >= 64 {
        return Amount::zero();
    }
    Amount(params.r0.atomic() >> halvings)
}

/// Compute full emission breakdown for a round.
pub fn calculate_round_emission(round: u64, params: &DAGEmissionParams) -> RoundEmission {
    let total_reward = calculate_round_reward(round, params);
    let halvings_applied = (round / params.halving_rounds) as u32;

    let base_emission = total_reward.percentage(params.base_emission_bps);
    let tx_fee_portion = total_reward.percentage(params.tx_fee_bps);
    let ai_commission_portion = total_reward.percentage(params.ai_commission_bps);
    let network_pool_portion = total_reward.percentage(params.network_pool_bps);
    let fee_cap_limit = total_reward.percentage(params.fee_cap_bps);

    RoundEmission {
        round,
        total_reward: total_reward.atomic(),
        base_emission: base_emission.atomic(),
        tx_fee_portion: tx_fee_portion.atomic(),
        ai_commission_portion: ai_commission_portion.atomic(),
        network_pool_portion: network_pool_portion.atomic(),
        fee_cap_limit: fee_cap_limit.atomic(),
        halvings_applied,
    }
}

/// Compute validator participation score.
fn calculate_participation_score(p: &ValidatorParticipation) -> f64 {
    let block_score = p.block_count as f64;
    let uptime_score = p.uptime_weight;
    let reputation_score = p.reputation_score as f64 / 10_000.0;
    let stake_score = (p.stake_weight as f64).ln_1p();
    block_score * 0.4 + uptime_score * 0.3 + reputation_score * 0.2 + stake_score * 0.1
}

/// DAG-Fair reward distribution.
pub fn distribute_dag_fair_rewards(
    round: u64,
    params: &DAGEmissionParams,
    participations: &[ValidatorParticipation],
    collected_fees: Amount,
    ai_commissions: Amount,
) -> Result<Vec<ValidatorReward>> {
    let round_emission = calculate_round_emission(round, params);
    if participations.is_empty() {
        return Ok(vec![]);
    }

    let effective_fees = collected_fees.min(Amount::from_atomic(round_emission.fee_cap_limit));
    let total_score: f64 = participations
        .iter()
        .map(calculate_participation_score)
        .sum();
    if total_score == 0.0 {
        return Ok(vec![]);
    }

    let mut rewards = Vec::new();
    for p in participations {
        let score = calculate_participation_score(p);
        let ratio = score / total_score;
        let multiplier = match p.role {
            ValidatorRole::Proposer => 1.2,
            ValidatorRole::Verifier => 1.0,
            ValidatorRole::AIService => 1.1,
        };

        let base =
            Amount::from_atomic(round_emission.base_emission).percentage((ratio * 1000.0) as u16);
        let tx = effective_fees.percentage((ratio * 1000.0) as u16);
        let ai = if p.role == ValidatorRole::AIService {
            ai_commissions.percentage((ratio * 1000.0) as u16)
        } else {
            Amount::zero()
        };
        let pool = Amount::from_atomic(round_emission.network_pool_portion)
            .percentage((1000.0 / participations.len() as f64) as u16);

        let total = base + tx + ai + pool;

        rewards.push(ValidatorReward {
            validator_id: p.validator_id,
            total_reward: total.atomic(),
            base_reward: base.atomic(),
            tx_fee_reward: tx.atomic(),
            ai_commission_reward: ai.atomic(),
            network_pool_dividend: pool.atomic(),
            role_multiplier: multiplier,
            participation_score: score,
        });
    }
    Ok(rewards)
}

/// Fee recycling settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingParams {
    pub recycle_bps: u16,
}

impl Default for FeeRecyclingParams {
    fn default() -> Self {
        Self {
            recycle_bps: 10_000,
        }
    }
}

/// Compute fees to recycle into the reward pool.
pub fn calculate_fee_recycling(fees: Amount, params: &FeeRecyclingParams) -> Amount {
    fees.percentage(params.recycle_bps)
}

// ---------------------------------------------------------------------
// Helper functions for testing and integration
// ---------------------------------------------------------------------

/// Calculate reward for a specific round using halving schedule
pub fn round_reward(round: u64, params: &ippan_economics::EmissionParams) -> u128 {
    if round == 0 {
        return 0;
    }
    
    let halving_interval = params.halving_interval_rounds.max(1);
    // Halving occurs at round = halving_interval, 2*halving_interval, etc.
    // Rounds 1 to halving_interval-1: epoch 0 (no halving)
    // Rounds halving_interval to 2*halving_interval-1: epoch 1 (first halving)
    let halvings = round / halving_interval;
    
    if halvings >= 64 {
        return 0;
    }
    
    (params.initial_round_reward_micro as u128) >> halvings.min(63)
}

/// Project cumulative supply at a given round using closed-form geometric series
pub fn projected_supply(rounds: u64, params: &ippan_economics::EmissionParams) -> u128 {
    if rounds == 0 {
        return 0;
    }
    
    let halving_interval = params.halving_interval_rounds.max(1);
    let r0 = params.initial_round_reward_micro as u128;
    
    // Calculate which halving epoch we're in
    let complete_epochs = rounds / halving_interval;
    let remaining_rounds = rounds % halving_interval;
    
    let mut supply = 0u128;
    
    // Sum rewards for all complete epochs
    // Each epoch i has halving_interval rounds with reward r0 >> i
    for epoch in 0..complete_epochs.min(64) {
        let epoch_reward = r0 >> epoch;
        supply = supply.saturating_add(epoch_reward * halving_interval as u128);
    }
    
    // Add rewards for remaining partial epoch
    if complete_epochs < 64 {
        let epoch_reward = r0 >> complete_epochs.min(63);
        supply = supply.saturating_add(epoch_reward * remaining_rounds as u128);
    }
    
    supply.min(params.max_supply_micro as u128)
}

/// Calculate rounds until supply cap is reached
pub fn rounds_until_cap(params: &ippan_economics::EmissionParams) -> u64 {
    // Binary search for the round that reaches the cap
    let mut low = 1u64;
    let mut high = u64::MAX / 2; // Reasonable upper bound
    
    // First check if cap is reachable
    let max_theoretical_supply = 2u128 * params.initial_round_reward_micro as u128 * params.halving_interval_rounds as u128;
    if params.max_supply_micro as u128 > max_theoretical_supply {
        // Cap is never reached with current parameters
        return u64::MAX;
    }
    
    while low < high {
        let mid = low + (high - low) / 2;
        let supply = projected_supply(mid, params);
        
        if supply >= params.max_supply_micro as u128 {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    
    low
}

/// Validator contribution for reward distribution
#[derive(Debug, Clone)]
pub struct ValidatorContribution {
    pub validator_id: [u8; 32],
    pub blocks_proposed: u32,
    pub blocks_verified: u32,
    pub reputation_score: u32, // 0-10000
    pub uptime_factor: u32,    // 0-10000
}

impl ValidatorContribution {
    /// Calculate weighted score for this validator
    pub fn weighted_score(&self, params: &ippan_economics::EmissionParams) -> u128 {
        // Weight: proposed blocks * proposer_weight + verified blocks * verifier_weight
        let proposer_weight = params.proposer_weight_bps as u128;
        let verifier_weight = params.verifier_weight_bps as u128;
        
        // Calculate block score with better precision (multiply before divide)
        let block_score = (self.blocks_proposed as u128 * proposer_weight 
            + self.blocks_verified as u128 * verifier_weight) / 10000;
        
        // Apply reputation and uptime factors
        let reputation_factor = self.reputation_score as u128;
        let uptime_factor = self.uptime_factor as u128;
        
        (block_score * reputation_factor * uptime_factor) / (10000 * 10000)
    }
}

/// Distribution result for a round
#[derive(Debug, Clone)]
pub struct RoundDistribution {
    pub round: u64,
    pub total_base_emission: u128,
    pub transaction_fees: u128,
    pub ai_commissions: u128,
    pub network_dividend: u128,
    pub total_distributed: u128,
    pub validator_rewards: std::collections::HashMap<[u8; 32], u128>,
}

/// Distribute round rewards among validators
pub fn distribute_round_reward(
    round: u64,
    params: &ippan_economics::EmissionParams,
    contributions: &[ValidatorContribution],
    transaction_fees: u128,
    ai_commissions: u128,
    network_pool: u128,
) -> RoundDistribution {
    use std::collections::HashMap;
    
    let base_emission = round_reward(round, params);
    
    // Calculate total weighted score
    let total_score: u128 = contributions
        .iter()
        .map(|c| c.weighted_score(params))
        .sum();
    
    let mut validator_rewards = HashMap::new();
    
    if total_score > 0 {
        // Distribute base emission proportionally
        let mut distributed_base = 0u128;
        
        for (idx, contribution) in contributions.iter().enumerate() {
            let score = contribution.weighted_score(params);
            
            // For last validator, give remainder to avoid rounding errors
            let base_share = if idx == contributions.len() - 1 {
                base_emission.saturating_sub(distributed_base)
            } else {
                (base_emission * score) / total_score
            };
            
            distributed_base = distributed_base.saturating_add(base_share);
            
            // Also distribute fees, commissions, and network dividend proportionally
            let fee_share = (transaction_fees * score) / total_score;
            let ai_share = (ai_commissions * score) / total_score;
            let network_share = (network_pool * score) / total_score;
            
            let total_reward = base_share + fee_share + ai_share + network_share;
            
            validator_rewards.insert(contribution.validator_id, total_reward);
        }
    }
    
    let total_distributed: u128 = validator_rewards.values().sum();
    
    RoundDistribution {
        round,
        total_base_emission: base_emission,
        transaction_fees,
        ai_commissions,
        network_dividend: network_pool,
        total_distributed,
        validator_rewards,
    }
}
