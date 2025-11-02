//! Types for L2 handle registry

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Human-readable handle identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Handle(pub String);

impl Handle {
    /// Create a new handle from string
    pub fn new(handle: impl Into<String>) -> Self {
        Self(handle.into())
    }

    /// Get the handle as string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate handle format
    pub fn is_valid(&self) -> bool {
        let handle = &self.0;
        handle.starts_with('@') && handle.contains('.') && handle.len() > 3 && handle.len() < 64
    }

    /// Get the TLD (top-level domain) of the handle
    pub fn tld(&self) -> Option<&str> {
        self.0.split('.').next_back()
    }

    /// Check if this is a premium TLD
    pub fn is_premium(&self) -> bool {
        matches!(self.tld(), Some("cyborg") | Some("iot") | Some("m"))
    }
}

/// Public key identifier (Ed25519)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    /// Create from byte array
    pub fn new(key: [u8; 32]) -> Self {
        Self(key)
    }

    /// Get as byte array
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Handle metadata stored on L2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleMetadata {
    /// Owner's public key
    pub owner: PublicKey,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Expiration timestamp (0 = never expires)
    pub expires_at: u64,
    /// Handle status
    pub status: HandleStatus,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// L1 anchor hash (points to L1 ownership proof)
    pub l1_anchor: Option<[u8; 32]>,
}

/// Handle status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandleStatus {
    Active,
    Suspended,
    Expired,
    Transferred,
}

impl Default for HandleMetadata {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            owner: PublicKey([0u8; 32]),
            created_at: now,
            updated_at: now,
            expires_at: 0, // Never expires by default
            status: HandleStatus::Active,
            metadata: HashMap::new(),
            l1_anchor: None,
        }
    }
}

/// Handle registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleRegistration {
    pub handle: Handle,
    pub owner: PublicKey,
    pub signature: Vec<u8>,
    pub metadata: HashMap<String, String>,
    pub expires_at: Option<u64>,
}

/// Handle update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleUpdate {
    pub handle: Handle,
    pub owner: PublicKey,
    pub signature: Vec<u8>,
    pub updates: HashMap<String, String>,
}

/// Handle transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleTransfer {
    pub handle: Handle,
    pub from_owner: PublicKey,
    pub to_owner: PublicKey,
    pub signature: Vec<u8>,
}

/// L1 ownership anchor (stored on L1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1OwnershipAnchor {
    /// Handle hash
    pub handle_hash: [u8; 32],
    /// Owner's public key
    pub owner: PublicKey,
    /// L2 storage location hash
    pub l2_location: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
    /// Signature proving ownership
    pub signature: Vec<u8>,
}
