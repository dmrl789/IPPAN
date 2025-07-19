use crate::Result;
use crate::crosschain::external_anchor::{AnchorTx, ProofType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Bridge endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEndpoint {
    /// Chain identifier
    pub chain_id: String,
    /// Accepted anchor proof types
    pub accepted_anchor_types: Vec<ProofType>,
    /// Latest anchor for this chain
    pub latest_anchor: Option<AnchorTx>,
    /// Bridge configuration
    pub config: BridgeConfig,
    /// Bridge status
    pub status: BridgeStatus,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Maximum anchor frequency (anchors per hour)
    pub max_anchor_frequency: u32,
    /// Minimum proof requirements
    pub min_proof_requirements: Vec<ProofType>,
    /// Trust level (0-100)
    pub trust_level: u8,
    /// Auto-approve anchors
    pub auto_approve: bool,
    /// Verification timeout (ms)
    pub verification_timeout_ms: u64,
    /// Maximum anchor age (hours)
    pub max_anchor_age_hours: u32,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            max_anchor_frequency: 60, // 1 anchor per minute
            min_proof_requirements: vec![ProofType::Signature],
            trust_level: 50,
            auto_approve: false,
            verification_timeout_ms: 5000,
            max_anchor_age_hours: 24,
        }
    }
}

/// Bridge status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BridgeStatus {
    /// Bridge is active
    Active,
    /// Bridge is paused
    Paused,
    /// Bridge is suspended
    Suspended,
    /// Bridge is being upgraded
    Upgrading,
    /// Bridge is deprecated
    Deprecated,
}

/// Bridge registry for managing external chain connections
pub struct BridgeRegistry {
    /// Registered bridge endpoints
    endpoints: HashMap<String, BridgeEndpoint>,
    /// Bridge statistics
    stats: BridgeStats,
    /// Global bridge configuration
    global_config: GlobalBridgeConfig,
}

/// Bridge statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStats {
    /// Total number of bridges
    pub total_bridges: usize,
    /// Active bridges
    pub active_bridges: usize,
    /// Total anchors processed
    pub total_anchors_processed: u64,
    /// Total verification attempts
    pub total_verifications: u64,
    /// Successful verifications
    pub successful_verifications: u64,
    /// Bridge uptime statistics
    pub uptime_stats: HashMap<String, f64>,
}

/// Global bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBridgeConfig {
    /// Default trust level for new bridges
    pub default_trust_level: u8,
    /// Maximum number of bridges
    pub max_bridges: usize,
    /// Global verification timeout (ms)
    pub global_verification_timeout_ms: u64,
    /// Enable bridge monitoring
    pub enable_monitoring: bool,
    /// Bridge health check interval (seconds)
    pub health_check_interval_seconds: u64,
}

impl Default for GlobalBridgeConfig {
    fn default() -> Self {
        Self {
            default_trust_level: 50,
            max_bridges: 100,
            global_verification_timeout_ms: 10000,
            enable_monitoring: true,
            health_check_interval_seconds: 300, // 5 minutes
        }
    }
}

