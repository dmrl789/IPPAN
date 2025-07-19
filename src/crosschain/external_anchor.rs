use crate::Result;
use crate::consensus::hashtimer::HashTimer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Anchor transaction from external chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorTx {
    /// External chain identifier (e.g., "starknet", "rollupX")
    pub external_chain_id: String,
    /// Hash of external state or block
    pub external_state_root: String,
    /// IPPAN HashTimer for precise timing
    pub timestamp: HashTimer,
    /// Type of proof provided (optional)
    pub proof_type: Option<ProofType>,
    /// Proof data (signature, zk-proof, etc.)
    pub proof_data: Vec<u8>,
}

/// Types of proofs that can be provided with anchor transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProofType {
    /// No proof (trust-based)
    None,
    /// Cryptographic signature from external validator
    Signature,
    /// Zero-knowledge proof (zk-STARK/SNARK)
    ZK,
    /// Merkle proof
    Merkle,
    /// Multi-signature proof
    MultiSig,
}

/// Anchor transaction with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorTxWithMetadata {
    /// The anchor transaction
    pub anchor_tx: AnchorTx,
    /// IPPAN block hash where this anchor was included
    pub block_hash: String,
    /// IPPAN round number
    pub round: u64,
    /// IPPAN block height
    pub height: u64,
    /// Submission timestamp
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    /// Validation status
    pub validation_status: ValidationStatus,
}

/// Validation status of anchor transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// Pending validation
    Pending,
    /// Validated successfully
    Valid,
    /// Validation failed
    Invalid,
    /// Validation error
    Error(String),
}

/// Anchor manager for handling external chain anchors
pub struct AnchorManager {
    /// Storage for anchor transactions
    anchors: Arc<RwLock<HashMap<String, Vec<AnchorTxWithMetadata>>>>,
    /// Maximum number of anchors to keep per chain
    max_anchor_history: usize,
    /// Validation rules for different proof types
    validation_rules: Arc<RwLock<HashMap<String, Vec<ProofType>>>>,
}

