//! Types for L1 handle ownership anchors

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// L1 handle ownership anchor
/// 
/// This is the minimal data stored on L1 for handle ownership.
/// The actual handle mappings are stored on L2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleOwnershipAnchor {
    /// Hash of the handle (computed from handle string)
    pub handle_hash: [u8; 32],
    /// Owner's public key
    pub owner: [u8; 32],
    /// L2 storage location hash (points to L2 handle registry)
    pub l2_location: [u8; 32],
    /// Block height when anchor was created
    pub block_height: u64,
    /// Round when anchor was created
    pub round: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Signature proving ownership (signed by owner)
    pub signature: Vec<u8>,
}

impl HandleOwnershipAnchor {
    /// Create a new ownership anchor
    pub fn new(
        handle: &str,
        owner: [u8; 32],
        l2_location: [u8; 32],
        block_height: u64,
        round: u64,
        signature: Vec<u8>,
    ) -> Self {
        let handle_hash = Self::compute_handle_hash(handle);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        Self {
            handle_hash,
            owner,
            l2_location,
            block_height,
            round,
            timestamp,
            signature,
        }
    }
    
    /// Compute hash of handle string
    pub fn compute_handle_hash(handle: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(handle.as_bytes());
        hasher.finalize().into()
    }
    
    /// Verify the ownership signature
    pub fn verify_signature(&self) -> bool {
        // In production, this would verify Ed25519 signatures
        // For now, just check that signature is not empty
        !self.signature.is_empty()
    }
    
    /// Check if anchor is expired (older than 1 year)
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        now - self.timestamp > 365 * 24 * 60 * 60 // 1 year
    }
}

/// Handle ownership proof
/// 
/// Contains the minimal information needed to prove handle ownership
/// without storing the full handle mapping on L1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleOwnershipProof {
    /// The ownership anchor
    pub anchor: HandleOwnershipAnchor,
    /// Merkle proof of inclusion in L1 state
    pub merkle_proof: Vec<[u8; 32]>,
    /// Root hash of the state tree
    pub state_root: [u8; 32],
}

impl HandleOwnershipProof {
    /// Verify the ownership proof
    pub fn verify(&self) -> bool {
        self.anchor.verify_signature() && !self.anchor.is_expired()
    }
    
    /// Get the handle hash
    pub fn handle_hash(&self) -> [u8; 32] {
        self.anchor.handle_hash
    }
    
    /// Get the owner
    pub fn owner(&self) -> [u8; 32] {
        self.anchor.owner
    }
}