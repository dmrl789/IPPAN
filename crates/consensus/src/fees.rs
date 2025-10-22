use ippan_types::Transaction;
/// Fee caps and recycling enforcement for protocol sustainability
///
/// This module implements:
/// - Hard fee caps per transaction type
/// - Fee validation during transaction admission
/// - Fee recycling to reward pool
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
    FeeAboveCap {
        kind: TxKind,
        actual: u128,
        cap: u128,
    },
    #[error("Invalid fee: cannot be zero")]
    ZeroFee,
    #[error("Cannot determine transaction type")]
    UnknownTxType,
}

/// Determine transaction kind from transaction data
///
/// Currently uses simple heuristics based on transaction fields.
/// In a full implementation, this would inspect transaction payload/opcodes.
pub fn classify_transaction(tx: &Transaction) -> TxKind {
    // For now, we classify based on topics
    // In the future, this would inspect the transaction payload
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
        // Default to transfer
        TxKind::Transfer
    }
}

/// Validate transaction fee against caps
///
/// # Arguments
/// * `tx` - Transaction to validate
/// * `fee` - Fee amount in µIPN
/// * `config` - Fee cap configuration
///
/// # Returns
/// Ok(()) if fee is valid, Err otherwise
pub fn validate_fee(tx: &Transaction, fee: u128, config: &FeeCapConfig) -> Result<(), FeeError> {
    if fee == 0 {
        return Err(FeeError::ZeroFee);
    }

    let kind = classify_transaction(tx);
    let cap = config.get_cap(kind);

    if fee > cap {
        return Err(FeeError::FeeAboveCap {
            kind,
            actual: fee,
            cap,
        });
    }

    Ok(())
}

/// Fee collector for tracking and recycling fees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCollector {
    /// Total fees collected since last recycling
    pub accumulated: u128,
    /// Round when last recycling occurred
    pub last_recycle_round: u64,
    /// Total fees collected all-time
    pub total_collected: u128,
    /// Total fees recycled all-time
    pub total_recycled: u128,
}

impl FeeCollector {
    /// Create a new fee collector
    pub fn new() -> Self {
        Self {
            accumulated: 0,
            last_recycle_round: 0,
            total_collected: 0,
            total_recycled: 0,
        }
    }

    /// Add fee to the collector
    pub fn collect(&mut self, fee: u128) {
        self.accumulated = self.accumulated.saturating_add(fee);
        self.total_collected = self.total_collected.saturating_add(fee);
    }

    /// Check if it's time to recycle fees
    ///
    /// # Arguments
    /// * `current_round` - Current round number
    /// * `recycle_interval` - Rounds between recycling
    pub fn should_recycle(&self, current_round: u64, recycle_interval: u64) -> bool {
        if self.accumulated == 0 {
            return false;
        }
        current_round >= self.last_recycle_round + recycle_interval
    }

    /// Recycle accumulated fees
    ///
    /// # Arguments
    /// * `current_round` - Current round number
    /// * `recycle_percentage` - Percentage of fees to recycle (10000 = 100%)
    ///
    /// # Returns
    /// Amount recycled to reward pool
    pub fn recycle(&mut self, current_round: u64, recycle_percentage: u16) -> u128 {
        let to_recycle = (self.accumulated * recycle_percentage as u128) / 10000;

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
        assert_eq!(
            classify_transaction(&create_test_transaction(vec![])),
            TxKind::Transfer
        );

        assert_eq!(
            classify_transaction(&create_test_transaction(vec!["ai_call".to_string()])),
            TxKind::AiCall
        );

        assert_eq!(
            classify_transaction(&create_test_transaction(
                vec!["contract_deploy".to_string()]
            )),
            TxKind::ContractDeploy
        );

        assert_eq!(
            classify_transaction(&create_test_transaction(vec!["governance".to_string()])),
            TxKind::Governance
        );
    }

