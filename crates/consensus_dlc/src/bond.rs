//! Validator bonding and slashing for DLC consensus
//! 
//! This module handles validator bonds (stake deposits), slashing
//! for malicious behavior, and bond management.

use crate::error::{DlcError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum validator bond amount (in micro-IPN)
pub const VALIDATOR_BOND: u64 = 10 * 10u64.pow(8); // 10 IPN

/// Minimum bond to become a validator
pub const MIN_VALIDATOR_BOND: u64 = 10 * 10u64.pow(8); // 10 IPN

/// Maximum bond amount
pub const MAX_VALIDATOR_BOND: u64 = 1_000_000 * 10u64.pow(8); // 1 million IPN

/// Slash percentage for double signing (in basis points)
pub const DOUBLE_SIGN_SLASH_BPS: u64 = 5000; // 50%

/// Slash percentage for downtime (in basis points)
pub const DOWNTIME_SLASH_BPS: u64 = 100; // 1%

/// Slash percentage for invalid block (in basis points)
pub const INVALID_BLOCK_SLASH_BPS: u64 = 1000; // 10%

/// Validator bond status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BondStatus {
    /// Bond is active and validator can participate
    Active,
    /// Bond is being unstaked (time-locked)
    Unstaking { unlock_round: u64 },
    /// Bond has been slashed
    Slashed { reason: String, amount: u64 },
    /// Bond is frozen (temporarily inactive)
    Frozen { reason: String },
    /// Bond has been withdrawn
    Withdrawn,
}

/// A validator's bond/stake
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorBond {
    /// Owner of the bond
    pub owner: String,
    /// Total bonded amount (in micro-IPN)
    pub amount: u64,
    /// Current status
    pub status: BondStatus,
    /// When the bond was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Total slashed amount (historical)
    pub total_slashed: u64,
    /// Slashing history
    pub slash_history: Vec<SlashEvent>,
}

impl ValidatorBond {
    /// Create a new validator bond
    pub fn new(owner: impl Into<String>, amount: u64) -> Result<Self> {
        let owner = owner.into();
        
        if amount < MIN_VALIDATOR_BOND {
            return Err(DlcError::InvalidBond(format!(
                "Bond amount {} is below minimum {}",
                amount, MIN_VALIDATOR_BOND
            )));
        }

        if amount > MAX_VALIDATOR_BOND {
            return Err(DlcError::InvalidBond(format!(
                "Bond amount {} exceeds maximum {}",
                amount, MAX_VALIDATOR_BOND
            )));
        }

        Ok(Self {
            owner,
            amount,
            status: BondStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            total_slashed: 0,
            slash_history: Vec::new(),
        })
    }

    /// Check if bond is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, BondStatus::Active)
    }

    /// Check if bond can participate in consensus
    pub fn can_participate(&self) -> bool {
        self.is_active() && self.amount >= MIN_VALIDATOR_BOND
    }

    /// Add more to the bond
    pub fn add_stake(&mut self, additional: u64) -> Result<()> {
        if !self.is_active() {
            return Err(DlcError::InvalidBond(
                "Cannot add stake to inactive bond".to_string(),
            ));
        }

        let new_amount = self.amount.saturating_add(additional);
        
        if new_amount > MAX_VALIDATOR_BOND {
            return Err(DlcError::InvalidBond(format!(
                "Total bond {} would exceed maximum {}",
                new_amount, MAX_VALIDATOR_BOND
            )));
        }

        self.amount = new_amount;
        self.updated_at = chrono::Utc::now();

        tracing::info!(
            "Added {} to bond for {}, new total: {}",
            additional,
            self.owner,
            self.amount
        );

        Ok(())
    }

    /// Initiate unstaking (with time lock)
    pub fn initiate_unstaking(&mut self, current_round: u64, lock_duration: u64) -> Result<()> {
        if !self.is_active() {
            return Err(DlcError::InvalidBond(
                "Bond must be active to unstake".to_string(),
            ));
        }

        let unlock_round = current_round + lock_duration;
        self.status = BondStatus::Unstaking { unlock_round };
        self.updated_at = chrono::Utc::now();

        tracing::info!(
            "Initiated unstaking for {}, unlock at round {}",
            self.owner,
            unlock_round
        );

        Ok(())
    }

    /// Complete unstaking and withdraw
    pub fn complete_unstaking(&mut self, current_round: u64) -> Result<u64> {
        match &self.status {
            BondStatus::Unstaking { unlock_round } => {
                if current_round < *unlock_round {
                    return Err(DlcError::InvalidBond(format!(
                        "Cannot withdraw before unlock round {}",
                        unlock_round
                    )));
                }

                let amount = self.amount;
                self.amount = 0;
                self.status = BondStatus::Withdrawn;
                self.updated_at = chrono::Utc::now();

                tracing::info!("Completed unstaking for {}, withdrew {}", self.owner, amount);

                Ok(amount)
            }
            _ => Err(DlcError::InvalidBond(
                "Bond must be in unstaking state".to_string(),
            )),
        }
    }

    /// Slash the bond for malicious behavior
    pub fn slash(&mut self, reason: String, percentage_bps: u64, round: u64) -> Result<u64> {
        if self.amount == 0 {
            return Err(DlcError::InvalidBond("Bond has no funds to slash".to_string()));
        }

        // Calculate slash amount
        let slash_amount = (self.amount as u128 * percentage_bps as u128 / 10_000u128) as u64;
        let slash_amount = slash_amount.min(self.amount);

        // Apply slash
        self.amount = self.amount.saturating_sub(slash_amount);
        self.total_slashed = self.total_slashed.saturating_add(slash_amount);

        // Record slash event
        let event = SlashEvent {
            reason: reason.clone(),
            amount: slash_amount,
            round,
            timestamp: chrono::Utc::now(),
        };
        self.slash_history.push(event);

        // Update status
        self.status = BondStatus::Slashed {
            reason: reason.clone(),
            amount: slash_amount,
        };
        self.updated_at = chrono::Utc::now();

        tracing::warn!(
            "Slashed {} from {} for '{}', remaining: {}",
            slash_amount,
            self.owner,
            reason,
            self.amount
        );

        Ok(slash_amount)
    }

    /// Freeze the bond (temporarily disable)
    pub fn freeze(&mut self, reason: String) -> Result<()> {
        if !self.is_active() {
            return Err(DlcError::InvalidBond("Bond must be active to freeze".to_string()));
        }

        self.status = BondStatus::Frozen { reason: reason.clone() };
        self.updated_at = chrono::Utc::now();

        tracing::warn!("Froze bond for {}: {}", self.owner, reason);

        Ok(())
    }

    /// Unfreeze the bond
    pub fn unfreeze(&mut self) -> Result<()> {
        if !matches!(self.status, BondStatus::Frozen { .. }) {
            return Err(DlcError::InvalidBond("Bond must be frozen to unfreeze".to_string()));
        }

        self.status = BondStatus::Active;
        self.updated_at = chrono::Utc::now();

        tracing::info!("Unfroze bond for {}", self.owner);

        Ok(())
    }

    /// Get bond value weight (for voting/selection purposes)
    pub fn voting_weight(&self) -> u64 {
        if self.is_active() {
            self.amount
        } else {
            0
        }
    }
}

