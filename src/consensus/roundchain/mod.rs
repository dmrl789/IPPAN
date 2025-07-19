//! zk-STARK integration for IPPAN roundchain
//!
//! This module provides zk-STARK proof generation and verification for rounds,
//! enabling sub-second deterministic finality despite intercontinental latency.

pub mod round_manager;
pub mod zk_prover;
pub mod proof_broadcast;
pub mod tx_verifier;
pub mod test_runner;
pub mod simple_test;

use serde::{Deserialize, Serialize};

/// Custom serialization for byte arrays
mod byte_array_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Option<[u8; 64]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(b) => b.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u8; 64]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Option<Vec<u8>> = Option::deserialize(deserializer)?;
        match bytes {
            Some(b) => {
                if b.len() != 64 {
                    return Err(serde::de::Error::custom("Invalid signature length"));
                }
                let mut signature = [0u8; 64];
                signature.copy_from_slice(&b);
                Ok(Some(signature))
            }
            None => Ok(None),
        }
    }
}

/// Round header containing zk-STARK proof metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundHeader {
    /// Round number
    pub round_number: u64,
    /// Merkle root of all transactions in the round
    pub merkle_root: [u8; 32],
    /// State root after processing all transactions
    pub state_root: [u8; 32],
    /// HashTimer timestamp for the round
    pub hashtimer_timestamp: u64,
    /// Validator ID that produced this round
    pub validator_id: [u8; 32],
    /// Round hash (commitment to all round data)
    pub round_hash: [u8; 32],
    /// Validator signature on the round header
    #[serde(with = "byte_array_serde")]
    pub validator_signature: Option<[u8; 64]>,
}

/// zk-STARK proof for a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkStarkProof {
    /// Proof data (Winterfell or custom STARK format)
    pub proof_data: Vec<u8>,
    /// Proof size in bytes
    pub proof_size: usize,
    /// Proving time in milliseconds
    pub proving_time_ms: u64,
    /// Verification time in milliseconds
    pub verification_time_ms: u64,
    /// Round number this proof covers
    pub round_number: u64,
    /// Number of transactions in the proof
    pub transaction_count: u32,
}

/// Round aggregation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundAggregation {
    /// Round header
    pub header: RoundHeader,
    /// zk-STARK proof
    pub zk_proof: ZkStarkProof,
    /// All transaction hashes in the round
    pub transaction_hashes: Vec<[u8; 32]>,
    /// Merkle tree for transaction inclusion proofs
    pub merkle_tree: MerkleTree,
}

/// Merkle tree for transaction inclusion proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root hash
    pub root: [u8; 32],
    /// Tree height
    pub height: u32,
    /// All nodes in the tree
    pub nodes: Vec<[u8; 32]>,
}

/// Transaction verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionVerification {
    /// Whether the transaction is included
    pub included: bool,
    /// Round number where transaction was included
    pub round: Option<u64>,
    /// HashTimer timestamp
    pub timestamp: Option<u64>,
    /// Merkle inclusion proof
    pub merkle_proof: Option<Vec<[u8; 32]>>,
    /// zk-STARK proof reference
    pub zk_proof_reference: Option<[u8; 32]>,
}

/// Round statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundStats {
    /// Round number
    pub round_number: u64,
    /// Number of blocks in the round
    pub block_count: u32,
    /// Number of transactions in the round
    pub transaction_count: u32,
    /// zk-STARK proof size in bytes
    pub proof_size: usize,
    /// Proving time in milliseconds
    pub proving_time_ms: u64,
    /// Verification time in milliseconds
    pub verification_time_ms: u64,
    /// Global propagation latency in milliseconds
    pub propagation_latency_ms: u64,
}

impl RoundHeader {
    /// Create a new round header
    pub fn new(
        round_number: u64,
        merkle_root: [u8; 32],
        state_root: [u8; 32],
        hashtimer_timestamp: u64,
        validator_id: [u8; 32],
    ) -> Self {
        let round_hash = Self::calculate_round_hash(
            round_number,
            merkle_root,
            state_root,
            hashtimer_timestamp,
            validator_id,
        );

        Self {
            round_number,
            merkle_root,
            state_root,
            hashtimer_timestamp,
            validator_id,
            round_hash,
            validator_signature: None,
        }
    }

