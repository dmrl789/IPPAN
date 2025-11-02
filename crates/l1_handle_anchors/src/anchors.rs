//! L1 handle ownership anchor management

use crate::errors::*;
use crate::types::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// L1 Handle Ownership Anchor Storage
///
/// Stores minimal ownership proofs on L1. The actual handle mappings
/// are stored on L2 and referenced by these anchors.
#[derive(Debug)]
pub struct L1HandleAnchorStorage {
    /// Handle hash to ownership anchor mapping
    anchors: Arc<RwLock<HashMap<[u8; 32], HandleOwnershipAnchor>>>,
    /// Owner to handle hashes mapping (for reverse lookup)
    #[allow(clippy::type_complexity)]
    owner_to_handles: Arc<RwLock<HashMap<[u8; 32], Vec<[u8; 32]>>>>,
}

impl L1HandleAnchorStorage {
    /// Create new L1 handle anchor storage
    pub fn new() -> Self {
        Self {
            anchors: Arc::new(RwLock::new(HashMap::new())),
            owner_to_handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a handle ownership anchor
    pub fn store_anchor(&self, anchor: HandleOwnershipAnchor) -> Result<()> {
        // Verify signature
        if !anchor.verify_signature() {
            return Err(HandleAnchorError::InvalidSignature);
        }

        // Store anchor
        {
            let mut anchors = self.anchors.write();
            anchors.insert(anchor.handle_hash, anchor.clone());
        }

        // Update owner mapping
        {
            let mut owner_map = self.owner_to_handles.write();
            owner_map
                .entry(anchor.owner)
                .or_default()
                .push(anchor.handle_hash);
        }

        Ok(())
    }

    /// Get ownership anchor by handle hash
    pub fn get_anchor(&self, handle_hash: &[u8; 32]) -> Result<HandleOwnershipAnchor> {
        let anchors = self.anchors.read();
        anchors
            .get(handle_hash)
            .cloned()
            .ok_or_else(|| HandleAnchorError::AnchorNotFound {
                handle_hash: hex::encode(handle_hash),
            })
    }

    /// Get ownership anchor by handle string
    pub fn get_anchor_by_handle(&self, handle: &str) -> Result<HandleOwnershipAnchor> {
        let handle_hash = HandleOwnershipAnchor::compute_handle_hash(handle);
        self.get_anchor(&handle_hash)
    }

    /// List all handles owned by a public key
    pub fn list_owner_handles(&self, owner: &[u8; 32]) -> Vec<[u8; 32]> {
        let owner_map = self.owner_to_handles.read();
        owner_map.get(owner).cloned().unwrap_or_default()
    }

    /// Create ownership proof for a handle
    pub fn create_ownership_proof(&self, handle: &str) -> Result<HandleOwnershipProof> {
        let anchor = self.get_anchor_by_handle(handle)?;

        // In production, this would create a real merkle proof
        // For now, just return a placeholder
        let merkle_proof = vec![];
        let state_root = [0u8; 32];

        Ok(HandleOwnershipProof {
            anchor,
            merkle_proof,
            state_root,
        })
    }

    /// Verify ownership proof
    pub fn verify_ownership_proof(&self, proof: &HandleOwnershipProof) -> bool {
        proof.verify()
    }

    /// Get all anchors (for testing/debugging)
    pub fn get_all_anchors(&self) -> Vec<HandleOwnershipAnchor> {
        let anchors = self.anchors.read();
        anchors.values().cloned().collect()
    }

    /// Remove expired anchors
    pub fn cleanup_expired(&self) -> usize {
        let mut removed = 0;

        // Find expired anchors
        let expired_hashes: Vec<[u8; 32]> = {
            let anchors = self.anchors.read();
            anchors
                .iter()
                .filter(|(_, anchor)| anchor.is_expired())
                .map(|(hash, _)| *hash)
                .collect()
        };

        // Remove expired anchors
        {
            let mut anchors = self.anchors.write();
            let mut owner_map = self.owner_to_handles.write();

            for hash in expired_hashes {
                if let Some(anchor) = anchors.remove(&hash) {
                    // Remove from owner mapping
                    if let Some(handles) = owner_map.get_mut(&anchor.owner) {
                        handles.retain(|h| h != &hash);
                    }
                    removed += 1;
                }
            }
        }

        removed
    }
}

impl Default for L1HandleAnchorStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_storage() {
        let storage = L1HandleAnchorStorage::new();
        let handle = "@test.ipn";
        let owner = [1u8; 32];
        let l2_location = [2u8; 32];

        let anchor =
            HandleOwnershipAnchor::new(handle, owner, l2_location, 100, 50, vec![1, 2, 3, 4]);

        assert!(storage.store_anchor(anchor.clone()).is_ok());
        assert_eq!(storage.get_anchor_by_handle(handle).unwrap(), anchor);
    }

    #[test]
    fn test_ownership_proof() {
        let storage = L1HandleAnchorStorage::new();
        let handle = "@test.ipn";
        let owner = [1u8; 32];
        let l2_location = [2u8; 32];

        let anchor =
            HandleOwnershipAnchor::new(handle, owner, l2_location, 100, 50, vec![1, 2, 3, 4]);

        storage.store_anchor(anchor).unwrap();

        let proof = storage.create_ownership_proof(handle).unwrap();
        assert!(storage.verify_ownership_proof(&proof));
    }
}
