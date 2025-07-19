//! Tests for IPPAN Randomness system

use ippan::consensus::randomness::{
    VrfProof, RandomnessBeacon, RandomnessManager, ConsensusRandomness,
    RandomnessValidation, RandomnessStats
};
use ed25519_dalek::Keypair;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_vrf_proof_creation() {
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    let proof = VrfProof::new(
        &keypair,
        "test_message",
        1,
        "test_validator"
    ).unwrap();

    assert_eq!(proof.message, "test_message");
    assert_eq!(proof.round, 1);
    assert_eq!(proof.validator_id, "test_validator");
    assert!(!proof.public_key.is_empty());
    assert!(!proof.signature.is_empty());
    assert!(!proof.proof.is_empty());
    assert!(!proof.hash.is_empty());
    assert!(proof.timestamp > 0);
}

#[tokio::test]
async fn test_vrf_proof_verification() {
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    let proof = VrfProof::new(
        &keypair,
        "test_message",
        1,
        "test_validator"
    ).unwrap();

    // Verify valid proof
    assert!(proof.verify().unwrap());

    // Test with invalid signature
    let mut invalid_proof = proof.clone();
    invalid_proof.signature = "invalid_signature".to_string();
    assert!(invalid_proof.verify().is_err());

    // Test with invalid proof hash
    let mut invalid_proof2 = proof.clone();
    invalid_proof2.proof = "invalid_proof".to_string();
    assert!(invalid_proof2.verify().is_err());
}

#[tokio::test]
async fn test_vrf_randomness_values() {
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    let proof = VrfProof::new(
        &keypair,
        "test_message",
        1,
        "test_validator"
    ).unwrap();

    // Test randomness value
    let randomness = proof.get_randomness_value();
    assert!(!randomness.is_empty());
    assert_eq!(randomness, proof.hash);

    // Test validator selection seed
    let validator_seed = proof.get_validator_selection_seed();
    assert!(validator_seed > 0);

    // Test block production seed
    let block_seed = proof.get_block_production_seed();
    assert!(block_seed > 0);

    // Seeds should be different
    assert_ne!(validator_seed, block_seed);
}

#[tokio::test]
async fn test_randomness_beacon_creation() {
    let beacon = RandomnessBeacon::new("test_beacon", 1, 3);

    assert_eq!(beacon.beacon_id, "test_beacon");
    assert_eq!(beacon.round, 1);
    assert_eq!(beacon.min_proofs_required, 3);
    assert_eq!(beacon.received_proofs, 0);
    assert!(!beacon.is_finalized);
    assert!(beacon.vrf_proofs.is_empty());
    assert!(beacon.combined_hash.is_empty());
    assert!(beacon.final_randomness.is_empty());
    assert!(beacon.timestamp > 0);
}

#[tokio::test]
async fn test_beacon_proof_addition() {
    let mut beacon = RandomnessBeacon::new("test_beacon", 1, 3);
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    let proof = VrfProof::new(
        &keypair,
        "test_message",
        1,
        "test_validator"
    ).unwrap();

    // Add valid proof
    assert!(beacon.add_proof(proof.clone()).is_ok());
    assert_eq!(beacon.received_proofs, 1);
    assert_eq!(beacon.vrf_proofs.len(), 1);

    // Try to add duplicate proof
    assert!(beacon.add_proof(proof).is_err());

    // Try to add proof with wrong round
    let proof2 = VrfProof::new(
        &keypair,
        "test_message2",
        2, // Wrong round
        "test_validator2"
    ).unwrap();
    assert!(beacon.add_proof(proof2).is_err());
}

#[tokio::test]
async fn test_beacon_finalization() {
    let mut beacon = RandomnessBeacon::new("test_beacon", 1, 2);
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    // Add first proof
    let proof1 = VrfProof::new(
        &keypair,
        "test_message1",
        1,
        "test_validator1"
    ).unwrap();
    beacon.add_proof(proof1).unwrap();

    // Should not be finalized yet
    assert!(!beacon.is_finalized);

    // Add second proof (meets minimum requirement)
    let proof2 = VrfProof::new(
        &keypair,
        "test_message2",
        1,
        "test_validator2"
    ).unwrap();
    beacon.add_proof(proof2).unwrap();

    // Should be finalized automatically
    assert!(beacon.is_finalized);
    assert!(!beacon.combined_hash.is_empty());
    assert!(!beacon.final_randomness.is_empty());
}

#[tokio::test]
async fn test_beacon_consensus_randomness() {
    let mut beacon = RandomnessBeacon::new("test_beacon", 1, 2);
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    // Add proofs
    let proof1 = VrfProof::new(
        &keypair,
        "test_message1",
        1,
        "test_validator1"
    ).unwrap();
    beacon.add_proof(proof1).unwrap();

    let proof2 = VrfProof::new(
        &keypair,
        "test_message2",
        1,
        "test_validator2"
    ).unwrap();
    beacon.add_proof(proof2).unwrap();

    // Get consensus randomness
    let consensus_randomness = beacon.get_consensus_randomness();
    assert!(consensus_randomness.is_some());

    let randomness = consensus_randomness.unwrap();
    assert_eq!(randomness.round, 1);
    assert_eq!(randomness.source_beacon, "test_beacon");
    assert!(!randomness.randomness_value.is_empty());
    assert!(randomness.validator_selection_seed > 0);
    assert!(randomness.block_production_seed > 0);
    assert!(randomness.timestamp > 0);
}

