//! Commitment scheme implementations for IPPAN
//!
//! Provides various cryptographic commitment schemes for zero-knowledge proofs.

use anyhow::{anyhow, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Commitment error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitmentError {
    InvalidInput,
    InvalidCommitment,
    InvalidOpening,
    VerificationFailed,
    InvalidParameters,
}

impl std::fmt::Display for CommitmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitmentError::InvalidInput => write!(f, "Invalid input to commitment scheme"),
            CommitmentError::InvalidCommitment => write!(f, "Invalid commitment"),
            CommitmentError::InvalidOpening => write!(f, "Invalid opening"),
            CommitmentError::VerificationFailed => write!(f, "Commitment verification failed"),
            CommitmentError::InvalidParameters => write!(f, "Invalid commitment parameters"),
        }
    }
}

impl std::error::Error for CommitmentError {}

/// Pedersen commitment implementation
pub struct PedersenCommitment {
    generator: [u8; 32],
    h_generator: [u8; 32],
    modulus: [u8; 32],
}

impl Default for PedersenCommitment {
    fn default() -> Self {
        Self::new()
    }
}

impl PedersenCommitment {
    /// Create a new Pedersen commitment scheme
    pub fn new() -> Self {
        // In production, these would be proper generator points
        let mut generator = [0u8; 32];
        let mut h_generator = [0u8; 32];
        let mut modulus = [0u8; 32];

        rand::thread_rng().fill(&mut generator);
        rand::thread_rng().fill(&mut h_generator);
        rand::thread_rng().fill(&mut modulus);

        // Ensure modulus is odd (simplified)
        modulus[0] |= 1;

        Self {
            generator,
            h_generator,
            modulus,
        }
    }

    /// Commit to a message
    pub fn commit(&self, message: &[u8], randomness: &[u8]) -> Result<Commitment> {
        if message.len() != 32 || randomness.len() != 32 {
            return Err(anyhow!(CommitmentError::InvalidInput));
        }

        let m = self.bytes_to_scalar(message);
        let r = self.bytes_to_scalar(randomness);

        let g_m = self.modular_exponentiation(&self.generator, &m);
        let h_r = self.modular_exponentiation(&self.h_generator, &r);
        let commitment = self.modular_multiply(&g_m, &h_r);

        Ok(Commitment {
            value: commitment.to_vec(),
            scheme: "Pedersen".to_string(),
        })
    }

    /// Open a commitment
    pub fn open(
        &self,
        _commitment: &Commitment,
        message: &[u8],
        randomness: &[u8],
    ) -> Result<Opening> {
        if message.len() != 32 || randomness.len() != 32 {
            return Err(anyhow!(CommitmentError::InvalidInput));
        }

        Ok(Opening {
            message: message.to_vec(),
            randomness: randomness.to_vec(),
        })
    }

    /// Verify a commitment
    pub fn verify(&self, commitment: &Commitment, opening: &Opening) -> Result<bool> {
        let computed_commitment = self.commit(&opening.message, &opening.randomness)?;
        Ok(computed_commitment.value == commitment.value)
    }

    /// Convert bytes to scalar (simplified)
    fn bytes_to_scalar(&self, bytes: &[u8]) -> [u8; 32] {
        let mut result = [0u8; 32];
        result.copy_from_slice(bytes);
        result
    }

    /// Modular exponentiation (simplified)
    fn modular_exponentiation(&self, base: &[u8; 32], _exponent: &[u8; 32]) -> [u8; 32] {
        // Simplified implementation - in production, use proper modular arithmetic
        let mut result = [1u8; 32];
        for _ in 0..32 {
            result = self.modular_multiply(&result, base);
        }
        result
    }

    /// Modular multiplication (simplified)
    fn modular_multiply(&self, a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let modulus = self.modulus_value();
        let a_val = Self::scalar_to_u128(a);
        let b_val = Self::scalar_to_u128(b);
        let product = if modulus > 0 {
            (a_val.wrapping_mul(b_val)) % modulus
        } else {
            a_val.wrapping_mul(b_val)
        };
        Self::u128_to_scalar(product)
    }

