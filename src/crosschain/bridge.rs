//! L2 Registry for managing L2 network registrations and parameters
//! 
//! This module provides the registry for L2 networks, tracking their parameters,
//! commit history, and exit status.

use crate::crosschain::types::{L2Params, ProofType, DataAvailabilityMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// L2 registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2RegistryConfig {
    /// Maximum number of L2s that can be registered
    pub max_l2_count: usize,
    /// Default challenge window for optimistic proofs (ms)
    pub default_challenge_window_ms: u64,
    /// Default maximum commit size (bytes)
    pub default_max_commit_size: usize,
    /// Default minimum epoch gap (ms)
    pub default_min_epoch_gap_ms: u64,
    /// Default data availability mode
    pub default_da_mode: DataAvailabilityMode,
}

impl Default for L2RegistryConfig {
    fn default() -> Self {
        Self {
            max_l2_count: 100,
            default_challenge_window_ms: 60000, // 1 minute
            default_max_commit_size: 16384,     // 16 KB
            default_min_epoch_gap_ms: 250,      // 250ms
            default_da_mode: DataAvailabilityMode::External,
        }
    }
}

/// L2 registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2RegistryEntry {
    /// L2 network parameters
    pub params: L2Params,
    /// Last commit epoch
    pub last_epoch: u64,
    /// Last commit timestamp
    pub last_commit_ts: u64,
    /// Total commits
    pub total_commits: usize,
    /// Total exits
    pub total_exits: usize,
    /// Registration timestamp
    pub registered_at: u64,
}

/// L2 registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2RegistryStats {
    /// Total registered L2s
    pub total_registered: usize,
    /// Total commits across all L2s
    pub total_commits: usize,
    /// Total exits across all L2s
    pub total_exits: usize,
    /// Average commits per L2
    pub avg_commits_per_l2: f64,
    /// Average exits per L2
    pub avg_exits_per_l2: f64,
}

impl Default for L2RegistryStats {
    fn default() -> Self {
        Self {
            total_registered: 0,
            total_commits: 0,
            total_exits: 0,
            avg_commits_per_l2: 0.0,
            avg_exits_per_l2: 0.0,
        }
    }
}

/// L2 Registry for managing L2 network registrations
#[derive(Default)]
pub struct L2Registry {
    /// Registry configuration
    config: L2RegistryConfig,
    /// Registered L2 networks
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
    pub async fn register(&mut self, l2_id: String, params: L2Params) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        // Check if L2 ID already exists
        if table.contains_key(&l2_id) {
            return Err(format!("L2 network '{}' is already registered", l2_id));
        }
        
        // Check if we've reached the maximum number of L2s
        if table.len() >= self.config.max_l2_count {
            return Err(format!("Maximum number of L2s ({}) reached", self.config.max_l2_count));
        }
        
        // Create registry entry
        let entry = L2RegistryEntry {
            params,
            last_epoch: 0,
            last_commit_ts: 0,
            total_commits: 0,
            total_exits: 0,
            registered_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        table.insert(l2_id.clone(), entry);
        info!("Registered L2 network '{}'", l2_id);
        
        Ok(())
    }

    /// Get L2 parameters for a registered network
    pub async fn get(&self, l2_id: &str) -> Option<L2Params> {
        let table = self.table.read().await;
        table.get(l2_id).map(|entry| entry.params.clone())
    }

    /// Check if an L2 network is registered
    pub async fn is_registered(&self, l2_id: &str) -> bool {
        let table = self.table.read().await;
        table.contains_key(l2_id)
    }

    /// Record a commit for an L2 network
    pub async fn record_commit(&mut self, l2_id: &str, epoch: u64, timestamp: u64) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        let entry = table.get_mut(l2_id)
            .ok_or_else(|| format!("L2 network '{}' not found", l2_id))?;
        
        // Validate epoch monotonicity
        if epoch <= entry.last_epoch {
            return Err(format!("Epoch {} must be greater than last epoch {}", epoch, entry.last_epoch));
        }
        
        // Validate rate limiting
        let time_since_last = timestamp.saturating_sub(entry.last_commit_ts);
        if time_since_last < entry.params.min_epoch_gap_ms {
            return Err(format!("Rate limit exceeded: {}ms since last commit, minimum gap is {}ms", 
                time_since_last, entry.params.min_epoch_gap_ms));
        }
        
        // Update entry
        entry.last_epoch = epoch;
        entry.last_commit_ts = timestamp;
        entry.total_commits += 1;
        
        debug!("Recorded commit for L2 '{}' at epoch {}", l2_id, epoch);
        Ok(())
    }

    /// Record an exit for an L2 network
    pub async fn record_exit(&mut self, l2_id: &str, nonce: u64) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        let entry = table.get_mut(l2_id)
            .ok_or_else(|| format!("L2 network '{}' not found", l2_id))?;
        
        entry.total_exits += 1;
        debug!("Recorded exit for L2 '{}' with nonce {}", l2_id, nonce);
        Ok(())
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> L2RegistryStats {
        let table = self.table.read().await;
        let total_registered = table.len();
        let total_commits: usize = table.values().map(|entry| entry.total_commits).sum();
        let total_exits: usize = table.values().map(|entry| entry.total_exits).sum();
        
        L2RegistryStats {
            total_registered,
            total_commits,
            total_exits,
            avg_commits_per_l2: if total_registered > 0 {
                total_commits as f64 / total_registered as f64
            } else {
                0.0
            },
            avg_exits_per_l2: if total_registered > 0 {
                total_exits as f64 / total_registered as f64
            } else {
                0.0
            },
        }
    }

    /// Get all registered L2 IDs
    pub async fn get_all_l2_ids(&self) -> Vec<String> {
        let table = self.table.read().await;
        table.keys().cloned().collect()
    }

    /// Get L2 registry entry
    pub async fn get_entry(&self, l2_id: &str) -> Option<L2RegistryEntry> {
        let table = self.table.read().await;
        table.get(l2_id).cloned()
    }

    /// Update L2 parameters
    pub async fn update_params(&mut self, l2_id: &str, new_params: L2Params) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        let entry = table.get_mut(l2_id)
            .ok_or_else(|| format!("L2 network '{}' not found", l2_id))?;
        
        entry.params = new_params;
        info!("Updated parameters for L2 network '{}'", l2_id);
        Ok(())
    }

    /// Deregister an L2 network
    pub async fn deregister(&mut self, l2_id: &str) -> Result<(), String> {
        let mut table = self.table.write().await;
        
        if table.remove(l2_id).is_some() {
            info!("Deregistered L2 network '{}'", l2_id);
            Ok(())
        } else {
            Err(format!("L2 network '{}' not found", l2_id))
        }
    }

    /// Get configuration
    pub fn get_config(&self) -> &L2RegistryConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: L2RegistryConfig) {
        self.config = config;
        info!("Updated L2 registry configuration");
    }
}


