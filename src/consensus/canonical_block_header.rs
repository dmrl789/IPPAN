//! Canonical Block Header implementation for IPPAN
//!
//! This module implements the canonical block header encoding as specified in the
//! IPPAN HashTimer v1 specification, ensuring parents are cryptographically committed.

use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::error::IppanError;

/// Domain separator for block header versioning
pub const BLOCK_HEADER_TAG: [u8; 16] = *b"IPPAN-BHdr-v1___"; // 16 bytes fixed

/// Canonical block header structure for v1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeaderV1 {
    /// Round number
    pub round: u64,
    /// Sequence number within round
    pub seq: u32,
    /// Producer node ID (16 bytes)
    pub producer_node_id: [u8; 16],
    /// Parent block hashes (1-8 parents, sorted lexicographically)
    pub parents: Vec<[u8; 32]>,
    /// Transaction merkle root (32 bytes)
    pub tx_merkle_root: [u8; 32],
    /// Optional metadata root (32 bytes, zeroes if None)
    pub meta_root: Option<[u8; 32]>,
}

impl BlockHeaderV1 {
    /// Create a new block header
    pub fn new(
        round: u64,
        seq: u32,
        producer_node_id: [u8; 16],
        parents: Vec<[u8; 32]>,
        tx_merkle_root: [u8; 32],
        meta_root: Option<[u8; 32]>,
    ) -> Result<Self> {
        // Validate parent count
        if parents.len() > 8 {
            return Err(IppanError::InvalidInput("Too many parents (max 8)".to_string()));
        }
        
        // Sort parents lexicographically for canonical ordering
        let mut sorted_parents = parents;
        sorted_parents.sort_unstable();

        Ok(Self {
            round,
            seq,
            producer_node_id,
            parents: sorted_parents,
            tx_merkle_root,
            meta_root,
        })
    }

    /// Encode the block header into canonical bytes
    pub fn encode(&self) -> Vec<u8> {
        let parent_count = self.parents.len();
        
        let mut buf = Vec::with_capacity(
            16 + // BLOCK_HEADER_TAG
            8 +  // round
            4 +  // seq
            16 + // producer_node_id
            1 +  // parent_count
            parent_count * 32 + // parents
            32 + // tx_merkle_root
            32   // meta_root
        );
        
        buf.extend_from_slice(&BLOCK_HEADER_TAG);
        buf.extend_from_slice(&self.round.to_le_bytes());
        buf.extend_from_slice(&self.seq.to_le_bytes());
        buf.extend_from_slice(&self.producer_node_id);
        buf.push(parent_count as u8);
        
        for parent in &self.parents {
            buf.extend_from_slice(parent);
        }
        
        buf.extend_from_slice(&self.tx_merkle_root);
        buf.extend_from_slice(&self.meta_root.unwrap_or([0u8; 32]));
        
        buf
    }

    /// Calculate the payload digest for this block header
    pub fn payload_digest(&self) -> [u8; 32] {
        let bytes = self.encode();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&result);
        digest
    }

    /// Get the number of parents
    pub fn parent_count(&self) -> usize {
        self.parents.len()
    }

    /// Check if this is a genesis block (no parents)
    pub fn is_genesis(&self) -> bool {
        self.parents.is_empty()
    }
}

/// Parent reference for validation
#[derive(Debug, Clone)]
pub struct ParentRef {
    pub hash: [u8; 32],
    pub round: u64,
}

/// Validation rules for block parents
pub fn validate_block_parents(
    round: u64,
    parents: &[ParentRef],
    max_parents: usize,
    get_exists_and_round: &mut dyn FnMut([u8; 32]) -> Option<u64>,
) -> Result<()> {
    use std::collections::HashSet;
    
    // Genesis blocks are allowed to have no parents
    if parents.is_empty() && round == 0 {
        return Ok(());
    }
    
    // Non-genesis blocks must have at least one parent
    if parents.is_empty() {
        return Err(IppanError::InvalidInput("Block must have at least 1 parent (genesis excepted)".to_string()));
    }
    
    // Check parent count limit
    if parents.len() > max_parents {
        return Err(IppanError::InvalidInput(format!("Too many parents (max {})", max_parents)));
    }
    
    // Check for duplicates
    let mut seen = HashSet::with_capacity(parents.len());
    for pr in parents {
        if !seen.insert(pr.hash) {
            return Err(IppanError::InvalidInput("Duplicate parent hash".to_string()));
        }
        
        // Check if parent exists and get its round
        let parent_round = get_exists_and_round(pr.hash)
            .ok_or_else(|| IppanError::InvalidInput(format!("Parent block not found: {}", hex::encode(pr.hash))))?;
        
        // Parent round must be <= current round
        if parent_round > round {
            return Err(IppanError::InvalidInput(format!("Parent round {} must be <= block round {}", parent_round, round)));
        }
    }
    
    Ok(())
}

