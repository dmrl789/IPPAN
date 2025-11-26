//! Validator Bonding Mechanism for DLC
//!
//! Requires validators to bond 10 IPN to participate in consensus.
//! Bonds can be slashed for misbehavior.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ippan_types::ValidatorId;

/// Required validator bond amount (10 IPN in micro-IPN)
pub const VALIDATOR_BOND_AMOUNT: u64 = 10 * 100_000_000; // 10 IPN

/// Minimum bond to remain active (allow up to 50% slashing before deactivation)
pub const MIN_BOND_AMOUNT: u64 = VALIDATOR_BOND_AMOUNT / 2;

/// Validator bond record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorBond {
    pub validator_id: ValidatorId,
    pub bonded_amount: u64,
    pub bonded_at: u64,
    pub is_active: bool,
    pub slashed_amount: u64,
    pub last_activity: u64,
}

impl ValidatorBond {
    pub fn new(validator_id: ValidatorId, amount: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            validator_id,
            bonded_amount: amount,
            bonded_at: now,
            is_active: amount >= MIN_BOND_AMOUNT,
            slashed_amount: 0,
            last_activity: now,
        }
    }

    /// Check if bond is valid
    pub fn is_valid(&self) -> bool {
        self.is_active && self.effective_bond() >= MIN_BOND_AMOUNT
    }

    /// Get effective bond after slashing
    pub fn effective_bond(&self) -> u64 {
        self.bonded_amount.saturating_sub(self.slashed_amount)
    }

    /// Slash a portion of the bond
    pub fn slash(&mut self, amount: u64) {
        self.slashed_amount += amount;
        if self.effective_bond() < MIN_BOND_AMOUNT {
            self.is_active = false;
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Manages validator bonds for DLC consensus
pub struct BondingManager {
    bonds: HashMap<ValidatorId, ValidatorBond>,
}

impl BondingManager {
    pub fn new() -> Self {
        Self {
            bonds: HashMap::new(),
        }
    }

    /// Add a validator bond
    pub fn add_bond(&mut self, validator_id: ValidatorId, amount: u64) -> Result<()> {
        if amount < VALIDATOR_BOND_AMOUNT {
            return Err(anyhow::anyhow!(
                "Bond amount must be at least {VALIDATOR_BOND_AMOUNT} micro-IPN (10 IPN)"
            ));
        }

        let bond = ValidatorBond::new(validator_id, amount);
        self.bonds.insert(validator_id, bond);

        Ok(())
    }

    /// Check if validator has a valid bond
    pub fn has_valid_bond(&self, validator_id: &ValidatorId) -> bool {
        self.bonds
            .get(validator_id)
            .map(|bond| bond.is_valid())
            .unwrap_or(false)
    }

    /// Get bond information
    pub fn get_bond(&self, validator_id: &ValidatorId) -> Option<&ValidatorBond> {
        self.bonds.get(validator_id)
    }

    /// Slash a validator's bond
    pub fn slash_bond(&mut self, validator_id: &ValidatorId, amount: u64) -> Result<()> {
        let bond = self
            .bonds
            .get_mut(validator_id)
            .ok_or_else(|| anyhow::anyhow!("Validator bond not found"))?;

        bond.slash(amount);

        Ok(())
    }

    /// Update validator activity
    pub fn update_activity(&mut self, validator_id: &ValidatorId) -> Result<()> {
        let bond = self
            .bonds
            .get_mut(validator_id)
            .ok_or_else(|| anyhow::anyhow!("Validator bond not found"))?;

        bond.update_activity();

        Ok(())
    }

    /// Get all active bonded validators
    pub fn get_active_validators(&self) -> Vec<ValidatorId> {
        self.bonds
            .iter()
            .filter(|(_, bond)| bond.is_valid())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get total bonded amount across all validators
    pub fn total_bonded(&self) -> u64 {
        self.bonds.values().map(|bond| bond.effective_bond()).sum()
    }

    /// Remove a bond (for withdrawals)
    pub fn remove_bond(&mut self, validator_id: &ValidatorId) -> Result<ValidatorBond> {
        self.bonds
            .remove(validator_id)
            .ok_or_else(|| anyhow::anyhow!("Validator bond not found"))
    }
}

impl Default for BondingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_creation() {
        let validator_id = [1u8; 32];
        let bond = ValidatorBond::new(validator_id, VALIDATOR_BOND_AMOUNT);

        assert_eq!(bond.validator_id, validator_id);
        assert_eq!(bond.bonded_amount, VALIDATOR_BOND_AMOUNT);
        assert!(bond.is_valid());
        assert_eq!(bond.effective_bond(), VALIDATOR_BOND_AMOUNT);
    }

    #[test]
    fn test_bond_slashing() {
        let validator_id = [1u8; 32];
        let mut bond = ValidatorBond::new(validator_id, VALIDATOR_BOND_AMOUNT);

        bond.slash(100_000_000); // Slash 1 IPN
        assert_eq!(bond.effective_bond(), VALIDATOR_BOND_AMOUNT - 100_000_000);
        assert!(bond.is_valid()); // Still above minimum

        bond.slash(900_000_000); // Slash 9 more IPN (total 10 IPN slashed)
        assert_eq!(bond.effective_bond(), 0);
        assert!(!bond.is_valid()); // Below minimum
    }

    #[test]
    fn test_bonding_manager() {
        let mut manager = BondingManager::new();
        let validator_id = [1u8; 32];

        // Add bond
        assert!(manager
            .add_bond(validator_id, VALIDATOR_BOND_AMOUNT)
            .is_ok());
        assert!(manager.has_valid_bond(&validator_id));

        // Check active validators
        let active = manager.get_active_validators();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], validator_id);
    }

    #[test]
    fn test_minimum_bond_requirement() {
        let mut manager = BondingManager::new();
        let validator_id = [1u8; 32];

        // Try to bond less than minimum
        assert!(manager
            .add_bond(validator_id, VALIDATOR_BOND_AMOUNT - 1)
            .is_err());

        // Bond exactly minimum
        assert!(manager
            .add_bond(validator_id, VALIDATOR_BOND_AMOUNT)
            .is_ok());
    }

    #[test]
    fn test_total_bonded() {
        let mut manager = BondingManager::new();

        manager.add_bond([1u8; 32], VALIDATOR_BOND_AMOUNT).unwrap();
        manager.add_bond([2u8; 32], VALIDATOR_BOND_AMOUNT).unwrap();
        manager
            .add_bond([3u8; 32], VALIDATOR_BOND_AMOUNT * 2)
            .unwrap();

        assert_eq!(manager.total_bonded(), VALIDATOR_BOND_AMOUNT * 4);
    }
}