impl AnchorManager {
    /// Create a new anchor manager
    pub async fn new(max_anchor_history: usize) -> Result<Self> {
        Ok(Self {
            anchors: Arc::new(RwLock::new(HashMap::new())),
            max_anchor_history,
            validation_rules: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Submit an anchor transaction
    pub async fn submit_anchor(&self, anchor_tx: AnchorTx) -> Result<String> {
        // Validate the anchor transaction
        self.validate_anchor(&anchor_tx).await?;
        
        // Create metadata
        let metadata = AnchorTxWithMetadata {
            anchor_tx: anchor_tx.clone(),
            block_hash: "pending".to_string(), // Will be updated when included in block
            round: 0, // Will be updated when included in block
            height: 0, // Will be updated when included in block
            submitted_at: chrono::Utc::now(),
            validation_status: ValidationStatus::Pending,
        };
        
        // Store the anchor
        {
            let mut anchors = self.anchors.write().await;
            let chain_anchors = anchors.entry(anchor_tx.external_chain_id.clone()).or_insert_with(Vec::new);
            chain_anchors.push(metadata);
            
            // Trim old anchors if we exceed the limit
            if chain_anchors.len() > self.max_anchor_history {
                let to_remove = chain_anchors.len() - self.max_anchor_history;
                chain_anchors.drain(0..to_remove);
            }
        }
        
        info!(
            "Submitted anchor for chain {} with state root {}",
            anchor_tx.external_chain_id, anchor_tx.external_state_root
        );
        
        Ok(format!("anchor_{}", anchor_tx.external_state_root))
    }

    /// Get the latest anchor for a specific chain
    pub async fn get_latest_anchor(&self, chain_id: &str) -> Result<Option<AnchorTx>> {
        let anchors = self.anchors.read().await;
        
        if let Some(chain_anchors) = anchors.get(chain_id) {
            if let Some(latest) = chain_anchors.last() {
                return Ok(Some(latest.anchor_tx.clone()));
            }
        }
        
        Ok(None)
    }

    /// Get recent anchors for a specific chain
    pub async fn get_recent_anchors(&self, chain_id: &str, limit: usize) -> Result<Vec<AnchorTx>> {
        let anchors = self.anchors.read().await;
        
        if let Some(chain_anchors) = anchors.get(chain_id) {
            let recent: Vec<AnchorTx> = chain_anchors
                .iter()
                .rev()
                .take(limit)
                .map(|a| a.anchor_tx.clone())
                .collect();
            return Ok(recent);
        }
        
        Ok(Vec::new())
    }

    /// Get all recent anchors across all chains
    pub async fn get_recent_anchors_all(&self, hours: u64) -> Result<Vec<AnchorTx>> {
        let anchors = self.anchors.read().await;
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
        
        let mut recent_anchors = Vec::new();
        
        for chain_anchors in anchors.values() {
            for anchor_meta in chain_anchors {
                if anchor_meta.submitted_at > cutoff_time {
                    recent_anchors.push(anchor_meta.anchor_tx.clone());
                }
            }
        }
        
        Ok(recent_anchors)
    }

    /// Update anchor metadata when included in a block
    pub async fn update_anchor_metadata(
        &self,
        chain_id: &str,
        state_root: &str,
        block_hash: &str,
        round: u64,
        height: u64,
    ) -> Result<()> {
        let mut anchors = self.anchors.write().await;
        
        if let Some(chain_anchors) = anchors.get_mut(chain_id) {
            for anchor_meta in chain_anchors {
                if anchor_meta.anchor_tx.external_state_root == state_root {
                    anchor_meta.block_hash = block_hash.to_string();
                    anchor_meta.round = round;
                    anchor_meta.height = height;
                    anchor_meta.validation_status = ValidationStatus::Valid;
                    
                    info!(
                        "Updated anchor metadata for chain {} state root {} in block {}",
                        chain_id, state_root, block_hash
                    );
                    return Ok(());
                }
            }
        }
        
        warn!(
            "Anchor not found for chain {} state root {}",
            chain_id, state_root
        );
        Ok(())
    }

    /// Set validation rules for a chain
    pub async fn set_validation_rules(&self, chain_id: &str, proof_types: Vec<ProofType>) -> Result<()> {
        let mut rules = self.validation_rules.write().await;
        rules.insert(chain_id.to_string(), proof_types);
        Ok(())
    }

    /// Validate an anchor transaction
    async fn validate_anchor(&self, anchor_tx: &AnchorTx) -> Result<()> {
        // Check if chain_id is not empty
        if anchor_tx.external_chain_id.is_empty() {
            return Err(crate::error::IppanError::Validation(
                "External chain ID cannot be empty".to_string()
            ));
        }
        
        // Check if state root is not empty
        if anchor_tx.external_state_root.is_empty() {
            return Err(crate::error::IppanError::Validation(
                "External state root cannot be empty".to_string()
            ));
        }
        
        // Check validation rules if they exist
        {
            let rules = self.validation_rules.read().await;
            if let Some(allowed_proof_types) = rules.get(&anchor_tx.external_chain_id) {
                if let Some(proof_type) = &anchor_tx.proof_type {
                    if !allowed_proof_types.contains(proof_type) {
                        return Err(crate::error::IppanError::Validation(
                            format!("Proof type {:?} not allowed for chain {}", proof_type, anchor_tx.external_chain_id)
                        ));
                    }
                } else {
                    // If no proof type is provided, check if None is allowed
                    if !allowed_proof_types.contains(&ProofType::None) {
                        return Err(crate::error::IppanError::Validation(
                            format!("No proof type provided but required for chain {}", anchor_tx.external_chain_id)
                        ));
                    }
                }
            }
        }
        
        // Validate proof data if proof type is provided
        if let Some(proof_type) = &anchor_tx.proof_type {
            match proof_type {
                ProofType::Signature => {
                    if anchor_tx.proof_data.len() < 64 {
                        return Err(crate::error::IppanError::Validation(
                            "Signature proof data must be at least 64 bytes".to_string()
                        ));
                    }
                }
                ProofType::ZK => {
                    if anchor_tx.proof_data.is_empty() {
                        return Err(crate::error::IppanError::Validation(
                            "ZK proof data cannot be empty".to_string()
                        ));
                    }
                }
                ProofType::Merkle => {
                    if anchor_tx.proof_data.len() < 32 {
                        return Err(crate::error::IppanError::Validation(
                            "Merkle proof data must be at least 32 bytes".to_string()
                        ));
                    }
                }
                ProofType::MultiSig => {
                    if anchor_tx.proof_data.len() < 64 {
                        return Err(crate::error::IppanError::Validation(
                            "Multi-sig proof data must be at least 64 bytes".to_string()
                        ));
                    }
                }
                ProofType::None => {
                    // No validation needed for None proof type
                }
            }
        }
        
        debug!("Anchor validation passed for chain {}", anchor_tx.external_chain_id);
        Ok(())
    }

    /// Get anchor statistics
    pub async fn get_anchor_stats(&self) -> Result<AnchorStats> {
        let anchors = self.anchors.read().await;
        
        let mut total_anchors = 0;
        let mut chain_count = 0;
        let mut proof_type_distribution = HashMap::new();
        
        for (chain_id, chain_anchors) in anchors.iter() {
            chain_count += 1;
            total_anchors += chain_anchors.len();
            
            for anchor_meta in chain_anchors {
                let proof_type = anchor_meta.anchor_tx.proof_type.as_ref()
                    .map(|pt| format!("{:?}", pt))
                    .unwrap_or_else(|| "None".to_string());
                
                *proof_type_distribution.entry(proof_type).or_insert(0) += 1;
            }
        }
        
        Ok(AnchorStats {
            total_anchors,
            chain_count,
            proof_type_distribution,
        })
    }
}

/// Anchor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorStats {
    pub total_anchors: usize,
    pub chain_count: usize,
    pub proof_type_distribution: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anchor_manager_creation() {
        let manager = AnchorManager::new(100).await.unwrap();
        assert_eq!(manager.max_anchor_history, 100);
    }

    #[tokio::test]
    async fn test_anchor_submission() {
        let manager = AnchorManager::new(100).await.unwrap();
        
        let anchor_tx = AnchorTx {
            external_chain_id: "testchain".to_string(),
            external_state_root: "0x1234567890abcdef".to_string(),
            timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
            proof_type: Some(ProofType::Signature),
            proof_data: vec![1; 64], // Valid signature length
        };
        
        let result = manager.submit_anchor(anchor_tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_anchor_validation_rules() {
        let manager = AnchorManager::new(100).await.unwrap();
        
        // Set validation rules
        manager.set_validation_rules("testchain", vec![ProofType::Signature, ProofType::ZK]).await.unwrap();
        
        // Test valid anchor
        let valid_anchor = AnchorTx {
            external_chain_id: "testchain".to_string(),
            external_state_root: "0x1234567890abcdef".to_string(),
            timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
            proof_type: Some(ProofType::Signature),
            proof_data: vec![1; 64],
        };
        
        let result = manager.submit_anchor(valid_anchor).await;
        assert!(result.is_ok());
        
        // Test invalid anchor (wrong proof type)
        let invalid_anchor = AnchorTx {
            external_chain_id: "testchain".to_string(),
            external_state_root: "0xabcdef1234567890".to_string(),
            timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
            proof_type: Some(ProofType::Merkle),
            proof_data: vec![1; 32],
        };
        
        let result = manager.submit_anchor(invalid_anchor).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_latest_anchor() {
        let manager = AnchorManager::new(100).await.unwrap();
        
        let anchor_tx = AnchorTx {
            external_chain_id: "testchain".to_string(),
            external_state_root: "0x1234567890abcdef".to_string(),
            timestamp: HashTimer::new([0u8; 32], [0u8; 32]),
            proof_type: Some(ProofType::Signature),
            proof_data: vec![1; 64],
        };
        
        manager.submit_anchor(anchor_tx).await.unwrap();
        
        let latest = manager.get_latest_anchor("testchain").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().external_state_root, "0x1234567890abcdef");
    }
} 