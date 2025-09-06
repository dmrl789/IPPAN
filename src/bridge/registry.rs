//! L2 Registry for managing L2 parameters and configurations
//! 
//! This module provides a registry for L2 networks, allowing IPPAN to
//! track and validate L2 commits and exits according to their specific parameters.

use crate::crosschain::types::{L2Params, ProofType, DataAvailabilityMode};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// L2 registry configuration
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct L2RegistryConfig {
    /// Maximum number of L2s that can be registered
    pub max_l2_count: usize,
    /// Default maximum commit size in bytes
    pub default_max_commit_size: usize,
    /// Default minimum epoch gap in milliseconds
    pub default_min_epoch_gap_ms: u64,
    /// Default challenge window for optimistic rollups in milliseconds
    pub default_challenge_window_ms: u64,
}

impl Default for L2RegistryConfig {
    fn default() -> Self {
        Self {
            max_l2_count: 100,
            default_max_commit_size: 16384, // 16 KB
            default_min_epoch_gap_ms: 250,  // 250ms
            default_challenge_window_ms: 60000, // 1 minute
        }
    }
}

/// L2 registry entry with metadata
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct L2RegistryEntry {
    /// L2 parameters
    pub params: L2Params,
    /// When this L2 was registered
    pub registered_at: u64,
    /// Last commit timestamp
    pub last_commit_at: Option<u64>,
    /// Last epoch number
    pub last_epoch: Option<u64>,
    /// Whether this L2 is active
    pub active: bool,
}

/// L2 Registry for managing L2 networks
#[derive(Default)]
pub struct L2Registry {
    /// Configuration
    config: L2RegistryConfig,
    /// Registered L2s
    table: Arc<RwLock<HashMap<String, L2RegistryEntry>>>,
}

impl L2Registry {
    /// Create a new L2 registry with default configuration
    pub fn new() -> Self {
        Self {
            config: L2RegistryConfig::default(),
            table: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new L2 registry with custom configuration
    pub fn with_config(config: L2RegistryConfig) -> Self {
        Self {
            config,
            table: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new L2 network
    pub async fn register(
        &self,
        l2_id: String,
        proof_type: ProofType,
        da_mode: Option<DataAvailabilityMode>,
        custom_params: Option<L2Params>,
    ) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        // Check if L2 already exists
        if table.contains_key(&l2_id) {
            return Err(format!("L2 '{}' is already registered", l2_id));
        }
        
        // Check maximum L2 count
        if table.len() >= self.config.max_l2_count {
            return Err("Maximum L2 count reached".to_string());
        }
        
        // Create L2 parameters
        let params = if let Some(custom) = custom_params {
            custom
        } else {
            let mut params = L2Params::default_for(proof_type.clone());
            if let Some(da_mode) = da_mode {
                params.da_mode = da_mode;
            }
            params
        };
        
        // Create registry entry
        let entry = L2RegistryEntry {
            params,
            registered_at: chrono::Utc::now().timestamp_millis() as u64,
            last_commit_at: None,
            last_epoch: None,
            active: true,
        };
        
        // Insert into registry
        table.insert(l2_id.clone(), entry);
        
        info!("Registered L2 '{}' with proof type {:?}", l2_id, proof_type);
        Ok(())
    }

    /// Get L2 parameters by ID
    pub async fn get(&self, l2_id: &str) -> Option<L2Params> {
        let table = self.table.read().await;
        table.get(l2_id).map(|entry| entry.params.clone())
    }

    /// Get L2 registry entry by ID
    pub async fn get_entry(&self, l2_id: &str) -> Option<L2RegistryEntry> {
        let table = self.table.read().await;
        table.get(l2_id).cloned()
    }

    /// Check if L2 is registered
    pub async fn is_registered(&self, l2_id: &str) -> bool {
        let table = self.table.read().await;
        table.contains_key(l2_id)
    }

    /// Update L2 parameters
    pub async fn update_params(
        &self,
        l2_id: &str,
        new_params: L2Params,
    ) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        if let Some(entry) = table.get_mut(l2_id) {
            entry.params = new_params;
            info!("Updated parameters for L2 '{}'", l2_id);
            Ok(())
        } else {
            Err(format!("L2 '{}' not found", l2_id))
        }
    }

    /// Deactivate an L2
    pub async fn deactivate(&self, l2_id: &str) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        if let Some(entry) = table.get_mut(l2_id) {
            entry.active = false;
            info!("Deactivated L2 '{}'", l2_id);
            Ok(())
        } else {
            Err(format!("L2 '{}' not found", l2_id))
        }
    }

    /// Reactivate an L2
    pub async fn reactivate(&self, l2_id: &str) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        if let Some(entry) = table.get_mut(l2_id) {
            entry.active = true;
            info!("Reactivated L2 '{}'", l2_id);
            Ok(())
        } else {
            Err(format!("L2 '{}' not found", l2_id))
        }
    }

