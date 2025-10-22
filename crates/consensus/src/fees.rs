//! Fee caps and recycling enforcement for protocol sustainability
//!
//! This module implements:
//! - Hard fee caps per transaction type
//! - Fee validation during transaction admission
//! - Fee recycling to the reward pool

use ippan_types::Transaction;
use serde::{Deserialize, Serialize};

/// Transaction types for fee categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    /// Standard transfer transaction
    Transfer,
    /// AI model call or inference
    AiCall,
    /// Contract deployment
    ContractDeploy,
    /// Contract interaction
    ContractCall,
    /// Governance proposal
    Governance,
    /// Validator stake/unstake
    Validator,
}

/// Fee cap configuration (all values in µIPN - micro IPN)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCapConfig {
    /// Maximum fee for transfer transactions (0.00001 IPN = 1,000 µIPN)
    pub cap_transfer: u128,
    /// Maximum fee for AI calls (0.000001 IPN = 100 µIPN)
    pub cap_ai_call: u128,
    /// Maximum fee for contract deployment (0.001 IPN = 100,000 µIPN)
    pub cap_contract_deploy: u128,
    /// Maximum fee for contract calls (0.0001 IPN = 10,000 µIPN)
    pub cap_contract_call: u128,
    /// Maximum fee for governance (0.0001 IPN = 10,000 µIPN)
    pub cap_governance: u128,
    /// Maximum fee for validator operations (0.0001 IPN = 10,000 µIPN)
    pub cap_validator: u128,
}

impl Default for FeeCapConfig {
    fn default() -> Self {
        Self {
            cap_transfer: 1_000,
            cap_ai_call: 100,
            cap_contract_deploy: 100_000,
            cap_contract_call: 10_000,
            cap_governance: 10_000,
            cap_validator: 10_000,
        }
    }
}

impl FeeCapConfig {
    /// Get the fee cap for a transaction kind
    pub fn get_cap(&self, kind: TxKind) -> u128 {
        match kind {
            TxKind::Transfer => self.cap_transfer,
            TxKind::AiCall => self.cap_ai_call,
            TxKind::ContractDeploy => self.cap_contract_deploy,
            TxKind::ContractCall => self.cap_contract_call,
            TxKind::Governance => self.cap_governance,
            TxKind::Validator => self.cap_validator,
        }
    }
}

/// Fee validation errors
#[derive(thiserror::Error, Debug)]
pub enum FeeError {
    #[error("Fee {actual} exceeds cap {cap} for {kind:?}")]
    FeeAboveCap { kind: TxKind, actual: u128, cap: u128 },
    #[error("Invalid fee: cannot be zero")]
    ZeroFee,
}

/// Determine transaction kind from transaction data
///
/// Uses transaction `topics` as lightweight classification hints.
/// In a full implementation, this would parse operation payloads or tags.
pub fn classify_transaction(tx: &Transaction) -> TxKind {
    if let Some(topic) = tx.topics.first() {
        match topic.as_str() {
            "ai_call" | "ai_inference" => TxKind::AiCall,
            "contract_deploy" => TxKind::ContractDeploy,
            "contract_call" => TxKind::ContractCall,
            "governance" | "proposal" => TxKind::Governance,
            "validator_stake" | "validator_unstake" => TxKind::Validator,
            _ => TxKind::Transfer,
        }
    } else {
        TxKind::Transfer
    }
}

/// Validate a transaction's fee against deterministic caps.
pub fn validate_fee(tx: &Transaction, fee: u128, config: &FeeCapConfig) -> Result<(), FeeError> {
    if fee == 0 {
        return Err(FeeError::ZeroFee);
    }

    let kind = classify_transaction(tx);
    let cap = config.get_cap(kind);

    if fee > cap {
        return Err(FeeError::FeeAboveCap { kind, actual: fee, cap });
    }

    Ok(())
}

/// Fee collector for tracking and recycling protocol fees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollector {
    /// Total fees accumulated since last recycling
    pub accumulated: u128,
    /// Round number when last recycling occurred
    pub last_recycle_round: u64,
    /// Lifetime counters
    pub total_collected: u128,
    pub total_recycled: u128,
}

impl FeeCollector {
    pub fn new() -> Self {
        Self {
            accumulated: 0,
            last_recycle_round: 0,
            total_collected: 0,
            total_recycled: 0,
        }
    }

    /// Add a fee to the accumulator
    pub fn collect(&mut self, fee: u128) {
        self.accumulated = self.accumulated.saturating_add(fee);
        self.total_collected = self.total_collected.saturating_add(fee);
    }

    /// Determine if recycling should trigger this round
    pub fn should_recycle(&self, current_round: u64, recycle_interval: u64) -> bool {
        self.accumulated > 0 && current_round >= self.last_recycle_round + recycle_interval
    }

    /// Perform recycling of collected fees
    pub fn recycle(&mut self, current_round: u64, recycle_percentage: u16) -> u128 {
        let to_recycle = (self.accumulated * recycle_percentage as u128) / 10_000;
        self.accumulated = self.accumulated.saturating_sub(to_recycle);
        self.total_recycled = self.total_recycled.saturating_add(to_recycle);
        self.last_recycle_round = current_round;
        to_recycle
    }
}

impl Default for FeeCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_transaction(topics: Vec<String>) -> Transaction {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        tx.topics = topics;
        tx
    }

    #[test]
    fn test_classify_transaction() {
        assert_eq!(classify_transaction(&create_test_transaction(vec![])), TxKind::Transfer);
        assert_eq!(
            classify_transaction(&create_test_transaction(vec!["ai_call".into()])),
            TxKind::AiCall
        );
        assert_eq!(
            classify_transaction(&create_test_transaction(vec!["contract_deploy".into()])),
            TxKind::ContractDeploy
        );
        assert_eq!(
            classify_transaction(&create_test_transaction(vec!["governance".into()])),
            TxKind::Governance
        );
    }

    #[test]
    fn test_validate_fee() {
        let config = FeeCapConfig::default();
        let tx = create_test_transaction(vec![]);
        assert!(validate_fee(&tx, 500, &config).is_ok());
        assert!(validate_fee(&tx, 0, &config).is_err());
        assert!(validate_fee(&tx, 2000, &config).is_err());
    }

    #[test]
    fn test_fee_collector_cycle() {
        let mut fc = FeeCollector::new();
        fc.collect(1000);
        assert!(fc.should_recycle(1000, 500));
        let recycled = fc.recycle(1000, 5000); // 50%
        assert_eq!(recycled, 500);
        assert_eq!(fc.accumulated, 500);
    }
}
