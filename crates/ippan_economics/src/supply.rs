//! Supply tracking and integrity verification

use crate::errors::*;
use crate::types::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

        // Check if this would exceed the supply cap
        if self.total_supply.saturating_add(amount) > self.supply_cap {
            warn!(
                "Emission would exceed supply cap: current={}, emission={}, cap={}",
                self.total_supply, amount, self.supply_cap
            );

            // Cap the emission to the remaining supply
            let capped_amount = self.supply_cap.saturating_sub(self.total_supply);
            self.total_supply = self.supply_cap;
            self.emission_history.insert(round, capped_amount);

            info!(
                "Capped emission for round {} to {} micro-IPN",
                round, capped_amount
            );
        } else {
            self.total_supply =
                self.total_supply
                    .checked_add(amount)
                    .ok_or(SupplyError::SupplyCapViolation(
                        "Supply addition overflow".to_string(),
                    ))?;
            self.emission_history.insert(round, amount);
        }

        self.current_round = round;
        debug!(
            "Recorded emission for round {}: {} micro-IPN",
            round, amount
        );
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
                    "Supply subtraction underflow".to_string(),
                ))?;

        self.burn_history.insert(round, amount);

        debug!("Recorded burn for round {}: {} micro-IPN", round, amount);
        Ok(())
    }

    /// Verify supply integrity against emission schedule
    pub fn verify_supply_integrity(
        &self,
        expected_supply: RewardAmount,
        tolerance: RewardAmount,
    ) -> Result<(), SupplyError> {
        let difference = if self.total_supply > expected_supply {
            self.total_supply - expected_supply
        } else {
            expected_supply - self.total_supply
        };

        if difference > tolerance {
            return Err(SupplyError::VerificationFailed(format!(
                "Supply verification failed: expected {}, actual {}",
                expected_supply, self.total_supply
            )));
        }

        info!(
            "Supply integrity verified: actual={}, expected={}, difference={}",
            self.total_supply, expected_supply, difference
        );
        Ok(())
    }

    /// Get current supply information
    pub fn get_supply_info(&self) -> SupplyInfo {
        let remaining_supply = self.supply_cap.saturating_sub(self.total_supply);
        let emission_percentage = if self.supply_cap > 0 {
            Decimal::from(self.total_supply) / Decimal::from(self.supply_cap) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Calculate next halving round (simplified - assumes 630M round halving interval)
        let halving_interval = 630_000_000;
        let next_halving_round = ((self.current_round / halving_interval) + 1) * halving_interval;

        SupplyInfo {
            total_supply: self.total_supply,
            supply_cap: self.supply_cap,
            remaining_supply,
            emission_percentage,
            current_round: self.current_round,
            next_halving_round,
        }
    }

    /// Get emission history for a range of rounds
    pub fn get_emission_history(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
    ) -> HashMap<RoundIndex, RewardAmount> {
        self.emission_history
            .iter()
            .filter(|(round, _)| **round >= start_round && **round <= end_round)
            .map(|(round, amount)| (*round, *amount))
            .collect()
    }

    /// Get burn history for a range of rounds
    pub fn get_burn_history(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
    ) -> HashMap<RoundIndex, RewardAmount> {
        self.burn_history
            .iter()
            .filter(|(round, _)| **round >= start_round && **round <= end_round)
            .map(|(round, amount)| (*round, *amount))
            .collect()
    }

    /// Calculate total emissions in a range
    pub fn calculate_total_emissions(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
    ) -> RewardAmount {
        self.get_emission_history(start_round, end_round)
            .values()
            .sum()
    }

    /// Calculate total burns in a range
    pub fn calculate_total_burns(
        &self,
        start_round: RoundIndex,
        end_round: RoundIndex,
    ) -> RewardAmount {
        self.get_burn_history(start_round, end_round).values().sum()
    }

    /// Perform comprehensive supply audit
    pub fn audit_supply(&self) -> SupplyAuditResult {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Check if supply exceeds cap
        if self.total_supply > self.supply_cap {
            issues.push(format!(
                "Supply exceeds cap: {} > {}",
                self.total_supply, self.supply_cap
            ));
        }

        // Check for negative supply (should never happen)
        if self.total_supply == 0 && self.current_round > 0 {
            warnings.push("Zero supply with non-zero rounds".to_string());
        }

        // Check for missing emission history
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

        // Calculate net supply change
        let total_emissions: RewardAmount = self.emission_history.values().sum();
        let total_burns: RewardAmount = self.burn_history.values().sum();
        let calculated_supply = total_emissions.saturating_sub(total_burns);

        if calculated_supply != self.total_supply {
            issues.push(format!(
                "Supply mismatch: recorded={}, calculated={}",
                self.total_supply, calculated_supply
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

    /// Update the last verification round
    pub fn update_verification_round(&mut self, round: RoundIndex) {
        self.last_verification_round = round;
    }

    /// Get current total supply
    pub fn total_supply(&self) -> RewardAmount {
        self.total_supply
    }

    /// Get supply cap
    pub fn supply_cap(&self) -> RewardAmount {
        self.supply_cap
    }

    /// Get current round
    pub fn current_round(&self) -> RoundIndex {
        self.current_round
    }
}

/// Result of a supply audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyAuditResult {
    /// Whether the supply is healthy (no critical issues)
    pub is_healthy: bool,
    /// Critical issues that need immediate attention
    pub issues: Vec<String>,
    /// Warnings that should be investigated
    pub warnings: Vec<String>,
    /// Total emissions recorded
    pub total_emissions: RewardAmount,
    /// Total burns recorded
    pub total_burns: RewardAmount,
    /// Net supply (emissions - burns)
    pub net_supply: RewardAmount,
}

impl Default for SupplyTracker {
    fn default() -> Self {
        Self::new(2_100_000_000_000) // 21M IPN in micro-IPN
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
    fn test_supply_cap_enforcement() {
        let mut tracker = SupplyTracker::new(100_000);

        // Try to emit more than the cap
        tracker.record_emission(1, 150_000).unwrap();
        assert_eq!(tracker.total_supply(), 100_000); // Should be capped
    }

    #[test]
    fn test_supply_verification() {
        let tracker = SupplyTracker::new(1_000_000);

        // Should pass with tolerance
        tracker.verify_supply_integrity(0, 1000).unwrap();

        // Should fail without tolerance
        assert!(tracker.verify_supply_integrity(100_000, 0).is_err());
    }
}