impl BridgeRegistry {
    /// Create a new bridge registry
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            stats: BridgeStats {
                total_bridges: 0,
                active_bridges: 0,
                total_anchors_processed: 0,
                total_verifications: 0,
                successful_verifications: 0,
                uptime_stats: HashMap::new(),
            },
            global_config: GlobalBridgeConfig::default(),
        }
    }

    /// Register a new bridge endpoint
    pub async fn register_endpoint(&mut self, endpoint: BridgeEndpoint) -> Result<()> {
        let chain_id = endpoint.chain_id.clone();
        
        // Check if we've reached the maximum number of bridges
        if self.endpoints.len() >= self.global_config.max_bridges {
            return Err(crate::error::IppanError::Validation(
                format!("Maximum number of bridges ({}) reached", self.global_config.max_bridges)
            ));
        }
        
        // Validate the endpoint
        self.validate_endpoint(&endpoint)?;
        
        // Add the endpoint
        self.endpoints.insert(chain_id.clone(), endpoint);
        
        // Update statistics
        self.stats.total_bridges = self.endpoints.len();
        self.stats.active_bridges = self.endpoints.values()
            .filter(|e| matches!(e.status, BridgeStatus::Active))
            .count();
        
        info!("Registered bridge endpoint for chain: {}", chain_id);
        Ok(())
    }

    /// Get a bridge endpoint
    pub fn get_endpoint(&self, chain_id: &str) -> Option<BridgeEndpoint> {
        self.endpoints.get(chain_id).cloned()
    }

    /// Get all bridge endpoints
    pub fn get_all_endpoints(&self) -> Vec<BridgeEndpoint> {
        self.endpoints.values().cloned().collect()
    }

    /// Update bridge endpoint
    pub async fn update_endpoint(&mut self, chain_id: &str, updates: BridgeEndpointUpdate) -> Result<()> {
        if let Some(endpoint) = self.endpoints.get_mut(chain_id) {
            if let Some(status) = updates.status {
                endpoint.status = status;
            }
            
            if let Some(config) = updates.config {
                endpoint.config = config;
            }
            
            if let Some(anchor) = updates.latest_anchor {
                endpoint.latest_anchor = Some(anchor);
                self.stats.total_anchors_processed += 1;
            }
            
            endpoint.last_activity = chrono::Utc::now();
            
            info!("Updated bridge endpoint for chain: {}", chain_id);
            Ok(())
        } else {
            Err(crate::error::IppanError::NotFound(
                format!("Bridge endpoint not found for chain: {}", chain_id)
            ))
        }
    }

    /// Remove a bridge endpoint
    pub async fn remove_endpoint(&mut self, chain_id: &str) -> Result<()> {
        if self.endpoints.remove(chain_id).is_some() {
            self.stats.total_bridges = self.endpoints.len();
            self.stats.active_bridges = self.endpoints.values()
                .filter(|e| matches!(e.status, BridgeStatus::Active))
                .count();
            
            info!("Removed bridge endpoint for chain: {}", chain_id);
            Ok(())
        } else {
            Err(crate::error::IppanError::NotFound(
                format!("Bridge endpoint not found for chain: {}", chain_id)
            ))
        }
    }

    /// Check if a bridge is active
    pub fn is_bridge_active(&self, chain_id: &str) -> bool {
        self.endpoints.get(chain_id)
            .map(|e| matches!(e.status, BridgeStatus::Active))
            .unwrap_or(false)
    }

    /// Get bridge statistics
    pub fn get_stats(&self) -> BridgeStats {
        self.stats.clone()
    }

    /// Update bridge statistics
    pub fn update_verification_stats(&mut self, success: bool) {
        self.stats.total_verifications += 1;
        if success {
            self.stats.successful_verifications += 1;
        }
    }

    /// Validate bridge endpoint
    fn validate_endpoint(&self, endpoint: &BridgeEndpoint) -> Result<()> {
        // Check if chain_id is not empty
        if endpoint.chain_id.is_empty() {
            return Err(crate::error::IppanError::Validation(
                "Chain ID cannot be empty".to_string()
            ));
        }
        
        // Check if chain_id is already registered
        if self.endpoints.contains_key(&endpoint.chain_id) {
            return Err(crate::error::IppanError::Validation(
                format!("Bridge endpoint already exists for chain: {}", endpoint.chain_id)
            ));
        }
        
        // Validate trust level
        if endpoint.config.trust_level > 100 {
            return Err(crate::error::IppanError::Validation(
                "Trust level must be between 0 and 100".to_string()
            ));
        }
        
        // Validate anchor frequency
        if endpoint.config.max_anchor_frequency == 0 {
            return Err(crate::error::IppanError::Validation(
                "Max anchor frequency must be greater than 0".to_string()
            ));
        }
        
        debug!("Bridge endpoint validation passed for chain: {}", endpoint.chain_id);
        Ok(())
    }

    /// Get bridges by status
    pub fn get_bridges_by_status(&self, status: BridgeStatus) -> Vec<BridgeEndpoint> {
        self.endpoints.values()
            .filter(|e| e.status == status)
            .cloned()
            .collect()
    }

    /// Get bridges by trust level
    pub fn get_bridges_by_trust_level(&self, min_trust_level: u8) -> Vec<BridgeEndpoint> {
        self.endpoints.values()
            .filter(|e| e.config.trust_level >= min_trust_level)
            .cloned()
            .collect()
    }

    /// Get bridge health status
    pub fn get_bridge_health(&self, chain_id: &str) -> Option<BridgeHealth> {
        self.endpoints.get(chain_id).map(|endpoint| {
            let now = chrono::Utc::now();
            let last_activity_age = now.signed_duration_since(endpoint.last_activity);
            
            BridgeHealth {
                chain_id: chain_id.to_string(),
                status: endpoint.status.clone(),
                last_activity_age_seconds: last_activity_age.num_seconds() as u64,
                trust_level: endpoint.config.trust_level,
                is_healthy: last_activity_age.num_hours() < 24, // Consider healthy if active in last 24 hours
            }
        })
    }

    /// Get all bridge health statuses
    pub fn get_all_bridge_health(&self) -> Vec<BridgeHealth> {
        self.endpoints.keys()
            .filter_map(|chain_id| self.get_bridge_health(chain_id))
            .collect()
    }

    /// Set global bridge configuration
    pub fn set_global_config(&mut self, config: GlobalBridgeConfig) {
        self.global_config = config;
    }

    /// Get global bridge configuration
    pub fn get_global_config(&self) -> GlobalBridgeConfig {
        self.global_config.clone()
    }
}

