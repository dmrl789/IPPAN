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

// -----------------------------------------------------------------------------
// Re-exports for external crates
// -----------------------------------------------------------------------------
pub use hash_functions::{Blake3, Keccak256, SHA256, SHA3_256, BLAKE2b, HashFunction};
pub use merkle_trees::{MerkleError, MerkleProof, MerkleTree};
pub use commitment_schemes::{Commitment, CommitmentError, PedersenCommitment};
pub use confidential::{
    validate_block as validate_confidential_block,
    validate_transaction as validate_confidential_transaction,
    ConfidentialTransactionError,
};

// -----------------------------------------------------------------------------
// KeyPair structure
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new Ed25519 key pair
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

    /// Get public key bytes
    pub fn public_key(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get private key bytes
    pub fn private_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a message (Ed25519)
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    /// Verify a message signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<()> {
        let sig = Signature::from_bytes(signature);
        self.verifying_key.verify(message, &sig)?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Crypto utilities
// -----------------------------------------------------------------------------
pub struct CryptoUtils;

impl CryptoUtils {
    /// Hash arbitrary data using Blake3
    pub fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[..32]);
        result
    }

    /// Generate random 32-byte nonce
    pub fn random_nonce() -> [u8; 32] {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill(&mut nonce);
        nonce
    }

    /// Placeholder confidential transaction validator
    pub fn validate_confidential_transaction(transaction_data: &[u8]) -> Result<bool> {
        if transaction_data.is_empty() {
            return Ok(false);
        }
        Ok(transaction_data.len() >= 32)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        assert_ne!(kp.public_key(), [0u8; 32]);
        assert_ne!(kp.private_key(), [0u8; 32]);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = KeyPair::generate();
        let msg = b"Hello, IPPAN!";
        let sig = kp.sign(msg);
        assert!(kp.verify(msg, &sig).is_ok());
    }

    #[test]
    fn test_blake3_hash_consistency() {
        let data = b"sample data";
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
    fn test_confidential_tx_placeholder() {
        assert!(!CryptoUtils::validate_confidential_transaction(&[]).unwrap());
        assert!(CryptoUtils::validate_confidential_transaction(&[1u8; 64]).unwrap());
    }
}
