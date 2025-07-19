pub mod external_anchor;
pub mod foreign_verifier;
pub mod bridge;
pub mod sync_light;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub use external_anchor::{AnchorTx, ProofType, AnchorManager};
pub use foreign_verifier::{ForeignVerifier, VerificationError, VerificationResult};
pub use bridge::{BridgeEndpoint, BridgeRegistry, BridgeConfig, BridgeStatus};
pub use sync_light::{LightSyncClient, LightSyncConfig};

/// Cross-chain manager that coordinates all cross-chain functionality
pub struct CrossChainManager {
    anchor_manager: Arc<AnchorManager>,
    foreign_verifier: Arc<ForeignVerifier>,
    bridge_registry: Arc<RwLock<BridgeRegistry>>,
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
        let anchor_manager = Arc::new(AnchorManager::new(config.max_anchor_history).await?);
        let foreign_verifier = Arc::new(ForeignVerifier::new().await?);
        let bridge_registry = Arc::new(RwLock::new(BridgeRegistry::new()));
        let light_sync_client = Arc::new(LightSyncClient::new(LightSyncConfig::default()).await?);
        
        Ok(Self {
            anchor_manager,
            foreign_verifier,
            bridge_registry,
            light_sync_client,
            config,
        })
    }

    /// Submit an anchor transaction
    pub async fn submit_anchor(&self, anchor_tx: AnchorTx) -> Result<String> {
        if !self.config.enable_anchoring {
            return Err(crate::error::IppanError::FeatureDisabled("Anchoring is disabled".to_string()));
        }
        
        self.anchor_manager.submit_anchor(anchor_tx).await
    }

    /// Get the latest anchor for a chain
    pub async fn get_latest_anchor(&self, chain_id: &str) -> Result<Option<AnchorTx>> {
        self.anchor_manager.get_latest_anchor(chain_id).await
    }

    /// Verify external inclusion proof
    pub async fn verify_external_inclusion(
        &self,
        chain_id: &str,
        tx_hash: &str,
        merkle_proof: &[u8],
    ) -> Result<VerificationResult> {
        if !self.config.enable_verification {
            return Err(crate::error::IppanError::FeatureDisabled("Verification is disabled".to_string()));
        }
        
        self.foreign_verifier.verify_external_inclusion(chain_id, tx_hash, merkle_proof).await
    }

    /// Register a bridge endpoint
    pub async fn register_bridge(&self, endpoint: BridgeEndpoint) -> Result<()> {
        let mut registry = self.bridge_registry.write().await;
        registry.register_endpoint(endpoint).await
    }

    /// Get bridge endpoint information
    pub async fn get_bridge_endpoint(&self, chain_id: &str) -> Result<Option<BridgeEndpoint>> {
        let registry = self.bridge_registry.read().await;
        Ok(registry.get_endpoint(chain_id))
    }

    /// Get light sync data for a specific round
    pub async fn get_light_sync_data(&self, round: u64) -> Result<Option<LightSyncData>> {
        if !self.config.light_sync_enabled {
            return Err(crate::error::IppanError::FeatureDisabled("Light sync is disabled".to_string()));
        }
        
        self.light_sync_client.get_sync_data(round).await
    }

    /// Get comprehensive cross-chain report
    pub async fn generate_cross_chain_report(&self) -> Result<CrossChainReport> {
        let anchors = self.anchor_manager.get_recent_anchors_all(24).await?; // Last 24 hours
        let bridges = self.bridge_registry.read().await.get_all_endpoints();
        let verification_stats = self.foreign_verifier.get_verification_stats().await?;
        
        Ok(CrossChainReport {
            total_anchors: anchors.len(),
            active_bridges: bridges.len(),
            verification_success_rate: if verification_stats.total_verifications > 0 {
                verification_stats.successful_verifications as f64 / verification_stats.total_verifications as f64
            } else {
                0.0
            },
            recent_anchors: anchors,
            bridge_endpoints: bridges,
            generated_at: chrono::Utc::now(),
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

/// Comprehensive cross-chain report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainReport {
    pub total_anchors: usize,
    pub active_bridges: usize,
    pub verification_success_rate: f64,
    pub recent_anchors: Vec<AnchorTx>,
    pub bridge_endpoints: Vec<BridgeEndpoint>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
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
    async fn test_anchor_submission() {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await.unwrap();
        
        let anchor_tx = AnchorTx {
            external_chain_id: "testchain".to_string(),
            external_state_root: "0x1234567890abcdef".to_string(),
            timestamp: crate::consensus::hashtimer::HashTimer::new("test_node", 1, 1),
            proof_type: Some(ProofType::Signature),
            proof_data: vec![1; 64], // Valid signature length
        };
        
        let result = manager.submit_anchor(anchor_tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bridge_registration() {
        let config = CrossChainConfig::default();
        let manager = CrossChainManager::new(config).await.unwrap();
        
        let endpoint = BridgeEndpoint {
            chain_id: "testchain".to_string(),
            accepted_anchor_types: vec![ProofType::Signature, ProofType::ZK],
            latest_anchor: None,
            config: BridgeConfig::default(),
            status: BridgeStatus::Active,
            last_activity: chrono::Utc::now(),
        };
        
        let result = manager.register_bridge(endpoint).await;
        assert!(result.is_ok());
    }
} 