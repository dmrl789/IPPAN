//! IPPAN — Fee Enforcement & Recycling Module
//!
//! Implements protocol-level fee validation and recycling.
//! Includes:
//! - Hard fee caps per transaction type (atomic units)
//! - Deterministic validation
//! - Weekly recycling into the reward pool

use ippan_types::{AtomicIPN, IPNAmount, IPNUnit, Transaction};
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

/// Fee cap configuration (values in atomic units)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCapConfig {
    pub cap_transfer: AtomicIPN,
    pub cap_ai_call: AtomicIPN,
    pub cap_contract_deploy: AtomicIPN,
    pub cap_contract_call: AtomicIPN,
    pub cap_governance: AtomicIPN,
    pub cap_validator: AtomicIPN,
}

impl Default for FeeCapConfig {
    fn default() -> Self {
        Self {
            cap_transfer: IPNAmount::from_unit(1_000, IPNUnit::MicroIPN).atomic(),          // 0.00001 IPN
            cap_ai_call: IPNAmount::from_unit(100, IPNUnit::MicroIPN).atomic(),             // 0.000001 IPN
            cap_contract_deploy: IPNAmount::from_unit(100_000, IPNUnit::MicroIPN).atomic(), // 0.001 IPN
            cap_contract_call: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),    // 0.0001 IPN
            cap_governance: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),       // 0.0001 IPN
            cap_validator: IPNAmount::from_unit(10_000, IPNUnit::MicroIPN).atomic(),        // 0.0001 IPN
        }
    }
}

impl FeeCapConfig {
    /// Return the cap value for the given transaction kind
    pub fn get_cap(&self, kind: TxKind) -> AtomicIPN {
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
    FeeAboveCap { kind: TxKind, actual: AtomicIPN, cap: AtomicIPN },
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
pub fn validate_fee(tx: &Transaction, fee: AtomicIPN, config: &FeeCapConfig) -> Result<(), FeeError> {
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
    /// Total fees accumulated since last recycling
    pub accumulated: AtomicIPN,
    /// Round number when last recycling occurred
    pub last_recycle_round: u64,
    /// Lifetime total fees collected
    pub total_collected: AtomicIPN,
    /// Lifetime total recycled fees
    pub total_recycled: AtomicIPN,
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

    /// Collect a transaction fee into the accumulator
    pub fn collect(&mut self, fee: AtomicIPN) {
        self.accumulated = self.accumulated.saturating_add(fee);
        self.total_collected = self.total_collected.saturating_add(fee);
    }

    /// Check if recycling should trigger at current round
    pub fn should_recycle(&self, current_round: u64, recycle_interval: u64) -> bool {
        self.accumulated > 0 && current_round >= self.last_recycle_round + recycle_interval
    }

    /// Perform recycling and return the amount recycled
    pub fn recycle(&mut self, current_round: u64, recycle_bps: u16) -> AtomicIPN {
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
        assert_eq!(classify_transaction(&tx_with_topic("random_topic")), TxKind::Transfer);
    }

    #[test]
    fn fee_validation_caps() {
        let cfg = FeeCapConfig::default();
        let tx = tx_with_topic("ai_call");
        let ai_call_cap = IPNAmount::from_unit(100, IPNUnit::MicroIPN).atomic();
        assert!(validate_fee(&tx, ai_call_cap / 2, &cfg).is_ok());
        assert!(validate_fee(&tx, ai_call_cap + 1, &cfg).is_err());
        let tx2 = tx_with_topic("contract_deploy");
        let deploy_cap = IPNAmount::from_unit(100_000, IPNUnit::MicroIPN).atomic();
        assert!(validate_fee(&tx2, deploy_cap - 1, &cfg).is_ok());
        assert!(validate_fee(&tx2, deploy_cap + 1, &cfg).is_err());
    }

    #[test]
    fn fee_collector_recycling() {
        let mut c = FeeCollector::new();
        let test_fee = IPNAmount::from_unit(1000, IPNUnit::MicroIPN).atomic();
        c.collect(test_fee);
        assert!(c.should_recycle(100, 10));
        let recycled = c.recycle(100, 5000);
        assert_eq!(recycled, test_fee / 2);
        assert_eq!(c.total_recycled, test_fee / 2);
        assert_eq!(c.accumulated, test_fee / 2);
    }
}
