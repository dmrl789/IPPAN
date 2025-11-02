//! IPPAN — Fee Enforcement & Recycling Module
//!
//! Implements protocol-level fee validation and recycling.
//! Includes:
//! - Hard fee caps per transaction type (atomic IPN units with 24 decimal precision)
//! - Deterministic validation
//! - Weekly recycling into the reward pool

use ippan_types::{Amount, Transaction};
use serde::{Deserialize, Serialize};

/// L1 Transaction category for fee classification
///
/// L1 has NO smart contracts - only pure consensus operations.
/// All smart contracts are handled in L2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    /// Standard peer-to-peer transfer
    Transfer,
    /// L2 state commitment (anchor)
    L2Anchor,
    /// L2 exit request
    L2Exit,
    /// Governance or proposal transaction
    Governance,
    /// Validator registration / staking operation
    Validator,
}

/// L1 Fee cap configuration (values in atomic IPN units)
///
/// Only L1 operations - no smart contracts or AI calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCapConfig {
    pub cap_transfer: Amount,
    pub cap_l2_anchor: Amount,
    pub cap_l2_exit: Amount,
    pub cap_governance: Amount,
    pub cap_validator: Amount,
}

impl Default for FeeCapConfig {
    fn default() -> Self {
        Self {
            cap_transfer: Amount::from_micro_ipn(100), // 0.0001 IPN (minimal)
            cap_l2_anchor: Amount::from_micro_ipn(1_000), // 0.001 IPN (L2 state commitment)
            cap_l2_exit: Amount::from_micro_ipn(2_000), // 0.002 IPN (L2 exit request)
            cap_governance: Amount::from_micro_ipn(10_000), // 0.01 IPN
            cap_validator: Amount::from_micro_ipn(10_000), // 0.01 IPN
        }
    }
}

impl FeeCapConfig {
    /// Return the cap value for the given transaction kind
    pub fn get_cap(&self, kind: TxKind) -> Amount {
        match kind {
            TxKind::Transfer => self.cap_transfer,
            TxKind::L2Anchor => self.cap_l2_anchor,
            TxKind::L2Exit => self.cap_l2_exit,
            TxKind::Governance => self.cap_governance,
            TxKind::Validator => self.cap_validator,
        }
    }
}

/// Errors raised during fee validation
#[derive(thiserror::Error, Debug)]
pub enum FeeError {
    #[error("Fee {actual} exceeds cap {cap} for {kind:?}")]
    FeeAboveCap {
        kind: TxKind,
        actual: Amount,
        cap: Amount,
    },
    #[error("Fee must be positive")]
    ZeroFee,
}

/// Determine L1 transaction kind heuristically
///
/// L1 only handles basic operations - no smart contracts or AI calls
pub fn classify_transaction(tx: &Transaction) -> TxKind {
    if let Some(topic) = tx.topics.first() {
        match topic.as_str() {
            "l2_anchor" | "l2_commit" => TxKind::L2Anchor,
            "l2_exit" | "l2_withdrawal" => TxKind::L2Exit,
            "governance" | "proposal" => TxKind::Governance,
            "validator_stake" | "validator_unstake" => TxKind::Validator,
            _ => TxKind::Transfer,
        }
    } else {
        TxKind::Transfer
    }
}

/// Validate fee for a transaction
pub fn validate_fee(tx: &Transaction, fee: Amount, config: &FeeCapConfig) -> Result<(), FeeError> {
    if fee.is_zero() {
        return Err(FeeError::ZeroFee);
    }

    let kind = classify_transaction(tx);
    let cap = config.get_cap(kind);

    if fee > cap {
        Err(FeeError::FeeAboveCap {
            kind,
            actual: fee,
            cap,
        })
    } else {
        Ok(())
    }
}

/// Fee collector and recycler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollector {
    /// Total fees accumulated since last recycling
    pub accumulated: Amount,
    /// Round number when last recycling occurred
    pub last_recycle_round: u64,
    /// Lifetime total fees collected
    pub total_collected: Amount,
    /// Lifetime total recycled fees
    pub total_recycled: Amount,
}

impl FeeCollector {
    pub fn new() -> Self {
        Self {
            accumulated: Amount::zero(),
            last_recycle_round: 0,
            total_collected: Amount::zero(),
            total_recycled: Amount::zero(),
        }
    }

    /// Collect a transaction fee into the accumulator
    pub fn collect(&mut self, fee: Amount) {
        self.accumulated = self.accumulated.saturating_add(fee);
        self.total_collected = self.total_collected.saturating_add(fee);
    }

    /// Check if recycling should trigger at current round
    pub fn should_recycle(&self, current_round: u64, recycle_interval: u64) -> bool {
        !self.accumulated.is_zero() && current_round >= self.last_recycle_round + recycle_interval
    }

    /// Perform recycling and return the amount recycled
    pub fn recycle(&mut self, current_round: u64, recycle_bps: u16) -> Amount {
        let amount = self.accumulated.percentage(recycle_bps);
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

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    fn tx_with_topic(topic: &str) -> Transaction {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_micro_ipn(1000), 1);
        tx.topics = vec![topic.to_string()];
        tx
    }

    #[test]
    fn classify_basic() {
        assert_eq!(
            classify_transaction(&tx_with_topic("l2_anchor")),
            TxKind::L2Anchor
        );
        assert_eq!(
            classify_transaction(&tx_with_topic("l2_exit")),
            TxKind::L2Exit
        );
        assert_eq!(
            classify_transaction(&tx_with_topic("governance")),
            TxKind::Governance
        );
        assert_eq!(
            classify_transaction(&tx_with_topic("validator_stake")),
            TxKind::Validator
        );
        assert_eq!(
            classify_transaction(&tx_with_topic("random_topic")),
            TxKind::Transfer
        );
    }

    #[test]
    fn fee_validation_caps() {
        let cfg = FeeCapConfig::default();
        let tx = tx_with_topic("l2_anchor");
        assert!(validate_fee(&tx, Amount::from_micro_ipn(500), &cfg).is_ok());
        assert!(validate_fee(&tx, Amount::from_micro_ipn(1001), &cfg).is_err());
        let tx2 = tx_with_topic("l2_exit");
        assert!(validate_fee(&tx2, Amount::from_micro_ipn(1_999), &cfg).is_ok());
        assert!(validate_fee(&tx2, Amount::from_micro_ipn(2_001), &cfg).is_err());
    }

    #[test]
    fn fee_collector_recycling() {
        let mut c = FeeCollector::new();
        let test_fee = Amount::from_micro_ipn(1000);
        c.collect(test_fee);
        assert!(c.should_recycle(100, 10));
        let recycled = c.recycle(100, 5000);
        assert_eq!(recycled, Amount::from_micro_ipn(500));
        assert_eq!(c.total_recycled, Amount::from_micro_ipn(500));
        assert_eq!(c.accumulated, Amount::from_micro_ipn(500));
    }
}
