//! IPPAN Cryptographic Primitives
//!
//! This crate provides essential cryptographic functionality for the IPPAN blockchain.
//! It includes key generation, signing, hashing, and basic encryption capabilities.

use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::Rng;

pub mod hash_functions;
pub mod merkle_trees;
pub mod commitment_schemes;
pub mod confidential;

pub use hash_functions::{HashFunction, Blake3, SHA256, Keccak256, SHA3_256, BLAKE2b};
pub use merkle_trees::{MerkleTree, MerkleProof, MerkleError};
pub use commitment_schemes::{PedersenCommitment, Commitment, CommitmentError};
pub use confidential::{validate_transaction as validate_confidential_transaction, validate_block as validate_confidential_block, ConfidentialTransactionError};

/// Cryptographic key pair for IPPAN
#[derive(Debug, Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let mut secret_key = [0u8; 32];
        rng.fill(&mut secret_key);
        let signing_key = SigningKey::from_bytes(&secret_key);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Get the public key as bytes
    pub fn public_key(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the private key as bytes
    pub fn private_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<()> {
        let sig = Signature::from_bytes(signature);
        self.verifying_key.verify(message, &sig)?;
        Ok(())
    }
}

/// Cryptographic utilities
pub struct CryptoUtils;

impl CryptoUtils {
    /// Hash data using Blake3
    pub fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[0..32]);
        result
    }

    /// Generate a random nonce
    pub fn random_nonce() -> [u8; 32] {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill(&mut nonce);
        nonce
    }

    /// Validate a confidential transaction
    /// This is a placeholder implementation for confidential transaction validation
    pub fn validate_confidential_transaction(transaction_data: &[u8]) -> Result<bool> {
        // Basic validation - in a real implementation, this would:
        // 1. Verify the transaction structure
        // 2. Check cryptographic proofs
        // 3. Validate commitments
        // 4. Verify range proofs
        // 5. Check balance equations
        
        if transaction_data.is_empty() {
            return Ok(false);
        }
        
        // For now, just check that the data is not empty and has a minimum length
        // In production, this would be much more sophisticated
        Ok(transaction_data.len() >= 32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();
        let private_key = keypair.private_key();

        assert_ne!(public_key, [0u8; 32]);
        assert_ne!(private_key, [0u8; 32]);
    }

    #[test]
    fn test_signing_and_verification() {
        let keypair = KeyPair::generate();
        let message = b"Hello, IPPAN!";

        let signature = keypair.sign(message);
        let result = keypair.verify(message, &signature);

        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_function() {
        let data = b"test data";
        let hash1 = CryptoUtils::hash(data);
        let hash2 = CryptoUtils::hash(data);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, [0u8; 32]);
    }
}