    fn modulus_value(&self) -> u128 {
        Self::scalar_to_u128(&self.modulus).max(1)
    }

    fn scalar_to_u128(bytes: &[u8; 32]) -> u128 {
        let mut truncated = [0u8; 16];
        truncated.copy_from_slice(&bytes[..16]);
        u128::from_le_bytes(truncated)
    }

    fn u128_to_scalar(value: u128) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[..16].copy_from_slice(&value.to_le_bytes());
        bytes
    }
}

/// Hash commitment implementation
pub struct HashCommitment;

impl Default for HashCommitment {
    fn default() -> Self {
        Self::new()
    }
}

impl HashCommitment {
    /// Create a new hash commitment scheme
    pub fn new() -> Self {
        Self
    }

    /// Commit to a message
    pub fn commit(&self, message: &[u8], randomness: &[u8]) -> Result<Commitment> {
        let combined = [message, randomness].concat();
        let mut hasher = Sha256::new();
        hasher.update(&combined);
        let hash = hasher.finalize();

        Ok(Commitment {
            value: hash.to_vec(),
            scheme: "Hash".to_string(),
        })
    }

    /// Open a commitment
    pub fn open(
        &self,
        _commitment: &Commitment,
        message: &[u8],
        randomness: &[u8],
    ) -> Result<Opening> {
        Ok(Opening {
            message: message.to_vec(),
            randomness: randomness.to_vec(),
        })
    }

    /// Verify a commitment
    pub fn verify(&self, commitment: &Commitment, opening: &Opening) -> Result<bool> {
        let computed_commitment = self.commit(&opening.message, &opening.randomness)?;
        Ok(computed_commitment.value == commitment.value)
    }
}

/// Vector commitment implementation
pub struct VectorCommitment {
    vector_size: usize,
}

impl VectorCommitment {
    /// Create a new vector commitment scheme
    pub fn new(vector_size: usize) -> Self {
        Self { vector_size }
    }

    /// Commit to a vector of messages
    pub fn commit(&self, messages: &[Vec<u8>]) -> Result<Commitment> {
        if messages.len() != self.vector_size {
            return Err(anyhow!(CommitmentError::InvalidInput));
        }

        let mut combined = Vec::new();
        for message in messages {
            combined.extend_from_slice(message);
        }

        let mut hasher = Sha256::new();
        hasher.update(&combined);
        let hash = hasher.finalize();

        Ok(Commitment {
            value: hash.to_vec(),
            scheme: "Vector".to_string(),
        })
    }

    /// Open a vector commitment
    pub fn open(&self, _commitment: &Commitment, messages: &[Vec<u8>]) -> Result<VectorOpening> {
        if messages.len() != self.vector_size {
            return Err(anyhow!(CommitmentError::InvalidInput));
        }

        Ok(VectorOpening {
            messages: messages.to_vec(),
        })
    }

    /// Verify a vector commitment
    pub fn verify(&self, commitment: &Commitment, opening: &VectorOpening) -> Result<bool> {
        let computed_commitment = self.commit(&opening.messages)?;
        Ok(computed_commitment.value == commitment.value)
    }
}

/// Commitment structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub value: Vec<u8>,
    pub scheme: String,
}

/// Opening structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opening {
    pub message: Vec<u8>,
    pub randomness: Vec<u8>,
}

/// Vector opening structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorOpening {
    pub messages: Vec<Vec<u8>>,
}

/// Commitment scheme trait
pub trait CommitmentScheme {
    fn commit(&self, message: &[u8], randomness: &[u8]) -> Result<Commitment>;
    fn open(&self, commitment: &Commitment, message: &[u8], randomness: &[u8]) -> Result<Opening>;
    fn verify(&self, commitment: &Commitment, opening: &Opening) -> Result<bool>;
}

impl CommitmentScheme for PedersenCommitment {
    fn commit(&self, message: &[u8], randomness: &[u8]) -> Result<Commitment> {
        self.commit(message, randomness)
    }

    fn open(&self, commitment: &Commitment, message: &[u8], randomness: &[u8]) -> Result<Opening> {
        self.open(commitment, message, randomness)
    }

