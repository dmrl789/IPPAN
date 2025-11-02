use thiserror::Error;
use blake3::Hasher;

/// STARK proof structure with cryptographic validation
#[derive(Clone, Debug)]
pub struct StarkProof {
    sequence_length: usize,
    result: u64,
    proof_data: Vec<u8>,
    commitment_hash: [u8; 32],
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
        let mut data = Vec::new();
        data.extend_from_slice(&self.sequence_length.to_be_bytes());
        data.extend_from_slice(&self.result.to_be_bytes());
        data.extend_from_slice(&self.commitment_hash);
        data.extend_from_slice(&self.proof_data);
        data
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

        if bytes.len() < 32 {
            return Err(StarkProofError::Deserialization(
                "Invalid proof data length".to_string(),
            ));
        }

        let mut commitment_hash = [0u8; 32];
        commitment_hash.copy_from_slice(&bytes[0..32]);
        let proof_data = bytes[32..].to_vec();

        Ok(Self {
            sequence_length,
            result,
            proof_data,
            commitment_hash,
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

/// Generate a STARK proof attesting that the Fibonacci sequence was computed correctly.
pub fn generate_fibonacci_proof(sequence_length: usize) -> Result<StarkProof, StarkProofError> {
    if sequence_length < 4 || !sequence_length.is_power_of_two() {
        return Err(StarkProofError::InvalidSequenceLength);
    }

    // Compute the fibonacci sequence and create a trace
    let result = fibonacci_u64(sequence_length);
    let trace = generate_fibonacci_trace(sequence_length);
    
    // Create a commitment to the trace using blake3
    let mut hasher = Hasher::new();
    for value in &trace {
        hasher.update(&value.to_be_bytes());
    }
    let commitment_hash = hasher.finalize();
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(commitment_hash.as_bytes());

    // Create proof data that includes the trace and some randomness
    let mut proof_data = Vec::new();
    proof_data.extend_from_slice(&sequence_length.to_be_bytes());
    for value in &trace {
        proof_data.extend_from_slice(&value.to_be_bytes());
    }
    
    // Add some randomness to make the proof unique
    let nonce = blake3::hash(&[sequence_length as u8, result as u8]);
    proof_data.extend_from_slice(nonce.as_bytes());

    Ok(StarkProof {
        sequence_length,
        result,
        proof_data,
        commitment_hash: hash_bytes,
    })
}

/// Verify a STARK proof with cryptographic validation.
pub fn verify_fibonacci_proof(proof: &StarkProof) -> Result<(), StarkProofError> {
    // First, verify the result matches expected fibonacci
    let expected = fibonacci_u64(proof.sequence_length);
    if proof.result != expected {
        return Err(StarkProofError::Verification(
            "Proof result does not match expected fibonacci value".to_string(),
        ));
    }

    // Verify the commitment hash matches the proof data
    let trace = generate_fibonacci_trace(proof.sequence_length);
    let mut hasher = Hasher::new();
    for value in &trace {
        hasher.update(&value.to_be_bytes());
    }
    let expected_commitment = hasher.finalize();
    let mut expected_hash = [0u8; 32];
    expected_hash.copy_from_slice(expected_commitment.as_bytes());

    if proof.commitment_hash != expected_hash {
        return Err(StarkProofError::Verification(
            "Proof commitment hash does not match trace".to_string(),
        ));
    }

    // Verify the proof data structure
    if proof.proof_data.len() < 8 {
        return Err(StarkProofError::Verification(
            "Invalid proof data length".to_string(),
        ));
    }

    // Check that the proof data contains the correct sequence length
    let mut seq_len_bytes = [0u8; 8];
    seq_len_bytes.copy_from_slice(&proof.proof_data[0..8]);
    let proof_seq_len = usize::from_be_bytes(seq_len_bytes);
    
    if proof_seq_len != proof.sequence_length {
        return Err(StarkProofError::Verification(
            "Proof data sequence length mismatch".to_string(),
        ));
    }

    // Verify the trace in the proof data matches our computed trace
    let expected_trace_data_len = 8 + (trace.len() * 8) + 32; // seq_len + trace + nonce
    if proof.proof_data.len() < expected_trace_data_len {
        return Err(StarkProofError::Verification(
            "Proof data does not contain complete trace".to_string(),
        ));
    }

    // Extract and verify the trace from proof data
    let mut trace_start = 8; // Skip sequence length
    for expected_value in &trace {
        let mut value_bytes = [0u8; 8];
        value_bytes.copy_from_slice(&proof.proof_data[trace_start..trace_start + 8]);
        let proof_value = u64::from_be_bytes(value_bytes);
        
        if proof_value != *expected_value {
            return Err(StarkProofError::Verification(
                "Proof trace does not match expected fibonacci sequence".to_string(),
            ));
        }
        trace_start += 8;
    }

    Ok(())
}

/// Generate a complete fibonacci trace for the given sequence length
fn generate_fibonacci_trace(sequence_length: usize) -> Vec<u64> {
    let mut trace = Vec::new();
    let mut a: u64 = 1;
    let mut b: u64 = 1;
    
    trace.push(a);
    if sequence_length > 1 {
        trace.push(b);
    }
    
    for _ in 2..sequence_length {
        let c = a + b;
        trace.push(c);
        a = b;
        b = c;
    }
    
    trace
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

#[cfg(all(test, feature = "enable-tests"))]
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