/// Slash event record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlashEvent {
    pub reason: String,
    pub amount: u64,
    pub round: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Bond manager for all validators
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BondManager {
    /// All validator bonds
    bonds: HashMap<String, ValidatorBond>,
    /// Total bonded amount across all validators
    total_bonded: u64,
    /// Total slashed amount (historical)
    total_slashed: u64,
    /// Unstaking lock duration in rounds
    unstaking_lock_rounds: u64,
}

impl BondManager {
    /// Create a new bond manager
    pub fn new(unstaking_lock_rounds: u64) -> Self {
        Self {
            bonds: HashMap::new(),
            total_bonded: 0,
            total_slashed: 0,
            unstaking_lock_rounds,
        }
    }

    /// Create a new bond for a validator
    pub fn create_bond(&mut self, validator_id: String, amount: u64) -> Result<()> {
        if self.bonds.contains_key(&validator_id) {
            return Err(DlcError::InvalidBond(format!(
                "Validator {} already has a bond",
                validator_id
            )));
        }

        let bond = ValidatorBond::new(validator_id.clone(), amount)?;
        self.total_bonded = self.total_bonded.saturating_add(amount);
        self.bonds.insert(validator_id, bond);

        Ok(())
    }

    /// Get a validator's bond
    pub fn get_bond(&self, validator_id: &str) -> Option<&ValidatorBond> {
        self.bonds.get(validator_id)
    }

    /// Get a mutable reference to a validator's bond
    pub fn get_bond_mut(&mut self, validator_id: &str) -> Option<&mut ValidatorBond> {
        self.bonds.get_mut(validator_id)
    }

    /// Add stake to existing bond
    pub fn add_stake(&mut self, validator_id: &str, amount: u64) -> Result<()> {
        let bond = self
            .bonds
            .get_mut(validator_id)
            .ok_or_else(|| DlcError::ValidatorNotFound(validator_id.to_string()))?;

        bond.add_stake(amount)?;
        self.total_bonded = self.total_bonded.saturating_add(amount);

        Ok(())
    }

    /// Initiate unstaking for a validator
    pub fn initiate_unstaking(&mut self, validator_id: &str, current_round: u64) -> Result<()> {
        let bond = self
            .bonds
            .get_mut(validator_id)
            .ok_or_else(|| DlcError::ValidatorNotFound(validator_id.to_string()))?;

        bond.initiate_unstaking(current_round, self.unstaking_lock_rounds)
    }

    /// Complete unstaking and withdraw
    pub fn complete_unstaking(&mut self, validator_id: &str, current_round: u64) -> Result<u64> {
        let bond = self
            .bonds
            .get_mut(validator_id)
            .ok_or_else(|| DlcError::ValidatorNotFound(validator_id.to_string()))?;

        let amount = bond.complete_unstaking(current_round)?;
        self.total_bonded = self.total_bonded.saturating_sub(amount);

        Ok(amount)
    }

