//! Merkle tree implementation for IPPAN
//!
//! Provides Merkle tree construction, proof generation, and verification
//! for efficient data integrity verification.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merkle tree error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerkleError {
    InvalidProof,
    InvalidLeaf,
    InvalidTree,
    EmptyTree,
    InvalidIndex,
}

impl std::fmt::Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MerkleError::InvalidProof => write!(f, "Invalid Merkle proof"),
            MerkleError::InvalidLeaf => write!(f, "Invalid leaf node"),
            MerkleError::InvalidTree => write!(f, "Invalid Merkle tree"),
            MerkleError::EmptyTree => write!(f, "Empty Merkle tree"),
            MerkleError::InvalidIndex => write!(f, "Invalid leaf index"),
        }
    }
}

impl std::error::Error for MerkleError {}

/// Merkle tree implementation
pub struct MerkleTree {
    leaves: Vec<Vec<u8>>,
    tree: Vec<Vec<u8>>,
    root: Option<Vec<u8>>,
    hash_function: Box<dyn Fn(&[u8]) -> Vec<u8>>,
}

impl MerkleTree {
    /// Create a new Merkle tree
    pub fn new(leaves: Vec<Vec<u8>>, hash_function: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Self {
        let mut tree = Self {
            leaves,
            tree: Vec::new(),
            root: None,
            hash_function,
        };
        tree.build_tree();
        tree
    }

    /// Build the Merkle tree
    fn build_tree(&mut self) {
        if self.leaves.is_empty() {
            return;
        }

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

                let mut combined = Vec::new();
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                
                let hash = (self.hash_function)(&combined);
                next_level.push(hash);
            }

            self.tree.extend_from_slice(&next_level);
            current_level = next_level;
        }

        self.root = current_level.first().cloned();
    }

    /// Get the root hash
    pub fn root(&self) -> Result<&[u8]> {
        self.root.as_deref().ok_or_else(|| anyhow!(MerkleError::EmptyTree))
    }

    /// Get the number of leaves
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Get a leaf by index
    pub fn get_leaf(&self, index: usize) -> Result<&[u8]> {
        self.leaves.get(index).map(|v| v.as_slice())
            .ok_or_else(|| anyhow!(MerkleError::InvalidIndex))
    }

    /// Generate a Merkle proof for a leaf
    pub fn generate_proof(&self, leaf_index: usize) -> Result<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        let mut proof = MerkleProof {
            leaf_index,
            leaf_hash: self.leaves[leaf_index].clone(),
            path: Vec::new(),
            root_hash: self.root()?.to_vec(),
        };

        let mut current_index = leaf_index;
        let mut level_start = 0;
        let mut level_size = self.leaves.len();

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
            return Ok(false);
        }

        let mut current_hash = proof.leaf_hash.clone();
        let mut current_index = proof.leaf_index;

        for sibling_hash in &proof.path {
            let mut combined = if current_index % 2 == 0 {
                let mut combined = Vec::new();
                combined.extend_from_slice(&current_hash);
                combined.extend_from_slice(sibling_hash);
                combined
            } else {
                let mut combined = Vec::new();
                combined.extend_from_slice(sibling_hash);
                combined.extend_from_slice(&current_hash);
                combined
            };

            current_hash = (self.hash_function)(&combined);
            current_index /= 2;
        }

        Ok(current_hash == proof.root_hash)
    }

    /// Update a leaf and rebuild the tree
    pub fn update_leaf(&mut self, index: usize, new_leaf: Vec<u8>) -> Result<()> {
        if index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        self.leaves[index] = new_leaf;
        self.tree.clear();
        self.root = None;
        self.build_tree();
        Ok(())
    }

    /// Add a new leaf to the tree
    pub fn add_leaf(&mut self, leaf: Vec<u8>) {
        self.leaves.push(leaf);
        self.tree.clear();
        self.root = None;
        self.build_tree();
    }

    /// Remove a leaf from the tree
    pub fn remove_leaf(&mut self, index: usize) -> Result<Vec<u8>> {
        if index >= self.leaves.len() {
            return Err(anyhow!(MerkleError::InvalidIndex));
        }

        let removed_leaf = self.leaves.remove(index);
        self.tree.clear();
        self.root = None;
        self.build_tree();
        Ok(removed_leaf)
    }

    /// Get the tree structure for debugging
    pub fn tree_structure(&self) -> Vec<Vec<u8>> {
        self.tree.clone()
    }

    /// Get the height of the tree
    pub fn height(&self) -> usize {
        if self.leaves.is_empty() {
            0
        } else {
            (self.leaves.len() as f64).log2().ceil() as usize
        }
    }
}

