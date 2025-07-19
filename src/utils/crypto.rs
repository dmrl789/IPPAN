//! Cryptographic utilities for IPPAN
//! 
//! This module provides cryptographic functions used throughout the IPPAN codebase.

use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use rand::{Rng, RngCore};


/// Generate a random Ed25519 keypair
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut rng = rand::thread_rng();
    let mut sk_bytes = [0u8; 32];
    rng.fill_bytes(&mut sk_bytes);
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Sign data with Ed25519
pub fn sign_data(signing_key: &SigningKey, data: &[u8]) -> Signature {
    signing_key.sign(data)
}

/// Verify Ed25519 signature
pub fn verify_signature(verifying_key: &VerifyingKey, data: &[u8], signature: &Signature) -> bool {
    verifying_key.verify(data, signature).is_ok()
}

/// Generate a random AES-256-GCM key
pub fn generate_aes_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Generate a random nonce for AES-GCM
pub fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

/// Encrypt data with AES-256-GCM
pub fn encrypt_aes_gcm(key: &[u8; 32], nonce: &[u8; 12], data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce);
    
    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| format!("Encryption failed: {}", e))?;
    Ok(ciphertext)
}

/// Decrypt data with AES-256-GCM
pub fn decrypt_aes_gcm(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce);
    
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    Ok(plaintext)
}

/// Calculate SHA-256 hash
pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.into()
}

/// Calculate double SHA-256 hash
pub fn double_sha256_hash(data: &[u8]) -> [u8; 32] {
    let first_hash = sha256_hash(data);
    sha256_hash(&first_hash)
}

/// Generate a random byte array
pub fn random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Generate a random u64
pub fn random_u64() -> u64 {
    rand::thread_rng().gen()
}

/// Generate a random u32
pub fn random_u32() -> u32 {
    rand::thread_rng().gen()
}

/// Create a Merkle tree from a list of hashes
pub fn create_merkle_tree(hashes: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
    if hashes.is_empty() {
        return vec![];
    }
    
    let mut tree = vec![hashes.to_vec()];
    let mut current_level = hashes.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        for chunk in current_level.chunks(2) {
            let mut combined = Vec::new();
            combined.extend_from_slice(&chunk[0]);
            
            if chunk.len() > 1 {
                combined.extend_from_slice(&chunk[1]);
            } else {
                // Duplicate the last element if odd number
                combined.extend_from_slice(&chunk[0]);
            }
            
            let hash = sha256_hash(&combined);
            next_level.push(hash);
        }
        
        tree.push(next_level.clone());
        current_level = next_level;
    }
    
    tree
}

/// Get Merkle root from tree
pub fn get_merkle_root(tree: &[Vec<[u8; 32]>]) -> Option<[u8; 32]> {
    tree.last().and_then(|level| level.first().copied())
}

/// Generate Merkle proof for a specific leaf
pub fn generate_merkle_proof(tree: &[Vec<[u8; 32]>], leaf_index: usize) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let mut current_index = leaf_index;
    
    for level in tree.iter().take(tree.len() - 1) {
        let sibling_index = if current_index % 2 == 0 {
            current_index + 1
        } else {
            current_index - 1
        };
        
        if sibling_index < level.len() {
            proof.push(level[sibling_index]);
        }
        
        current_index /= 2;
    }
    
    proof
}

/// Verify Merkle proof
pub fn verify_merkle_proof(
    leaf_hash: &[u8; 32],
    proof: &[[u8; 32]],
    root: &[u8; 32],
    leaf_index: usize,
) -> bool {
    let mut current_hash = *leaf_hash;
    let mut current_index = leaf_index;
    
    for proof_element in proof {
        let mut combined = Vec::new();
        
        if current_index % 2 == 0 {
            combined.extend_from_slice(&current_hash);
            combined.extend_from_slice(proof_element);
        } else {
            combined.extend_from_slice(proof_element);
            combined.extend_from_slice(&current_hash);
        }
        
        current_hash = sha256_hash(&combined);
        current_index /= 2;
    }
    
    current_hash == *root
}

