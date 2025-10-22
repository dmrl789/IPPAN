use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction types for fee calculation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    /// Simple transfer transaction
    Transfer,
    /// AI model call transaction
    AiCall,
    /// Governance proposal transaction
    Governance,
    /// Validator registration transaction
    ValidatorRegistration,
    /// Contract deployment transaction
    ContractDeployment,
    /// Contract execution transaction
    ContractExecution,
}

/// Fee caps for different transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCaps {
    /// Fee cap for transfer transactions (in micro-IPN)
    pub transfer: u128,
    /// Fee cap for AI call transactions (in micro-IPN)
    pub ai_call: u128,
    /// Fee cap for governance transactions (in micro-IPN)
    pub governance: u128,
    /// Fee cap for validator registration (in micro-IPN)
    pub validator_registration: u128,
    /// Fee cap for contract deployment (in micro-IPN)
    pub contract_deployment: u128,
    /// Fee cap for contract execution (in micro-IPN)
    pub contract_execution: u128,
}

impl Default for FeeCaps {
    fn default() -> Self {
        Self {
            transfer: 1_000, // 0.000001 IPN
            ai_call: 100, // 0.0000001 IPN
            governance: 10_000, // 0.00001 IPN
            validator_registration: 100_000, // 0.0001 IPN
            contract_deployment: 50_000, // 0.00005 IPN
            contract_execution: 5_000, // 0.000005 IPN
        }
    }
}

/// Fee recycling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeRecyclingConfig {
    /// Recycling interval in rounds
    pub recycling_interval: u64,
    /// Percentage of fees to recycle (0.0 to 1.0)
    pub recycling_percentage: f64,
    /// Minimum amount to trigger recycling
    pub min_recycling_amount: u128,
}

impl Default for FeeRecyclingConfig {
    fn default() -> Self {
        Self {
            recycling_interval: 1008, // ~1 week at 1 round per 10 minutes
            recycling_percentage: 0.8, // 80% of fees
            min_recycling_amount: 1_000_000, // 0.001 IPN
        }
    }
}

/// Fee manager for enforcing caps and recycling
pub struct FeeManager {
    caps: FeeCaps,
    recycling_config: FeeRecyclingConfig,
    collected_fees: u128,
    last_recycling_round: u64,
}

impl FeeManager {
    /// Create a new fee manager
    pub fn new(caps: FeeCaps, recycling_config: FeeRecyclingConfig) -> Self {
        Self {
            caps,
            recycling_config,
            collected_fees: 0,
            last_recycling_round: 0,
        }
    }

    /// Validate a transaction fee against the cap
    pub fn validate_fee(&self, tx_type: TransactionType, fee: u128) -> Result<()> {
        let cap = self.get_fee_cap(tx_type);
        
        if fee > cap {
            return Err(anyhow::anyhow!(
                "Transaction fee {} exceeds cap {} for type {:?}",
                fee,
                cap,
                tx_type
            ));
        }
        
        Ok(())
    }

    /// Get the fee cap for a transaction type
    pub fn get_fee_cap(&self, tx_type: TransactionType) -> u128 {
        match tx_type {
            TransactionType::Transfer => self.caps.transfer,
            TransactionType::AiCall => self.caps.ai_call,
            TransactionType::Governance => self.caps.governance,
            TransactionType::ValidatorRegistration => self.caps.validator_registration,
            TransactionType::ContractDeployment => self.caps.contract_deployment,
            TransactionType::ContractExecution => self.caps.contract_execution,
        }
    }

    /// Collect fees from a transaction
    pub fn collect_fee(&mut self, fee: u128) {
        self.collected_fees = self.collected_fees.saturating_add(fee);
    }

    /// Process fee recycling if needed
    pub fn process_recycling(&mut self, current_round: u64) -> Result<u128> {
        if current_round - self.last_recycling_round >= self.recycling_config.recycling_interval {
            if self.collected_fees >= self.recycling_config.min_recycling_amount {
                let recycling_amount = (self.collected_fees as f64 * self.recycling_config.recycling_percentage) as u128;
                self.collected_fees = self.collected_fees.saturating_sub(recycling_amount);
                self.last_recycling_round = current_round;
                return Ok(recycling_amount);
            }
        }
        Ok(0)
    }

    /// Get current collected fees
    pub fn get_collected_fees(&self) -> u128 {
        self.collected_fees
    }

    /// Get fee caps
    pub fn get_fee_caps(&self) -> &FeeCaps {
        &self.caps
    }

    /// Update fee caps
    pub fn update_fee_caps(&mut self, caps: FeeCaps) {
        self.caps = caps;
    }

    /// Get recycling configuration
    pub fn get_recycling_config(&self) -> &FeeRecyclingConfig {
        &self.recycling_config
    }

    /// Update recycling configuration
    pub fn update_recycling_config(&mut self, config: FeeRecyclingConfig) {
        self.recycling_config = config;
    }
}

