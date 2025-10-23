//! Core emission calculation engine for DAG-Fair emission

use crate::types::*;
use crate::errors::*;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Core emission engine implementing DAG-Fair emission logic
#[derive(Debug, Clone)]
pub struct EmissionEngine {
    /// Current emission parameters
    params: EmissionParams,
    /// Current round index
    current_round: RoundIndex,
    /// Total supply emitted so far
    total_emitted: RewardAmount,
}

impl EmissionEngine {
    /// Create a new emission engine with default parameters
    pub fn new() -> Self {
        Self::with_params(EmissionParams::default())
    }

    /// Create a new emission engine with custom parameters
    pub fn with_params(params: EmissionParams) -> Self {
        Self {
            params,
            current_round: 0,
            total_emitted: 0,
        }
    }

    /// Calculate the reward for a specific round using the halving formula
    /// R(t) = R₀ / 2^(⌊t/Tₕ⌋)
    pub fn calculate_round_reward(&self, round: RoundIndex) -> Result<RewardAmount, EmissionError> {
        if round == 0 {
            return Ok(0); // Genesis round has no emission
        }

        let halving_epoch = round / self.params.halving_interval;
        
        // Check for overflow in halving calculation
        if halving_epoch > 64 {
            return Err(EmissionError::MathematicalOverflow);
        }

        // Calculate reward: R₀ / 2^halving_epoch
        let reward = self.params.initial_round_reward >> halving_epoch;
        
        // Ensure we don't go below 1 micro-IPN
        Ok(reward.max(1))
    }

    /// Calculate the total reward for a range of rounds
    pub fn calculate_total_reward_range(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
    ) -> Result<RewardAmount, EmissionError> {
        if start_round >= end_round {
            return Ok(0);
        }

        let mut total = 0u64;
        for round in start_round..end_round {
            total = total
                .checked_add(self.calculate_round_reward(round)?)
                .ok_or(EmissionError::MathematicalOverflow)?;
        }

        Ok(total)
    }

    /// Calculate the cumulative supply up to a specific round
    pub fn calculate_cumulative_supply(&self, round: RoundIndex) -> Result<RewardAmount, EmissionError> {
        self.calculate_total_reward_range(1, round + 1)
    }

    /// Check if the supply cap would be exceeded by emitting the given amount
    pub fn would_exceed_supply_cap(&self, additional_amount: RewardAmount) -> bool {
        self.total_emitted
            .checked_add(additional_amount)
            .map_or(true, |total| total > self.params.total_supply_cap)
    }

