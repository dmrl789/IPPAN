use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Node hash
    pub hash: [u8; 32],
    /// Left child (if not leaf)
    pub left: Option<Box<MerkleNode>>,
    /// Right child (if not leaf)
    pub right: Option<Box<MerkleNode>>,
    /// Data chunk (if leaf)
    pub data: Option<Vec<u8>>,
    /// Chunk index (if leaf)
    pub index: Option<usize>,
}

/// Merkle proof for a specific chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Chunk index
    pub chunk_index: usize,
    /// Chunk data
    pub chunk_data: Vec<u8>,
    /// Sibling hashes for path to root
    pub siblings: Vec<[u8; 32]>,
    /// Path direction (true = right, false = left)
    pub path: Vec<bool>,
}

/// Proof of storage verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResult {
    /// Whether the proof is valid
    pub valid: bool,
    /// File hash
    pub file_hash: [u8; 32],
    /// Merkle root
    pub merkle_root: [u8; 32],
    /// Verification timestamp
    pub timestamp: u64,
    /// Error message if invalid
    pub error: Option<String>,
}

/// Proof of storage manager
pub struct ProofOfStorage {
    /// Merkle tree for the file
    merkle_tree: Option<MerkleNode>,
    /// File hash
    file_hash: [u8; 32],
    /// Chunk size in bytes
    chunk_size: usize,
}

impl ProofOfStorage {
    /// Create a new proof of storage instance
    pub fn new(file_hash: [u8; 32], chunk_size: usize) -> Self {
        Self {
            merkle_tree: None,
            file_hash,
            chunk_size,
        }
    }

    /// Build Merkle tree from file data
    pub fn build_tree(&mut self, file_data: &[u8]) -> Result<[u8; 32]> {
        let chunks = self.chunk_data(file_data);
        let leaves = self.create_leaves(&chunks);
        let root = self.build_tree_from_leaves(&leaves)?;
        self.merkle_tree = Some(root.clone());
        
        Ok(root.hash)
    }

    /// Generate proof for a specific chunk
    pub fn generate_proof(&self, chunk_index: usize) -> Result<MerkleProof> {
        let tree = self.merkle_tree.as_ref()
            .ok_or_else(|| crate::IppanError::Storage("Merkle tree not built".to_string()))?;
        
        let mut proof = MerkleProof {
            chunk_index,
            chunk_data: Vec::new(),
            siblings: Vec::new(),
            path: Vec::new(),
        };
        
        self.generate_proof_recursive(tree, chunk_index, &mut proof)?;
        Ok(proof)
    }

    /// Verify a Merkle proof
    pub fn verify_proof(&self, proof: &MerkleProof, expected_root: &[u8; 32]) -> Result<bool> {
        let mut current_hash = self.hash_chunk(&proof.chunk_data);
        
        for (i, &sibling_hash) in proof.siblings.iter().enumerate() {
            let is_right = proof.path.get(i).copied().unwrap_or(false);
            
            if is_right {
                // Current is left child
                let mut hasher = Sha256::new();
                hasher.update(&current_hash);
                hasher.update(&sibling_hash);
                current_hash = hasher.finalize().into();
            } else {
                // Current is right child
                let mut hasher = Sha256::new();
                hasher.update(&sibling_hash);
                hasher.update(&current_hash);
                current_hash = hasher.finalize().into();
            }
        }
        
        Ok(current_hash == *expected_root)
    }

    /// Perform spot check verification
    pub fn spot_check(&self, chunks: &[Vec<u8>], sample_indices: &[usize]) -> Result<Vec<ProofResult>> {
        let mut results = Vec::new();
        
        for &index in sample_indices {
            if index >= chunks.len() {
                results.push(ProofResult {
                    valid: false,
                    file_hash: self.file_hash,
                    merkle_root: [0; 32],
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    error: Some("Chunk index out of bounds".to_string()),
                });
                continue;
            }
            
            let proof = self.generate_proof(index)?;
            let is_valid = self.verify_proof(&proof, &self.get_root_hash()?)?;
            
            results.push(ProofResult {
                valid: is_valid,
                file_hash: self.file_hash,
                merkle_root: self.get_root_hash()?,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                error: None,
            });
        }
        
        Ok(results)
    }

    /// Get random sample indices for spot checking
    pub fn get_random_samples(&self, total_chunks: usize, sample_count: usize) -> Vec<usize> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut samples = Vec::new();
        