/// Derive a child key from a parent key using HMAC
pub fn derive_child_key(parent_key: &[u8; 32], child_index: u32) -> [u8; 32] {
    let mut combined = Vec::new();
    combined.extend_from_slice(parent_key);
    combined.extend_from_slice(&child_index.to_le_bytes());
    
    sha256_hash(&combined)
}

/// Create a deterministic but unpredictable value from a seed
pub fn deterministic_random(seed: &[u8]) -> [u8; 32] {
    sha256_hash(seed)
}

/// Hash a string to a fixed-size array
pub fn hash_string(s: &str) -> [u8; 32] {
    sha256_hash(s.as_bytes())
}

/// Create a commitment from data and randomness
pub fn create_commitment(data: &[u8], randomness: &[u8]) -> [u8; 32] {
    let mut combined = Vec::new();
    combined.extend_from_slice(data);
    combined.extend_from_slice(randomness);
    
    sha256_hash(&combined)
}

/// Verify a commitment
pub fn verify_commitment(commitment: &[u8; 32], data: &[u8], randomness: &[u8]) -> bool {
    let computed_commitment = create_commitment(data, randomness);
    commitment == &computed_commitment
}

/// Generate a proof of work for a given difficulty
pub fn generate_proof_of_work(data: &[u8], difficulty: u32) -> (u64, [u8; 32]) {
    let mut nonce = 0u64;
    
    // Use a simpler target calculation to avoid overflow
    let target = if difficulty <= 64 {
        2u64.pow(64 - difficulty)
    } else {
        1u64 // Very high difficulty
    };
    
    loop {
        let mut combined = Vec::new();
        combined.extend_from_slice(data);
        combined.extend_from_slice(&nonce.to_le_bytes());
        
        let hash = sha256_hash(&combined);
        let hash_value = u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7]
        ]);
        
        if hash_value < target {
            return (nonce, hash);
        }
        
        nonce += 1;
    }
}

/// Verify proof of work
pub fn verify_proof_of_work(data: &[u8], nonce: u64, difficulty: u32) -> bool {
    let mut combined = Vec::new();
    combined.extend_from_slice(data);
    combined.extend_from_slice(&nonce.to_le_bytes());
    
    let hash = sha256_hash(&combined);
    let hash_value = u64::from_le_bytes([
        hash[0], hash[1], hash[2], hash[3],
        hash[4], hash[5], hash[6], hash[7]
    ]);
    
    // Use the same target calculation as generate_proof_of_work
    let target = if difficulty <= 64 {
        2u64.pow(64 - difficulty)
    } else {
        1u64 // Very high difficulty
    };
    
    hash_value < target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let (signing_key, verifying_key) = generate_keypair();
        let data = b"test data";
        let signature = sign_data(&signing_key, data);
        
        assert!(verify_signature(&verifying_key, data, &signature));
    }

    #[test]
    fn test_aes_encryption() {
        let key = generate_aes_key();
        let nonce = generate_nonce();
        let data = b"secret data";
        
        let encrypted = encrypt_aes_gcm(&key, &nonce, data).unwrap();
        let decrypted = decrypt_aes_gcm(&key, &nonce, &encrypted).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }

    #[test]
    fn test_merkle_tree() {
        let hashes = vec![
            sha256_hash(b"data1"),
            sha256_hash(b"data2"),
            sha256_hash(b"data3"),
            sha256_hash(b"data4"),
        ];
        
        let tree = create_merkle_tree(&hashes);
        let root = get_merkle_root(&tree).unwrap();
        
        // Test proof generation and verification
        let proof = generate_merkle_proof(&tree, 1);
        assert!(verify_merkle_proof(&hashes[1], &proof, &root, 1));
    }

    #[test]
    fn test_proof_of_work() {
        let data = b"test data";
        let difficulty = 8; // Low difficulty for testing
        
        let (nonce, hash) = generate_proof_of_work(data, difficulty);
        assert!(verify_proof_of_work(data, nonce, difficulty));
    }
}
