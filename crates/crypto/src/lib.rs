//! IPPAN Cryptographic Primitives
//!
//! This crate provides essential cryptographic functionality for the IPPAN blockchain.
//! It includes key generation, signing, hashing, and basic encryption capabilities.
//!
//! ## Modules
//! - `hash_functions`: Implements multiple hashing algorithms (Blake3, SHA2, Keccak, etc.)
//! - `merkle_trees`: Provides Merkle tree construction and proof verification
//! - `commitment_schemes`: Contains Pedersen and other zero-knowledge commitments
//! - `validators`: Cryptographic validation helpers for confidential blocks and transactions

use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::Rng;
use thiserror::Error;

// -----------------------------------------------------------------------------
// Error Types
// -----------------------------------------------------------------------------

/// Cryptographic operation errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Signature verification failed")]
    VerificationFailed,

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),
}

// -----------------------------------------------------------------------------
// Module exports
// -----------------------------------------------------------------------------
pub mod commitment_schemes;
pub mod hash_functions;
pub mod merkle_trees;
pub mod validators;

pub use commitment_schemes::{Commitment, CommitmentError, PedersenCommitment};
pub use hash_functions::{BLAKE2b, Blake3, HashFunction, Keccak256, SHA256, SHA3_256};
pub use merkle_trees::{MerkleError, MerkleProof, MerkleTree};
pub use validators::{validate_confidential_block, validate_confidential_transaction};

// -----------------------------------------------------------------------------
// Ed25519 Key Management
// -----------------------------------------------------------------------------

/// Cryptographic key pair for IPPAN (Ed25519)
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

    /// Create KeyPair from existing private key bytes
    pub fn from_private_key(private_key: &[u8; 32]) -> Result<Self, CryptoError> {
        let signing_key = SigningKey::from_bytes(private_key);
        let verifying_key = signing_key.verifying_key();
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    /// Create KeyPair from hex-encoded private key
    pub fn from_private_key_hex(hex_key: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(hex_key)
            .map_err(|e| CryptoError::DecodingError(format!("Invalid hex: {e}")))?;

        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                actual: bytes.len(),
            });
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Self::from_private_key(&key_bytes)
    }

    /// Export private key as hex string
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.private_key())
    }

    /// Export public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key())
    }

    /// Sign a message (Ed25519)
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    /// Verify a message signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<(), CryptoError> {
        let sig = Signature::from_bytes(signature);
        self.verifying_key
            .verify(message, &sig)
            .map_err(|_| CryptoError::VerificationFailed)?;
        Ok(())
    }

    /// Verify a signature with a specific public key
    pub fn verify_with_public_key(
        message: &[u8],
        signature: &[u8; 64],
        public_key: &[u8; 32],
    ) -> Result<(), CryptoError> {
        let verifying_key = VerifyingKey::from_bytes(public_key)
            .map_err(|e| CryptoError::InvalidPublicKey(format!("{e}")))?;

        let sig = Signature::from_bytes(signature);
        verifying_key
            .verify(message, &sig)
            .map_err(|_| CryptoError::VerificationFailed)?;
        Ok(())
    }

    /// Generate IPPAN address from public key
    pub fn generate_address(&self) -> String {
        ippan_types::address::encode_address(&self.public_key())
    }
}

// -----------------------------------------------------------------------------
// Crypto Utilities
// -----------------------------------------------------------------------------

/// Utility functions for hashing and randomness
pub struct CryptoUtils;

impl CryptoUtils {
    /// Hash arbitrary data using Blake3 (32-byte output)
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

