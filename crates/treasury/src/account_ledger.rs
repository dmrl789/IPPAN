//! Account ledger interface for reward distribution

use ippan_types::{MicroIPN, ValidatorId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Interface for account ledger operations
pub trait AccountLedger: Send + Sync {
    /// Credit a validator's account with micro-IPN
    fn credit_validator(&mut self, validator_id: &ValidatorId, amount: MicroIPN) -> Result<()>;

    /// Get a validator's balance
    fn get_validator_balance(&self, validator_id: &ValidatorId) -> Result<MicroIPN>;

    /// Debit a validator's account (for fees, penalties, etc.)
    fn debit_validator(&mut self, validator_id: &ValidatorId, amount: MicroIPN) -> Result<()>;

    /// Get total supply
    fn get_total_supply(&self) -> Result<MicroIPN>;

    /// Get all validator balances
    fn get_all_balances(&self) -> Result<HashMap<ValidatorId, MicroIPN>>;
}

// -----------------------------------------------------------------------------
// 🧠 In-memory implementation
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InMemoryAccountLedger {
    balances: HashMap<ValidatorId, MicroIPN>,
    total_supply: MicroIPN,
}

impl InMemoryAccountLedger {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            total_supply: 0,
        }
    }

    pub fn with_supply(initial_supply: MicroIPN) -> Self {
        Self {
            balances: HashMap::new(),
            total_supply: initial_supply,
        }
    }
}

impl AccountLedger for InMemoryAccountLedger {
    fn credit_validator(&mut self, validator_id: &ValidatorId, amount: MicroIPN) -> Result<()> {
        let current_balance = self.balances.get(validator_id).copied().unwrap_or(0);
        let new_balance = current_balance.saturating_add(amount);
        self.balances.insert(validator_id.clone(), new_balance);
        self.total_supply = self.total_supply.saturating_add(amount);
        Ok(())
    }

    fn get_validator_balance(&self, validator_id: &ValidatorId) -> Result<MicroIPN> {
        Ok(self.balances.get(validator_id).copied().unwrap_or(0))
    }

    fn debit_validator(&mut self, validator_id: &ValidatorId, amount: MicroIPN) -> Result<()> {
        let current_balance = self.balances.get(validator_id).copied().unwrap_or(0);
        if current_balance < amount {
            return Err(anyhow::anyhow!("Insufficient balance"));
        }
        let new_balance = current_balance - amount;
        self.balances.insert(validator_id.clone(), new_balance);
        self.total_supply = self.total_supply.saturating_sub(amount);
        Ok(())
    }

    fn get_total_supply(&self) -> Result<MicroIPN> {
        Ok(self.total_supply)
    }

    fn get_all_balances(&self) -> Result<HashMap<ValidatorId, MicroIPN>> {
        Ok(self.balances.clone())
    }
}

// -----------------------------------------------------------------------------
// 🧪 Mock ledger for testing
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Default)]
pub struct MockAccountLedger {
    balances: HashMap<ValidatorId, MicroIPN>,
    total_supply: MicroIPN,
    credit_calls: Vec<(ValidatorId, MicroIPN)>,
    debit_calls: Vec<(ValidatorId, MicroIPN)>,
}

impl MockAccountLedger {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            total_supply: 0,
            credit_calls: Vec::new(),
            debit_calls: Vec::new(),
        }
    }

    pub fn get_credit_calls(&self) -> &[(ValidatorId, MicroIPN)] {
        &self.credit_calls
    }

    pub fn get_debit_calls(&self) -> &[(ValidatorId, MicroIPN)] {
        &self.debit_calls
    }

    pub fn clear_calls(&mut self) {
        self.credit_calls.clear();
        self.debit_calls.clear();
    }
}

impl AccountLedger for MockAccountLedger {
    fn credit_validator(&mut self, validator_id: &ValidatorId, amount: MicroIPN) -> Result<()> {
        self.credit_calls.push((validator_id.clone(), amount));
        let current_balance = self.balances.get(validator_id).copied().unwrap_or(0);
        let new_balance = current_balance.saturating_add(amount);
        self.balances.insert(validator_id.clone(), new_balance);
        self.total_supply = self.total_supply.saturating_add(amount);
        Ok(())
    }

    fn get_validator_balance(&self, validator_id: &ValidatorId) -> Result<MicroIPN> {
        Ok(self.balances.get(validator_id).copied().unwrap_or(0))
    }

    fn debit_validator(&mut self, validator_id: &ValidatorId, amount: MicroIPN) -> Result<()> {
        self.debit_calls.push((validator_id.clone(), amount));
        let current_balance = self.balances.get(validator_id).copied().unwrap_or(0);
        if current_balance < amount {
            return Err(anyhow::anyhow!("Insufficient balance"));
        }
        let new_balance = current_balance - amount;
        self.balances.insert(validator_id.clone(), new_balance);
        self.total_supply = self.total_supply.saturating_sub(amount);
        Ok(())
    }

    fn get_total_supply(&self) -> Result<MicroIPN> {
        Ok(self.total_supply)
    }

    fn get_all_balances(&self) -> Result<HashMap<ValidatorId, MicroIPN>> {
        Ok(self.balances.clone())
    }
}

// -----------------------------------------------------------------------------
// ✅ Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_ledger_creation() {
        let ledger = InMemoryAccountLedger::new();
        assert_eq!(ledger.get_total_supply().unwrap(), 0);
    }

    #[test]
    fn test_in_memory_ledger_operations() {
        let mut ledger = InMemoryAccountLedger::new();
        let validator_id = ValidatorId(hex::encode([1u8; 32]));

        // Credit
        ledger.credit_validator(&validator_id, 1000).unwrap();
        assert_eq!(ledger.get_validator_balance(&validator_id).unwrap(), 1000);
        assert_eq!(ledger.get_total_supply().unwrap(), 1000);

        // Debit
        ledger.debit_validator(&validator_id, 300).unwrap();
        assert_eq!(ledger.get_validator_balance(&validator_id).unwrap(), 700);
        assert_eq!(ledger.get_total_supply().unwrap(), 700);
    }

    #[test]
    fn test_insufficient_balance() {
        let mut ledger = InMemoryAccountLedger::new();
        let validator_id = ValidatorId(hex::encode([1u8; 32]));

        ledger.credit_validator(&validator_id, 1000).unwrap();

        // Try to debit more than available
        let result = ledger.debit_validator(&validator_id, 1500);
        assert!(result.is_err());
        assert_eq!(ledger.get_validator_balance(&validator_id).unwrap(), 1000);
    }

    #[test]
    fn test_mock_ledger_calls() {
        let mut mock = MockAccountLedger::new();
        let validator_id = ValidatorId(hex::encode([1u8; 32]));

        mock.credit_validator(&validator_id, 1000).unwrap();
        mock.debit_validator(&validator_id, 300).unwrap();

        let credit_calls = mock.get_credit_calls();
        let debit_calls = mock.get_debit_calls();

        assert_eq!(credit_calls.len(), 1);
        assert_eq!(debit_calls.len(), 1);
        assert_eq!(credit_calls[0], (validator_id.clone(), 1000));
        assert_eq!(debit_calls[0], (validator_id.clone(), 300));
    }
}
