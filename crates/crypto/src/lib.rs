use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod confidential;
pub mod zk_stark;
pub mod encryption;
pub mod key_management;
pub mod signature_schemes;
pub mod hash_functions;
pub mod merkle_trees;
pub mod commitment_schemes;

pub use confidential::{
    validate_block as validate_confidential_block,
    validate_transaction as validate_confidential_transaction, ConfidentialTransactionError,
};
pub use zk_stark::{generate_fibonacci_proof, verify_fibonacci_proof, StarkProof, StarkProofError};
pub use encryption::{AES256GCM, ChaCha20Poly1305, EncryptionError};
pub use key_management::{KeyManager, KeyStore, KeyDerivation};
pub use signature_schemes::{MultiSig, ThresholdSignature, SchnorrSignature};
pub use hash_functions::{HashFunction, Blake3, SHA256, Keccak256};
pub use merkle_trees::{MerkleTree, MerkleProof, MerkleError};
pub use commitment_schemes::{PedersenCommitment, Commitment, CommitmentError};

/// Cryptographic key pair for IPPAN
#[derive(Debug, Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let mut secret_key = [0u8; 32];
        rng.fill_bytes(&mut secret_key);
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
        OsRng.fill_bytes(&mut nonce);
        nonce
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