#[tokio::test]
async fn test_beacon_timeout() {
    let beacon = RandomnessBeacon::new("test_beacon", 1, 3);

    // Should not timeout immediately
    assert!(!beacon.has_timed_out(1000)); // 1 second timeout

    // Create beacon with old timestamp
    let mut old_beacon = RandomnessBeacon::new("old_beacon", 1, 3);
    old_beacon.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() - 10; // 10 seconds ago

    assert!(old_beacon.has_timed_out(5000)); // 5 second timeout
}

#[tokio::test]
async fn test_randomness_manager_creation() {
    let manager = RandomnessManager::new(3, 30000).unwrap();

    // Test initial state
    let stats = manager.get_randomness_stats().await;
    assert_eq!(stats.total_beacons, 0);
    assert_eq!(stats.finalized_beacons, 0);
    assert_eq!(stats.total_proofs, 0);
    assert_eq!(stats.min_proofs_required, 3);
}

#[tokio::test]
async fn test_randomness_manager_vrf_generation() {
    let manager = RandomnessManager::new(3, 30000).unwrap();

    // Generate VRF proof
    let proof = manager.generate_vrf_proof(
        "test_message",
        1,
        "test_validator"
    ).await.unwrap();

    assert_eq!(proof.message, "test_message");
    assert_eq!(proof.round, 1);
    assert_eq!(proof.validator_id, "test_validator");
    assert!(proof.verify().unwrap());
}

#[tokio::test]
async fn test_randomness_manager_beacon_creation() {
    let manager = RandomnessManager::new(3, 30000).unwrap();

    // Create beacon
    assert!(manager.create_beacon("test_beacon", 1).await.is_ok());

    // Try to create duplicate beacon
    assert!(manager.create_beacon("test_beacon", 1).await.is_err());

    // Get beacon
    let beacon = manager.get_beacon("test_beacon").await;
    assert!(beacon.is_some());
    
    let beacon = beacon.unwrap();
    assert_eq!(beacon.beacon_id, "test_beacon");
    assert_eq!(beacon.round, 1);
}

#[tokio::test]
async fn test_randomness_manager_proof_addition() {
    let manager = RandomnessManager::new(3, 30000).unwrap();

    // Create beacon
    manager.create_beacon("test_beacon", 1).await.unwrap();

    // Generate and add proof
    let proof = manager.generate_vrf_proof(
        "test_message",
        1,
        "test_validator"
    ).await.unwrap();

    assert!(manager.add_proof_to_beacon("test_beacon", proof).await.is_ok());

    // Check beacon state
    let beacon = manager.get_beacon("test_beacon").await.unwrap();
    assert_eq!(beacon.received_proofs, 1);
    assert_eq!(beacon.vrf_proofs.len(), 1);
}

#[tokio::test]
async fn test_randomness_manager_beacon_finalization() {
    let manager = RandomnessManager::new(2, 30000).unwrap();

    // Create beacon
    manager.create_beacon("test_beacon", 1).await.unwrap();

    // Add proofs
    for i in 0..2 {
        let proof = manager.generate_vrf_proof(
            &format!("test_message_{}", i),
            1,
            &format!("test_validator_{}", i)
        ).await.unwrap();

        manager.add_proof_to_beacon("test_beacon", proof).await.unwrap();
    }

    // Finalize beacon
    assert!(manager.finalize_beacon("test_beacon").await.is_ok());

    // Check consensus randomness
    let consensus_randomness = manager.get_consensus_randomness(1).await;
    assert!(consensus_randomness.is_some());

    let randomness = consensus_randomness.unwrap();
    assert_eq!(randomness.round, 1);
    assert_eq!(randomness.source_beacon, "test_beacon");
}

#[tokio::test]
async fn test_randomness_manager_validation() {
    let manager = RandomnessManager::new(3, 30000).unwrap();

    // Generate valid proof
    let proof = manager.generate_vrf_proof(
        "test_message",
        1,
        "test_validator"
    ).await.unwrap();

    // Validate proof
    let validation = manager.validate_proof(&proof).await;
    assert!(validation.is_valid);
    assert!(validation.error_message.is_none());
    assert!(validation.verification_time_ms > 0);
    assert_eq!(validation.proof_count, 1);

    // Test invalid proof
    let mut invalid_proof = proof.clone();
    invalid_proof.signature = "invalid_signature".to_string();
    
    let validation = manager.validate_proof(&invalid_proof).await;
    assert!(!validation.is_valid);
    assert!(validation.error_message.is_some());
}

