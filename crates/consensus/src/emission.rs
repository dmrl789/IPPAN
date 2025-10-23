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

use serde::{Deserialize, Serialize};
use anyhow::Result;

/// DAG-Fair emission parameters for IPPAN BlockDAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGEmissionParams {
    /// Initial reward per round (in µIPN — micro-IPN)
    /// Default: 0.0001 IPN = 10,000 µIPN per round
    pub r0: u128,
    /// Number of rounds between halvings (≈ 2 years at 100ms rounds)
    /// Default: 315,000,000 rounds (≈ 2 years)
    pub halving_rounds: u64,
    /// Total supply cap (21 M IPN = 21,000,000,000,000 µIPN)
    pub supply_cap: u128,
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
            r0: 10_000,
            // Halving every ~2 years at 100ms rounds (315,000,000 rounds)
            halving_rounds: 315_000_000,
            // 21 million IPN = 21,000,000,000,000 µIPN
            supply_cap: 21_000_000_00000000,
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
        }
    }
}

impl DAGEmissionParams {
    /// Validate emission parameters
    pub fn validate(&self) -> Result<()> {
        if self.r0 == 0 {
            return Err(anyhow::anyhow!("Initial reward must be positive"));
        }
        if self.halving_rounds == 0 {
            return Err(anyhow::anyhow!("Halving rounds must be positive"));
        }
        if self.supply_cap == 0 {
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
        let ipn_per_round = self.r0 as f64 / 1_000_000.0; // Convert µIPN to IPN
        rounds_per_year * ipn_per_round
    }
    
    /// Get projected total emission time (years)
    pub fn projected_emission_years(&self) -> f64 {
        let mut total_rounds = 0u64;
        let mut halvings = 0u32;
        
        loop {
            let reward = if halvings >= 64 { 0 } else { self.r0 >> halvings };
            if reward == 0 {
                break;
            }
            
            let rounds_in_halving = self.halving_rounds;
            let _total_reward_in_halving = reward.saturating_mul(rounds_in_halving as u128);
            
            if total_rounds.saturating_add(rounds_in_halving) as u128 * reward > self.supply_cap {
                // Calculate partial halving period
                let remaining_supply = self.supply_cap.saturating_sub(
                    total_rounds as u128 * reward
                );
                let remaining_rounds = remaining_supply / reward;
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
pub fn calculate_round_reward(round: u64, params: &DAGEmissionParams) -> u128 {
    if round == 0 {
        return 0;
    }
    let halvings = (round / params.halving_rounds) as u32;
    if halvings >= 64 {
        return 0;
    }
    params.r0 >> halvings
}

/// Calculate comprehensive round emission breakdown
pub fn calculate_round_emission(round: u64, params: &DAGEmissionParams) -> RoundEmission {
    let total_reward = calculate_round_reward(round, params);
    let halvings_applied = (round / params.halving_rounds) as u32;
    
    let base_emission = (total_reward * params.base_emission_bps as u128) / 10_000;
    let tx_fee_portion = (total_reward * params.tx_fee_bps as u128) / 10_000;
    let ai_commission_portion = (total_reward * params.ai_commission_bps as u128) / 10_000;
    let network_pool_portion = (total_reward * params.network_pool_bps as u128) / 10_000;
    let fee_cap_limit = (total_reward * params.fee_cap_bps as u128) / 10_000;
    
    RoundEmission {
        round,
        total_reward,
        base_emission,
        tx_fee_portion,
        ai_commission_portion,
        network_pool_portion,
        fee_cap_limit,
        halvings_applied,
    }
}

/// DAG-Fair reward distribution for a round
pub fn distribute_dag_fair_rewards(
    round: u64,
    params: &DAGEmissionParams,
    participations: &[ValidatorParticipation],
    collected_fees: u128,
    ai_commissions: u128,
) -> Result<Vec<ValidatorReward>> {
    let round_emission = calculate_round_emission(round, params);
    
    if participations.is_empty() {
        return Ok(vec![]);
    }
    
    // Apply fee cap
    let effective_fees = collected_fees.min(round_emission.fee_cap_limit);
    
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
        let base_reward = ((round_emission.base_emission as f64 * participation_ratio) as u128)
            .saturating_mul((role_multiplier * 1000.0) as u128) / 1000;
        
        // Transaction fee reward (proportional to participation)
        let tx_fee_reward = ((effective_fees as f64 * participation_ratio) as u128)
            .saturating_mul((role_multiplier * 1000.0) as u128) / 1000;
        
        // AI commission reward (only for AI service providers)
        let ai_commission_reward = if participation.role == ValidatorRole::AIService {
            ((ai_commissions as f64 * participation_ratio) as u128)
                .saturating_mul((role_multiplier * 1000.0) as u128) / 1000
        } else {
            0
        };
        
        // Network pool dividend (distributed equally among all participants)
        let network_pool_dividend = (round_emission.network_pool_portion / participations.len() as u128)
            .saturating_mul((role_multiplier * 1000.0) as u128) / 1000;
        
        let total_reward = base_reward + tx_fee_reward + ai_commission_reward + network_pool_dividend;
        
        rewards.push(ValidatorReward {
            validator_id: participation.validator_id,
            total_reward,
            base_reward,
            tx_fee_reward,
            ai_commission_reward,
            network_pool_dividend,
            role_multiplier,
            participation_score,
        });
    }
    
    Ok(rewards)
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
pub fn projected_supply(rounds: u64, params: &DAGEmissionParams) -> u128 {
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
        curve.push((round, round_reward, cumulative_supply));
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
        
        let mut annual_emission = 0u128;
        for round in start_round..=end_round {
            annual_emission = annual_emission.saturating_add(calculate_round_reward(round, params));
        }
        
        let cumulative_supply = projected_supply(end_round, params);
        schedule.push((year, annual_emission, cumulative_supply));
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
    total_emitted: u128,
    /// Fee collection pool
    fee_pool: u128,
    /// AI commission pool
    ai_commission_pool: u128,
    /// Network reward pool
    network_pool: u128,
}

impl DAGEmissionSystem {
    /// Create a new DAG-Fair emission system
    pub fn new(params: DAGEmissionParams) -> Result<Self> {
        params.validate()?;
        Ok(Self {
            params,
            current_round: 0,
            total_emitted: 0,
            fee_pool: 0,
            ai_commission_pool: 0,
            network_pool: 0,
        })
    }
    
    /// Process a round and distribute rewards
    pub fn process_round(
        &mut self,
        participations: &[ValidatorParticipation],
        collected_fees: u128,
        ai_commissions: u128,
    ) -> Result<Vec<ValidatorReward>> {
        self.current_round += 1;
        
        // Add to pools
        self.fee_pool = self.fee_pool.saturating_add(collected_fees);
        self.ai_commission_pool = self.ai_commission_pool.saturating_add(ai_commissions);
        
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
        self.total_emitted = self.total_emitted.saturating_add(round_emission.total_reward);
        
        // Clear pools after distribution
        self.fee_pool = 0;
        self.ai_commission_pool = 0;
        
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
    pub fn total_emitted(&self) -> u128 {
        self.total_emitted
    }
    
    /// Get remaining supply
    pub fn remaining_supply(&self) -> u128 {
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
            total_emitted: self.total_emitted,
            remaining_supply: self.remaining_supply(),
            supply_cap: self.params.supply_cap,
            emission_percentage: (self.total_emitted as f64 / self.params.supply_cap as f64) * 100.0,
            annual_emission_rate: self.params.annual_emission_rate(),
            projected_completion_years: self.params.projected_emission_years(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_params() -> DAGEmissionParams {
        DAGEmissionParams {
            r0: 10_000,
            halving_rounds: 1000,
            supply_cap: 21_000_000_00000000,
            round_duration_ms: 100,
            fee_cap_bps: 1000,
            ai_commission_bps: 1000,
            network_pool_bps: 500,
            base_emission_bps: 6000,
            tx_fee_bps: 2500,
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
        assert_eq!(calculate_round_reward(999, &params), 10_000);
        assert_eq!(calculate_round_reward(1000, &params), 5_000);
        assert_eq!(calculate_round_reward(2000, &params), 2_500);
        assert_eq!(calculate_round_reward(3000, &params), 1_250);
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
        
        let rewards = distribute_dag_fair_rewards(1, &params, &participations, 1000, 500).unwrap();
        
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
        assert_eq!(s1, 10_000 * 1000);
        assert_eq!(s2, 10_000 * 1000 + 5_000 * 1000);
    }

    #[test]
    fn test_supply_cap_enforced() {
        let params = DAGEmissionParams {
            supply_cap: 50_000,
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
        assert_eq!(system.total_emitted(), 0);
        
        let participations = vec![
            create_test_participation([1u8; 32], ValidatorRole::Proposer),
        ];
        
        let rewards = system.process_round(&participations, 1000, 500).unwrap();
        assert_eq!(system.current_round(), 1);
        assert!(system.total_emitted() > 0);
        assert!(!rewards.is_empty());
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
}