/// Merkle proof structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub leaf_hash: Vec<u8>,
    pub path: Vec<Vec<u8>>,
    pub root_hash: Vec<u8>,
}

impl MerkleProof {
    /// Create a new Merkle proof
    pub fn new(leaf_index: usize, leaf_hash: Vec<u8>, path: Vec<Vec<u8>>, root_hash: Vec<u8>) -> Self {
        Self {
            leaf_index,
            leaf_hash,
            path,
            root_hash,
        }
    }

    /// Verify the proof against a root hash
    pub fn verify(&self, root_hash: &[u8]) -> bool {
        self.root_hash == root_hash
    }

    /// Get the proof size
    pub fn size(&self) -> usize {
        self.path.len()
    }
}

/// Sparse Merkle tree implementation
pub struct SparseMerkleTree {
    leaves: HashMap<u64, Vec<u8>>,
    root: Option<Vec<u8>>,
    hash_function: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    default_leaf: Vec<u8>,
}

impl SparseMerkleTree {
    /// Create a new sparse Merkle tree
    pub fn new(hash_function: Box<dyn Fn(&[u8]) -> Vec<u8>>, default_leaf: Vec<u8>) -> Self {
        Self {
            leaves: HashMap::new(),
            root: None,
            hash_function,
            default_leaf,
        }
    }

    /// Set a leaf at a specific index
    pub fn set_leaf(&mut self, index: u64, leaf: Vec<u8>) {
        self.leaves.insert(index, leaf);
        self.root = None;
    }

    /// Get a leaf at a specific index
    pub fn get_leaf(&self, index: u64) -> &[u8] {
        self.leaves.get(&index).map(|v| v.as_slice()).unwrap_or(&self.default_leaf)
    }

    /// Remove a leaf at a specific index
    pub fn remove_leaf(&mut self, index: u64) -> Option<Vec<u8>> {
        let result = self.leaves.remove(&index);
        self.root = None;
        result
    }

    /// Compute the root hash
    pub fn compute_root(&mut self) -> Result<Vec<u8>> {
        if self.leaves.is_empty() {
            return Ok(self.default_leaf.clone());
        }

        let max_index = self.leaves.keys().max().unwrap_or(&0);
        let tree_size = 2_u64.pow((*max_index as f64).log2().ceil() as u32);
        
        let mut current_level = Vec::new();
        for i in 0..tree_size {
            current_level.push(self.get_leaf(i).to_vec());
        }

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for i in (0..current_level.len()).step_by(2) {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    &current_level[i]
                };

                let mut combined = Vec::new();
                combined.extend_from_slice(left);
                combined.extend_from_slice(right);
                
                let hash = (self.hash_function)(&combined);
                next_level.push(hash);
            }

            current_level = next_level;
        }

        self.root = current_level.first().cloned();
        Ok(self.root.as_ref().unwrap().clone())
    }

    /// Get the root hash
    pub fn root(&self) -> Result<&[u8]> {
        self.root.as_deref().ok_or_else(|| anyhow!(MerkleError::EmptyTree))
    }

    /// Generate a proof for a leaf
    pub fn generate_proof(&mut self, index: u64) -> Result<MerkleProof> {
        self.compute_root()?;
        
        let leaf_hash = self.get_leaf(index).to_vec();
        let mut path = Vec::new();
        
        let mut current_index = index;
        let mut tree_size = 2_u64.pow((index as f64).log2().ceil() as u32);
        
        while tree_size > 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < tree_size {
                let sibling_hash = self.get_leaf(sibling_index).to_vec();
                path.push(sibling_hash);
            }

            current_index /= 2;
            tree_size /= 2;
        }

        Ok(MerkleProof {
            leaf_index: index as usize,
            leaf_hash,
            path,
            root_hash: self.root()?.to_vec(),
        })
    }
}

