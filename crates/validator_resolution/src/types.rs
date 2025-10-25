//! Types for validator resolution

use serde::{Deserialize, Serialize};
use ippan_ippan_economics::ValidatorId;

/// Resolved validator information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedValidator {
    /// The original ValidatorId
    pub id: ValidatorId,
    /// Resolved public key
    pub public_key: [u8; 32],
    /// Resolution method used
    pub resolution_method: ResolutionMethod,
    /// Additional metadata
    pub metadata: Option<ValidatorMetadata>,
}

/// Resolution method used
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionMethod {
    /// Direct public key (no resolution needed)
    Direct,
    /// Resolved via L2 handle registry
    L2HandleRegistry,
    /// Resolved via L1 ownership anchor
    L1OwnershipAnchor,
    /// Resolved via registry alias
    RegistryAlias,
}

/// Validator metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorMetadata {
    /// Handle if resolved from handle
    pub handle: Option<String>,
    /// Creation timestamp
    pub created_at: Option<u64>,
    /// Last updated timestamp
    pub updated_at: Option<u64>,
    /// Status
    pub status: Option<String>,
    /// Additional custom metadata
    pub custom: std::collections::HashMap<String, String>,
}

impl ResolvedValidator {
    /// Create a new resolved validator
    pub fn new(
        id: ValidatorId,
        public_key: [u8; 32],
        resolution_method: ResolutionMethod,
    ) -> Self {
        Self {
            id,
            public_key,
            resolution_method,
            metadata: None,
        }
    }
    
    /// Create with metadata
    pub fn with_metadata(
        id: ValidatorId,
        public_key: [u8; 32],
        resolution_method: ResolutionMethod,
        metadata: ValidatorMetadata,
    ) -> Self {
        Self {
            id,
            public_key,
            resolution_method,
            metadata: Some(metadata),
        }
    }
    
    /// Get the public key as bytes
    pub fn public_key_bytes(&self) -> &[u8; 32] {
        &self.public_key
    }
    
    /// Check if this was resolved from a handle
    pub fn is_handle_resolved(&self) -> bool {
        matches!(self.resolution_method, ResolutionMethod::L2HandleRegistry)
    }
}