    /// Calculate round hash
    fn calculate_round_hash(
        round_number: u64,
        merkle_root: [u8; 32],
        state_root: [u8; 32],
        hashtimer_timestamp: u64,
        validator_id: [u8; 32],
    ) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(round_number.to_le_bytes());
        hasher.update(merkle_root);
        hasher.update(state_root);
        hasher.update(hashtimer_timestamp.to_le_bytes());
        hasher.update(validator_id);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Sign the round header
    pub fn sign(&mut self, _private_key: &[u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement Ed25519 or BLS signature
        // For now, we'll use a placeholder
        self.validator_signature = Some([0u8; 64]);
        Ok(())
    }

    /// Verify the round header signature
    pub fn verify_signature(&self, _public_key: &[u8; 32]) -> bool {
        // TODO: Implement signature verification
        // For now, return true if signature exists
        self.validator_signature.is_some()
    }
}

impl MerkleTree {
    /// Create a new Merkle tree from transaction hashes
    pub fn new(transaction_hashes: Vec<[u8; 32]>) -> Self {
        let mut nodes = transaction_hashes.clone();
        let height = Self::calculate_height(nodes.len());
        
        // Build the tree bottom-up
        let mut level_size = nodes.len();
        let mut level_start = 0;
        
        while level_size > 1 {
            let next_level_size = (level_size + 1) / 2;
            let next_level_start = nodes.len();
            
            for i in 0..next_level_size {
                let left_idx = level_start + i * 2;
                let right_idx = left_idx + 1;
                
                let left_hash = if left_idx < level_start + level_size {
                    nodes[left_idx]
                } else {
                    [0u8; 32]
                };
                
                let right_hash = if right_idx < level_start + level_size {
                    nodes[right_idx]
                } else {
                    left_hash
                };
                
                let parent_hash = Self::hash_pair(left_hash, right_hash);
                nodes.push(parent_hash);
            }
            
            level_start = next_level_start;
            level_size = next_level_size;
        }
        
        let root = if !nodes.is_empty() {
            nodes[nodes.len() - 1]
        } else {
            [0u8; 32]
        };
        
        Self {
            root,
            height,
            nodes,
        }
    }

    /// Generate inclusion proof for a transaction
    pub fn generate_inclusion_proof(&self, transaction_index: usize) -> Option<Vec<[u8; 32]>> {
        let leaf_count = (self.nodes.len() + 1) / 2;
        if transaction_index >= leaf_count {
            return None;
        }
        
        let mut proof = Vec::new();
        let mut index = transaction_index;
        let mut level_start = 0;
        let mut level_size = leaf_count;
        
        while level_size > 1 {
            let sibling_index = if index % 2 == 0 {
                index + 1
            } else {
                index - 1
            };
            
            if sibling_index < level_start + level_size {
                let sibling_pos = level_start + sibling_index;
                if sibling_pos < self.nodes.len() {
                    proof.push(self.nodes[sibling_pos]);
                }
            }
            
            index = index / 2;
            level_start += level_size;
            level_size = (level_size + 1) / 2;
        }
        
        Some(proof)
    }

    /// Verify inclusion proof
    pub fn verify_inclusion_proof(
        &self,
        transaction_hash: [u8; 32],
        proof: &[[u8; 32]],
        index: usize,
    ) -> bool {
        let mut current_hash = transaction_hash;
        let mut current_index = index;
        
        for proof_hash in proof {
            if current_index % 2 == 0 {
                current_hash = Self::hash_pair(current_hash, *proof_hash);
            } else {
                current_hash = Self::hash_pair(*proof_hash, current_hash);
            }
            current_index = current_index / 2;
        }
        
        current_hash == self.root
    }

    /// Hash a pair of hashes
    fn hash_pair(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(left);
        hasher.update(right);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Calculate tree height
    fn calculate_height(leaf_count: usize) -> u32 {
        if leaf_count == 0 {
            0
        } else {
            (leaf_count as f64).log2().ceil() as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_header_creation() {
        let header = RoundHeader::new(
            1,
            [1u8; 32],
            [2u8; 32],
            1234567890,
            [3u8; 32],
        );
        
        assert_eq!(header.round_number, 1);
        assert_eq!(header.merkle_root, [1u8; 32]);
        assert_eq!(header.state_root, [2u8; 32]);
    }

    #[test]
    fn test_merkle_tree_creation() {
        let hashes = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];
        
        let tree = MerkleTree::new(hashes);
        assert_eq!(tree.height, 2);
        assert!(!tree.nodes.is_empty());
    }

    #[test]
    fn test_merkle_inclusion_proof() {
        let hashes = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];
        
        let tree = MerkleTree::new(hashes);
        let proof = tree.generate_inclusion_proof(0).unwrap();
        
        assert!(tree.verify_inclusion_proof([1u8; 32], &proof, 0));
    }
} 