//! Fee management for AI Registry

use crate::{
    errors::{RegistryError, Result},
    types::*,
    storage::RegistryStorage,
};
use ai_core::types::{ModelId, ModelMetadata};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Fee manager for AI Registry
pub struct FeeManager {
    /// Storage backend
    storage: RegistryStorage,
    /// Fee structures
    fee_structures: HashMap<FeeType, FeeStructure>,
    /// Configuration
    config: RegistryConfig,
    /// Fee collection statistics
    stats: FeeStats,
}

/// Fee collection statistics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FeeStats {
    /// Total fees collected
    pub total_fees_collected: u64,
    /// Fees by type
    pub fees_by_type: HashMap<FeeType, u64>,
    /// Fees by model
    pub fees_by_model: HashMap<ModelId, u64>,
    /// Fees by user
    pub fees_by_user: HashMap<String, u64>,
}

/// Fee calculation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeeCalculation {
    /// Fee type
    pub fee_type: FeeType,
    /// Calculated fee amount
    pub amount: u64,
    /// Base fee
    pub base_fee: u64,
    /// Unit fee
    pub unit_fee: u64,
    /// Units
    pub units: u64,
    /// Calculation method
    pub method: FeeCalculationMethod,
}

impl FeeManager {
    /// Create a new fee manager
    pub fn new(storage: RegistryStorage, config: RegistryConfig) -> Self {
        let mut fee_structures = HashMap::new();
        
        // Initialize default fee structures
        fee_structures.insert(FeeType::Registration, FeeStructure {
            fee_type: FeeType::Registration,
            base_fee: config.min_registration_fee,
            unit_fee: 1, // 1 per byte
            min_fee: config.min_registration_fee,
            max_fee: config.max_registration_fee,
            calculation_method: FeeCalculationMethod::Linear,
        });
        
        fee_structures.insert(FeeType::Execution, FeeStructure {
            fee_type: FeeType::Execution,
            base_fee: config.default_execution_fee,
            unit_fee: 1, // 1 per execution
            min_fee: 1,
            max_fee: 1000000, // 1M max
            calculation_method: FeeCalculationMethod::Fixed,
        });
        
        fee_structures.insert(FeeType::Storage, FeeStructure {
            fee_type: FeeType::Storage,
            base_fee: 0,
            unit_fee: config.storage_fee_per_byte_per_day,
            min_fee: 0,
            max_fee: 1000000, // 1M max per day
            calculation_method: FeeCalculationMethod::Linear,
        });
        
        fee_structures.insert(FeeType::Proposal, FeeStructure {
            fee_type: FeeType::Proposal,
            base_fee: config.proposal_fee,
            unit_fee: 0,
            min_fee: config.proposal_fee,
            max_fee: config.proposal_fee,
            calculation_method: FeeCalculationMethod::Fixed,
        });
        
        fee_structures.insert(FeeType::Update, FeeStructure {
            fee_type: FeeType::Update,
            base_fee: config.min_registration_fee / 2, // Half of registration fee
            unit_fee: 1, // 1 per byte changed
            min_fee: 1,
            max_fee: config.max_registration_fee / 2,
            calculation_method: FeeCalculationMethod::Linear,
        });
        
        Self {
            storage,
            fee_structures,
            config,
            stats: FeeStats::default(),
        }
    }

    /// Calculate fee for a specific operation
    pub fn calculate_fee(
        &self,
        fee_type: FeeType,
        model_metadata: Option<&ModelMetadata>,
        units: Option<u64>,
        additional_data: Option<HashMap<String, String>>,
    ) -> Result<FeeCalculation> {
        info!("Calculating fee for: {:?}", fee_type);
        
        let fee_structure = self.fee_structures
            .get(&fee_type)
            .ok_or_else(|| RegistryError::FeeCalculationError(
                format!("Fee structure not found for type: {:?}", fee_type)
            ))?;
        
        let (amount, calculated_units) = match fee_structure.calculation_method {
            FeeCalculationMethod::Fixed => {
                (fee_structure.base_fee, 1)
            },
            FeeCalculationMethod::Linear => {
                let calculated_units = if let Some(u) = units {
                    u
                } else {
                    self.calculate_units(fee_type, model_metadata, additional_data.clone())?
                };
                let unit_fee = fee_structure.unit_fee * calculated_units;
                (fee_structure.base_fee + unit_fee, calculated_units)
            },
            FeeCalculationMethod::Logarithmic => {
                let calculated_units = if let Some(u) = units {
                    u
                } else {
                    self.calculate_units(fee_type, model_metadata, additional_data.clone())?
                };
                let log_units = (calculated_units as f64).ln().max(1.0) as u64;
                let unit_fee = fee_structure.unit_fee * log_units;
                (fee_structure.base_fee + unit_fee, calculated_units)
            },
            FeeCalculationMethod::Step => {
                let calculated_units = if let Some(u) = units {
                    u
                } else {
                    self.calculate_units(fee_type, model_metadata, additional_data.clone())?
                };
                let steps = (calculated_units + 999) / 1000; // 1000 units per step
                let unit_fee = fee_structure.unit_fee * steps;
                (fee_structure.base_fee + unit_fee, calculated_units)
            },
        };
        
        // Apply min/max bounds
        let final_amount = amount
            .max(fee_structure.min_fee)
            .min(fee_structure.max_fee);
        
        let calculation = FeeCalculation {
            fee_type,
            amount: final_amount,
            base_fee: fee_structure.base_fee,
            unit_fee: fee_structure.unit_fee,
            units: calculated_units,
            method: fee_structure.calculation_method,
        };
        
        info!("Fee calculated: {} (base: {}, unit: {}, units: {})", 
            final_amount, fee_structure.base_fee, fee_structure.unit_fee, calculated_units);
        
        Ok(calculation)
    }

