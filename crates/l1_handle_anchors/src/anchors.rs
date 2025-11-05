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
        use ippan_crypto::MerkleTree;
        use sha2::{Digest, Sha256};

        let anchor = self.get_anchor_by_handle(handle)?;

        // Build merkle tree from all anchors
        let mut all_anchors = self.get_all_anchors();
        if all_anchors.is_empty() {
            return Err(HandleAnchorError::AnchorNotFound {
                handle_hash: hex::encode(anchor.handle_hash),
            });
        }

        // Sort anchors by handle_hash for deterministic ordering
        all_anchors.sort_by(|a, b| a.handle_hash.cmp(&b.handle_hash));

        // Create leaves: hash of each anchor's critical fields
        let leaves: Vec<Vec<u8>> = all_anchors
            .iter()
            .map(|a| {
                let mut hasher = Sha256::new();
                hasher.update(&a.handle_hash);
                hasher.update(&a.owner);
                hasher.update(&a.l2_location);
                hasher.update(&a.timestamp.to_le_bytes());
                hasher.finalize().as_slice().to_vec()
            })
            .collect();

        // Create merkle tree
        let tree = MerkleTree::new(leaves.clone()).map_err(|e| {
            HandleAnchorError::StorageError(anyhow::anyhow!("Failed to build merkle tree: {}", e))
        })?;

        let state_root_vec = tree.root().ok_or_else(|| {
            HandleAnchorError::StorageError(anyhow::anyhow!("Merkle tree has no root"))
        })?;
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(state_root_vec.as_slice());

        // Find index of our anchor's leaf
        let target_leaf = {
            let mut hasher = Sha256::new();
            hasher.update(&anchor.handle_hash);
            hasher.update(&anchor.owner);
            hasher.update(&anchor.l2_location);
            hasher.update(&anchor.timestamp.to_le_bytes());
            hasher.finalize().as_slice().to_vec()
        };

        let index = leaves
            .iter()
            .position(|l| l == &target_leaf)
            .ok_or_else(|| HandleAnchorError::AnchorNotFound {
                handle_hash: hex::encode(anchor.handle_hash),
            })?;

        // Generate merkle proof
        let proof = tree.generate_proof(index).map_err(|e| {
            HandleAnchorError::StorageError(anyhow::anyhow!("Failed to generate proof: {}", e))
        })?;

        // Convert proof path to fixed-size arrays
        let merkle_proof: Vec<[u8; 32]> = proof
            .path
            .iter()
            .map(|v| {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(v.as_slice());
                arr
            })
            .collect();

        Ok(HandleOwnershipProof {
            anchor,
            leaf_index: proof.leaf_index,
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

#[cfg(test)]
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

    #[test]
    fn test_merkle_proof_with_multiple_anchors() {
        let storage = L1HandleAnchorStorage::new();

        // Create multiple anchors to test different tree positions
        let handles = vec!["@alice.ipn", "@bob.ipn", "@charlie.ipn", "@david.ipn"];

        let owner = [1u8; 32];
        let l2_location = [2u8; 32];

        // Store all anchors with slightly different timestamps to ensure uniqueness
        for (i, handle) in handles.iter().enumerate() {
            let mut anchor = HandleOwnershipAnchor::new(
                handle,
                owner,
                l2_location,
                100 + i as u64, // Different block heights
                50 + i as u64,  // Different rounds
                vec![1, 2, 3, 4],
            );
            storage.store_anchor(anchor).unwrap();
        }

        // Verify proof for each anchor (tests different leaf indices)
        for (i, handle) in handles.iter().enumerate() {
            let proof = storage.create_ownership_proof(handle).unwrap();

            // Debug output
            eprintln!(
                "Handle {}: {}, leaf_index: {}, proof_len: {}",
                i,
                handle,
                proof.leaf_index,
                proof.merkle_proof.len()
            );

            // Verify proof is valid
            assert!(
                storage.verify_ownership_proof(&proof),
                "Proof verification failed for handle: {} (index {}, proof_len {})",
                handle,
                proof.leaf_index,
                proof.merkle_proof.len()
            );

            // Verify proof contains correct data
            assert_eq!(proof.anchor.owner, owner);
            assert!(!proof.merkle_proof.is_empty());
            assert_ne!(proof.state_root, [0u8; 32]);
        }
    }

    #[test]
    fn test_merkle_proof_invalid_modification() {
        let storage = L1HandleAnchorStorage::new();

        let handle = "@test.ipn";
        let owner = [1u8; 32];
        let l2_location = [2u8; 32];

        // Store anchor
        let anchor =
            HandleOwnershipAnchor::new(handle, owner, l2_location, 100, 50, vec![1, 2, 3, 4]);
        storage.store_anchor(anchor.clone()).unwrap();

        // Create valid proof
        let mut proof = storage.create_ownership_proof(handle).unwrap();
        assert!(storage.verify_ownership_proof(&proof));

        // Modify state root - should fail verification
        proof.state_root = [99u8; 32];
        assert!(!storage.verify_ownership_proof(&proof));

        // Restore state root, modify anchor owner - should fail
        let mut proof2 = storage.create_ownership_proof(handle).unwrap();
        proof2.anchor.owner = [99u8; 32];
        assert!(!storage.verify_ownership_proof(&proof2));
    }
}
