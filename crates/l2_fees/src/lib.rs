//! L2 Fee System for Smart Contracts and AI Operations
//!
//! Handles fees for:
//! - Smart contract deployment/execution
//! - AI model registration, inference, and storage
//! - Federated learning and proof-of-inference validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// L2 transaction types (smart contracts and AI operations)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum L2TxKind {
    ContractDeploy,
    ContractCall,
    AIModelRegister,
    AIModelInference,
    AIModelStorage,
    AIModelUpdate,
    FederatedLearning,
    ProofOfInference,
}

/// L2 fee structure for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2FeeStructure {
    pub base_fees: HashMap<L2TxKind, u64>,
    pub unit_fees: HashMap<L2TxKind, u64>,
    pub max_fees: HashMap<L2TxKind, u64>,
    pub min_fees: HashMap<L2TxKind, u64>,
}

impl Default for L2FeeStructure {
    fn default() -> Self {
        let mut base_fees = HashMap::new();
        let mut unit_fees = HashMap::new();
        let mut max_fees = HashMap::new();
        let mut min_fees = HashMap::new();

        // Smart contract fees
        base_fees.insert(L2TxKind::ContractDeploy, 50_000);
        base_fees.insert(L2TxKind::ContractCall, 5_000);
        unit_fees.insert(L2TxKind::ContractDeploy, 1);
        unit_fees.insert(L2TxKind::ContractCall, 10);
        max_fees.insert(L2TxKind::ContractDeploy, 1_000_000);
        max_fees.insert(L2TxKind::ContractCall, 100_000);
        min_fees.insert(L2TxKind::ContractDeploy, 10_000);
        min_fees.insert(L2TxKind::ContractCall, 1_000);

        // AI model operations
        base_fees.insert(L2TxKind::AIModelRegister, 1_000);
        base_fees.insert(L2TxKind::AIModelInference, 100);
        base_fees.insert(L2TxKind::AIModelStorage, 0);
        base_fees.insert(L2TxKind::AIModelUpdate, 500);
        base_fees.insert(L2TxKind::FederatedLearning, 2_000);
        base_fees.insert(L2TxKind::ProofOfInference, 1_500);

        unit_fees.insert(L2TxKind::AIModelRegister, 1);
        unit_fees.insert(L2TxKind::AIModelInference, 10);
        unit_fees.insert(L2TxKind::AIModelStorage, 1);
        unit_fees.insert(L2TxKind::AIModelUpdate, 1);
        unit_fees.insert(L2TxKind::FederatedLearning, 100);
        unit_fees.insert(L2TxKind::ProofOfInference, 50);

        max_fees.insert(L2TxKind::AIModelRegister, 100_000);
        max_fees.insert(L2TxKind::AIModelInference, 10_000);
        max_fees.insert(L2TxKind::AIModelStorage, 50_000);
        max_fees.insert(L2TxKind::AIModelUpdate, 50_000);
        max_fees.insert(L2TxKind::FederatedLearning, 20_000);
        max_fees.insert(L2TxKind::ProofOfInference, 15_000);

        min_fees.insert(L2TxKind::AIModelRegister, 1_000);
        min_fees.insert(L2TxKind::AIModelInference, 100);
        min_fees.insert(L2TxKind::AIModelStorage, 0);
        min_fees.insert(L2TxKind::AIModelUpdate, 500);
        min_fees.insert(L2TxKind::FederatedLearning, 2_000);
        min_fees.insert(L2TxKind::ProofOfInference, 1_500);

        Self {
            base_fees,
            unit_fees,
            max_fees,
            min_fees,
        }
    }
}

impl L2FeeStructure {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum L2FeeError {
    #[error("No base fee configured for {0:?}")]
    MissingBaseFee(L2TxKind),
}

/// L2 fee calculation result
#[derive(Debug, Clone)]
pub struct L2FeeCalculation {
    pub tx_kind: L2TxKind,
    pub base_fee: u64,
    pub unit_fee: u64,
    pub units: u64,
    pub total_fee: u64,
    pub calculation_method: String,
}

/// L2 fee manager
#[derive(Debug, Clone)]
pub struct L2FeeManager {
    pub fee_structure: L2FeeStructure,
    pub collected_fees: HashMap<L2TxKind, u64>,
    pub total_collected: u64,
}

impl L2FeeManager {
    pub fn new() -> Self {
        Self {
            fee_structure: L2FeeStructure::default(),
            collected_fees: HashMap::new(),
            total_collected: 0,
        }
    }