    /// Validate confidential transaction raw data structure
    ///
    /// Expected format for minimal confidential transaction:
    /// [version(1)] [num_inputs(1)] [commitments(32*n)] [range_proof_len(4)] [range_proof(variable)] [signature(64)]
    ///
    /// This is a lightweight structural validation. Full cryptographic verification
    /// (bulletproofs, etc.) would be done in consensus.
    pub fn validate_confidential_transaction(transaction_data: &[u8]) -> Result<bool> {
        if transaction_data.is_empty() {
            return Ok(false);
        }

        // Minimum size: version(1) + num_inputs(1) + commitment(32) + proof_len(4) + sig(64) = 102 bytes
        if transaction_data.len() < 102 {
            return Ok(false);
        }

        let mut offset = 0;

        // Read version
        let _version = transaction_data[offset];
        offset += 1;

        // Read number of inputs/outputs
        let num_commitments = transaction_data[offset] as usize;
        offset += 1;

        if num_commitments == 0 || num_commitments > 255 {
            return Ok(false);
        }

        // Validate commitments (each is 32 bytes)
        let commitments_size = num_commitments * 32;
        if offset + commitments_size > transaction_data.len() {
            return Ok(false);
        }

        // Check each commitment is a valid (non-zero) point
        for _i in 0..num_commitments {
            let commitment = &transaction_data[offset..offset + 32];
            if commitment == [0u8; 32] {
                // All-zero commitment is invalid
                return Ok(false);
            }
            offset += 32;
        }

        // Read range proof length
        if offset + 4 > transaction_data.len() {
            return Ok(false);
        }
        let proof_len = u32::from_le_bytes([
            transaction_data[offset],
            transaction_data[offset + 1],
            transaction_data[offset + 2],
            transaction_data[offset + 3],
        ]) as usize;
        offset += 4;

        // Validate proof exists and has reasonable size
        if proof_len == 0 || proof_len > 10000 {
            // Range proof should be present and not unreasonably large
            return Ok(false);
        }

        if offset + proof_len > transaction_data.len() {
            return Ok(false);
        }
        offset += proof_len;

        // Validate signature (64 bytes for Ed25519)
        if offset + 64 != transaction_data.len() {
            return Ok(false);
        }

        let signature = &transaction_data[offset..offset + 64];
        // Check signature is not all zeros
        if signature == [0u8; 64] {
            return Ok(false);
        }

        // All structural checks passed
        Ok(true)
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
    fn test_keypair_from_private_key() {
        let kp1 = KeyPair::generate();
        let private_key = kp1.private_key();

        let kp2 = KeyPair::from_private_key(&private_key).unwrap();
        assert_eq!(kp1.public_key(), kp2.public_key());
        assert_eq!(kp1.private_key(), kp2.private_key());
    }

    #[test]
    fn test_keypair_from_hex() {
        let kp1 = KeyPair::generate();
        let hex_key = kp1.private_key_hex();

        let kp2 = KeyPair::from_private_key_hex(&hex_key).unwrap();
        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    fn test_invalid_hex_key() {
        let result = KeyPair::from_private_key_hex("invalid_hex");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CryptoError::DecodingError(_)));
    }

