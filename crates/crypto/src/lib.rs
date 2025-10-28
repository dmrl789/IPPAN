//! IPPAN Cryptographic Primitives
//!
//! This crate provides essential cryptographic functionality for the IPPAN blockchain.
//! It includes key generation, signing, hashing, and basic encryption capabilities.

use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::Rng;

pub mod commitment_schemes;
pub mod hash_functions;
pub mod merkle_trees;
pub mod confidential;

#[cfg(feature = "stark-verification")]
pub mod zk_stark;

// Re-export modules and types
pub use hash_functions::{Blake3, Keccak256, SHA256, SHA3_256, BLAKE2b, HashFunction};
pub use merkle_trees::{MerkleError, MerkleProof, MerkleTree};
pub use commitment_schemes::{Commitment, CommitmentError, PedersenCommitment};
pub use confidential::{
    validate_block as validate_confidential_block,
    validate_transaction as validate_confidential_transaction,
    ConfidentialTransactionError,
};

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
        result.copy_from_slice(&hash.as_bytes()[..32]);
        result
    }

    /// Generate a random nonce
    pub fn random_nonce() -> [u8; 32] {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill(&mut nonce);
        nonce
    }

    /// Basic confidential transaction validator placeholder
    pub fn validate_confidential_transaction(transaction_data: &[u8]) -> Result<bool> {
        // Placeholder: real version checks commitments and range proofs
        if transaction_data.is_empty() {
            return Ok(false);
        }
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
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, IPPAN!";
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_blake3_hash_consistency() {
        let data = b"test data";
        let h1 = CryptoUtils::hash(data);
        let h2 = CryptoUtils::hash(data);
        assert_eq!(h1, h2);
        assert_ne!(h1, [0u8; 32]);
    }

    #[test]
    fn test_random_nonce_uniqueness() {
        let n1 = CryptoUtils::random_nonce();
        let n2 = CryptoUtils::random_nonce();
        assert_ne!(n1, n2);
    }

    #[test]
    fn test_confidential_transaction_placeholder() {
        assert!(!CryptoUtils::validate_confidential_transaction(&[]).unwrap());
        assert!(CryptoUtils::validate_confidential_transaction(&[1u8; 64]).unwrap());
    }
}
