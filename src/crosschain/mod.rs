pub mod external_anchor;
pub mod foreign_verifier;
pub mod bridge;
pub mod sync_light;
pub mod types;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub use external_anchor::{ExternalAnchorData, L2AnchorHandler};
pub use foreign_verifier::{L2Verifier, DefaultL2Verifier, L2VerificationContext, VerifyError};
pub use bridge::{L2Registry, L2RegistryConfig, L2RegistryEntry, L2RegistryStats};
pub use sync_light::{LightSyncClient, LightSyncConfig};
pub use types::{L2CommitTx, L2ExitTx, L2Params, AnchorEvent, ExitStatus, L2ExitRecord, L2ValidationError, ProofType, DataAvailabilityMode};

/// Cross-chain manager that coordinates all cross-chain functionality
pub struct CrossChainManager {
    l2_anchor_handler: Arc<L2AnchorHandler>,
    l2_verifier: Arc<DefaultL2Verifier>,
    l2_registry: Arc<RwLock<L2Registry>>,
    light_sync_client: Arc<LightSyncClient>,
    config: CrossChainConfig,
}

#[derive(Debug, Clone)]
pub struct CrossChainConfig {
    pub enable_anchoring: bool,
    pub enable_verification: bool,
    pub enable_light_sync: bool,
    pub max_anchor_history: usize,
    pub verification_timeout_ms: u64,
    pub light_sync_enabled: bool,
}

impl Default for CrossChainConfig {
    fn default() -> Self {
        Self {
            enable_anchoring: true,
            enable_verification: true,
            enable_light_sync: true,
            max_anchor_history: 1000,
            verification_timeout_ms: 5000,
            light_sync_enabled: true,
        }
    }
}

impl CrossChainManager {
    /// Create a new cross-chain manager
    pub async fn new(config: CrossChainConfig) -> Result<Self> {
        let l2_anchor_handler = Arc::new(L2AnchorHandler::new());
        let l2_verifier = Arc::new(DefaultL2Verifier);
        let l2_registry = Arc::new(RwLock::new(L2Registry::new()));
        let light_sync_client = Arc::new(LightSyncClient::new(LightSyncConfig::default()).await?);
        
        Ok(Self {
            l2_anchor_handler,
            l2_verifier,
            l2_registry,
            light_sync_client,
            config,
        })
    }

    /// Submit an L2 commit transaction
    pub async fn submit_l2_commit(&self, commit: L2CommitTx) -> Result<String> {
        if !self.config.enable_anchoring {
            return Err(crate::error::IppanError::FeatureDisabled("L2 anchoring is disabled".to_string()));
        }
        
        // Validate the commit
        let mut registry = self.l2_registry.write().await;
        self.l2_verifier.verify_commit(&commit, &*registry).await
            .map_err(|e| crate::error::IppanError::Validation(e.to_string()))?;
        
        // Create anchor event
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let event = self.l2_anchor_handler.handle_l2_commit(&commit, timestamp).await
            .map_err(|e| crate::error::IppanError::Validation(e))?;
        
        // Record in registry
        registry.record_commit(&commit.l2_id, commit.epoch, timestamp).await
            .map_err(|e| crate::error::IppanError::Validation(e))?;
        
        Ok(format!("l2_commit_{}", commit.l2_id))
    }

    /// Get the latest L2 anchor for a chain
    pub async fn get_latest_l2_anchor(&self, l2_id: &str) -> Result<Option<AnchorEvent>> {
        Ok(self.l2_anchor_handler.get_latest_l2_event(l2_id).await)
    }

    /// Verify L2 exit transaction
    pub async fn verify_l2_exit(&self, exit: L2ExitTx) -> Result<()> {
        if !self.config.enable_verification {
            return Err(crate::error::IppanError::FeatureDisabled("L2 verification is disabled".to_string()));
        }
        
        let registry = self.l2_registry.read().await;
        self.l2_verifier.verify_exit(&exit, &*registry).await
            .map_err(|e| crate::error::IppanError::Validation(e.to_string()))
    }

    /// Register an L2 network
    pub async fn register_l2(&self, l2_id: String, params: L2Params) -> Result<()> {
        let mut registry = self.l2_registry.write().await;
        registry.register(l2_id, params).await
            .map_err(|e| crate::error::IppanError::Validation(e))
    }

    /// Get light sync data for a specific round
    pub async fn get_light_sync_data(&self, round: u64) -> Result<Option<LightSyncData>> {
        if !self.config.light_sync_enabled {
            return Err(crate::error::IppanError::FeatureDisabled("Light sync is disabled".to_string()));
        }
        
        self.light_sync_client.get_sync_data(round).await
    }

    /// Get comprehensive L2 report
    pub async fn generate_l2_report(&self) -> Result<L2Report> {
        let registry = self.l2_registry.read().await;
        let stats = registry.get_stats().await;
        
        Ok(L2Report {
            total_l2s: stats.total_registered,
            active_l2s: stats.total_registered, // All registered L2s are considered active
            total_commits: stats.total_commits,
            total_exits: stats.total_exits,
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}

/// Light sync data for ultra-light clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSyncData {
    pub round: u64,
    pub hashtimer: crate::consensus::hashtimer::HashTimer,
    pub merkle_root: String,
    pub zk_proof: Option<Vec<u8>>,
    pub anchor_headers: Vec<AnchorHeader>,
}

/// Anchor header for light sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorHeader {
    pub chain_id: String,
    pub state_root: String,
    pub timestamp: u64,
    pub round: u64,
}

/// Comprehensive L2 report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Report {
    pub total_l2s: usize,
    pub active_l2s: usize,
    pub total_commits: usize,
    pub total_exits: usize,
    pub generated_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cross_chain_manager_creation() {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await.unwrap();
        
        // Test that manager was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_l2_commit_submission() {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await.unwrap();
        
        let commit_tx = L2CommitTx {
            l2_id: "test-l2".to_string(),
            epoch: 1,
            state_root: [1u8; 32],
            da_hash: [2u8; 32],
            proof_type: ProofType::ZkGroth16,
            proof: vec![1; 64],
            inline_data: None,
        };
        
        let result = manager.submit_l2_commit(commit_tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_l2_registration() {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await.unwrap();
        
        let params = L2Params {
            proof_type: ProofType::ZkGroth16,
            da_mode: DataAvailabilityMode::External,
            challenge_window_ms: 60000,
            max_commit_size: 16384,
            min_epoch_gap_ms: 250,
        };
        
        let result = manager.register_l2("test-l2".to_string(), params).await;
        assert!(result.is_ok());
    }
} 