    /// Record a commit for an L2
    pub async fn record_commit(
        &self,
        l2_id: &str,
        epoch: u64,
        timestamp: u64,
    ) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        if let Some(entry) = table.get_mut(l2_id) {
            // Validate epoch monotonicity
            if let Some(last_epoch) = entry.last_epoch {
                if epoch <= last_epoch {
                    return Err(format!("Epoch {} must be greater than last epoch {}", epoch, last_epoch));
                }
            }
            
            entry.last_commit_at = Some(timestamp);
            entry.last_epoch = Some(epoch);
            
            debug!("Recorded commit for L2 '{}' at epoch {}", l2_id, epoch);
            Ok(())
        } else {
            Err(format!("L2 '{}' not found", l2_id))
        }
    }

    /// Get all registered L2s
    pub async fn list_all(&self) -> Vec<(String, L2RegistryEntry)> {
        let table = self.table.read().await;
        table.iter().map(|(id, entry)| (id.clone(), entry.clone())).collect()
    }

    /// Get active L2s only
    pub async fn list_active(&self) -> Vec<(String, L2RegistryEntry)> {
        let table = self.table.read().await;
        table
            .iter()
            .filter(|(_, entry)| entry.active)
            .map(|(id, entry)| (id.clone(), entry.clone()))
            .collect()
    }

    /// Get registry statistics
    pub async fn stats(&self) -> L2RegistryStats {
        let table = self.table.read().await;
        let total = table.len();
        let active = table.values().filter(|entry| entry.active).count();
        
        L2RegistryStats {
            total_l2s: total,
            active_l2s: active,
            max_l2s: self.config.max_l2_count,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &L2RegistryConfig {
        &self.config
    }
}

/// L2 registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2RegistryStats {
    /// Total number of registered L2s
    pub total_l2s: usize,
    /// Number of active L2s
    pub active_l2s: usize,
    /// Maximum number of L2s allowed
    pub max_l2s: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crosschain::types::ProofType;

    #[tokio::test]
    async fn test_register_l2() {
        let registry = L2Registry::new();
        
        // Register a new L2
        let result = registry.register(
            "test-l2".to_string(),
            ProofType::ZkGroth16,
            None,
            None,
        ).await;
        
        assert!(result.is_ok());
        assert!(registry.is_registered("test-l2").await);
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let registry = L2Registry::new();
        
        // Register first time
        registry.register("test-l2".to_string(), ProofType::ZkGroth16, None, None).await.unwrap();
        
        // Try to register again
        let result = registry.register("test-l2".to_string(), ProofType::Optimistic, None, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_record_commit() {
        let registry = L2Registry::new();
        
        // Register L2
        registry.register("test-l2".to_string(), ProofType::ZkGroth16, None, None).await.unwrap();
        
        // Record first commit
        let result = registry.record_commit("test-l2", 1, 1000).await;
        assert!(result.is_ok());
        
        // Record second commit
        let result = registry.record_commit("test-l2", 2, 2000).await;
        assert!(result.is_ok());
        
        // Try to record epoch regression
        let result = registry.record_commit("test-l2", 1, 3000).await;
        assert!(result.is_err());
    }
}