    /// Collect fee for an operation
    pub async fn collect_fee(
        &mut self,
        fee_type: FeeType,
        model_id: Option<&ModelId>,
        user: &str,
        amount: u64,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        info!("Collecting fee: {:?} amount={} user={}", fee_type, amount, user);
        
        // Record fee collection
        self.storage.record_fee_collection(
            fee_type,
            model_id,
            user,
            amount,
            metadata,
        ).await?;
        
        // Update statistics
        self.update_fee_stats(fee_type, model_id, user, amount);
        
        info!("Fee collected successfully");
        Ok(())
    }

    /// Get fee statistics
    pub fn get_fee_stats(&self) -> &FeeStats {
        &self.stats
    }

    /// Get fee structure for a type
    pub fn get_fee_structure(&self, fee_type: &FeeType) -> Option<&FeeStructure> {
        self.fee_structures.get(fee_type)
    }

    /// Update fee structure
    pub fn update_fee_structure(&mut self, fee_structure: FeeStructure) -> Result<()> {
        info!("Updating fee structure: {:?}", fee_structure.fee_type);
        
        // Validate fee structure
        self.validate_fee_structure(&fee_structure)?;
        
        // Update fee structure
        self.fee_structures.insert(fee_structure.fee_type.clone(), fee_structure);
        
        info!("Fee structure updated successfully");
        Ok(())
    }

    /// Calculate units for fee calculation
    fn calculate_units(
        &self,
        fee_type: FeeType,
        model_metadata: Option<&ModelMetadata>,
        additional_data: Option<HashMap<String, String>>,
    ) -> Result<u64> {
        match fee_type {
            FeeType::Registration => {
                if let Some(metadata) = model_metadata {
                    Ok(metadata.size_bytes)
                } else {
                    Err(RegistryError::FeeCalculationError(
                        "Model metadata required for registration fee calculation".to_string()
                    ))
                }
            },
            FeeType::Execution => {
                Ok(1) // 1 execution
            },
            FeeType::Storage => {
                if let Some(metadata) = model_metadata {
                    Ok(metadata.size_bytes)
                } else {
                    Err(RegistryError::FeeCalculationError(
                        "Model metadata required for storage fee calculation".to_string()
                    ))
                }
            },
            FeeType::Proposal => {
                Ok(1) // 1 proposal
            },
            FeeType::Update => {
                if let Some(data) = additional_data {
                    if let Some(size_change) = data.get("size_change") {
                        Ok(size_change.parse::<u64>().unwrap_or(0))
                    } else {
                        Ok(0)
                    }
                } else {
                    Ok(0)
                }
            },
        }
    }

    /// Update fee statistics
    fn update_fee_stats(
        &mut self,
        fee_type: FeeType,
        model_id: Option<&ModelId>,
        user: &str,
        amount: u64,
    ) {
        self.stats.total_fees_collected += amount;
        
        // Update fees by type
        *self.stats.fees_by_type.entry(fee_type).or_insert(0) += amount;
        
        // Update fees by model
        if let Some(model_id) = model_id {
            *self.stats.fees_by_model.entry(model_id.clone()).or_insert(0) += amount;
        }
        
        // Update fees by user
        *self.stats.fees_by_user.entry(user.to_string()).or_insert(0) += amount;
    }

    /// Validate fee structure
    fn validate_fee_structure(&self, fee_structure: &FeeStructure) -> Result<()> {
        // Check min/max bounds
        if fee_structure.min_fee > fee_structure.max_fee {
            return Err(RegistryError::FeeCalculationError(
                "Minimum fee cannot be greater than maximum fee".to_string()
            ));
        }
        
        // Check base fee is within bounds
        if fee_structure.base_fee < fee_structure.min_fee || fee_structure.base_fee > fee_structure.max_fee {
            return Err(RegistryError::FeeCalculationError(
                "Base fee must be within min/max bounds".to_string()
            ));
        }
        
        // Check unit fee is reasonable
        if fee_structure.unit_fee > 1000000 { // 1M max per unit
            return Err(RegistryError::FeeCalculationError(
                "Unit fee is too high".to_string()
            ));
        }
        
        Ok(())
    }
}