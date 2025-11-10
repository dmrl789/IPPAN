//! Supply tracking and integrity verification
//!
//! Tracks deterministic token supply over time and ensures that emissions,
//! burns, and caps remain within protocol-defined limits. Used by the DAG-Fair
//! emission engine to enforce the 21 million IPN lifetime ceiling.

use crate::errors::*;
use crate::types::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use tracing::{debug, error, info, warn};

/// Supply tracking and verification system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyTracker {
    /// Current total supply (in micro-IPN)
    total_supply: RewardAmount,
    /// Supply cap (in micro-IPN)
    supply_cap: RewardAmount,
    /// Current round index
    current_round: RoundIndex,
    /// Emission history for verification
    emission_history: HashMap<RoundIndex, RewardAmount>,
    /// Burn history for verification
    burn_history: HashMap<RoundIndex, RewardAmount>,
    /// Network dividend accumulation history
    dividend_history: HashMap<RoundIndex, RewardAmount>,
    /// Total accumulated network dividend
    total_dividend_accumulated: RewardAmount,
    /// Last verification round
    last_verification_round: RoundIndex,
}

impl SupplyTracker {
    /// Create a new supply tracker
    pub fn new(supply_cap: RewardAmount) -> Self {
        Self {
            total_supply: 0,
            supply_cap,
            current_round: 0,
            emission_history: HashMap::new(),
            burn_history: HashMap::new(),
            dividend_history: HashMap::new(),
            total_dividend_accumulated: 0,
            last_verification_round: 0,
        }
    }

    /// Record emission for a round
    pub fn record_emission(
        &mut self,
        round: RoundIndex,
        amount: RewardAmount,
    ) -> Result<(), SupplyError> {
        if round <= self.current_round {
            return Err(SupplyError::InvalidSupplyData(format!(
                "Cannot record emission for past round: {} <= {}",
                round, self.current_round
            )));
        }

        // Cap enforcement
        if self.total_supply.saturating_add(amount) > self.supply_cap {
            warn!(
                "Emission would exceed cap: current={}, emission={}, cap={}",
                self.total_supply, amount, self.supply_cap
            );

            let capped_amount = self.supply_cap.saturating_sub(self.total_supply);
            self.total_supply = self.supply_cap;
            self.emission_history.insert(round, capped_amount);

            info!("Capped emission for round {} to {}", round, capped_amount);
        } else {
            self.total_supply =
                self.total_supply
                    .checked_add(amount)
                    .ok_or(SupplyError::SupplyCapViolation(
                        "Overflow adding supply".into(),
                    ))?;
            self.emission_history.insert(round, amount);
        }

        self.current_round = round;
        debug!("Recorded emission for round {}: {}", round, amount);
        Ok(())
    }

    /// Record burn for a round (excess fees, rounding errors, etc.)
    pub fn record_burn(
        &mut self,
        round: RoundIndex,
        amount: RewardAmount,
    ) -> Result<(), SupplyError> {
        if amount > self.total_supply {
            return Err(SupplyError::InvalidSupplyData(format!(
                "Cannot burn more than total supply: {} > {}",
                amount, self.total_supply
            )));
        }

        self.total_supply =
            self.total_supply
                .checked_sub(amount)
                .ok_or(SupplyError::SupplyCapViolation(
                    "Underflow subtracting supply".into(),
                ))?;

        self.burn_history.insert(round, amount);
        debug!("Recorded burn for round {}: {}", round, amount);
        Ok(())
    }

    /// Verify supply integrity against expected schedule
    pub fn verify_supply_integrity(
        &self,
        expected_supply: RewardAmount,
        tolerance: RewardAmount,
    ) -> Result<(), SupplyError> {
        let diff = self.total_supply.abs_diff(expected_supply);

        if diff > tolerance {
            error!(
                "Supply verification failed: expected {}, actual {}, diff {}",
                expected_supply, self.total_supply, diff
            );
            return Err(SupplyError::VerificationFailed(format!(
                "Supply mismatch: expected {}, actual {}",
                expected_supply, self.total_supply
            )));
        }

        info!(
            "Supply integrity verified: actual={}, expected={}, diff={}",
            self.total_supply, expected_supply, diff
        );
        Ok(())
    }

