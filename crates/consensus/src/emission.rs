//! DAG-Fair Emission Module
//!
//! Deterministic round-based reward emission for IPPAN BlockDAG.
//! Implements the DAG-Fair emission model with:
//! - Round-based rewards (not per-block)
//! - Deterministic halving schedule
//! - Proportional distribution among validators
//! - 21M IPN supply cap
//! - Fee recycling and AI micro-service commissions
//! - Governance-controlled parameters
//!
//! All amounts use atomic IPN units with 24 decimal precision.

use ippan_types::{Amount, SUPPLY_CAP};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;

/// DAG-Fair emission parameters for IPPAN BlockDAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGEmissionParams {
    /// Initial reward per round (in atomic IPN units with 24 decimal precision)
    /// Default: 0.0001 IPN = 10,000 µIPN per round
    pub r0: Amount,
    /// Number of rounds between halvings (≈ 2 years at 100ms rounds)
    /// Default: 315,000,000 rounds (≈ 2 years)
    pub halving_rounds: u64,
    /// Total supply cap (21 M IPN in atomic units)
    pub supply_cap: Amount,
    /// Round duration in milliseconds (for emission rate calculations)
    pub round_duration_ms: u64,
    /// Fee cap as percentage of round reward (basis points)
    /// Default: 10% = 1000 basis points
    pub fee_cap_bps: u16,
    /// AI micro-service commission percentage (basis points)
    /// Default: 10% = 1000 basis points
    pub ai_commission_bps: u16,
    /// Network reward pool dividend percentage (basis points)
    /// Default: 5% = 500 basis points
    pub network_pool_bps: u16,
    /// Base emission percentage (basis points)
    /// Default: 60% = 6000 basis points
    pub base_emission_bps: u16,
    /// Transaction fee percentage (basis points)
    /// Default: 25% = 2500 basis points
    pub tx_fee_bps: u16,
    /// Proposer reward percentage (basis points; 20% = 2000)
    pub proposer_bps: u16,
    /// Verifier reward percentage (basis points; 80% = 8000)
    pub verifier_bps: u16,
}

/// Validator role in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorRole {
    /// Block proposer (higher reward multiplier)
    Proposer,
    /// Block verifier (standard reward)
    Verifier,
    /// AI micro-service provider (commission-based)
    AIService,
}

/// Validator participation in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorParticipation {
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Role in this round
    pub role: ValidatorRole,
    /// Number of blocks produced/verified
    pub block_count: usize,
    /// Uptime weight (0.0 to 1.0)
    pub uptime_weight: f64,
    /// Reputation score (0 to 10000)
    pub reputation_score: u16,
    /// Stake weight
    pub stake_weight: u64,
}

/// Round emission calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundEmission {
    /// Round number
    pub round: u64,
    /// Total reward for this round (µIPN)
    pub total_reward: u128,
    /// Base emission portion
    pub base_emission: u128,
    /// Transaction fee portion
    pub tx_fee_portion: u128,
    /// AI commission portion
    pub ai_commission_portion: u128,
    /// Network pool portion
    pub network_pool_portion: u128,
    /// Fee cap limit for this round
    pub fee_cap_limit: u128,
    /// Number of halvings applied
    pub halvings_applied: u32,
}

/// Validator reward distribution for a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReward {
    /// Validator ID
    pub validator_id: [u8; 32],
    /// Total reward for this validator (µIPN)
    pub total_reward: u128,
    /// Base emission reward
    pub base_reward: u128,
    /// Transaction fee reward
    pub tx_fee_reward: u128,
    /// AI commission reward
    pub ai_commission_reward: u128,
    /// Network pool dividend
    pub network_pool_dividend: u128,
    /// Role multiplier applied
    pub role_multiplier: f64,
    /// Participation score
    pub participation_score: f64,
}

impl Default for DAGEmissionParams {
    fn default() -> Self {
        Self {
            // 0.0001 IPN per round = 10,000 µIPN
            r0: Amount::from_micro_ipn(10_000),
            // Halving every ~2 years at 100ms rounds (315,000,000 rounds)
            halving_rounds: 315_000_000,
            // 21 million IPN in atomic units
            supply_cap: Amount(SUPPLY_CAP),
            // 100ms round duration
            round_duration_ms: 100,
            // Fee cap at 10% of round reward
            fee_cap_bps: 1000,
            // AI commission at 10%
            ai_commission_bps: 1000,
            // Network pool at 5%
            network_pool_bps: 500,
            // Base emission at 60%
            base_emission_bps: 6000,
            // Transaction fees at 25%
            tx_fee_bps: 2500,
            // Proposer reward at 20%
            proposer_bps: 2000,
            // Verifier reward at 80%
            verifier_bps: 8000,
        }
    }
}

