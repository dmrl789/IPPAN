use std::fmt;
use thiserror::Error;

/// Simplified STARK proof structure for basic validation
#[derive(Clone, Debug)]
pub struct StarkProof {
    sequence_length: usize,
    result: u64,
    proof_data: Vec<u8>,
}

impl StarkProof {
    /// Returns the number of terms produced by the underlying Fibonacci computation.
    pub fn sequence_length(&self) -> usize {
        self.sequence_length
    }

    /// Returns the final term of the Fibonacci sequence as an integer.
    pub fn result(&self) -> u64 {
        self.result
    }

    /// Serializes the proof into bytes suitable for storage or transport.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.proof_data.clone()
    }

    /// Reconstruct a `StarkProof` instance from serialized bytes and public inputs.
    pub fn from_bytes(
        sequence_length: usize,
        result: u64,
        bytes: &[u8],
    ) -> Result<Self, StarkProofError> {
        if sequence_length < 4 || !sequence_length.is_power_of_two() {
            return Err(StarkProofError::InvalidSequenceLength);
        }

        Ok(Self {
            sequence_length,
            result,
            proof_data: bytes.to_vec(),
        })
    }
}

/// Errors that can be encountered while generating or verifying STARK proofs.
#[derive(Debug, Error)]
pub enum StarkProofError {
    /// The caller supplied an invalid sequence length.
    #[error("sequence length must be a power of two and at least 4 steps long")]
    InvalidSequenceLength,
    /// Proof generation failed.
    #[error("failed to generate STARK proof: {0}")]
    Proving(String),
    /// Verification failed for the supplied proof.
    #[error("failed to verify STARK proof: {0}")]
    Verification(String),
    /// Deserialization of proof bytes failed.
    #[error("failed to decode STARK proof: {0}")]
    Deserialization(String),
}

/// Generate a simplified STARK proof for testing purposes.
pub fn generate_fibonacci_proof(sequence_length: usize) -> Result<StarkProof, StarkProofError> {
    if sequence_length < 4 || !sequence_length.is_power_of_two() {
        return Err(StarkProofError::InvalidSequenceLength);
    }

    // Simplified implementation - just compute the fibonacci number
    let result = fibonacci_u64(sequence_length);
    let proof_data = format!("fibonacci_{}_{}", sequence_length, result).into_bytes();

    Ok(StarkProof {
        sequence_length,
        result,
        proof_data,
    })
}

/// Verify a simplified STARK proof.
pub fn verify_fibonacci_proof(proof: &StarkProof) -> Result<(), StarkProofError> {
    // Simplified verification - just check that the result matches expected fibonacci
    let expected = fibonacci_u64(proof.sequence_length);
    if proof.result != expected {
        return Err(StarkProofError::Verification(
            "Proof result does not match expected fibonacci value".to_string(),
        ));
    }
    Ok(())
}

fn fibonacci_u64(n: usize) -> u64 {
    let mut a: u64 = 1;
    let mut b: u64 = 1;
    for _ in 2..n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_and_verifies_proof() {
        let proof = generate_fibonacci_proof(32).expect("proof generation should succeed");
        assert_eq!(proof.sequence_length(), 32);
        assert_eq!(proof.result(), fibonacci_u64(32));
        verify_fibonacci_proof(&proof).expect("verification should succeed");
    }

    #[test]
    fn detecting_tampering() {
        let mut proof = generate_fibonacci_proof(32).expect("proof generation should succeed");
        proof.result += 1;
        assert!(verify_fibonacci_proof(&proof).is_err());
    }
}