//! Merkle tree implementations for IPPAN
//!
//! Provides Merkle tree and Sparse Merkle tree functionality for data integrity.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merkle tree error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerkleError {
    EmptyTree,
    InvalidIndex,
    InvalidProof,
    HashMismatch,
    InvalidLeafCount,
}

impl std::fmt::Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MerkleError::EmptyTree => write!(f, "Cannot operate on empty tree"),
            MerkleError::InvalidIndex => write!(f, "Invalid leaf index"),
            MerkleError::InvalidProof => write!(f, "Invalid Merkle proof"),
            MerkleError::HashMismatch => write!(f, "Hash mismatch in proof verification"),
            MerkleError::InvalidLeafCount => write!(f, "Invalid number of leaves"),
        }
    }
}

impl std::error::Error for MerkleError {}

/// Merkle tree implementation
pub struct MerkleTree {
    leaves: Vec<Vec<u8>>,
    tree: Vec<Vec<u8>>,
    root: Option<Vec<u8>>,
}

impl MerkleTree {
    /// Create a new Merkle tree from leaves
    pub fn new(leaves: Vec<Vec<u8>>) -> Result<Self> {
        if leaves.is_empty() {
            return Err(anyhow!(MerkleError::EmptyTree));
        }

        let mut tree = MerkleTree {
            leaves: leaves.clone(),
            tree: Vec::new(),
            root: None,
        };

        tree.build_tree()?;
        Ok(tree)
    }

    /// Build the Merkle tree
    fn build_tree(&mut self) -> Result<()> {
        let mut current_level = self.leaves.clone();
        self.tree = current_level.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    &current_level[i] // Duplicate last element if odd number
                };

                let combined = [left.as_slice(), right.as_slice()].concat();
                let hash = self.hash(&combined);
                next_level.push(hash);
            }

            self.tree.extend(next_level.clone());
            current_level = next_level;
        }

        self.root = current_level.first().cloned();
        Ok(())
    }

    /// Get the Merkle root
    pub fn root(&self) -> Option<&Vec<u8>> {
        self.root.as_ref()
    }

    /// Get the number of leaves
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Get a leaf by index
    pub fn get_leaf(&self, index: usize) -> Result<&Vec<u8>> {
        self.leaves
            .get(index)
            .ok_or_else(|| anyhow!(MerkleError::InvalidIndex))
    }

    /// Generate a Merkle proof for a leaf
    pub fn generate_proof(&self, leaf_index: usize) -> Result<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        let mut proof = MerkleProof {
            leaf_index,
            path: Vec::new(),
            leaf_hash: self.leaves[leaf_index].clone(),
        };

        let mut current_index = leaf_index;
        let mut level_size = self.leaves.len();
        let mut level_start = 0;

        while level_size > 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < level_size {
                let sibling_hash = self.tree[level_start + sibling_index].clone();
                proof.path.push(sibling_hash);
            }

            current_index /= 2;
            level_start += level_size;
            level_size = (level_size + 1) / 2;
        }

        Ok(proof)
    }

    /// Verify a Merkle proof
    pub fn verify_proof(&self, proof: &MerkleProof) -> Result<bool> {
        if proof.leaf_index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        let mut current_hash = proof.leaf_hash.clone();
        let mut current_index = proof.leaf_index;

        for sibling_hash in &proof.path {
            let combined = if current_index % 2 == 0 {
                [current_hash.as_slice(), sibling_hash.as_slice()].concat()
            } else {
                [sibling_hash.as_slice(), current_hash.as_slice()].concat()
            };

            current_hash = self.hash(&combined);
            current_index /= 2;
        }

        Ok(current_hash == *self.root.as_ref().unwrap())
    }

    /// Update a leaf and rebuild the tree
    pub fn update_leaf(&mut self, index: usize, new_leaf: Vec<u8>) -> Result<()> {
        if index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        self.leaves[index] = new_leaf;
        self.build_tree()?;
        Ok(())
    }

    /// Add a new leaf to the tree
    pub fn add_leaf(&mut self, leaf: Vec<u8>) -> Result<()> {
        self.leaves.push(leaf);
        self.build_tree()?;
        Ok(())
    }

    /// Remove a leaf from the tree
    pub fn remove_leaf(&mut self, index: usize) -> Result<()> {
        if index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        self.leaves.remove(index);
        if self.leaves.is_empty() {
            self.tree.clear();
            self.root = None;
        } else {
            self.build_tree()?;
        }
        Ok(())
    }

    /// Get the tree structure for debugging
    pub fn tree_structure(&self) -> &Vec<Vec<u8>> {
        &self.tree
    }

    /// Get the height of the tree
    pub fn height(&self) -> usize {
        if self.leaves.is_empty() {
            0
        } else {
            (self.leaves.len() as f64).log2().ceil() as usize
        }
    }

    /// Hash function used by the tree
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }
}

/// Merkle proof structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub path: Vec<Vec<u8>>,
    pub leaf_hash: Vec<u8>,
}