impl DAGEmissionParams {
    /// Validate emission parameters
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
        if self.round_duration_ms == 0 {
            return Err(anyhow::anyhow!("Round duration must be positive"));
        }
        
        // Check that percentages add up to 100%
        let total_bps = self.base_emission_bps + self.tx_fee_bps + 
                       self.ai_commission_bps + self.network_pool_bps;
        if total_bps != 10_000 {
            return Err(anyhow::anyhow!(
                "Emission percentages must sum to 100% (10,000 basis points), got {}", 
                total_bps
            ));
        }
        
        // Validate individual percentages
        if self.fee_cap_bps > 10_000 {
            return Err(anyhow::anyhow!("Fee cap cannot exceed 100%"));
        }
        if self.ai_commission_bps > 10_000 {
            return Err(anyhow::anyhow!("AI commission cannot exceed 100%"));
        }
        
        Ok(())
    }
    
    /// Get annual emission rate (IPN per year)
    pub fn annual_emission_rate(&self) -> f64 {
        let rounds_per_year = (365.25 * 24.0 * 3600.0 * 1000.0) / self.round_duration_ms as f64;
        let ipn_per_round = self.r0.to_ipn();
        rounds_per_year * ipn_per_round as f64
    }
    
    /// Get projected total emission time (years)
    pub fn projected_emission_years(&self) -> f64 {
        let mut total_rounds = 0u64;
        let mut halvings = 0u32;
        
        loop {
            let reward = if halvings >= 64 { 
                Amount::zero() 
            } else { 
                Amount(self.r0.atomic() >> halvings) 
            };
            if reward.is_zero() {
                break;
            }
            
            let rounds_in_halving = self.halving_rounds;
            let _total_reward_in_halving = reward.atomic().saturating_mul(rounds_in_halving as u128);
            
            if total_rounds.saturating_add(rounds_in_halving) as u128 * reward.atomic() > self.supply_cap.atomic() {
                // Calculate partial halving period
                let remaining_supply = self.supply_cap.atomic().saturating_sub(
                    total_rounds as u128 * reward.atomic()
                );
                let remaining_rounds = remaining_supply / reward.atomic();
                total_rounds += remaining_rounds as u64;
                break;
            }
            
            total_rounds += rounds_in_halving;
            halvings += 1;
        }
        
        total_rounds as f64 * self.round_duration_ms as f64 / (365.25 * 24.0 * 3600.0 * 1000.0)
    }
}

/// Compute per-round reward using DAG-Fair halving schedule
/// Formula: R(t) = R₀ / 2^(⌊t/Th⌋)
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

/// Calculate comprehensive round emission breakdown
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

/// Distribution of rewards for a single round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    pub total: Amount,
    pub proposer_reward: Amount,
    pub verifier_pool: Amount,
    pub verifier_count: usize,
    pub per_verifier: Amount,
}