    /// Return current supply snapshot
    pub fn get_supply_info(&self) -> SupplyInfo {
        let remaining = self.supply_cap.saturating_sub(self.total_supply);
        let percent = if self.supply_cap > 0 {
            Decimal::from(self.total_supply) / Decimal::from(self.supply_cap) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let halving_interval = 630_000_000;
        let next_halving_round = ((self.current_round / halving_interval) + 1) * halving_interval;

        SupplyInfo {
            total_supply: self.total_supply,
            supply_cap: self.supply_cap,
            remaining_supply: remaining,
            emission_percentage: percent,
            current_round: self.current_round,
            next_halving_round,
        }
    }

    /// Emission history within range
    pub fn get_emission_history(
        &self,
        start: RoundIndex,
        end: RoundIndex,
    ) -> HashMap<RoundIndex, RewardAmount> {
        self.emission_history
            .iter()
            .filter(|(r, _)| **r >= start && **r <= end)
            .map(|(r, a)| (*r, *a))
            .collect()
    }

    /// Burn history within range
    pub fn get_burn_history(
        &self,
        start: RoundIndex,
        end: RoundIndex,
    ) -> HashMap<RoundIndex, RewardAmount> {
        self.burn_history
            .iter()
            .filter(|(r, _)| **r >= start && **r <= end)
            .map(|(r, a)| (*r, *a))
            .collect()
    }

    /// Total emissions in range
    pub fn calculate_total_emissions(&self, start: RoundIndex, end: RoundIndex) -> RewardAmount {
        self.get_emission_history(start, end).values().sum()
    }

    /// Total burns in range
    pub fn calculate_total_burns(&self, start: RoundIndex, end: RoundIndex) -> RewardAmount {
        self.get_burn_history(start, end).values().sum()
    }

    /// Perform comprehensive audit of supply
    pub fn audit_supply(&self) -> SupplyAuditResult {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        if self.total_supply > self.supply_cap {
            issues.push(format!(
                "Supply exceeds cap: {} > {}",
                self.total_supply, self.supply_cap
            ));
        }

        if self.total_supply == 0 && self.current_round > 0 {
            warnings.push("Zero supply with non-zero rounds".into());
        }

        let expected_rounds = self
            .current_round
            .saturating_sub(self.last_verification_round);
        if expected_rounds > 0 && self.emission_history.len() < expected_rounds as usize {
            warnings.push(format!(
                "Missing emission history: expected {} rounds, found {}",
                expected_rounds,
                self.emission_history.len()
            ));
        }

        let total_emissions: RewardAmount = self.emission_history.values().sum();
        let total_burns: RewardAmount = self.burn_history.values().sum();
        let computed = total_emissions.saturating_sub(total_burns);

        if computed != self.total_supply {
            issues.push(format!(
                "Supply mismatch: recorded={}, computed={}",
                self.total_supply, computed
            ));
        }

        SupplyAuditResult {
            is_healthy: issues.is_empty(),
            issues,
            warnings,
            total_emissions,
            total_burns,
            net_supply: self.total_supply,
        }
    }

    pub fn update_verification_round(&mut self, round: RoundIndex) {
        self.last_verification_round = round;
    }

    pub fn total_supply(&self) -> RewardAmount {
        self.total_supply
    }

    pub fn supply_cap(&self) -> RewardAmount {
        self.supply_cap
    }

    pub fn current_round(&self) -> RoundIndex {
        self.current_round
    }

    /// Record network dividend accumulation for a round
    pub fn record_dividend(
        &mut self,
        round: RoundIndex,
        amount: RewardAmount,
    ) -> Result<(), SupplyError> {
        self.total_dividend_accumulated =
            self.total_dividend_accumulated.checked_add(amount).ok_or(
                SupplyError::InvalidSupplyData("Overflow adding dividend".into()),
            )?;

        self.dividend_history.insert(round, amount);
        debug!("Recorded dividend for round {}: {}", round, amount);
        Ok(())
    }

    /// Get total accumulated network dividend
    pub fn total_dividend_accumulated(&self) -> RewardAmount {
        self.total_dividend_accumulated
    }

    /// Distribute accumulated dividend and reset counter
    pub fn distribute_dividend(&mut self, amount: RewardAmount) -> Result<(), SupplyError> {
        if amount > self.total_dividend_accumulated {
            return Err(SupplyError::InvalidSupplyData(format!(
                "Cannot distribute more dividend than accumulated: {} > {}",
                amount, self.total_dividend_accumulated
            )));
        }

        self.total_dividend_accumulated =
            self.total_dividend_accumulated.checked_sub(amount).ok_or(
                SupplyError::InvalidSupplyData("Underflow subtracting dividend".into()),
            )?;

        info!(
            "Distributed {} micro-IPN from network dividend pool",
            amount
        );
        Ok(())
    }

    /// Get dividend history within range
    pub fn get_dividend_history(
        &self,
        start: RoundIndex,
        end: RoundIndex,
    ) -> HashMap<RoundIndex, RewardAmount> {
        self.dividend_history
            .iter()
            .filter(|(r, _)| **r >= start && **r <= end)
            .map(|(r, a)| (*r, *a))
            .collect()
    }
}

/// Deterministic reward for a specific round based on emission parameters.
pub fn scheduled_round_reward(round: RoundIndex, params: &EmissionParams) -> RewardAmount {
    if round == 0 {
        return 0;
    }

    let halving_interval = params.halving_interval_rounds.max(1);
    let halving_epoch = (round - 1) / halving_interval;

    if halving_epoch >= 64 {
        return 0;
    }

    params
        .initial_round_reward_micro
        .checked_shr(halving_epoch as u32)
        .unwrap_or(0)
}

/// Projected cumulative supply after a number of rounds following the halving schedule.
pub fn projected_supply(rounds: RoundIndex, params: &EmissionParams) -> u128 {
    if rounds == 0 {
        return 0;
    }

    let halving_interval = params.halving_interval_rounds.max(1);
    let mut remaining_rounds = rounds;
    let mut reward: u128 = params.initial_round_reward_micro as u128;
    let mut total: u128 = 0;
    let mut epoch: u32 = 0;

    while remaining_rounds > 0 && reward > 0 && epoch < 64 {
        let rounds_this_epoch = remaining_rounds.min(halving_interval);
        let epoch_emission = reward.saturating_mul(rounds_this_epoch as u128);
        total = total.saturating_add(epoch_emission);

        if total >= params.max_supply_micro as u128 {
            return params.max_supply_micro as u128;
        }

        remaining_rounds -= rounds_this_epoch;
        reward >>= 1;
        epoch += 1;
    }

    total.min(params.max_supply_micro as u128)
}

/// Compute the theoretical round at which the supply cap would be reached, if achievable.
pub fn rounds_until_supply_cap(params: &EmissionParams) -> Option<RoundIndex> {
    if params.initial_round_reward_micro == 0 {
        return None;
    }

    let halving_interval = params.halving_interval_rounds.max(1) as u128;
    let mut reward: u128 = params.initial_round_reward_micro as u128;
    let mut emitted: u128 = 0;
    let mut rounds_accumulated: u128 = 0;

    for _epoch in 0..64 {
        if reward == 0 {
            break;
        }

        let epoch_capacity = reward.saturating_mul(halving_interval);
        let cap = params.max_supply_micro as u128;

        if emitted.saturating_add(epoch_capacity) >= cap {
            let remaining = cap.saturating_sub(emitted);
            let rounds_needed = remaining.div_ceil(reward); // ceil division
            let total_rounds =
                rounds_accumulated.saturating_add(rounds_needed.min(halving_interval));
            return total_rounds.try_into().ok();
        }

        emitted = emitted.saturating_add(epoch_capacity);
        rounds_accumulated = rounds_accumulated.saturating_add(halving_interval);
        reward >>= 1;
    }

    None
}

/// Result of a supply audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyAuditResult {
    pub is_healthy: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub total_emissions: RewardAmount,
    pub total_burns: RewardAmount,
    pub net_supply: RewardAmount,
}

