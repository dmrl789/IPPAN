//! IPPAN — Fee Enforcement & Recycling Module
//!
//! Implements protocol-level fee validation and recycling.
//! Includes:
//! - Hard fee caps per transaction type (µIPN units)
//! - Deterministic validation
//! - Weekly recycling into the reward pool

use ippan_types::Transaction;
use serde::{Deserialize, Serialize};

/// Transaction category for fee classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    /// Standard peer-to-peer transfer
    Transfer,
    /// AI model call or inference
    AiCall,
    /// Smart contract deployment
    ContractDeploy,
    /// Smart contract execution
    ContractCall,
    /// Governance or proposal transaction
    Governance,
    /// Validator registration / staking operation
    Validator,
}

/// Fee cap configuration (values in µIPN)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCapConfig {
    pub cap_transfer: u128,
    pub cap_ai_call: u128,
    pub cap_contract_deploy: u128,
    pub cap_contract_call: u128,
    pub cap_governance: u128,
    pub cap_validator: u128,
}

impl Default for FeeCapConfig {
    fn default() -> Self {
        Self {
            cap_transfer: 1_000,          // 0.00001 IPN
            cap_ai_call: 100,             // 0.000001 IPN
            cap_contract_deploy: 100_000, // 0.001 IPN
            cap_contract_call: 10_000,    // 0.0001 IPN
            cap_governance: 10_000,       // 0.0001 IPN
            cap_validator: 10_000,        // 0.0001 IPN
        }
    }
}

impl FeeCapConfig {
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

/// Errors raised during fee validation
#[derive(thiserror::Error, Debug)]
pub enum FeeError {
    #[error("Fee {actual} exceeds cap {cap} for {kind:?}")]
    FeeAboveCap { kind: TxKind, actual: u128, cap: u128 },
    #[error("Fee must be positive")]
    ZeroFee,
}

/// Determine transaction kind heuristically
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

/// Validate fee for a transaction
pub fn validate_fee(tx: &Transaction, fee: u128, config: &FeeCapConfig) -> Result<(), FeeError> {
    if fee == 0 {
        return Err(FeeError::ZeroFee);
    }

    let kind = classify_transaction(tx);
    let cap = config.get_cap(kind);

    if fee > cap {
        Err(FeeError::FeeAboveCap { kind, actual: fee, cap })
    } else {
        Ok(())
    }
}

/// Fee collector and recycler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollector {
    pub accumulated: u128,
    pub last_recycle_round: u64,
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

    pub fn collect(&mut self, fee: u128) {
        self.accumulated = self.accumulated.saturating_add(fee);
        self.total_collected = self.total_collected.saturating_add(fee);
    }

    pub fn should_recycle(&self, current_round: u64, recycle_interval: u64) -> bool {
        self.accumulated > 0 && current_round >= self.last_recycle_round + recycle_interval
    }

    pub fn recycle(&mut self, current_round: u64, recycle_bps: u16) -> u128 {
        let amount = (self.accumulated * recycle_bps as u128) / 10_000;
        self.accumulated = self.accumulated.saturating_sub(amount);
        self.total_recycled = self.total_recycled.saturating_add(amount);
        self.last_recycle_round = current_round;
        amount
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

    fn tx_with_topic(topic: &str) -> Transaction {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        tx.topics = vec![topic.to_string()];
        tx
    }

    #[test]
    fn classify_basic() {
        assert_eq!(classify_transaction(&tx_with_topic("ai_call")), TxKind::AiCall);
        assert_eq!(classify_transaction(&tx_with_topic("contract_deploy")), TxKind::ContractDeploy);
        assert_eq!(classify_transaction(&tx_with_topic("governance")), TxKind::Governance);
        assert_eq!(classify_transaction(&tx_with_topic("validator_stake")), TxKind::Validator);
    }

    #[test]
    fn fee_validation_caps() {
        let cfg = FeeCapConfig::default();
        let tx = tx_with_topic("ai_call");
        assert!(validate_fee(&tx, 50, &cfg).is_ok());
        assert!(validate_fee(&tx, 101, &cfg).is_err());
    }

    #[test]
    fn fee_collector_recycling() {
        let mut c = FeeCollector::new();
        c.collect(1000);
        assert!(c.should_recycle(100, 10));
        let recycled = c.recycle(100, 5000);
        assert_eq!(recycled, 500);
        assert_eq!(c.total_recycled, 500);
    }
}