/// DAG-Fair reward distribution for a round
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
    
    // Apply fee cap
    let effective_fees = collected_fees.min(Amount::from_atomic(round_emission.fee_cap_limit));
    
    // Calculate total participation score
    let total_participation_score: f64 = participations.iter()
        .map(|p| calculate_participation_score(p))
        .sum();
    
    if total_participation_score == 0.0 {
        return Ok(vec![]);
    }
    
    let mut rewards = Vec::new();
    
    for participation in participations {
        let participation_score = calculate_participation_score(participation);
        let participation_ratio = participation_score / total_participation_score;
        
        // Calculate role multiplier
        let role_multiplier = match participation.role {
            ValidatorRole::Proposer => 1.2, // 20% bonus for proposers
            ValidatorRole::Verifier => 1.0, // Standard reward
            ValidatorRole::AIService => 1.1, // 10% bonus for AI services
        };
        
        // Base emission reward
        let base_reward = Amount::from_atomic(round_emission.base_emission)
            .percentage((participation_ratio * 1000.0) as u16)
            .percentage((role_multiplier * 1000.0) as u16);
        
        // Transaction fee reward (proportional to participation)
        let tx_fee_reward = effective_fees
            .percentage((participation_ratio * 1000.0) as u16)
            .percentage((role_multiplier * 1000.0) as u16);
        
        // AI commission reward (only for AI service providers)
        let ai_commission_reward = if participation.role == ValidatorRole::AIService {
            ai_commissions
                .percentage((participation_ratio * 1000.0) as u16)
                .percentage((role_multiplier * 1000.0) as u16)
        } else {
            Amount::zero()
        };
        
        // Network pool dividend (distributed equally among all participants)
        let network_pool_dividend = Amount::from_atomic(round_emission.network_pool_portion)
            .percentage((1000.0 / participations.len() as f64) as u16)
            .percentage((role_multiplier * 1000.0) as u16);
        
        let total_reward = base_reward + tx_fee_reward + ai_commission_reward + network_pool_dividend;
        
        rewards.push(ValidatorReward {
            validator_id: participation.validator_id,
            total_reward: total_reward.atomic(),
            base_reward: base_reward.atomic(),
            tx_fee_reward: tx_fee_reward.atomic(),
            ai_commission_reward: ai_commission_reward.atomic(),
            network_pool_dividend: network_pool_dividend.atomic(),
            role_multiplier,
            participation_score,
        });
    }
    
    Ok(rewards)
}