        for _ in 0..sample_count {
            let index = rng.gen_range(0..total_chunks);
            samples.push(index);
        }
        
        samples.sort();
        samples.dedup();
        samples
    }

    /// Chunk data into fixed-size pieces
    fn chunk_data(&self, data: &[u8]) -> Vec<Vec<u8>> {
        data.chunks(self.chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Create leaf nodes from chunks
    fn create_leaves(&self, chunks: &[Vec<u8>]) -> Vec<MerkleNode> {
        chunks.iter().enumerate().map(|(index, chunk)| {
            MerkleNode {
                hash: self.hash_chunk(chunk),
                left: None,
                right: None,
                data: Some(chunk.clone()),
                index: Some(index),
            }
        }).collect()
    }

    /// Build tree from leaves
    fn build_tree_from_leaves(&self, leaves: &[MerkleNode]) -> Result<MerkleNode> {
        if leaves.is_empty() {
            return Err(crate::IppanError::Storage("No leaves provided".to_string()));
        }
        
        if leaves.len() == 1 {
            return Ok(leaves[0].clone());
        }
        
        let mut current_level: Vec<MerkleNode> = leaves.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 1 {
                    next_level.push(chunk[0].clone());
                } else {
                    let left = Box::new(chunk[0].clone());
                    let right = Box::new(chunk[1].clone());
                    
                    let mut hasher = Sha256::new();
                    hasher.update(&left.hash);
                    hasher.update(&right.hash);
                    let hash = hasher.finalize().into();
                    
                    next_level.push(MerkleNode {
                        hash,
                        left: Some(left),
                        right: Some(right),
                        data: None,
                        index: None,
                    });
                }
            }
            
            current_level = next_level;
        }
        
        Ok(current_level[0].clone())
    }

    /// Generate proof recursively
    fn generate_proof_recursive(&self, node: &MerkleNode, target_index: usize, proof: &mut MerkleProof) -> Result<()> {
        if let Some(index) = node.index {
            if index == target_index {
                proof.chunk_data = node.data.clone().unwrap_or_default();
                return Ok(());
            }
        }
        
        if let (Some(left), Some(right)) = (&node.left, &node.right) {
            // Check if target is in left subtree
            if self.index_in_subtree(left, target_index) {
                proof.path.push(false); // Go left
                proof.siblings.push(right.hash);
                self.generate_proof_recursive(left, target_index, proof)?;
            } else {
                proof.path.push(true); // Go right
                proof.siblings.push(left.hash);
                self.generate_proof_recursive(right, target_index, proof)?;
            }
        }
        
        Ok(())
    }

    /// Check if index is in subtree
    fn index_in_subtree(&self, node: &MerkleNode, target_index: usize) -> bool {
        if let Some(index) = node.index {
            return index == target_index;
        }
        
        if let (Some(left), Some(right)) = (&node.left, &node.right) {
            return self.index_in_subtree(left, target_index) || self.index_in_subtree(right, target_index);
        }
        
        false
    }

    /// Hash a chunk
    fn hash_chunk(&self, chunk: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(chunk);
        hasher.finalize().into()
    }

    /// Get root hash
    fn get_root_hash(&self) -> Result<[u8; 32]> {
        self.merkle_tree.as_ref()
            .map(|tree| tree.hash)
            .ok_or_else(|| crate::IppanError::Storage("Merkle tree not built".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_building() {
        let file_hash = [1u8; 32];
        let mut proof = ProofOfStorage::new(file_hash, 1024);
        
        let data = b"This is test data for building a Merkle tree to verify storage proofs in the IPPAN network.";
        let root = proof.build_tree(data).unwrap();
        
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_proof_generation_and_verification() {
        let file_hash = [2u8; 32];
        let mut proof = ProofOfStorage::new(file_hash, 16);
        
        let data = b"Test data for proof verification";
        let root = proof.build_tree(data).unwrap();
        
        let proof_data = proof.generate_proof(0).unwrap();
        let is_valid = proof.verify_proof(&proof_data, &root).unwrap();
        
        assert!(is_valid);
    }

    #[test]
    fn test_spot_checking() {
        let file_hash = [3u8; 32];
        let mut proof = ProofOfStorage::new(file_hash, 8);
        
        let data = b"Data for spot checking";
        proof.build_tree(data).unwrap();
        
        let chunks = proof.chunk_data(data);
        let samples = proof.get_random_samples(chunks.len(), 3);
        let results = proof.spot_check(&chunks, &samples).unwrap();
        
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.valid);
        }
    }
}
