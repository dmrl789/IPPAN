//! Emission engine for IPPAN economics

use crate::errors::*;
use crate::types::*;
use rust_decimal::Decimal;
use tracing::info;

/// Core emission engine implementing DAG-Fair emission logic
#[derive(Debug, Clone)]
pub struct EmissionEngine {
    /// Current emission parameters
    params: EmissionParams,
    /// Current round index
    current_round: RoundIndex,
    /// Total supply emitted so far
    total_supply: RewardAmount,
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
            total_supply: 0,
        }
    }

    /// Calculate the reward for a specific round
    pub fn calculate_round_reward(&self, round: RoundIndex) -> Result<RewardAmount, EmissionError> {
        if round == 0 {
            return Ok(0);
        }

        // Calculate halving epoch
        let halving_epoch = (round - 1) / self.params.halving_interval_rounds;

        // Calculate reward with halving
        let base_reward = self.params.initial_round_reward_micro;
        let halved_reward = base_reward >> halving_epoch.min(63); // Prevent overflow

        // Check if we've reached the supply cap
        if self.total_supply + halved_reward > self.params.max_supply_micro {
            let remaining = self
                .params
                .max_supply_micro
                .saturating_sub(self.total_supply);
            return Ok(remaining);
        }

        Ok(halved_reward)
    }

    /// Advance to the next round and update supply
    pub fn advance_round(&mut self, round: RoundIndex) -> Result<RewardAmount, EmissionError> {
        if round <= self.current_round {
            return Err(EmissionError::InvalidRoundIndex(round));
        }

        let reward = self.calculate_round_reward(round)?;

        // Update supply
        self.total_supply =
            self.total_supply
                .checked_add(reward)
                .ok_or(EmissionError::CalculationOverflow(
                    "Supply addition overflow".to_string(),
                ))?;

        self.current_round = round;

        info!("Advanced to round {}: {} micro-IPN emitted", round, reward);
        Ok(reward)
    }

    /// Get current supply information
    pub fn get_supply_info(&self) -> SupplyInfo {
        let remaining = self
            .params
            .max_supply_micro
            .saturating_sub(self.total_supply);
        let emission_percentage = if self.params.max_supply_micro > 0 {
            Decimal::from(self.total_supply) / Decimal::from(self.params.max_supply_micro)
        } else {
            Decimal::ZERO
        };

        let next_halving_round = ((self.current_round / self.params.halving_interval_rounds) + 1)
            * self.params.halving_interval_rounds
            + 1;

        SupplyInfo {
            total_supply: self.total_supply,
            supply_cap: self.params.max_supply_micro,
            remaining_supply: remaining,
            emission_percentage,
            current_round: self.current_round,
            next_halving_round,
        }
    }

    /// Generate emission curve for a range of rounds
    pub fn generate_emission_curve(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
        step: RoundIndex,
    ) -> Result<Vec<EmissionCurvePoint>, EmissionError> {
        if start_round > end_round {
            return Err(EmissionError::InvalidRoundIndex(start_round));
        }

        let mut curve = Vec::new();
        let mut cumulative_supply = self.total_supply;

        for round in (start_round..=end_round).step_by(step as usize) {
            let reward = self.calculate_round_reward(round)?;
            cumulative_supply =
                cumulative_supply
                    .checked_add(reward)
                    .ok_or(EmissionError::CalculationOverflow(
                        "Cumulative supply overflow".to_string(),
                    ))?;

            let halving_epoch = (round - 1) / self.params.halving_interval_rounds;
            let annual_issuance = reward * 315_360_000; // Approximate seconds per year at 10 rounds/second

            curve.push(EmissionCurvePoint {
                round,
                reward_per_round: reward,
                annual_issuance,
                cumulative_supply,
                halving_epoch: halving_epoch as u32,
            });
        }

        Ok(curve)
    }

    /// Get current emission parameters
    pub fn params(&self) -> &EmissionParams {
        &self.params
    }

    /// Update emission parameters (requires governance approval)
    pub fn update_params(&mut self, new_params: EmissionParams) -> Result<(), EmissionError> {
        // Validate new parameters
        if new_params.initial_round_reward_micro == 0 {
            return Err(EmissionError::InvalidParameters(
                "Initial round reward must be > 0".to_string(),
            ));
        }

        if new_params.halving_interval_rounds == 0 {
            return Err(EmissionError::InvalidParameters(
                "Halving interval must be > 0".to_string(),
            ));
        }

        if new_params.max_supply_micro == 0 {
            return Err(EmissionError::InvalidParameters(
                "Total supply cap must be > 0".to_string(),
            ));
        }

        self.params = new_params;
        info!("Updated emission parameters: {:?}", self.params);
        Ok(())
    }

    /// Get current round
    pub fn current_round(&self) -> RoundIndex {
        self.current_round
    }

    /// Get total supply emitted
    pub fn total_supply(&self) -> RewardAmount {
        self.total_supply
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
        let halving_round = engine.params().halving_interval_rounds + 1;
        assert_eq!(engine.calculate_round_reward(halving_round).unwrap(), 5_000);
    }

    #[test]
    fn test_supply_cap_enforcement() {
        let mut params = EmissionParams::default();
        params.max_supply_micro = 100_000; // Very low cap for testing
        params.initial_round_reward_micro = 50_000; // Higher than cap

        let mut engine = EmissionEngine::with_params(params);

        // First round should be capped
        assert_eq!(engine.calculate_round_reward(1).unwrap(), 50_000);

        // Advance to round 1
        engine.advance_round(1).unwrap();

        // Second round should be capped to remaining supply
        assert_eq!(engine.calculate_round_reward(2).unwrap(), 50_000);

        // Advance to round 2
        engine.advance_round(2).unwrap();

        // Third round should be 0 (cap reached)
        assert_eq!(engine.calculate_round_reward(3).unwrap(), 0);
    }

    #[test]
    fn test_emission_curve_generation() {
        let engine = EmissionEngine::new();
        let curve = engine.generate_emission_curve(1, 100, 10).unwrap();

        assert!(!curve.is_empty());
        assert_eq!(curve[0].round, 1);
        assert_eq!(curve[0].reward_per_round, 10_000);
    }

    #[test]
    fn test_supply_tracking() {
        let mut engine = EmissionEngine::new();

        // Advance through several rounds
        for round in 1..=1000 {
            engine.advance_round(round).unwrap();
        }

        let supply_info = engine.get_supply_info();
        assert!(supply_info.total_supply <= engine.params().max_supply_micro);
    }
}