    #[instrument(skip(self, additional_data))]
    pub fn calculate_fee(
        &self,
        tx_kind: L2TxKind,
        units: Option<u64>,
        additional_data: Option<HashMap<String, u64>>,
    ) -> Result<L2FeeCalculation, L2FeeError> {
        let base_fee = self
            .fee_structure
            .base_fees
            .get(&tx_kind)
            .copied()
            .ok_or(L2FeeError::MissingBaseFee(tx_kind))?;

        let unit_fee = self
            .fee_structure
            .unit_fees
            .get(&tx_kind)
            .copied()
            .unwrap_or(0);

        let data_ref = additional_data.as_ref();
        let units = units.unwrap_or_else(|| self.calculate_units(tx_kind, data_ref));
        let unit_cost = unit_fee.saturating_mul(units);
        let total_fee = base_fee.saturating_add(unit_cost);

        // Enforce bounds
        let min_fee = *self.fee_structure.min_fees.get(&tx_kind).unwrap_or(&0);
        let max_fee = *self
            .fee_structure
            .max_fees
            .get(&tx_kind)
            .unwrap_or(&u64::MAX);
        let final_fee = total_fee.clamp(min_fee, max_fee);

        let calculation_method = match tx_kind {
            L2TxKind::ContractDeploy | L2TxKind::AIModelRegister | L2TxKind::AIModelUpdate => {
                "base + (size_bytes * unit_fee)"
            }
            L2TxKind::ContractCall | L2TxKind::AIModelInference => "base + (gas_units * unit_fee)",
            L2TxKind::AIModelStorage => "size_mb * days * unit_fee",
            L2TxKind::FederatedLearning => "base + (rounds * unit_fee)",
            L2TxKind::ProofOfInference => "base + (complexity * unit_fee)",
        }
        .to_string();

        let calculation = L2FeeCalculation {
            tx_kind,
            base_fee,
            unit_fee,
            units,
            total_fee: final_fee,
            calculation_method,
        };

        debug!(?calculation, "l2 fee calculated");
        Ok(calculation)
    }

    fn calculate_units(&self, tx_kind: L2TxKind, data_ref: Option<&HashMap<String, u64>>) -> u64 {
        match tx_kind {
            L2TxKind::ContractDeploy | L2TxKind::AIModelRegister | L2TxKind::AIModelUpdate => {
                data_ref
                    .and_then(|d| d.get("size_bytes").copied())
                    .unwrap_or(1000)
            }
            L2TxKind::ContractCall | L2TxKind::AIModelInference => data_ref
                .and_then(|d| {
                    d.get("gas_units")
                        .copied()
                        .or_else(|| d.get("compute_units").copied())
                })
                .unwrap_or(1000),
            L2TxKind::AIModelStorage => {
                let size = data_ref
                    .and_then(|d| d.get("size_mb"))
                    .copied()
                    .unwrap_or(1);
                let days = data_ref.and_then(|d| d.get("days")).copied().unwrap_or(1);
                size * days
            }
            L2TxKind::FederatedLearning => {
                data_ref.and_then(|d| d.get("rounds")).copied().unwrap_or(1)
            }
            L2TxKind::ProofOfInference => data_ref
                .and_then(|d| d.get("complexity"))
                .copied()
                .unwrap_or(100),
        }
    }

    pub fn collect_fee(&mut self, tx_kind: L2TxKind, amount: u64) {
        *self.collected_fees.entry(tx_kind).or_insert(0) += amount;
        self.total_collected += amount;
    }

    pub fn get_statistics(&self) -> L2FeeStatistics {
        L2FeeStatistics {
            total_collected: self.total_collected,
            fees_by_type: self.collected_fees.clone(),
            fee_structure: self.fee_structure.clone(),
        }
    }

    pub fn update_fee_structure(&mut self, new_structure: L2FeeStructure) {
        self.fee_structure = new_structure;
    }
}

/// L2 fee statistics
#[derive(Debug, Clone)]
pub struct L2FeeStatistics {
    pub total_collected: u64,
    pub fees_by_type: HashMap<L2TxKind, u64>,
    pub fee_structure: L2FeeStructure,
}

impl Default for L2FeeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl L2FeeCalculation {
    pub fn to_amount(&self) -> ippan_types::Amount {
        ippan_types::Amount::from_micro_ipn(self.total_fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_fee_calculation() {
        let manager = L2FeeManager::new();

        let mut data = HashMap::new();
        data.insert("size_bytes".to_string(), 5000);
        let result = manager
            .calculate_fee(L2TxKind::ContractDeploy, None, Some(data))
            .unwrap();
        assert_eq!(result.base_fee, 50_000);
        assert_eq!(result.units, 5000);
        assert_eq!(result.total_fee, 55_000);

        let mut data = HashMap::new();
        data.insert("compute_units".to_string(), 1000);
        let result = manager
            .calculate_fee(L2TxKind::AIModelInference, None, Some(data))
            .unwrap();
        assert_eq!(result.base_fee, 100);
        assert_eq!(result.units, 1000);
        assert_eq!(result.total_fee, 10_000); // capped by max fee
    }

    #[test]
    fn test_fee_collection() {
        let mut manager = L2FeeManager::new();
        manager.collect_fee(L2TxKind::ContractDeploy, 50_000);
        manager.collect_fee(L2TxKind::AIModelInference, 1_000);

        let stats = manager.get_statistics();
        assert_eq!(stats.total_collected, 51_000);
        assert_eq!(
            stats.fees_by_type.get(&L2TxKind::ContractDeploy),
            Some(&50_000)
        );
        assert_eq!(
            stats.fees_by_type.get(&L2TxKind::AIModelInference),
            Some(&1_000)
        );
    }

    #[test]
    fn test_fee_bounds() {
        let manager = L2FeeManager::new();

        let result = manager
            .calculate_fee(L2TxKind::ContractDeploy, Some(1), None)
            .unwrap();
        assert!(result.total_fee >= 10_000);

        let mut data = HashMap::new();
        data.insert("size_bytes".to_string(), 1_000_000);
        let result = manager
            .calculate_fee(L2TxKind::ContractDeploy, None, Some(data))
            .unwrap();
        assert!(result.total_fee <= 1_000_000);
    }
}
