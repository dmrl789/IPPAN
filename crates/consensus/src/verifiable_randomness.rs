//! Verifiable Randomness for AI Consensus
//! 
//! Implements cryptographically verifiable random selection using:
//! - HashTimer-based entropy
//! - Node state commitments
//! - Verifiable random functions (VRF)
//! - Selection proofs

use crate::types::{HashTimer, IppanTimeMicros};
use crate::crypto::{SigningKey, VerifyingKey, Signature, hash_blake3};
use crate::time::IppanTime;

use std::collections::HashMap;
use std::fmt;

use serde::{Serialize, Deserialize};
use blake3::Hasher;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use ed25519_dalek::{SigningKey as Ed25519SigningKey, VerifyingKey as Ed25519VerifyingKey, Signature as Ed25519Signature, Signer, Verifier};

/// Verifiable Random Number Generator
/// 
/// Uses HashTimer + node state + VRF for cryptographically verifiable randomness
#[derive(Debug, Clone)]
pub struct VerifiableRng {
    /// Current node ID
    node_id: [u8; 32],
    /// Signing key for VRF
    signing_key: Ed25519SigningKey,
    /// Verifying key for VRF
    verifying_key: Ed25519VerifyingKey,
    /// Current entropy state
    entropy_state: EntropyState,
    /// Selection history for verification
    selection_history: Vec<SelectionRecord>,
}

/// Entropy state for randomness generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyState {
    /// HashTimer used for current entropy
    pub hashtimer: HashTimer,
    /// Node state hash
    pub node_state_hash: [u8; 32],
    /// Previous entropy
    pub previous_entropy: [u8; 32],
    /// Current entropy
    pub current_entropy: [u8; 32],
    /// Entropy counter
    pub counter: u64,
}

/// Selection record for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRecord {
    /// Round number
    pub round: u64,
    /// HashTimer used
    pub hashtimer: HashTimer,
    /// Node state hash
    pub node_state_hash: [u8; 32],
    /// Selection entropy
    pub selection_entropy: [u8; 32],
    /// VRF proof
    pub vrf_proof: VRFProof,
    /// Selected validators
    pub selected_validators: Vec<[u8; 32]>,
    /// Selection weights
    pub selection_weights: HashMap<[u8; 32], f64>,
    /// Timestamp
    pub timestamp: u64,
}

/// VRF (Verifiable Random Function) proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VRFProof {
    /// VRF output
    pub output: [u8; 32],
    /// VRF proof
    pub proof: [u8; 64],
    /// Verifying key
    pub verifying_key: [u8; 32],
}

/// Selection proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionProof {
    /// HashTimer used for selection
    pub hashtimer: HashTimer,
    /// Node state hash
    pub node_state_hash: [u8; 32],
    /// Selection entropy
    pub selection_entropy: [u8; 32],
    /// VRF proof
    pub vrf_proof: VRFProof,
    /// Selection record
    pub selection_record: SelectionRecord,
}

/// Verifiable selection result
#[derive(Debug, Clone)]
pub struct VerifiableSelection {
    /// Selected proposer
    pub proposer: [u8; 32],
    /// Selected verifiers
    pub verifiers: Vec<[u8; 32]>,
    /// Selection proof
    pub proof: SelectionProof,
    /// Selection weights
    pub weights: HashMap<[u8; 32], f64>,
}

impl VerifiableRng {
    /// Create a new verifiable RNG
    pub fn new(node_id: [u8; 32], signing_key: Ed25519SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        
        Self {
            node_id,
            signing_key,
            verifying_key,
            entropy_state: EntropyState::new(),
            selection_history: Vec::new(),
        }
    }