/// Check for cycles in the block DAG
pub fn check_acyclicity(
    block_hash: [u8; 32],
    parents: &[ParentRef],
    get_ancestors: &mut dyn FnMut([u8; 32]) -> Result<Vec<[u8; 32]>>,
) -> Result<()> {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![block_hash];
    
    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            return Err(IppanError::InvalidInput("Cycle detected in block DAG".to_string()));
        }
        
        // Get ancestors of current block
        let ancestors = get_ancestors(current)?;
        for ancestor in ancestors {
            if !visited.contains(&ancestor) {
                stack.push(ancestor);
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_encoding() {
        let header = BlockHeaderV1::new(
            8784975040,
            1,
            [0x76, 0x68, 0x7a, 0x7e, 0x80, 0x84, 0x9e, 0xa0, 0xc7, 0xc0, 0xa0, 0xd9, 0xf4, 0xd0, 0xa3, 0xb1],
            vec![
                [0xaa; 32],
                [0xbb; 32],
            ],
            [0u8; 32],
            None,
        ).unwrap();

        let encoded = header.encode();
        let digest = header.payload_digest();
        
        // Verify encoding is deterministic
        let encoded2 = header.encode();
        assert_eq!(encoded, encoded2);
        
        // Verify digest is deterministic
        let digest2 = header.payload_digest();
        assert_eq!(digest, digest2);
        
        // Verify parent sorting
        assert_eq!(header.parents[0], [0xaa; 32]);
        assert_eq!(header.parents[1], [0xbb; 32]);
    }

    #[test]
    fn test_genesis_block() {
        let header = BlockHeaderV1::new(
            0,
            1,
            [0x76, 0x68, 0x7a, 0x7e, 0x80, 0x84, 0x9e, 0xa0, 0xc7, 0xc0, 0xa0, 0xd9, 0xf4, 0xd0, 0xa3, 0xb1],
            vec![], // No parents for genesis
            [0u8; 32],
            None,
        ).unwrap();

        assert!(header.is_genesis());
        assert_eq!(header.parent_count(), 0);
    }

    #[test]
    fn test_parent_validation() {
        let mut block_db = std::collections::HashMap::new();
        block_db.insert([0xaa; 32], 8784975037);
        block_db.insert([0xbb; 32], 8784975039);
        
        let parents = vec![
            ParentRef { hash: [0xaa; 32], round: 8784975037 },
            ParentRef { hash: [0xbb; 32], round: 8784975039 },
        ];
        
        let mut get_exists_and_round = |hash: [u8; 32]| -> Option<u64> {
            block_db.get(&hash).copied()
        };
        
        // Should pass validation
        validate_block_parents(8784975040, &parents, 8, &mut get_exists_and_round).unwrap();
        
        // Should fail with non-existent parent
        let invalid_parents = vec![
            ParentRef { hash: [0xcc; 32], round: 8784975040 },
        ];
        
        let result = validate_block_parents(8784975040, &invalid_parents, 8, &mut get_exists_and_round);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_many_parents() {
        let header = BlockHeaderV1::new(
            8784975040,
            1,
            [0x76, 0x68, 0x7a, 0x7e, 0x80, 0x84, 0x9e, 0xa0, 0xc7, 0xc0, 0xa0, 0xd9, 0xf4, 0xd0, 0xa3, 0xb1],
            vec![[0u8; 32]; 9], // 9 parents (exceeds limit of 8)
            [0u8; 32],
            None,
        );
        
        assert!(header.is_err());
    }
}
