//! L2 Fee System for Smart Contracts and AI Operations
//!
//! This module handles all fees for L2 operations including:
//! - Smart contract deployment and execution
//! - AI model registration, inference, and storage
//! - Complex computations and applications

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// L2 transaction types (smart contracts and AI operations)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum L2TxKind {
    /// Smart contract deployment
    ContractDeploy,
    /// Smart contract execution
    ContractCall,
    /// AI model registration
    AIModelRegister,
    /// AI model inference
    AIModelInference,
    /// AI model storage
    AIModelStorage,
    /// AI model update
    AIModelUpdate,
    /// Federated learning
    FederatedLearning,
    /// Proof of inference
    ProofOfInference,
}

/// L2 fee structure for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2FeeStructure {
    /// Base fees for each operation type (in micro-IPN)
    pub base_fees: HashMap<L2TxKind, u64>,
    
    /// Unit fees for variable-cost operations (in micro-IPN per unit)
    pub unit_fees: HashMap<L2TxKind, u64>,
    
    /// Maximum fees to prevent excessive charges
    pub max_fees: HashMap<L2TxKind, u64>,
    
    /// Minimum fees to prevent spam
    pub min_fees: HashMap<L2TxKind, u64>,
}

impl Default for L2FeeStructure {
    fn default() -> Self {
        let mut base_fees = HashMap::new();
        let mut unit_fees = HashMap::new();
        let mut max_fees = HashMap::new();
        let mut min_fees = HashMap::new();
        
        // Smart contract fees
        base_fees.insert(L2TxKind::ContractDeploy, 50_000); // 0.05 IPN
        base_fees.insert(L2TxKind::ContractCall, 5_000);    // 0.005 IPN
        
        unit_fees.insert(L2TxKind::ContractDeploy, 1);      // 1 µIPN per byte
        unit_fees.insert(L2TxKind::ContractCall, 10);       // 10 µIPN per gas unit
        
        max_fees.insert(L2TxKind::ContractDeploy, 1_000_000); // 1 IPN max
        max_fees.insert(L2TxKind::ContractCall, 100_000);     // 0.1 IPN max
        
        min_fees.insert(L2TxKind::ContractDeploy, 10_000);   // 0.01 IPN min
        min_fees.insert(L2TxKind::ContractCall, 1_000);      // 0.001 IPN min
        
        // AI model fees
        base_fees.insert(L2TxKind::AIModelRegister, 1_000);     // 0.001 IPN
        base_fees.insert(L2TxKind::AIModelInference, 100);      // 0.0001 IPN
        base_fees.insert(L2TxKind::AIModelStorage, 0);          // No base fee
        base_fees.insert(L2TxKind::AIModelUpdate, 500);         // 0.0005 IPN
        base_fees.insert(L2TxKind::FederatedLearning, 2_000);   // 0.002 IPN
        base_fees.insert(L2TxKind::ProofOfInference, 1_500);    // 0.0015 IPN
        
        unit_fees.insert(L2TxKind::AIModelRegister, 1);         // 1 µIPN per byte
        unit_fees.insert(L2TxKind::AIModelInference, 10);       // 10 µIPN per compute unit
        unit_fees.insert(L2TxKind::AIModelStorage, 1);          // 1 µIPN per MB per day
        unit_fees.insert(L2TxKind::AIModelUpdate, 1);           // 1 µIPN per byte changed
        unit_fees.insert(L2TxKind::FederatedLearning, 100);     // 100 µIPN per round
        unit_fees.insert(L2TxKind::ProofOfInference, 50);       // 50 µIPN per complexity unit
        
        max_fees.insert(L2TxKind::AIModelRegister, 100_000);    // 0.1 IPN max
        max_fees.insert(L2TxKind::AIModelInference, 10_000);    // 0.01 IPN max
        max_fees.insert(L2TxKind::AIModelStorage, 50_000);      // 0.05 IPN max per day
        max_fees.insert(L2TxKind::AIModelUpdate, 50_000);       // 0.05 IPN max
        max_fees.insert(L2TxKind::FederatedLearning, 20_000);   // 0.02 IPN max
        max_fees.insert(L2TxKind::ProofOfInference, 15_000);    // 0.015 IPN max
        
        min_fees.insert(L2TxKind::AIModelRegister, 1_000);      // 0.001 IPN min
        min_fees.insert(L2TxKind::AIModelInference, 100);       // 0.0001 IPN min
        min_fees.insert(L2TxKind::AIModelStorage, 0);           // No minimum
        min_fees.insert(L2TxKind::AIModelUpdate, 500);          // 0.0005 IPN min
        min_fees.insert(L2TxKind::FederatedLearning, 2_000);    // 0.002 IPN min
        min_fees.insert(L2TxKind::ProofOfInference, 1_500);     // 0.0015 IPN min
        
        Self {
            base_fees,
            unit_fees,
            max_fees,
            min_fees,
        }
    }
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
    /// Create a new L2 fee manager
    pub fn new() -> Self {
        Self {
            fee_structure: L2FeeStructure::default(),
            collected_fees: HashMap::new(),
            total_collected: 0,
        }
    }
    
