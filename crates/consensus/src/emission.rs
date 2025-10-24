//! IPPAN DAG-Fair Emission Module
//!
//! Implements deterministic round-based emission for the IPPAN BlockDAG.
//! Each round emits a fixed reward (R₀ / 2^⌊t / Tₕ⌋) subdivided by validator participation,
//! uptime, and AI reputation. Designed for 21 M IPN total supply and ≈2-year halving cycles.

use ippan_types::{Amount, SUPPLY_CAP};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;

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
        let total_bps = self.base_emission_bps + self.tx_fee_bps +
                        self.ai_commission_bps + self.network_pool_bps;
        if total_bps != 10_000 {
            return Err(anyhow::anyhow!("Percentages must sum to 10_000 bps, got {}", total_bps));
        }
        Ok(())
    }
}

/// Validator role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorRole { Proposer, Verifier, AIService }

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
    let total_score: f64 = participations.iter().map(calculate_participation_score).sum();
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

        let base = Amount::from_atomic(round_emission.base_emission)
            .percentage((ratio * 1000.0) as u16);
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
        Self { recycle_bps: 10_000 }
    }
}

/// Compute fees to recycle into the reward pool.
pub fn calculate_fee_recycling(fees: Amount, params: &FeeRecyclingParams) -> Amount {
    fees.percentage(params.recycle_bps)
}