/// Bridge endpoint update
#[derive(Debug, Clone)]
pub struct BridgeEndpointUpdate {
    pub status: Option<BridgeStatus>,
    pub config: Option<BridgeConfig>,
    pub latest_anchor: Option<AnchorTx>,
}

/// Bridge health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealth {
    pub chain_id: String,
    pub status: BridgeStatus,
    pub last_activity_age_seconds: u64,
    pub trust_level: u8,
    pub is_healthy: bool,
}

/// Bridge monitoring event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEvent {
    pub chain_id: String,
    pub event_type: BridgeEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: String,
}

/// Bridge event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeEventType {
    BridgeRegistered,
    BridgeRemoved,
    BridgeStatusChanged,
    AnchorReceived,
    VerificationAttempted,
    VerificationSucceeded,
    VerificationFailed,
    BridgeUnhealthy,
    BridgeRecovered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_registry_creation() {
        let registry = BridgeRegistry::new();
        assert_eq!(registry.stats.total_bridges, 0);
        assert_eq!(registry.stats.active_bridges, 0);
    }

    #[tokio::test]
    async fn test_bridge_registration() {
        let mut registry = BridgeRegistry::new();
        
        let endpoint = BridgeEndpoint {
            chain_id: "testchain".to_string(),
            accepted_anchor_types: vec![ProofType::Signature, ProofType::ZK],
            latest_anchor: None,
            config: BridgeConfig::default(),
            status: BridgeStatus::Active,
            last_activity: chrono::Utc::now(),
        };
        
        let result = registry.register_endpoint(endpoint).await;
        assert!(result.is_ok());
        assert_eq!(registry.stats.total_bridges, 1);
        assert_eq!(registry.stats.active_bridges, 1);
    }

    #[tokio::test]
    async fn test_bridge_endpoint_retrieval() {
        let mut registry = BridgeRegistry::new();
        
        let endpoint = BridgeEndpoint {
            chain_id: "testchain".to_string(),
            accepted_anchor_types: vec![ProofType::Signature],
            latest_anchor: None,
            config: BridgeConfig::default(),
            status: BridgeStatus::Active,
            last_activity: chrono::Utc::now(),
        };
        
        registry.register_endpoint(endpoint).await.unwrap();
        
        let retrieved = registry.get_endpoint("testchain");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().chain_id, "testchain");
    }

    #[tokio::test]
    async fn test_bridge_removal() {
        let mut registry = BridgeRegistry::new();
        
        let endpoint = BridgeEndpoint {
            chain_id: "testchain".to_string(),
            accepted_anchor_types: vec![ProofType::Signature],
            latest_anchor: None,
            config: BridgeConfig::default(),
            status: BridgeStatus::Active,
            last_activity: chrono::Utc::now(),
        };
        
        registry.register_endpoint(endpoint).await.unwrap();
        assert_eq!(registry.stats.total_bridges, 1);
        
        registry.remove_endpoint("testchain").await.unwrap();
        assert_eq!(registry.stats.total_bridges, 0);
    }

    #[tokio::test]
    async fn test_bridge_health_check() {
        let mut registry = BridgeRegistry::new();
        
        let endpoint = BridgeEndpoint {
            chain_id: "testchain".to_string(),
            accepted_anchor_types: vec![ProofType::Signature],
            latest_anchor: None,
            config: BridgeConfig::default(),
            status: BridgeStatus::Active,
            last_activity: chrono::Utc::now(),
        };
        
        registry.register_endpoint(endpoint).await.unwrap();
        
        let health = registry.get_bridge_health("testchain");
        assert!(health.is_some());
        assert!(health.unwrap().is_healthy);
    }
} 