/// Legacy distribution function for backward compatibility
pub fn distribute_round_reward(
    round: u64,
    params: &DAGEmissionParams,
    block_count: usize,
    verifier_count: usize,
) -> RoundRewardDistribution {
    let total = calculate_round_reward(round, params);
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

/// Calculate participation score for a validator
fn calculate_participation_score(participation: &ValidatorParticipation) -> f64 {
    let block_score = participation.block_count as f64;
    let uptime_score = participation.uptime_weight;
    let reputation_score = participation.reputation_score as f64 / 10000.0;
    let stake_score = (participation.stake_weight as f64).ln_1p(); // Log scale for stake
    
    // Weighted combination: 40% blocks, 30% uptime, 20% reputation, 10% stake
    block_score * 0.4 + uptime_score * 0.3 + reputation_score * 0.2 + stake_score * 0.1
}

/// Project total supply emitted after given number of rounds
pub fn projected_supply(rounds: u64, params: &DAGEmissionParams) -> Amount {
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

/// Calculate emission curve data for visualization
pub fn calculate_emission_curve(
    params: &DAGEmissionParams,
    max_rounds: u64,
    sample_interval: u64,
) -> Vec<(u64, u128, u128)> {
    let mut curve = Vec::new();
    
    for round in (0..=max_rounds).step_by(sample_interval as usize) {
        let round_reward = calculate_round_reward(round, params);
        let cumulative_supply = projected_supply(round, params);
        curve.push((round, round_reward.atomic(), cumulative_supply.atomic()));
    }
    
    curve
}

/// Calculate annual emission schedule
pub fn calculate_annual_emission_schedule(
    params: &DAGEmissionParams,
    years: u32,
) -> Vec<(u32, u128, u128)> {
    let rounds_per_year = (365.25 * 24.0 * 3600.0 * 1000.0) / params.round_duration_ms as f64;
    let mut schedule = Vec::new();
    
    for year in 1..=years {
        let start_round = ((year - 1) as f64 * rounds_per_year) as u64 + 1;
        let end_round = (year as f64 * rounds_per_year) as u64;
        
        let mut annual_emission = Amount::zero();
        for round in start_round..=end_round {
            annual_emission = annual_emission + calculate_round_reward(round, params);
        }
        
        let cumulative_supply = projected_supply(end_round, params);
        schedule.push((year, annual_emission.atomic(), cumulative_supply.atomic()));
    }
    
    schedule
}

/// DAG-Fair emission system manager
pub struct DAGEmissionSystem {
    /// Current emission parameters
    params: DAGEmissionParams,
    /// Current round
    current_round: u64,
    /// Total supply emitted so far
    total_emitted: Amount,
    /// Fee collection pool
    fee_pool: Amount,
    /// AI commission pool
    ai_commission_pool: Amount,
    /// Network reward pool
    network_pool: Amount,
}

impl DAGEmissionSystem {
    /// Create a new DAG-Fair emission system
    pub fn new(params: DAGEmissionParams) -> Result<Self> {
        params.validate()?;
        Ok(Self {
            params,
            current_round: 0,
            total_emitted: Amount::zero(),
            fee_pool: Amount::zero(),
            ai_commission_pool: Amount::zero(),
            network_pool: Amount::zero(),
        })
    }
    
    /// Process a round and distribute rewards
    pub fn process_round(
        &mut self,
        participations: &[ValidatorParticipation],
        collected_fees: Amount,
        ai_commissions: Amount,
    ) -> Result<Vec<ValidatorReward>> {
        self.current_round += 1;
        
        // Add to pools
        self.fee_pool = self.fee_pool + collected_fees;
        self.ai_commission_pool = self.ai_commission_pool + ai_commissions;
        
        // Distribute rewards
        let rewards = distribute_dag_fair_rewards(
            self.current_round,
            &self.params,
            participations,
            self.fee_pool,
            self.ai_commission_pool,
        )?;
        
        // Update total emitted
        let round_emission = calculate_round_emission(self.current_round, &self.params);
        self.total_emitted = self.total_emitted + Amount::from_atomic(round_emission.total_reward);
        
        // Clear pools after distribution
        self.fee_pool = Amount::zero();
        self.ai_commission_pool = Amount::zero();
        
        Ok(rewards)
    }
    
    /// Get current emission parameters
    pub fn get_params(&self) -> &DAGEmissionParams {
        &self.params
    }
    
    /// Update emission parameters (requires governance approval)
    pub fn update_params(&mut self, new_params: DAGEmissionParams) -> Result<()> {
        new_params.validate()?;
        self.params = new_params;
        Ok(())
    }
    
    /// Get current round
    pub fn current_round(&self) -> u64 {
        self.current_round
    }
    
    /// Get total emitted supply
    pub fn total_emitted(&self) -> Amount {
        self.total_emitted
    }
    
    /// Get remaining supply
    pub fn remaining_supply(&self) -> Amount {
        self.params.supply_cap.saturating_sub(self.total_emitted)
    }
    
    /// Check if emission is complete
    pub fn is_emission_complete(&self) -> bool {
        self.total_emitted >= self.params.supply_cap
    }
    
    /// Get emission statistics
    pub fn get_emission_stats(&self) -> EmissionStats {
        EmissionStats {
            current_round: self.current_round,
            total_emitted: self.total_emitted.atomic(),
            remaining_supply: self.remaining_supply().atomic(),
            supply_cap: self.params.supply_cap.atomic(),
            emission_percentage: (self.total_emitted.to_ipn() as f64 / self.params.supply_cap.to_ipn() as f64) * 100.0,
            annual_emission_rate: self.params.annual_emission_rate(),
            projected_completion_years: self.params.projected_emission_years(),
        }
    }
}

/// Fee recycling parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingParams {
    /// Percentage of fees to recycle (basis points)
    pub recycle_bps: u16,
}

impl Default for FeeRecyclingParams {
    fn default() -> Self {
        Self {
            recycle_bps: 10_000, // 100% by default
        }
    }
}

/// Compute amount of fees to recycle back into reward pool
pub fn calculate_fee_recycling(collected_fees: Amount, params: &FeeRecyclingParams) -> Amount {
    collected_fees.percentage(params.recycle_bps)
}

/// Emission statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionStats {
    pub current_round: u64,
    pub total_emitted: u128,
    pub remaining_supply: u128,
    pub supply_cap: u128,
    pub emission_percentage: f64,
    pub annual_emission_rate: f64,
    pub projected_completion_years: f64,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_params() -> DAGEmissionParams {
        DAGEmissionParams {
            r0: Amount::from_micro_ipn(10_000),
            halving_rounds: 1000,
            supply_cap: Amount::from_atomic(21_000_000_00000000),
            round_duration_ms: 100,
            fee_cap_bps: 1000,
            ai_commission_bps: 1000,
            network_pool_bps: 500,
            base_emission_bps: 6000,
            tx_fee_bps: 2500,
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }

    fn create_test_participation(validator_id: [u8; 32], role: ValidatorRole) -> ValidatorParticipation {
        ValidatorParticipation {
            validator_id,
            role,
            block_count: 1,
            uptime_weight: 0.95,
            reputation_score: 8000,
            stake_weight: 1000000,
        }
    }

    #[test]
    fn test_round_reward_halving() {
        let params = create_test_params();
        assert_eq!(calculate_round_reward(999, &params).atomic(), 10_000);
        assert_eq!(calculate_round_reward(1000, &params).atomic(), 5_000);
        assert_eq!(calculate_round_reward(2000, &params).atomic(), 2_500);
        assert_eq!(calculate_round_reward(3000, &params).atomic(), 1_250);
    }

    #[test]
    fn test_round_emission_calculation() {
        let params = create_test_params();
        let emission = calculate_round_emission(1, &params);
        
        assert_eq!(emission.total_reward, 10_000);
        assert_eq!(emission.base_emission, 6_000); // 60%
        assert_eq!(emission.tx_fee_portion, 2_500); // 25%
        assert_eq!(emission.ai_commission_portion, 1_000); // 10%
        assert_eq!(emission.network_pool_portion, 500); // 5%
        assert_eq!(emission.halvings_applied, 0);
    }

    #[test]
    fn test_dag_fair_distribution() {
        let params = create_test_params();
        let participations = vec![
            create_test_participation([1u8; 32], ValidatorRole::Proposer),
            create_test_participation([2u8; 32], ValidatorRole::Verifier),
            create_test_participation([3u8; 32], ValidatorRole::AIService),
        ];
        
        let rewards = distribute_dag_fair_rewards(1, &params, &participations, Amount::from_micro_ipn(1000), Amount::from_micro_ipn(500)).unwrap();
        
        assert_eq!(rewards.len(), 3);
        
        // Proposer should get highest reward due to 1.2x multiplier
        let proposer_reward = rewards.iter().find(|r| r.validator_id == [1u8; 32]).unwrap();
        assert!(proposer_reward.total_reward > 0);
        assert_eq!(proposer_reward.role_multiplier, 1.2);
        
        // AI service should get commission
        let ai_reward = rewards.iter().find(|r| r.validator_id == [3u8; 32]).unwrap();
        assert!(ai_reward.ai_commission_reward > 0);
    }

    #[test]
    fn test_projected_supply_growth() {
        let params = create_test_params();
        let s1 = projected_supply(1000, &params);
        let s2 = projected_supply(2000, &params);
        assert_eq!(s1.atomic(), 10_000 * 1000);
        assert_eq!(s2.atomic(), 10_000 * 1000 + 5_000 * 1000);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let params = DAGEmissionParams {
            supply_cap: Amount::from_atomic(50_000),
            ..create_test_params()
        };
        let supply = projected_supply(10_000_000, &params);
        assert!(supply <= params.supply_cap);
    }

    #[test]
    fn test_emission_system() {
        let params = create_test_params();
        let mut system = DAGEmissionSystem::new(params).unwrap();
        
        assert_eq!(system.current_round(), 0);
        assert_eq!(system.total_emitted(), Amount::zero());
        
        let participations = vec![
            create_test_participation([1u8; 32], ValidatorRole::Proposer),
        ];
        
        let rewards = system.process_round(&participations, Amount::from_micro_ipn(1000), Amount::from_micro_ipn(500)).unwrap();
        assert_eq!(system.current_round(), 1);
        assert!(system.total_emitted() > Amount::zero());
        assert!(!rewards.is_empty());
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
    fn test_parameter_validation() {
        let mut params = create_test_params();
        assert!(params.validate().is_ok());
        
        // Test invalid percentages
        params.base_emission_bps = 7000; // This would make total > 100%
        assert!(params.validate().is_err());
        
        // Test valid percentages
        params.base_emission_bps = 6000;
        params.tx_fee_bps = 2500;
        params.ai_commission_bps = 1000;
        params.network_pool_bps = 500;
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_emission_curve() {
        let params = create_test_params();
        let curve = calculate_emission_curve(&params, 5000, 1000);
        
        assert!(!curve.is_empty());
        assert_eq!(curve[0].0, 0); // First entry should be round 0
        assert_eq!(curve[0].1, 0); // Round 0 should have 0 reward
    }

    #[test]
    fn test_annual_emission_schedule() {
        let params = create_test_params();
        let schedule = calculate_annual_emission_schedule(&params, 3);
        
        assert_eq!(schedule.len(), 3);
        assert!(schedule[0].1 > 0); // First year should have emission
        assert!(schedule[1].1 < schedule[0].1); // Second year should have less due to halving
    }

    #[test]
    fn test_participation_score_calculation() {
        let participation = create_test_participation([1u8; 32], ValidatorRole::Proposer);
        let score = calculate_participation_score(&participation);
        
        assert!(score > 0.0);
        assert!(score <= 10.0); // Reasonable upper bound
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
