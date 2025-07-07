//! Verifiable randomness implementation
//! 
//! Handles verifiable random number generation for validator selection

use crate::{
    consensus::ippan_time::IppanTimeManager,
    error::{IppanError, Result},
    NodeId,
};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use super::SignatureWrapper;

/// Custom serialization for byte arrays
mod byte_array_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Invalid proof length"));
        }
        let mut proof = [0u8; 64];
        proof.copy_from_slice(&bytes);
        Ok(proof)
    }
}

/// Verifiable Random Function (VRF) for validator selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrfOutput {
    /// The random output
    pub output: [u8; 32],
    
    /// The proof
    pub proof: [u8; 64],
    
    /// The round number
    pub round: u64,
    
    /// The node ID that generated this VRF
    pub node_id: NodeId,
}

/// Verifiable Random Function (VRF) proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VRFProof {
    /// The proof
    #[serde(with = "byte_array_serde")]
    pub proof: [u8; 64],
    /// The output
    pub output: [u8; 32],
    /// Round number this proof is for
    pub round: u64,
}

/// Randomness engine for validator selection
pub struct RandomnessEngine {
    /// Number of validators per round
    validators_per_round: usize,
    
    /// IPPAN Time manager for seeding
    time_manager: Arc<IppanTimeManager>,
    
    /// VRF outputs for each round
    vrf_outputs: Arc<RwLock<HashMap<u64, HashMap<NodeId, VrfOutput>>>>,
    
    /// Selected validators for each round
    selected_validators: Arc<RwLock<HashMap<u64, Vec<NodeId>>>>,
    
    /// All known validators
    known_validators: Arc<RwLock<Vec<NodeId>>>,
}

impl RandomnessEngine {
    /// Create a new randomness engine
    pub fn new(validators_per_round: usize, time_manager: Arc<IppanTimeManager>) -> Self {
        Self {
            validators_per_round,
            time_manager,
            vrf_outputs: Arc::new(RwLock::new(HashMap::new())),
            selected_validators: Arc::new(RwLock::new(HashMap::new())),
            known_validators: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a validator to the known validators list
    pub async fn add_validator(&self, node_id: NodeId) {
        let mut validators = self.known_validators.write().await;
        if !validators.contains(&node_id) {
            validators.push(node_id);
            info!("Added validator: {:?}", node_id);
        }
    }
    
    /// Remove a validator from the known validators list
    pub async fn remove_validator(&self, node_id: &NodeId) {
        let mut validators = self.known_validators.write().await;
        validators.retain(|id| id != node_id);
        info!("Removed validator: {:?}", node_id);
    }
    
    /// Generate VRF output for a round
    pub async fn generate_vrf(&self, round: u64, node_id: NodeId) -> Result<VrfOutput> {
        // Get current IPPAN Time for seeding
        let current_time = self.time_manager.current_time().await;
        
        // Create seed from round and time
        let mut seed = [0u8; 32];
        seed[..8].copy_from_slice(&round.to_be_bytes());
        seed[8..16].copy_from_slice(&current_time.to_be_bytes());
        seed[16..24].copy_from_slice(&node_id[..8]);
        
        // Generate random output using SHA256
        let mut hasher = Sha256::new();
        hasher.update(&seed);
        hasher.update(&node_id);
        let output = hasher.finalize();
        
        let mut vrf_output = [0u8; 32];
        vrf_output.copy_from_slice(&output);
        
        // Generate proof (in a real implementation, this would be a cryptographic proof)
        let mut proof = [0u8; 64];
        proof[..32].copy_from_slice(&vrf_output);
        proof[32..].copy_from_slice(&node_id[..32]);
        
        let vrf = VrfOutput {
            output: vrf_output,
            proof,
            round,
            node_id,
        };
        
        // Store VRF output
        {
            let mut outputs = self.vrf_outputs.write().await;
            outputs.entry(round).or_insert_with(HashMap::new).insert(node_id, vrf.clone());
        }
        
        debug!("Generated VRF for round {}: {:?}", round, vrf_output);
        Ok(vrf)
    }
    
    /// Verify a VRF output
    pub async fn verify_vrf(&self, vrf: &VrfOutput) -> Result<bool> {
        // Recalculate the VRF output
        let expected_vrf = self.generate_vrf(vrf.round, vrf.node_id).await?;
        
        // Compare outputs
        Ok(vrf.output == expected_vrf.output)
    }
    
    /// Select validators for a round
    pub async fn select_validators(&self, round: u64) -> Result<Vec<NodeId>> {
        // Check if we already have selected validators for this round
        {
            let selected = self.selected_validators.read().await;
            if let Some(validators) = selected.get(&round) {
                return Ok(validators.clone());
            }
        }
        
        let validators = self.known_validators.read().await;
        if validators.is_empty() {
            return Ok(Vec::new());
        }
        
        // Get VRF outputs for this round
        let vrf_outputs = {
            let outputs = self.vrf_outputs.read().await;
            outputs.get(&round).cloned().unwrap_or_default()
        };
        
        if vrf_outputs.is_empty() {
            warn!("No VRF outputs available for round {}", round);
            return Ok(Vec::new());
        }
        
        // Sort validators by VRF output (lower is better)
        let mut validator_scores: Vec<(NodeId, [u8; 32])> = vrf_outputs
            .iter()
            .map(|(node_id, vrf)| (*node_id, vrf.output))
            .collect();
        
        validator_scores.sort_by(|a, b| a.1.cmp(&b.1));
        
        // Select top validators
        let selected: Vec<NodeId> = validator_scores
            .into_iter()
            .take(self.validators_per_round.min(validators.len()))
            .map(|(node_id, _)| node_id)
            .collect();
        
        // Store selected validators
        {
            let mut selected_map = self.selected_validators.write().await;
            selected_map.insert(round, selected.clone());
        }
        
        info!("Selected {} validators for round {}: {:?}", selected.len(), round, selected);
        Ok(selected)
    }
    
    /// Check if a node is a validator for a specific round
    pub async fn is_validator_for_round(&self, round: u64, node_id: &NodeId) -> Result<bool> {
        let validators = self.select_validators(round).await?;
        Ok(validators.contains(node_id))
    }
    
    /// Get VRF output for a node in a round
    pub async fn get_vrf_output(&self, round: u64, node_id: &NodeId) -> Option<VrfOutput> {
        let outputs = self.vrf_outputs.read().await;
        outputs.get(&round)?.get(node_id).cloned()
    }
    
    /// Get all VRF outputs for a round
    pub async fn get_round_vrf_outputs(&self, round: u64) -> HashMap<NodeId, VrfOutput> {
        let outputs = self.vrf_outputs.read().await;
        outputs.get(&round).cloned().unwrap_or_default()
    }
    
    /// Get selected validators for a round
    pub async fn get_round_validators(&self, round: u64) -> Vec<NodeId> {
        let selected = self.selected_validators.read().await;
        selected.get(&round).cloned().unwrap_or_default()
    }
    
    /// Get total number of known validators
    pub async fn total_validators(&self) -> usize {
        self.known_validators.read().await.len()
    }
    
    /// Get number of validators per round
    pub fn validators_per_round(&self) -> usize {
        self.validators_per_round
    }
    
    /// Clear old VRF outputs (older than 1000 rounds)
    pub async fn clear_old_outputs(&self, current_round: u64) {
        let mut outputs = self.vrf_outputs.write().await;
        outputs.retain(|&round, _| round + 1000 > current_round);
        
        let mut selected = self.selected_validators.write().await;
        selected.retain(|&round, _| round + 1000 > current_round);
        
        debug!("Cleared VRF outputs older than round {}", current_round - 1000);
    }
    
    /// Generate deterministic random number for testing
    pub fn generate_test_random(&self, seed: u64) -> u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        rng.gen()
    }
}

impl VrfOutput {
    /// Create a new VRF output
    pub fn new(output: [u8; 32], proof: [u8; 64], round: u64, node_id: NodeId) -> Self {
        Self {
            output,
            proof,
            round,
            node_id,
        }
    }
    