    #[test]
    fn test_invalid_key_length() {
        let result = KeyPair::from_private_key_hex("aabbcc");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::InvalidKeyLength { .. }
        ));
    }

    #[test]
    fn test_verify_with_public_key() {
        let kp = KeyPair::generate();
        let msg = b"Test message";
        let sig = kp.sign(msg);
        let pubkey = kp.public_key();

        assert!(KeyPair::verify_with_public_key(msg, &sig, &pubkey).is_ok());
    }

    #[test]
    fn test_verify_invalid_signature() {
        let kp = KeyPair::generate();
        let msg = b"Test message";
        let invalid_sig = [0u8; 64];

        let result = kp.verify(msg, &invalid_sig);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::VerificationFailed
        ));
    }

    #[test]
    fn test_generate_address() {
        let kp = KeyPair::generate();
        let address = kp.generate_address();

        // Base58Check addresses should not be empty and should be valid
        assert!(!address.is_empty());
        assert!(ippan_types::address::is_valid_address(&address));
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
    fn test_confidential_tx_validation() {
        // Empty transaction should fail
        assert!(!CryptoUtils::validate_confidential_transaction(&[]).unwrap());

        // Too short transaction should fail
        assert!(!CryptoUtils::validate_confidential_transaction(&[1u8; 50]).unwrap());

        // Valid minimal transaction: version + 1 commitment + proof + signature
        let mut valid_tx = Vec::new();
        valid_tx.push(1); // version
        valid_tx.push(1); // 1 commitment
        valid_tx.extend_from_slice(&[1u8; 32]); // commitment (non-zero)
        valid_tx.extend_from_slice(&100u32.to_le_bytes()); // proof length = 100
        valid_tx.extend_from_slice(&[2u8; 100]); // proof data
        valid_tx.extend_from_slice(&[3u8; 64]); // signature (non-zero)

        assert!(CryptoUtils::validate_confidential_transaction(&valid_tx).unwrap());
    }

    #[test]
    fn test_confidential_tx_validation_zero_commitment() {
        // Transaction with all-zero commitment should fail
        let mut invalid_tx = Vec::new();
        invalid_tx.push(1); // version
        invalid_tx.push(1); // 1 commitment
        invalid_tx.extend_from_slice(&[0u8; 32]); // all-zero commitment (invalid)
        invalid_tx.extend_from_slice(&100u32.to_le_bytes()); // proof length
        invalid_tx.extend_from_slice(&[2u8; 100]); // proof
        invalid_tx.extend_from_slice(&[3u8; 64]); // signature

        assert!(!CryptoUtils::validate_confidential_transaction(&invalid_tx).unwrap());
    }

    #[test]
    fn test_confidential_tx_validation_zero_signature() {
        // Transaction with all-zero signature should fail
        let mut invalid_tx = Vec::new();
        invalid_tx.push(1); // version
        invalid_tx.push(1); // 1 commitment
        invalid_tx.extend_from_slice(&[1u8; 32]); // commitment
        invalid_tx.extend_from_slice(&100u32.to_le_bytes()); // proof length
        invalid_tx.extend_from_slice(&[2u8; 100]); // proof
        invalid_tx.extend_from_slice(&[0u8; 64]); // all-zero signature (invalid)

        assert!(!CryptoUtils::validate_confidential_transaction(&invalid_tx).unwrap());
    }

    #[test]
    fn test_confidential_tx_validation_multiple_commitments() {
        // Valid transaction with multiple commitments
        let mut valid_tx = Vec::new();
        valid_tx.push(1); // version
        valid_tx.push(3); // 3 commitments
        valid_tx.extend_from_slice(&[1u8; 32]); // commitment 1
        valid_tx.extend_from_slice(&[2u8; 32]); // commitment 2
        valid_tx.extend_from_slice(&[3u8; 32]); // commitment 3
        valid_tx.extend_from_slice(&200u32.to_le_bytes()); // proof length
        valid_tx.extend_from_slice(&[4u8; 200]); // proof
        valid_tx.extend_from_slice(&[5u8; 64]); // signature

        assert!(CryptoUtils::validate_confidential_transaction(&valid_tx).unwrap());
    }

    #[test]
    fn test_confidential_tx_validation_invalid_proof_size() {
        // Proof too large
        let mut invalid_tx = Vec::new();
        invalid_tx.push(1); // version
        invalid_tx.push(1); // 1 commitment
        invalid_tx.extend_from_slice(&[1u8; 32]); // commitment
        invalid_tx.extend_from_slice(&20000u32.to_le_bytes()); // proof length too large
        invalid_tx.extend_from_slice(&[2u8; 100]); // not enough proof data
        invalid_tx.extend_from_slice(&[3u8; 64]); // signature

        assert!(!CryptoUtils::validate_confidential_transaction(&invalid_tx).unwrap());

        // Proof size zero
        let mut invalid_tx2 = Vec::new();
        invalid_tx2.push(1); // version
        invalid_tx2.push(1); // 1 commitment
        invalid_tx2.extend_from_slice(&[1u8; 32]); // commitment
        invalid_tx2.extend_from_slice(&0u32.to_le_bytes()); // proof length = 0 (invalid)
        invalid_tx2.extend_from_slice(&[3u8; 64]); // signature

        assert!(!CryptoUtils::validate_confidential_transaction(&invalid_tx2).unwrap());
    }
}