impl Default for SupplyTracker {
    fn default() -> Self {
        Self::new(21_000_000_000_000) // 21M IPN in micro-IPN (21M * 1M µIPN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supply_tracking() {
        let mut tracker = SupplyTracker::new(1_000_000);

        tracker.record_emission(1, 100_000).unwrap();
        assert_eq!(tracker.total_supply(), 100_000);

        tracker.record_emission(2, 200_000).unwrap();
        assert_eq!(tracker.total_supply(), 300_000);

        tracker.record_burn(2, 50_000).unwrap();
        assert_eq!(tracker.total_supply(), 250_000);
    }

    #[test]
    fn test_cap_enforcement() {
        let mut tracker = SupplyTracker::new(100_000);
        tracker.record_emission(1, 150_000).unwrap();
        assert_eq!(tracker.total_supply(), 100_000);
    }

    #[test]
    fn test_verification_tolerance() {
        let tracker = SupplyTracker::new(1_000_000);
        tracker.verify_supply_integrity(0, 1_000).unwrap();
        assert!(tracker.verify_supply_integrity(100_000, 0).is_err());
    }

    #[test]
    fn test_scheduled_round_reward_halving() {
        let params = EmissionParams::default();
        assert_eq!(
            scheduled_round_reward(1, &params),
            params.initial_round_reward_micro
        );

        let halving_round = params.halving_interval_rounds + 1;
        assert_eq!(
            scheduled_round_reward(halving_round, &params),
            params.initial_round_reward_micro / 2
        );
    }

    #[test]
    fn test_projected_supply_monotonic_and_capped() {
        let params = EmissionParams::default();
        let year_rounds = 315_360_000; // ~1 year @ 10 rounds/sec
        let supply_year1 = projected_supply(year_rounds, &params);
        let supply_year2 = projected_supply(year_rounds * 2, &params);
        assert!(supply_year2 > supply_year1);
        assert!(supply_year2 <= params.max_supply_micro as u128);
    }

    #[test]
    fn test_rounds_until_supply_cap_for_custom_params() {
        let params = EmissionParams {
            initial_round_reward_micro: 100_000,
            halving_interval_rounds: 10_000,
            max_supply_micro: 1_500_000_000,
            fee_cap_fraction: Decimal::new(1, 1),
            proposer_weight_bps: 2000,
            verifier_weight_bps: 8000,
        };

        let rounds = rounds_until_supply_cap(&params);
        assert!(rounds.is_some());
    }

    #[test]
    fn test_dividend_tracking() {
        let mut tracker = SupplyTracker::new(1_000_000);

        // Record dividend accumulation
        tracker.record_dividend(1, 100).unwrap();
        assert_eq!(tracker.total_dividend_accumulated(), 100);

        tracker.record_dividend(2, 150).unwrap();
        assert_eq!(tracker.total_dividend_accumulated(), 250);

        // Distribute some dividend
        tracker.distribute_dividend(100).unwrap();
        assert_eq!(tracker.total_dividend_accumulated(), 150);

        // Cannot distribute more than accumulated
        assert!(tracker.distribute_dividend(200).is_err());
    }
}