/// Sparse Merkle tree implementation
pub struct SparseMerkleTree {
    leaves: HashMap<Vec<u8>, Vec<u8>>,
    root: Vec<u8>,
    depth: usize,
}

impl SparseMerkleTree {
    /// Create a new sparse Merkle tree
    pub fn new(depth: usize) -> Self {
        let empty_hash = Self::empty_hash(depth);
        Self {
            leaves: HashMap::new(),
            root: empty_hash,
            depth,
        }
    }

    /// Set a leaf in the sparse tree
    pub fn set_leaf(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        if key.len() != self.depth / 8 {
            return Err(anyhow!("Invalid key length"));
        }

        self.leaves.insert(key, value);
        self.update_root();
        Ok(())
    }

    /// Get a leaf from the sparse tree
    pub fn get_leaf(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.leaves.get(key)
    }

    /// Remove a leaf from the sparse tree
    pub fn remove_leaf(&mut self, key: &[u8]) -> Result<()> {
        self.leaves.remove(key);
        self.update_root();
        Ok(())
    }

    /// Compute the root of the sparse tree
    pub fn compute_root(&self) -> Vec<u8> {
        self.root.clone()
    }

    /// Get the current root
    pub fn root(&self) -> &Vec<u8> {
        &self.root
    }

    /// Update the root after changes
    fn update_root(&mut self) {
        self.root = self.compute_root_recursive(&Vec::new(), 0);
    }

    /// Recursively compute the root
    fn compute_root_recursive(&self, key: &[u8], level: usize) -> Vec<u8> {
        if level == self.depth {
            return self
                .leaves
                .get(key)
                .map(|v| self.hash(v))
                .unwrap_or_else(|| Self::empty_hash(0));
        }

        let left_key = [key, &[0]].concat();
        let right_key = [key, &[1]].concat();

        let left_hash = self.compute_root_recursive(&left_key, level + 1);
        let right_hash = self.compute_root_recursive(&right_key, level + 1);

        let combined = [left_hash.as_slice(), right_hash.as_slice()].concat();
        self.hash(&combined)
    }

    /// Compute empty hash for a given level
    fn empty_hash(level: usize) -> Vec<u8> {
        if level == 0 {
            vec![0u8; 32] // Empty leaf hash
        } else {
            let child_hash = Self::empty_hash(level - 1);
            let combined = [child_hash.as_slice(), child_hash.as_slice()].concat();
            Self::hash_static(&combined)
        }
    }

    /// Static hash function
    fn hash_static(data: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// Hash function
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        Self::hash_static(data)
    }
}

/// Merkle tree builder for complex trees
pub struct MerkleTreeBuilder {
    leaves: Vec<Vec<u8>>,
}

impl MerkleTreeBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }

    /// Add a leaf to the builder
    pub fn add_leaf(mut self, leaf: Vec<u8>) -> Self {
        self.leaves.push(leaf);
        self
    }

    /// Add multiple leaves to the builder
    pub fn add_leaves(mut self, leaves: Vec<Vec<u8>>) -> Self {
        self.leaves.extend(leaves);
        self
    }

    /// Build the Merkle tree
    pub fn build(self) -> Result<MerkleTree> {
        MerkleTree::new(self.leaves)
    }
}

impl Default for MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_creation() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];

        let tree = MerkleTree::new(leaves).unwrap();
        assert_eq!(tree.leaf_count(), 4);
        assert!(tree.root().is_some());
    }

    #[test]
    fn test_merkle_proof_generation() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];

        let tree = MerkleTree::new(leaves).unwrap();
        let proof = tree.generate_proof(0).unwrap();

        assert_eq!(proof.leaf_index, 0);
        assert_eq!(proof.leaf_hash, b"leaf1".to_vec());
    }

    #[test]
    fn test_merkle_proof_verification() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];

        let tree = MerkleTree::new(leaves).unwrap();
        let proof = tree.generate_proof(0).unwrap();
        let is_valid = tree.verify_proof(&proof).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_sparse_merkle_tree() {
        let mut tree = SparseMerkleTree::new(256);

        tree.set_leaf(vec![1, 2, 3, 4], b"value1".to_vec()).unwrap();
        tree.set_leaf(vec![5, 6, 7, 8], b"value2".to_vec()).unwrap();

        assert_eq!(tree.get_leaf(&[1, 2, 3, 4]), Some(&b"value1".to_vec()));
        assert_eq!(tree.get_leaf(&[5, 6, 7, 8]), Some(&b"value2".to_vec()));
    }

    #[test]
    fn test_merkle_tree_builder() {
        let tree = MerkleTreeBuilder::new()
            .add_leaf(b"leaf1".to_vec())
            .add_leaf(b"leaf2".to_vec())
            .add_leaves(vec![b"leaf3".to_vec(), b"leaf4".to_vec()])
            .build()
            .unwrap();

        assert_eq!(tree.leaf_count(), 4);
    }
}