/// Merkle tree builder
pub struct MerkleTreeBuilder {
    leaves: Vec<Vec<u8>>,
    hash_function: Option<Box<dyn Fn(&[u8]) -> Vec<u8>>>,
}

impl MerkleTreeBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            hash_function: None,
        }
    }

    /// Add a leaf
    pub fn add_leaf(mut self, leaf: Vec<u8>) -> Self {
        self.leaves.push(leaf);
        self
    }

    /// Add multiple leaves
    pub fn add_leaves(mut self, leaves: Vec<Vec<u8>>) -> Self {
        self.leaves.extend(leaves);
        self
    }

    /// Set the hash function
    pub fn with_hash_function(mut self, hash_function: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Self {
        self.hash_function = Some(hash_function);
        self
    }

    /// Build the Merkle tree
    pub fn build(self) -> Result<MerkleTree> {
        let hash_function = self.hash_function.ok_or_else(|| anyhow!("Hash function not set"))?;
        Ok(MerkleTree::new(self.leaves, hash_function))
    }
}

impl Default for MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha256_hash(data: &[u8]) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    #[test]
    fn test_merkle_tree_creation() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];
        
        let tree = MerkleTree::new(leaves, Box::new(sha256_hash));
        assert_eq!(tree.leaf_count(), 4);
        assert!(tree.root().is_ok());
    }

    #[test]
    fn test_merkle_proof_generation() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];
        
        let tree = MerkleTree::new(leaves, Box::new(sha256_hash));
        let proof = tree.generate_proof(0).unwrap();
        
        assert_eq!(proof.leaf_index, 0);
        assert_eq!(proof.leaf_hash, b"leaf1");
        assert!(!proof.path.is_empty());
    }

    #[test]
    fn test_merkle_proof_verification() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];
        
        let tree = MerkleTree::new(leaves, Box::new(sha256_hash));
        let proof = tree.generate_proof(0).unwrap();
        
        assert!(tree.verify_proof(&proof).unwrap());
    }

    #[test]
    fn test_merkle_tree_update() {
        let leaves = vec![
            b"leaf1".to_vec(),
            b"leaf2".to_vec(),
            b"leaf3".to_vec(),
            b"leaf4".to_vec(),
        ];
        
        let mut tree = MerkleTree::new(leaves, Box::new(sha256_hash));
        let original_root = tree.root().unwrap().to_vec();
        
        tree.update_leaf(0, b"new_leaf1".to_vec()).unwrap();
        let new_root = tree.root().unwrap();
        
        assert_ne!(original_root, new_root);
    }

    #[test]
    fn test_sparse_merkle_tree() {
        let mut sparse_tree = SparseMerkleTree::new(
            Box::new(sha256_hash),
            b"default".to_vec()
        );
        
        sparse_tree.set_leaf(5, b"leaf5".to_vec());
        sparse_tree.set_leaf(10, b"leaf10".to_vec());
        
        assert!(sparse_tree.compute_root().is_ok());
        assert_eq!(sparse_tree.get_leaf(5), b"leaf5");
        assert_eq!(sparse_tree.get_leaf(10), b"leaf10");
        assert_eq!(sparse_tree.get_leaf(15), b"default");
    }

    #[test]
    fn test_merkle_tree_builder() {
        let tree = MerkleTreeBuilder::new()
            .add_leaf(b"leaf1".to_vec())
            .add_leaf(b"leaf2".to_vec())
            .add_leaf(b"leaf3".to_vec())
            .with_hash_function(Box::new(sha256_hash))
            .build()
            .unwrap();
        
        assert_eq!(tree.leaf_count(), 3);
        assert!(tree.root().is_ok());
    }
}