    /// Calculate fee for an L2 operation
    pub fn calculate_fee(
        &self,
        tx_kind: L2TxKind,
        units: Option<u64>,
        additional_data: Option<HashMap<String, u64>>,
    ) -> Result<L2FeeCalculation, String> {
        let base_fee = self.fee_structure.base_fees
            .get(&tx_kind)
            .copied()
            .ok_or_else(|| format!("No base fee configured for {:?}", tx_kind))?;
        
        let unit_fee = self.fee_structure.unit_fees
            .get(&tx_kind)
            .copied()
            .unwrap_or(0);
        
        let units = units.unwrap_or_else(|| self.calculate_units(tx_kind, &additional_data));
        let unit_cost = unit_fee * units;
        let total_fee = base_fee + unit_cost;
        
        // Apply min/max bounds
        let min_fee = self.fee_structure.min_fees
            .get(&tx_kind)
            .copied()
            .unwrap_or(0);
        let max_fee = self.fee_structure.max_fees
            .get(&tx_kind)
            .copied()
            .unwrap_or(u64::MAX);
        
        let final_fee = total_fee.max(min_fee).min(max_fee);
        
        let calculation_method = match tx_kind {
            L2TxKind::ContractDeploy | L2TxKind::AIModelRegister | L2TxKind::AIModelUpdate => {
                "base + (size_bytes * unit_fee)".to_string()
            }
            L2TxKind::ContractCall | L2TxKind::AIModelInference => {
                "base + (gas_units * unit_fee)".to_string()
            }
            L2TxKind::AIModelStorage => {
                "size_mb * days * unit_fee".to_string()
            }
            L2TxKind::FederatedLearning => {
                "base + (rounds * unit_fee)".to_string()
            }
            L2TxKind::ProofOfInference => {
                "base + (complexity * unit_fee)".to_string()
            }
        };
        
        Ok(L2FeeCalculation {
            tx_kind,
            base_fee,
            unit_fee,
            units,
            total_fee: final_fee,
            calculation_method,
        })
    }
    
    /// Calculate units for variable-cost operations
    fn calculate_units(&self, tx_kind: L2TxKind, additional_data: &Option<HashMap<String, u64>>) -> u64 {
        match tx_kind {
            L2TxKind::ContractDeploy | L2TxKind::AIModelRegister | L2TxKind::AIModelUpdate => {
                additional_data
                    .as_ref()
                    .and_then(|data| data.get("size_bytes").copied())
                    .unwrap_or(1000) // Default 1KB
            }
            L2TxKind::ContractCall | L2TxKind::AIModelInference => {
                additional_data
                    .as_ref()
                    .and_then(|data| data.get("gas_units").or_else(|| data.get("compute_units")).copied())
                    .unwrap_or(1000) // Default 1000 gas/compute units
            }
            L2TxKind::AIModelStorage => {
                let size_mb = additional_data
                    .as_ref()
                    .and_then(|data| data.get("size_mb").copied())
                    .unwrap_or(1);
                let days = additional_data
                    .as_ref()
                    .and_then(|data| data.get("days").copied())
                    .unwrap_or(1);
                size_mb * days
            }
            L2TxKind::FederatedLearning => {
                additional_data
                    .as_ref()
                    .and_then(|data| data.get("rounds").copied())
                    .unwrap_or(1)
            }
            L2TxKind::ProofOfInference => {
                additional_data
                    .as_ref()
                    .and_then(|data| data.get("complexity").copied())
                    .unwrap_or(100)
            }
        }
    }
    
    /// Collect fee for an L2 operation
    pub fn collect_fee(&mut self, tx_kind: L2TxKind, amount: u64) {
        *self.collected_fees.entry(tx_kind).or_insert(0) += amount;
        self.total_collected += amount;
    }
    
    /// Get fee statistics
    pub fn get_statistics(&self) -> L2FeeStatistics {
        L2FeeStatistics {
            total_collected: self.total_collected,
            fees_by_type: self.collected_fees.clone(),
            fee_structure: self.fee_structure.clone(),
        }
    }
    
    /// Update fee structure
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_fee_calculation() {
        let manager = L2FeeManager::new();
        
        // Test smart contract deployment
        let mut data = HashMap::new();
        data.insert("size_bytes".to_string(), 5000);
        
        let result = manager.calculate_fee(L2TxKind::ContractDeploy, None, Some(data)).unwrap();
        assert_eq!(result.base_fee, 50_000);
        assert_eq!(result.units, 5000);
        assert_eq!(result.total_fee, 55_000); // 50_000 + (5000 * 1)
        
        // Test AI model inference
        let mut data = HashMap::new();
        data.insert("compute_units".to_string(), 1000);
        
        let result = manager.calculate_fee(L2TxKind::AIModelInference, None, Some(data)).unwrap();
        assert_eq!(result.base_fee, 100);
        assert_eq!(result.units, 1000);
        assert_eq!(result.total_fee, 10_100); // 100 + (1000 * 10)
    }
    
    #[test]
    fn test_fee_collection() {
        let mut manager = L2FeeManager::new();
        
        manager.collect_fee(L2TxKind::ContractDeploy, 50_000);
        manager.collect_fee(L2TxKind::AIModelInference, 1_000);
        
        let stats = manager.get_statistics();
        assert_eq!(stats.total_collected, 51_000);
        assert_eq!(stats.fees_by_type.get(&L2TxKind::ContractDeploy), Some(&50_000));
        assert_eq!(stats.fees_by_type.get(&L2TxKind::AIModelInference), Some(&1_000));
    }
    
    #[test]
    fn test_fee_bounds() {
        let manager = L2FeeManager::new();
        
        // Test minimum fee enforcement
        let result = manager.calculate_fee(L2TxKind::ContractDeploy, Some(1), None).unwrap();
        assert!(result.total_fee >= 10_000); // Minimum fee
        
        // Test maximum fee enforcement
        let mut data = HashMap::new();
        data.insert("size_bytes".to_string(), 1_000_000); // Very large contract
        
        let result = manager.calculate_fee(L2TxKind::ContractDeploy, None, Some(data)).unwrap();
        assert!(result.total_fee <= 1_000_000); // Maximum fee
    }
}