#[tokio::test]
async fn test_randomness_manager_seeds() {
    let manager = RandomnessManager::new(2, 30000).unwrap();

    // Create beacon and add proofs
    manager.create_beacon("test_beacon", 1).await.unwrap();

    for i in 0..2 {
        let proof = manager.generate_vrf_proof(
            &format!("test_message_{}", i),
            1,
            &format!("test_validator_{}", i)
        ).await.unwrap();

        manager.add_proof_to_beacon("test_beacon", proof).await.unwrap();
    }

    manager.finalize_beacon("test_beacon").await.unwrap();

    // Get seeds
    let validator_seed = manager.get_validator_selection_seed(1).await;
    assert!(validator_seed.is_some());
    assert!(validator_seed.unwrap() > 0);

    let block_seed = manager.get_block_production_seed(1).await;
    assert!(block_seed.is_some());
    assert!(block_seed.unwrap() > 0);

    // Seeds should be different
    assert_ne!(validator_seed.unwrap(), block_seed.unwrap());

    // Test non-existent round
    let non_existent_seed = manager.get_validator_selection_seed(999).await;
    assert!(non_existent_seed.is_none());
}

#[tokio::test]
async fn test_randomness_manager_statistics() {
    let manager = RandomnessManager::new(2, 30000).unwrap();

    // Create multiple beacons
    for i in 0..3 {
        manager.create_beacon(&format!("beacon_{}", i), i).await.unwrap();
    }

    // Add proofs to first beacon
    manager.create_beacon("test_beacon", 10).await.unwrap();
    for i in 0..2 {
        let proof = manager.generate_vrf_proof(
            &format!("test_message_{}", i),
            10,
            &format!("test_validator_{}", i)
        ).await.unwrap();

        manager.add_proof_to_beacon("test_beacon", proof).await.unwrap();
    }

    manager.finalize_beacon("test_beacon").await.unwrap();

    // Get statistics
    let stats = manager.get_randomness_stats().await;
    assert_eq!(stats.total_beacons, 4);
    assert_eq!(stats.finalized_beacons, 1);
    assert_eq!(stats.total_proofs, 2);
    assert_eq!(stats.total_consensus_rounds, 1);
    assert_eq!(stats.min_proofs_required, 2);
}

#[tokio::test]
async fn test_randomness_manager_cleanup() {
    let manager = RandomnessManager::new(2, 1000).unwrap(); // 1 second timeout

    // Create beacon
    manager.create_beacon("test_beacon", 1).await.unwrap();

    // Check initial state
    let stats = manager.get_randomness_stats().await;
    assert_eq!(stats.total_beacons, 1);

    // Clean up old beacons
    manager.cleanup_old_beacons().await;

    // Check state after cleanup (should still have beacon since it's recent)
    let stats = manager.get_randomness_stats().await;
    assert_eq!(stats.total_beacons, 1);
}

#[tokio::test]
async fn test_vrf_proof_serialization() {
    let keypair = Keypair::generate(&mut rand::thread_rng());
    
    let proof = VrfProof::new(
        &keypair,
        "test_message",
        1,
        "test_validator"
    ).unwrap();

    // Test serialization
    let serialized = serde_json::to_string(&proof).unwrap();
    let deserialized: VrfProof = serde_json::from_str(&serialized).unwrap();

    assert_eq!(proof.public_key, deserialized.public_key);
    assert_eq!(proof.message, deserialized.message);
    assert_eq!(proof.signature, deserialized.signature);
    assert_eq!(proof.proof, deserialized.proof);
    assert_eq!(proof.hash, deserialized.hash);
    assert_eq!(proof.round, deserialized.round);
    assert_eq!(proof.validator_id, deserialized.validator_id);
}

#[tokio::test]
async fn test_beacon_serialization() {
    let beacon = RandomnessBeacon::new("test_beacon", 1, 3);

    // Test serialization
    let serialized = serde_json::to_string(&beacon).unwrap();
    let deserialized: RandomnessBeacon = serde_json::from_str(&serialized).unwrap();

    assert_eq!(beacon.beacon_id, deserialized.beacon_id);
    assert_eq!(beacon.round, deserialized.round);
    assert_eq!(beacon.min_proofs_required, deserialized.min_proofs_required);
    assert_eq!(beacon.received_proofs, deserialized.received_proofs);
    assert_eq!(beacon.is_finalized, deserialized.is_finalized);
}

#[tokio::test]
async fn test_consensus_randomness_serialization() {
    let consensus_randomness = ConsensusRandomness {
        round: 1,
        randomness_value: "test_randomness".to_string(),
        source_beacon: "test_beacon".to_string(),
        validator_selection_seed: 12345,
        block_production_seed: 67890,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    // Test serialization
    let serialized = serde_json::to_string(&consensus_randomness).unwrap();
    let deserialized: ConsensusRandomness = serde_json::from_str(&serialized).unwrap();

    assert_eq!(consensus_randomness.round, deserialized.round);
    assert_eq!(consensus_randomness.randomness_value, deserialized.randomness_value);
    assert_eq!(consensus_randomness.source_beacon, deserialized.source_beacon);
    assert_eq!(consensus_randomness.validator_selection_seed, deserialized.validator_selection_seed);
    assert_eq!(consensus_randomness.block_production_seed, deserialized.block_production_seed);
    assert_eq!(consensus_randomness.timestamp, deserialized.timestamp);
} 