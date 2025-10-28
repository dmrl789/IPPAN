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
    pub fn record_emission(&mut self, round: RoundIndex, amount: RewardAmount) -> Result<(), SupplyError> {
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
            self.total_supply = self
                .total_supply
                .checked_add(amount)
                .ok_or(SupplyError::SupplyCapViolation("Overflow adding supply".into()))?;
            self.emission_history.insert(round, amount);
        }

        self.current_round = round;
        debug!("Recorded emission for round {}: {}", round, amount);
        Ok(())
    }

    /// Record burn for a round (excess fees, rounding errors, etc.)
    pub fn record_burn(&mut self, round: RoundIndex, amount: RewardAmount) -> Result<(), SupplyError> {
        if amount > self.total_supply {
            return Err(SupplyError::InvalidSupplyData(format!(
                "Cannot burn more than total supply: {} > {}",
                amount, self.total_supply
            )));
        }

        self.total_supply = self
            .total_supply
            .checked_sub(amount)
            .ok_or(SupplyError::SupplyCapViolation("Underflow subtracting supply".into()))?;

        self.burn_history.insert(round, amount);
        debug!("Recorded burn for round {}: {}", round, amount);
        Ok(())
    }

    /// Verify supply integrity against expected schedule
    pub fn verify_supply_integrity(&self, expected_supply: RewardAmount, tolerance: RewardAmount) -> Result<(), SupplyError> {
        let diff = if self.total_supply > expected_supply {
            self.total_supply - expected_supply
        } else {
            expected_supply - self.total_supply
        };

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
    pub fn get_emission_history(&self, start: RoundIndex, end: RoundIndex) -> HashMap<RoundIndex, RewardAmount> {
        self.emission_history
            .iter()
            .filter(|(r, _)| **r >= start && **r <= end)
            .map(|(r, a)| (*r, *a))
            .collect()
    }

    /// Burn history within range
    pub fn get_burn_history(&self, start: RoundIndex, end: RoundIndex) -> HashMap<RoundIndex, RewardAmount> {
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

        let expected_rounds = self.current_round.saturating_sub(self.last_verification_round);
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
        Self::new(2_100_000_000_000) // 21 M IPN in micro-IPN
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
}