impl Default for FeeManager {
    fn default() -> Self {
        Self::new(FeeCaps::default(), FeeRecyclingConfig::default())
    }
}

/// Convenience function to validate a transaction fee
pub fn validate_transaction_fee(tx_type: TransactionType, fee: u128, caps: &FeeCaps) -> Result<()> {
    let cap = match tx_type {
        TransactionType::Transfer => caps.transfer,
        TransactionType::AiCall => caps.ai_call,
        TransactionType::Governance => caps.governance,
        TransactionType::ValidatorRegistration => caps.validator_registration,
        TransactionType::ContractDeployment => caps.contract_deployment,
        TransactionType::ContractExecution => caps.contract_execution,
    };
    
    if fee > cap {
        return Err(anyhow::anyhow!(
            "Transaction fee {} exceeds cap {} for type {:?}",
            fee,
            cap,
            tx_type
        ));
    }
    
    Ok(())
}

/// Calculate fee for a transaction based on its type and size
pub fn calculate_transaction_fee(tx_type: TransactionType, size: u64, caps: &FeeCaps) -> u128 {
    let base_fee = match tx_type {
        TransactionType::Transfer => caps.transfer,
        TransactionType::AiCall => caps.ai_call,
        TransactionType::Governance => caps.governance,
        TransactionType::ValidatorRegistration => caps.validator_registration,
        TransactionType::ContractDeployment => caps.contract_deployment,
        TransactionType::ContractExecution => caps.contract_execution,
    };
    
    // Add size-based fee (1 micro-IPN per 100 bytes)
    let size_fee = (size / 100) as u128;
    
    base_fee + size_fee
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_validation() {
        let caps = FeeCaps::default();
        let manager = FeeManager::new(caps.clone(), FeeRecyclingConfig::default());
        
        // Valid fees
        assert!(manager.validate_fee(TransactionType::Transfer, 500).is_ok());
        assert!(manager.validate_fee(TransactionType::AiCall, 50).is_ok());
        
        // Invalid fees (exceed caps)
        assert!(manager.validate_fee(TransactionType::Transfer, 2000).is_err());
        assert!(manager.validate_fee(TransactionType::AiCall, 200).is_err());
    }

    #[test]
    fn test_fee_collection() {
        let mut manager = FeeManager::default();
        
        assert_eq!(manager.get_collected_fees(), 0);
        
        manager.collect_fee(1000);
        assert_eq!(manager.get_collected_fees(), 1000);
        
        manager.collect_fee(500);
        assert_eq!(manager.get_collected_fees(), 1500);
    }

    #[test]
    fn test_fee_recycling() {
        let mut manager = FeeManager::new(
            FeeCaps::default(),
            FeeRecyclingConfig {
                recycling_interval: 10,
                recycling_percentage: 0.5,
                min_recycling_amount: 1000,
                ..Default::default()
            }
        );
        
        // Add some fees
        manager.collect_fee(2000);
        assert_eq!(manager.get_collected_fees(), 2000);
        
        // Process recycling (should recycle 50% = 1000)
        let recycled = manager.process_recycling(10).unwrap();
        assert_eq!(recycled, 1000);
        assert_eq!(manager.get_collected_fees(), 1000);
        
        // Process recycling again (should not recycle yet)
        let recycled = manager.process_recycling(15).unwrap();
        assert_eq!(recycled, 0);
    }

    #[test]
    fn test_fee_calculation() {
        let caps = FeeCaps::default();
        
        // Transfer with 500 bytes
        let fee = calculate_transaction_fee(TransactionType::Transfer, 500, &caps);
        assert_eq!(fee, caps.transfer + 5); // 1000 + 5
        
        // AI call with 50 bytes
        let fee = calculate_transaction_fee(TransactionType::AiCall, 50, &caps);
        assert_eq!(fee, caps.ai_call + 0); // 100 + 0
    }

    #[test]
    fn test_fuzz_fee_validation() {
        let caps = FeeCaps::default();
        
        // Test various fee values for each transaction type
        let test_cases = vec![
            (TransactionType::Transfer, 0, true),
            (TransactionType::Transfer, caps.transfer, true),
            (TransactionType::Transfer, caps.transfer + 1, false),
            (TransactionType::AiCall, 0, true),
            (TransactionType::AiCall, caps.ai_call, true),
            (TransactionType::AiCall, caps.ai_call + 1, false),
            (TransactionType::Governance, caps.governance, true),
            (TransactionType::Governance, caps.governance + 1, false),
        ];
        
        for (tx_type, fee, should_pass) in test_cases {
            let result = validate_transaction_fee(tx_type, fee, &caps);
            if should_pass {
                assert!(result.is_ok(), "Fee {} should be valid for {:?}", fee, tx_type);
            } else {
                assert!(result.is_err(), "Fee {} should be invalid for {:?}", fee, tx_type);
            }
        }
    }
}