    /// Generate verifiable random selection
    pub fn verifiable_selection(
        &mut self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], f64>,
        verifier_count: usize,
        round: u64,
    ) -> Result<VerifiableSelection, VerifiableRngError> {
        // Update entropy state
        self.update_entropy_state(round)?;
        
        // Generate VRF output
        let vrf_output = self.generate_vrf_output()?;
        
        // Create selection entropy from VRF output
        let selection_entropy = self.create_selection_entropy(&vrf_output)?;
        
        // Perform weighted random selection
        let (proposer, verifiers) = self.weighted_random_selection(
            candidates,
            weights,
            verifier_count,
            &selection_entropy,
        )?;
        
        // Create VRF proof
        let vrf_proof = self.create_vrf_proof(&vrf_output)?;
        
        // Create selection record
        let selection_record = SelectionRecord {
            round,
            hashtimer: self.entropy_state.hashtimer,
            node_state_hash: self.entropy_state.node_state_hash,
            selection_entropy,
            vrf_proof: vrf_proof.clone(),
            selected_validators: {
                let mut selected = vec![proposer];
                selected.extend_from_slice(&verifiers);
                selected
            },
            selection_weights: weights.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        // Create selection proof
        let proof = SelectionProof {
            hashtimer: self.entropy_state.hashtimer,
            node_state_hash: self.entropy_state.node_state_hash,
            selection_entropy,
            vrf_proof,
            selection_record: selection_record.clone(),
        };
        
        // Record selection
        self.selection_history.push(selection_record);
        
        // Keep only recent history
        if self.selection_history.len() > 1000 {
            self.selection_history.remove(0);
        }
        
        Ok(VerifiableSelection {
            proposer,
            verifiers,
            proof,
            weights: weights.clone(),
        })
    }

    /// Update entropy state for new round
    fn update_entropy_state(&mut self, round: u64) -> Result<(), VerifiableRngError> {
        // Create new HashTimer for this round
        let hashtimer = HashTimer::now_round("selection", &[], &[], &self.node_id);
        
        // Update node state hash
        let node_state_hash = self.compute_node_state_hash(round)?;
        
        // Update entropy
        let previous_entropy = self.entropy_state.current_entropy;
        let current_entropy = self.compute_current_entropy(&hashtimer, &node_state_hash, &previous_entropy)?;
        
        self.entropy_state = EntropyState {
            hashtimer,
            node_state_hash,
            previous_entropy,
            current_entropy,
            counter: self.entropy_state.counter + 1,
        };
        
        Ok(())
    }

    /// Generate VRF output
    fn generate_vrf_output(&self) -> Result<[u8; 32], VerifiableRngError> {
        // Create VRF input from entropy state
        let vrf_input = self.create_vrf_input()?;
        
        // Generate VRF output using Ed25519
        let vrf_output = self.compute_vrf_output(&vrf_input)?;
        
        Ok(vrf_output)
    }

    /// Create VRF input
    fn create_vrf_input(&self) -> Result<Vec<u8>, VerifiableRngError> {
        let mut input = Vec::new();
        
        // Add HashTimer
        input.extend_from_slice(&self.entropy_state.hashtimer.to_bytes());
        
        // Add node state hash
        input.extend_from_slice(&self.entropy_state.node_state_hash);
        
        // Add previous entropy
        input.extend_from_slice(&self.entropy_state.previous_entropy);
        
        // Add counter
        input.extend_from_slice(&self.entropy_state.counter.to_le_bytes());
        
        // Add node ID
        input.extend_from_slice(&self.node_id);
        
        Ok(input)
    }

    /// Compute VRF output
    fn compute_vrf_output(&self, input: &[u8]) -> Result<[u8; 32], VerifiableRngError> {
        // Use Ed25519 for VRF-like functionality
        let signature = self.signing_key.sign(input);
        
        // Hash the signature to get VRF output
        let mut hasher = Hasher::new();
        hasher.update(&signature.to_bytes());
        hasher.update(input);
        
        let mut output = [0u8; 32];
        output.copy_from_slice(&hasher.finalize().as_bytes()[..32]);
        
        Ok(output)
    }

    /// Create selection entropy from VRF output
    fn create_selection_entropy(&self, vrf_output: &[u8; 32]) -> Result<[u8; 32], VerifiableRngError> {
        let mut hasher = Hasher::new();
        hasher.update(vrf_output);
        hasher.update(&self.entropy_state.hashtimer.to_bytes());
        hasher.update(&self.entropy_state.node_state_hash);
        
        let mut entropy = [0u8; 32];
        entropy.copy_from_slice(&hasher.finalize().as_bytes()[..32]);
        
        Ok(entropy)
    }

    /// Weighted random selection using verifiable entropy
    fn weighted_random_selection(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], f64>,
        verifier_count: usize,
        entropy: &[u8; 32],
    ) -> Result<([u8; 32], Vec<[u8; 32]>), VerifiableRngError> {
        // Create RNG from entropy
        let mut rng = self.create_rng_from_entropy(entropy)?;
        
        // Select proposer
        let proposer = self.select_proposer(candidates, weights, &mut rng)?;
        
        // Select verifiers
        let verifiers = self.select_verifiers(candidates, weights, verifier_count, &proposer, &mut rng)?;
        
        Ok((proposer, verifiers))
    }

    /// Create RNG from entropy
    fn create_rng_from_entropy(&self, entropy: &[u8; 32]) -> Result<StdRng, VerifiableRngError> {
        // Use entropy as seed for deterministic RNG
        let seed: [u8; 32] = *entropy;
        Ok(StdRng::from_seed(seed))
    }

    /// Select proposer using weighted random selection
    fn select_proposer(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], f64>,
        rng: &mut StdRng,
    ) -> Result<[u8; 32], VerifiableRngError> {
        let total_weight: f64 = weights.values().sum();
        let random_value: f64 = rng.gen_range(0.0..total_weight);
        
        let mut cumulative_weight = 0.0;
        for &candidate in candidates {
            if let Some(&weight) = weights.get(&candidate) {
                cumulative_weight += weight;
                if random_value <= cumulative_weight {
                    return Ok(candidate);
                }
            }
        }
        
        // Fallback to first candidate
        candidates.first()
            .copied()
            .ok_or(VerifiableRngError::NoCandidates)
    }

    /// Select verifiers using weighted random selection
    fn select_verifiers(
        &self,
        candidates: &[[u8; 32]],
        weights: &HashMap<[u8; 32], f64>,
        verifier_count: usize,
        proposer: &[u8; 32],
        rng: &mut StdRng,
    ) -> Result<Vec<[u8; 32]>, VerifiableRngError> {
        let mut verifiers = Vec::new();
        let mut remaining_candidates: Vec<[u8; 32]> = candidates
            .iter()
            .filter(|&&candidate| candidate != *proposer)
            .copied()
            .collect();
        
        let mut remaining_weights: HashMap<[u8; 32], f64> = remaining_candidates
            .iter()
            .filter_map(|&candidate| {
                weights.get(&candidate).map(|&weight| (candidate, weight))
            })
            .collect();
        
        for _ in 0..verifier_count.min(remaining_candidates.len()) {
            let verifier = self.select_proposer(&remaining_candidates, &remaining_weights, rng)?;
            verifiers.push(verifier);
            
            // Remove selected verifier from remaining candidates
            remaining_candidates.retain(|&candidate| candidate != verifier);
            remaining_weights.remove(&verifier);
        }
        
        Ok(verifiers)
    }

    /// Create VRF proof
    fn create_vrf_proof(&self, vrf_output: &[u8; 32]) -> Result<VRFProof, VerifiableRngError> {
        let vrf_input = self.create_vrf_input()?;
        let signature = self.signing_key.sign(&vrf_input);
        
        Ok(VRFProof {
            output: *vrf_output,
            proof: signature.to_bytes(),
            verifying_key: self.verifying_key.to_bytes(),
        })
    }

    /// Compute node state hash
    fn compute_node_state_hash(&self, round: u64) -> Result<[u8; 32], VerifiableRngError> {
        let mut hasher = Hasher::new();
        
        // Add round number
        hasher.update(&round.to_le_bytes());
        
        // Add node ID
        hasher.update(&self.node_id);
        
        // Add current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        hasher.update(&timestamp.to_le_bytes());
        
        // Add previous entropy
        hasher.update(&self.entropy_state.previous_entropy);
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize().as_bytes()[..32]);
        
        Ok(hash)
    }

    /// Compute current entropy
    fn compute_current_entropy(
        &self,
        hashtimer: &HashTimer,
        node_state_hash: &[u8; 32],
        previous_entropy: &[u8; 32],
    ) -> Result<[u8; 32], VerifiableRngError> {
        let mut hasher = Hasher::new();
        
        hasher.update(&hashtimer.to_bytes());
        hasher.update(node_state_hash);
        hasher.update(previous_entropy);
        hasher.update(&self.node_id);
        
        let mut entropy = [0u8; 32];
        entropy.copy_from_slice(&hasher.finalize().as_bytes()[..32]);
        
        Ok(entropy)
    }

    /// Verify a selection proof
    pub fn verify_selection_proof(
        &self,
        proof: &SelectionProof,
        expected_candidates: &[[u8; 32]],
        expected_weights: &HashMap<[u8; 32], f64>,
    ) -> Result<bool, VerifiableRngError> {
        // Verify VRF proof
        if !self.verify_vrf_proof(&proof.vrf_proof)? {
            return Ok(false);
        }
        
        // Verify selection entropy
        let expected_entropy = self.create_selection_entropy(&proof.vrf_proof.output)?;
        if expected_entropy != proof.selection_entropy {
            return Ok(false);
        }
        
        // Verify node state hash
        let expected_node_state_hash = self.compute_node_state_hash(proof.selection_record.round)?;
        if expected_node_state_hash != proof.node_state_hash {
            return Ok(false);
        }
        
        // Verify HashTimer
        if proof.hashtimer != proof.selection_record.hashtimer {
            return Ok(false);
        }
        
        // Verify selection consistency
        self.verify_selection_consistency(
            &proof.selection_record,
            expected_candidates,
            expected_weights,
        )
    }

    /// Verify VRF proof
    fn verify_vrf_proof(&self, vrf_proof: &VRFProof) -> Result<bool, VerifiableRngError> {
        // Recreate VRF input
        let vrf_input = self.create_vrf_input()?;
        
        // Verify signature
        let signature = Ed25519Signature::from_bytes(&vrf_proof.proof)
            .map_err(|_| VerifiableRngError::InvalidSignature)?;
        
        let verifying_key = Ed25519VerifyingKey::from_bytes(&vrf_proof.verifying_key)
            .map_err(|_| VerifiableRngError::InvalidVerifyingKey)?;
        
        if !verifying_key.verify(&vrf_input, &signature).is_ok() {
            return Ok(false);
        }
        
        // Verify VRF output
        let expected_output = self.compute_vrf_output(&vrf_input)?;
        Ok(expected_output == vrf_proof.output)
    }

    /// Verify selection consistency
    fn verify_selection_consistency(
        &self,
        selection_record: &SelectionRecord,
        expected_candidates: &[[u8; 32]],
        expected_weights: &HashMap<[u8; 32], f64>,
    ) -> Result<bool, VerifiableRngError> {
        // Check that all selected validators are in expected candidates
        for &validator in &selection_record.selected_validators {
            if !expected_candidates.contains(&validator) {
                return Ok(false);
            }
        }
        
        // Check that selection weights match expected weights
        for (validator, &weight) in &selection_record.selection_weights {
            if expected_weights.get(validator) != Some(&weight) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Get selection history
    pub fn get_selection_history(&self) -> &[SelectionRecord] {
        &self.selection_history
    }

    /// Clear selection history
    pub fn clear_selection_history(&mut self) {
        self.selection_history.clear();
    }
}

impl EntropyState {
    /// Create new entropy state
    pub fn new() -> Self {
        Self {
            hashtimer: HashTimer::now_round("initial", &[], &[], &[0u8; 32]),
            node_state_hash: [0u8; 32],
            previous_entropy: [0u8; 32],
            current_entropy: [0u8; 32],
            counter: 0,
        }
    }
}

/// Verifiable RNG errors
#[derive(Debug, Clone)]
pub enum VerifiableRngError {
    /// No candidates available
    NoCandidates,
    /// Invalid signature
    InvalidSignature,
    /// Invalid verifying key
    InvalidVerifyingKey,
    /// VRF computation failed
    VRFComputationFailed,
    /// Entropy generation failed
    EntropyGenerationFailed,
    /// Selection verification failed
    SelectionVerificationFailed,
}

impl fmt::Display for VerifiableRngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifiableRngError::NoCandidates => write!(f, "No candidates available for selection"),
            VerifiableRngError::InvalidSignature => write!(f, "Invalid signature in VRF proof"),
            VerifiableRngError::InvalidVerifyingKey => write!(f, "Invalid verifying key in VRF proof"),
            VerifiableRngError::VRFComputationFailed => write!(f, "VRF computation failed"),
            VerifiableRngError::EntropyGenerationFailed => write!(f, "Entropy generation failed"),
            VerifiableRngError::SelectionVerificationFailed => write!(f, "Selection verification failed"),
        }
    }
}

impl std::error::Error for VerifiableRngError {}

/// Verifiable RNG result type
pub type VerifiableRngResult<T> = Result<T, VerifiableRngError>;