    #[test]
    fn test_validate_fee_success() {
        let config = FeeCapConfig::default();
        let tx = create_test_transaction(vec![]);

        assert!(validate_fee(&tx, 500, &config).is_ok());
        assert!(validate_fee(&tx, 1_000, &config).is_ok());
    }

    #[test]
    fn test_validate_fee_above_cap() {
        let config = FeeCapConfig::default();
        let tx = create_test_transaction(vec![]);

        let result = validate_fee(&tx, 1_001, &config);
        assert!(result.is_err());

        match result {
            Err(FeeError::FeeAboveCap { kind, actual, cap }) => {
                assert_eq!(kind, TxKind::Transfer);
                assert_eq!(actual, 1_001);
                assert_eq!(cap, 1_000);
            }
            _ => panic!("Expected FeeAboveCap error"),
        }
    }

    #[test]
    fn test_validate_fee_zero() {
        let config = FeeCapConfig::default();
        let tx = create_test_transaction(vec![]);

        let result = validate_fee(&tx, 0, &config);
        assert!(matches!(result, Err(FeeError::ZeroFee)));
    }

    #[test]
    fn test_ai_call_fee_cap() {
        let config = FeeCapConfig::default();
        let tx = create_test_transaction(vec!["ai_call".to_string()]);

        assert!(validate_fee(&tx, 100, &config).is_ok());
        assert!(validate_fee(&tx, 101, &config).is_err());
    }

    #[test]
    fn test_contract_deploy_fee_cap() {
        let config = FeeCapConfig::default();
        let tx = create_test_transaction(vec!["contract_deploy".to_string()]);

        assert!(validate_fee(&tx, 100_000, &config).is_ok());
        assert!(validate_fee(&tx, 100_001, &config).is_err());
    }

    #[test]
    fn test_fee_collector_collect() {
        let mut collector = FeeCollector::new();

        collector.collect(100);
        assert_eq!(collector.accumulated, 100);
        assert_eq!(collector.total_collected, 100);

        collector.collect(200);
        assert_eq!(collector.accumulated, 300);
        assert_eq!(collector.total_collected, 300);
    }

    #[test]
    fn test_fee_collector_should_recycle() {
        let mut collector = FeeCollector::new();
        collector.collect(1000);

        // Not time yet
        assert!(!collector.should_recycle(100, 1000));

        // Time to recycle
        assert!(collector.should_recycle(1000, 1000));
        assert!(collector.should_recycle(1500, 1000));
    }

    #[test]
    fn test_fee_collector_recycle() {
        let mut collector = FeeCollector::new();
        collector.collect(1000);

        // Recycle 100%
        let recycled = collector.recycle(1000, 10000);
        assert_eq!(recycled, 1000);
        assert_eq!(collector.accumulated, 0);
        assert_eq!(collector.total_recycled, 1000);
        assert_eq!(collector.last_recycle_round, 1000);
    }

    #[test]
    fn test_fee_collector_recycle_partial() {
        let mut collector = FeeCollector::new();
        collector.collect(1000);

        // Recycle 50%
        let recycled = collector.recycle(1000, 5000);
        assert_eq!(recycled, 500);
        assert_eq!(collector.accumulated, 500);
        assert_eq!(collector.total_recycled, 500);
    }

    #[test]
    fn test_fee_collector_no_recycle_when_empty() {
        let collector = FeeCollector::new();
        assert!(!collector.should_recycle(1000, 100));
    }

    #[test]
    fn test_get_cap() {
        let config = FeeCapConfig::default();

        assert_eq!(config.get_cap(TxKind::Transfer), 1_000);
        assert_eq!(config.get_cap(TxKind::AiCall), 100);
        assert_eq!(config.get_cap(TxKind::ContractDeploy), 100_000);
        assert_eq!(config.get_cap(TxKind::ContractCall), 10_000);
        assert_eq!(config.get_cap(TxKind::Governance), 10_000);
        assert_eq!(config.get_cap(TxKind::Validator), 10_000);
    }
}