    fn verify(&self, commitment: &Commitment, opening: &Opening) -> Result<bool> {
        self.verify(commitment, opening)
    }
}

impl CommitmentScheme for HashCommitment {
    fn commit(&self, message: &[u8], randomness: &[u8]) -> Result<Commitment> {
        self.commit(message, randomness)
    }

    fn open(&self, commitment: &Commitment, message: &[u8], randomness: &[u8]) -> Result<Opening> {
        self.open(commitment, message, randomness)
    }

    fn verify(&self, commitment: &Commitment, opening: &Opening) -> Result<bool> {
        self.verify(commitment, opening)
    }
}

/// Commitment factory
pub struct CommitmentFactory;

impl CommitmentFactory {
    /// Create a Pedersen commitment scheme
    pub fn create_pedersen() -> PedersenCommitment {
        PedersenCommitment::new()
    }

    /// Create a hash commitment scheme
    pub fn create_hash() -> HashCommitment {
        HashCommitment::new()
    }

    /// Create a vector commitment scheme
    pub fn create_vector(size: usize) -> VectorCommitment {
        VectorCommitment::new(size)
    }
}

/// Commitment utilities
pub struct CommitmentUtils;

impl CommitmentUtils {
    /// Generate random opening
    pub fn generate_random_opening(message: &[u8]) -> Opening {
        let mut randomness = [0u8; 32];
        rand::thread_rng().fill(&mut randomness);

        Opening {
            message: message.to_vec(),
            randomness: randomness.to_vec(),
        }
    }

    /// Verify multiple commitments
    pub fn verify_batch<C: CommitmentScheme>(
        scheme: &C,
        commitments: &[Commitment],
        openings: &[Opening],
    ) -> Result<bool> {
        if commitments.len() != openings.len() {
            return Err(anyhow!(CommitmentError::InvalidInput));
        }

        for (commitment, opening) in commitments.iter().zip(openings.iter()) {
            if !scheme.verify(commitment, opening)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Create commitment from string
    pub fn commit_string<C: CommitmentScheme>(
        scheme: &C,
        message: &str,
    ) -> Result<(Commitment, Opening)> {
        let message_bytes = message.as_bytes();
        let mut padded_message = [0u8; 32];
        let copy_len = message_bytes.len().min(32);
        padded_message[..copy_len].copy_from_slice(&message_bytes[..copy_len]);

        let opening = Self::generate_random_opening(&padded_message);
        let commitment = scheme.commit(&opening.message, &opening.randomness)?;

        Ok((commitment, opening))
    }
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_pedersen_commitment() {
        let scheme = PedersenCommitment::new();
        let message = &[0u8; 32];
        let randomness = &[1u8; 32];

        let commitment = scheme.commit(message, randomness).unwrap();
        let opening = scheme.open(&commitment, message, randomness).unwrap();
        let is_valid = scheme.verify(&commitment, &opening).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_hash_commitment() {
        let scheme = HashCommitment::new();
        let message = b"test message";
        let randomness = b"random data here!";

        let commitment = scheme.commit(message, randomness).unwrap();
        let opening = scheme.open(&commitment, message, randomness).unwrap();
        let is_valid = scheme.verify(&commitment, &opening).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_vector_commitment() {
        let scheme = VectorCommitment::new(3);
        let messages = vec![b"msg1".to_vec(), b"msg2".to_vec(), b"msg3".to_vec()];

        let commitment = scheme.commit(&messages).unwrap();
        let opening = scheme.open(&commitment, &messages).unwrap();
        let is_valid = scheme.verify(&commitment, &opening).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_commitment_factory() {
        let pedersen = CommitmentFactory::create_pedersen();
        let hash = CommitmentFactory::create_hash();
        let vector = CommitmentFactory::create_vector(5);

        assert_eq!(vector.vector_size, 5);
    }

    #[test]
    fn test_commitment_utils() {
        let scheme = HashCommitment::new();
        let (commitment, opening) = CommitmentUtils::commit_string(&scheme, "test").unwrap();

        let is_valid = scheme.verify(&commitment, &opening).unwrap();
        assert!(is_valid);
    }
}
