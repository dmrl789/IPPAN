use crate::{Error, Result, crypto::Hash};

/// Compute merkle root from transaction IDs using Blake3
pub fn compute_merkle_root(tx_ids: &[Hash]) -> Result<Hash> {
    if tx_ids.is_empty() {
        return Err(Error::Validation("Cannot compute merkle root from empty list".to_string()));
    }
    
    if tx_ids.len() == 1 {
        return Ok(tx_ids[0]);
    }
    
    // Build the merkle tree bottom-up
    let mut current_level: Vec<Hash> = tx_ids.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        // Process pairs of hashes
        for chunk in current_level.chunks(2) {
            let mut combined = Vec::new();
            combined.extend_from_slice(&chunk[0]);
            
            if chunk.len() == 2 {
                combined.extend_from_slice(&chunk[1]);
            } else {
                // Odd number of elements, duplicate the last one
                combined.extend_from_slice(&chunk[0]);
            }
            
            let hash = crate::crypto::blake3_hash(&combined);
            next_level.push(hash);
        }
        
        current_level = next_level;
    }
    
    Ok(current_level[0])
}

/// Compute merkle root with proof
pub fn compute_merkle_root_with_proof(tx_ids: &[Hash], target_index: usize) -> Result<(Hash, Vec<Hash>)> {
    if tx_ids.is_empty() {
        return Err(Error::Validation("Cannot compute merkle root from empty list".to_string()));
    }
    
    if target_index >= tx_ids.len() {
        return Err(Error::Validation("Target index out of bounds".to_string()));
    }
    
    let mut proof = Vec::new();
    let mut current_level: Vec<Hash> = tx_ids.to_vec();
    let mut current_index = target_index;
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        let next_index = current_index / 2;
        
        for chunk in current_level.chunks(2) {
            let mut combined = Vec::new();
            combined.extend_from_slice(&chunk[0]);
            
            if chunk.len() == 2 {
                combined.extend_from_slice(&chunk[1]);
            } else {
                combined.extend_from_slice(&chunk[0]);
            }
            
            let hash = crate::crypto::blake3_hash(&combined);
            next_level.push(hash);
        }
        
        // Add sibling to proof
        if current_index % 2 == 0 {
            if current_index + 1 < current_level.len() {
                proof.push(current_level[current_index + 1]);
            } else {
                proof.push(current_level[current_index]);
            }
        } else {
            proof.push(current_level[current_index - 1]);
        }
        
        current_level = next_level;
        current_index = next_index;
    }
    
    Ok((current_level[0], proof))
}

/// Verify merkle proof
pub fn verify_merkle_proof(root: &Hash, leaf: &Hash, proof: &[Hash], index: usize) -> Result<bool> {
    let mut current_hash = *leaf;
    let mut current_index = index;
    
    for sibling in proof {
        let mut combined = Vec::new();
        
        if current_index % 2 == 0 {
            combined.extend_from_slice(&current_hash);
            combined.extend_from_slice(sibling);
        } else {
            combined.extend_from_slice(sibling);
            combined.extend_from_slice(&current_hash);
        }
        
        current_hash = crate::crypto::blake3_hash(&combined);
        current_index /= 2;
    }
    
    Ok(current_hash == *root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::blake3_hash;

    #[test]
    fn test_merkle_root_single_element() {
        let tx_id = [1u8; 32];
        let root = compute_merkle_root(&[tx_id]).unwrap();
        assert_eq!(root, tx_id);
    }

    #[test]
    fn test_merkle_root_two_elements() {
        let tx_id1 = [1u8; 32];
        let tx_id2 = [2u8; 32];
        
        let mut combined = Vec::new();
        combined.extend_from_slice(&tx_id1);
        combined.extend_from_slice(&tx_id2);
        let expected = blake3_hash(&combined);
        
        let root = compute_merkle_root(&[tx_id1, tx_id2]).unwrap();
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_three_elements() {
        let tx_ids = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = compute_merkle_root(&tx_ids).unwrap();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_merkle_root_empty() {
        let result = compute_merkle_root(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_proof() {
        let tx_ids = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let target_index = 1;
        
        let (root, proof) = compute_merkle_root_with_proof(&tx_ids, target_index).unwrap();
        
        // Verify the proof
        let is_valid = verify_merkle_proof(&root, &tx_ids[target_index], &proof, target_index).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_merkle_proof_deterministic() {
        let tx_ids = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        
        let root1 = compute_merkle_root(&tx_ids).unwrap();
        let root2 = compute_merkle_root(&tx_ids).unwrap();
        
        assert_eq!(root1, root2);
    }
}