    /// Get the current supply information
    pub fn get_supply_info(&self) -> SupplyInfo {
        let remaining_supply = self.params.total_supply_cap.saturating_sub(self.total_emitted);
        let emission_percentage = if self.params.total_supply_cap > 0 {
            Decimal::from(self.total_emitted) / Decimal::from(self.params.total_supply_cap) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        SupplyInfo {
            total_supply: self.total_emitted,
            supply_cap: self.params.total_supply_cap,
            remaining_supply,
            emission_percentage,
            current_round: self.current_round,
            next_halving_round: ((self.current_round / self.params.halving_interval) + 1) * self.params.halving_interval,
        }
    }

    /// Generate emission curve data points for a range of rounds
    pub fn generate_emission_curve(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
        step: RoundIndex,
    ) -> Result<Vec<EmissionCurvePoint>, EmissionError> {
        let mut points = Vec::new();
        let mut cumulative_supply = self.calculate_cumulative_supply(start_round)?;

        for round in (start_round..=end_round).step_by(step as usize) {
            let reward_per_round = self.calculate_round_reward(round)?;
            let halving_epoch = round / self.params.halving_interval;
            
            // Calculate annual issuance (approximate)
            let annual_issuance = reward_per_round * 31_536_000; // 10 rounds/second * 365.25 days

            points.push(EmissionCurvePoint {
                round,
                reward_per_round,
                annual_issuance,
                cumulative_supply,
                halving_epoch: halving_epoch as u32,
            });

            cumulative_supply = cumulative_supply
                .checked_add(reward_per_round)
                .ok_or(EmissionError::MathematicalOverflow)?;
        }

        Ok(points)
    }

    /// Update emission parameters (requires governance approval)
    pub fn update_params(&mut self, new_params: EmissionParams) -> Result<(), GovernanceError> {
        // Validate new parameters
        self.validate_emission_params(&new_params)?;
        
        info!("Updating emission parameters: {:?}", new_params);
        self.params = new_params;
        Ok(())
    }

    /// Validate emission parameters
    fn validate_emission_params(&self, params: &EmissionParams) -> Result<(), GovernanceError> {
        if params.initial_round_reward == 0 {
            return Err(GovernanceError::InvalidParameter {
                param: "initial_round_reward".to_string(),
                value: params.initial_round_reward.to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if params.halving_interval == 0 {
            return Err(GovernanceError::InvalidParameter {
                param: "halving_interval".to_string(),
                value: params.halving_interval.to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if params.total_supply_cap == 0 {
            return Err(GovernanceError::InvalidParameter {
                param: "total_supply_cap".to_string(),
                value: params.total_supply_cap.to_string(),
                reason: "Must be greater than 0".to_string(),
            });
        }

        if params.fee_cap_fraction < Decimal::ZERO || params.fee_cap_fraction > Decimal::ONE {
            return Err(GovernanceError::InvalidParameter {
                param: "fee_cap_fraction".to_string(),
                value: params.fee_cap_fraction.to_string(),
                reason: "Must be between 0 and 1".to_string(),
            });
        }

        Ok(())
    }

    /// Advance to the next round and update internal state
    pub fn advance_round(&mut self, round: RoundIndex) -> Result<(), EmissionError> {
        if round <= self.current_round {
            return Err(EmissionError::InvalidRoundIndex(round));
        }

        // Calculate reward for the new round
        let round_reward = self.calculate_round_reward(round)?;
        
        // Check supply cap
        if self.would_exceed_supply_cap(round_reward) {
            warn!("Supply cap would be exceeded at round {}, capping emission", round);
            self.total_emitted = self.params.total_supply_cap;
        } else {
            self.total_emitted = self.total_emitted
                .checked_add(round_reward)
                .ok_or(EmissionError::MathematicalOverflow)?;
        }

        self.current_round = round;
        
        debug!("Advanced to round {}, total emitted: {}", round, self.total_emitted);
        Ok(())
    }

    /// Get current emission parameters
    pub fn params(&self) -> &EmissionParams {
        &self.params
    }

    /// Get current round
    pub fn current_round(&self) -> RoundIndex {
        self.current_round
    }

    /// Get total emitted supply
    pub fn total_emitted(&self) -> RewardAmount {
        self.total_emitted
    }
}

impl Default for EmissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_reward_calculation() {
        let engine = EmissionEngine::new();
        
        // First round should have full reward
        assert_eq!(engine.calculate_round_reward(1).unwrap(), 10_000);
        
        // After halving interval, reward should be halved
        let halving_round = engine.params().halving_interval;
        assert_eq!(engine.calculate_round_reward(halving_round).unwrap(), 5_000);
        
        // After two halvings, reward should be quartered
        let double_halving_round = halving_round * 2;
        assert_eq!(engine.calculate_round_reward(double_halving_round).unwrap(), 2_500);
    }

    #[test]
    fn test_supply_cap_enforcement() {
        let mut engine = EmissionEngine::new();
        
        // Advance through many rounds to test supply cap
        for round in 1..=1000 {
            engine.advance_round(round).unwrap();
        }
        
        let supply_info = engine.get_supply_info();
        assert!(supply_info.total_supply <= engine.params().total_supply_cap);
    }

    #[test]
    fn test_emission_curve_generation() {
        let engine = EmissionEngine::new();
        let curve = engine.generate_emission_curve(1, 100, 10).unwrap();
        
        assert!(!curve.is_empty());
        assert_eq!(curve[0].round, 1);
        assert_eq!(curve[0].reward_per_round, 10_000);
    }
}