    /// Slash a validator's bond
    pub fn slash_validator(
        &mut self,
        validator_id: &str,
        reason: String,
        percentage_bps: u64,
        round: u64,
    ) -> Result<u64> {
        let bond = self
            .bonds
            .get_mut(validator_id)
            .ok_or_else(|| DlcError::ValidatorNotFound(validator_id.to_string()))?;

        let slashed_amount = bond.slash(reason, percentage_bps, round)?;
        self.total_bonded = self.total_bonded.saturating_sub(slashed_amount);
        self.total_slashed = self.total_slashed.saturating_add(slashed_amount);

        Ok(slashed_amount)
    }

    /// Get all active validators
    pub fn active_validators(&self) -> Vec<String> {
        self.bonds
            .iter()
            .filter(|(_, bond)| bond.can_participate())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get total voting power of all active validators
    pub fn total_voting_power(&self) -> u64 {
        self.bonds
            .values()
            .map(|bond| bond.voting_weight())
            .sum()
    }

    /// Get bond statistics
    pub fn stats(&self) -> BondStats {
        let active_count = self.active_validators().len();
        let total_validators = self.bonds.len();
        let avg_bond = if total_validators > 0 {
            self.total_bonded / total_validators as u64
        } else {
            0
        };

        BondStats {
            total_validators,
            active_validators: active_count,
            total_bonded: self.total_bonded,
            total_slashed: self.total_slashed,
            average_bond: avg_bond,
        }
    }
}

/// Bond statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondStats {
    pub total_validators: usize,
    pub active_validators: usize,
    pub total_bonded: u64,
    pub total_slashed: u64,
    pub average_bond: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_creation() {
        let bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        assert_eq!(bond.amount, VALIDATOR_BOND);
        assert!(bond.is_active());
    }

    #[test]
    fn test_bond_min_amount() {
        let result = ValidatorBond::new("validator1", MIN_VALIDATOR_BOND - 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_stake() {
        let mut bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        bond.add_stake(VALIDATOR_BOND).unwrap();
        assert_eq!(bond.amount, VALIDATOR_BOND * 2);
    }

    #[test]
    fn test_slashing() {
        let mut bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        let slashed = bond.slash("test violation".to_string(), 1000, 1).unwrap();
        
        // 10% slash
        assert_eq!(slashed, VALIDATOR_BOND / 10);
        assert_eq!(bond.amount, VALIDATOR_BOND - slashed);
        assert_eq!(bond.total_slashed, slashed);
    }

    #[test]
    fn test_unstaking() {
        let mut bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        
        bond.initiate_unstaking(100, 50).unwrap();
        
        match bond.status {
            BondStatus::Unstaking { unlock_round } => {
                assert_eq!(unlock_round, 150);
            }
            _ => panic!("Expected unstaking status"),
        }
    }

    #[test]
    fn test_complete_unstaking() {
        let mut bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        
        bond.initiate_unstaking(100, 50).unwrap();
        let withdrawn = bond.complete_unstaking(150).unwrap();
        
        assert_eq!(withdrawn, VALIDATOR_BOND);
        assert_eq!(bond.amount, 0);
    }

    #[test]
    fn test_early_withdrawal_fails() {
        let mut bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        
        bond.initiate_unstaking(100, 50).unwrap();
        let result = bond.complete_unstaking(140);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_freeze_unfreeze() {
        let mut bond = ValidatorBond::new("validator1", VALIDATOR_BOND).unwrap();
        
        bond.freeze("test reason".to_string()).unwrap();
        assert!(!bond.can_participate());
        
        bond.unfreeze().unwrap();
        assert!(bond.can_participate());
    }

    #[test]
    fn test_bond_manager() {
        let mut manager = BondManager::new(100);
        
        manager.create_bond("val1".to_string(), VALIDATOR_BOND).unwrap();
        manager.create_bond("val2".to_string(), VALIDATOR_BOND * 2).unwrap();
        
        assert_eq!(manager.total_bonded, VALIDATOR_BOND * 3);
        assert_eq!(manager.active_validators().len(), 2);
    }

    #[test]
    fn test_voting_weight() {
        let active_bond = ValidatorBond::new("val1", VALIDATOR_BOND).unwrap();
        assert_eq!(active_bond.voting_weight(), VALIDATOR_BOND);
        
        let mut slashed_bond = ValidatorBond::new("val2", VALIDATOR_BOND).unwrap();
        slashed_bond.slash("test".to_string(), 10000, 1).unwrap();
        assert_eq!(slashed_bond.voting_weight(), 0);
    }

    #[test]
    fn test_bond_stats() {
        let mut manager = BondManager::new(100);
        
        manager.create_bond("val1".to_string(), VALIDATOR_BOND).unwrap();
        manager.create_bond("val2".to_string(), VALIDATOR_BOND).unwrap();
        
        let stats = manager.stats();
        assert_eq!(stats.total_validators, 2);
        assert_eq!(stats.active_validators, 2);
    }
}
