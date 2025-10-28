//! Types for validator resolution
//!
//! Defines canonical types used for validator identity resolution across
//! L1 anchors, L2 handle registry, and direct Ed25519 identifiers.

use ippan_economics::ValidatorId;
use serde::{Deserialize, Serialize};

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

/// Resolution method used for validator identification
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

/// Extended metadata about a validator (from registry or anchor)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorMetadata {
    /// Handle if resolved from handle (e.g., "@alice.ipn")
    pub handle: Option<String>,
    /// Creation timestamp (HashTimer or UNIX micros)
    pub created_at: Option<u64>,
    /// Last updated timestamp
    pub updated_at: Option<u64>,
    /// Status (e.g., "active", "revoked", "expired")
    pub status: Option<String>,
    /// Arbitrary custom metadata (JSON key–value map)
    pub custom: std::collections::HashMap<String, String>,
}

impl ResolvedValidator {
    /// Create a new resolved validator
    pub fn new(id: ValidatorId, public_key: [u8; 32], resolution_method: ResolutionMethod) -> Self {
        Self {
            id,
            public_key,
            resolution_method,
            metadata: None,
        }
    }

    /// Create a resolved validator with metadata
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

    /// Get the public key as raw bytes
    pub fn public_key_bytes(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// Check if this was resolved from a human-readable handle
    pub fn is_handle_resolved(&self) -> bool {
        matches!(self.resolution_method, ResolutionMethod::L2HandleRegistry)
    }
}