    /// Get the random output
    pub fn output(&self) -> &[u8; 32] {
        &self.output
    }
    
    /// Get the proof
    pub fn proof(&self) -> &[u8; 64] {
        &self.proof
    }
    
    /// Get the round number
    pub fn round(&self) -> u64 {
        self.round
    }
    
    /// Get the node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }
    
    /// Convert output to u64 for comparison
    pub fn output_as_u64(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.output[..8]);
        u64::from_be_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ippan_time::IppanTimeManager;
    
    #[tokio::test]
    async fn test_vrf_generation() {
        let node_id = [1u8; 32];
        let time_manager = Arc::new(IppanTimeManager::new(node_id, 1));
        let engine = RandomnessEngine::new(5, time_manager);
        
        let vrf = engine.generate_vrf(1, node_id).await.unwrap();
        
        assert_eq!(vrf.round(), 1);
        assert_eq!(vrf.node_id(), &node_id);
        assert_ne!(vrf.output(), &[0u8; 32]);
    }
    
    #[tokio::test]
    async fn test_validator_selection() {
        let node_id = [1u8; 32];
        let time_manager = Arc::new(IppanTimeManager::new(node_id, 1));
        let engine = RandomnessEngine::new(3, time_manager);
        
        // Add some validators
        engine.add_validator([1u8; 32]).await;
        engine.add_validator([2u8; 32]).await;
        engine.add_validator([3u8; 32]).await;
        engine.add_validator([4u8; 32]).await;
        engine.add_validator([5u8; 32]).await;
        
        // Generate VRF outputs
        engine.generate_vrf(1, [1u8; 32]).await.unwrap();
        engine.generate_vrf(1, [2u8; 32]).await.unwrap();
        engine.generate_vrf(1, [3u8; 32]).await.unwrap();
        engine.generate_vrf(1, [4u8; 32]).await.unwrap();
        engine.generate_vrf(1, [5u8; 32]).await.unwrap();
        
        // Select validators
        let validators = engine.select_validators(1).await.unwrap();
        
        assert_eq!(validators.len(), 3);
        assert!(validators.iter().all(|id| {
            [1u8; 32] == *id || [2u8; 32] == *id || [3u8; 32] == *id || [4u8; 32] == *id || [5u8; 32] == *id
        }));
    }
    
    #[tokio::test]
    async fn test_validator_check() {
        let node_id = [1u8; 32];
        let time_manager = Arc::new(IppanTimeManager::new(node_id, 1));
        let engine = RandomnessEngine::new(2, time_manager);
        
        // Add validators
        engine.add_validator([1u8; 32]).await;
        engine.add_validator([2u8; 32]).await;
        engine.add_validator([3u8; 32]).await;
        
        // Generate VRF outputs
        engine.generate_vrf(1, [1u8; 32]).await.unwrap();
        engine.generate_vrf(1, [2u8; 32]).await.unwrap();
        engine.generate_vrf(1, [3u8; 32]).await.unwrap();
        
        // Check if validators
        let is_validator1 = engine.is_validator_for_round(1, &[1u8; 32]).await.unwrap();
        let is_validator2 = engine.is_validator_for_round(1, &[2u8; 32]).await.unwrap();
        let is_validator3 = engine.is_validator_for_round(1, &[3u8; 32]).await.unwrap();
        
        // At least one should be a validator
        assert!(is_validator1 || is_validator2 || is_validator3);
    }
}
