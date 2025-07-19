//! Tests for IPPAN zk-STARK system

use ippan::consensus::zk_stark::{
    ZkStarkProof, ZkStarkCircuit, ZkStarkManager, ZkStarkAggregation,
    CircuitType, ZkStarkVerification, ZkStarkStats, ProofMetadata
};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_zk_stark_proof_creation() {
    let proof = ZkStarkProof::new(
        "test_circuit",
        vec!["input1".to_string(), "input2".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).unwrap();

    assert_eq!(proof.circuit_id, "test_circuit");
    assert_eq!(proof.round, 1);
    assert_eq!(proof.validator_id, "test_validator");
    assert_eq!(proof.public_inputs, vec!["input1", "input2"]);
    assert_eq!(proof.proof_data, "test_proof_data");
    assert!(!proof.proof_id.is_empty());
    assert!(!proof.commitment.is_empty());
    assert!(proof.timestamp > 0);
    assert_eq!(proof.proof_size_bytes, "test_proof_data".len());
    assert_eq!(proof.verification_time_ms, 0);
    assert!(!proof.is_valid);
}

#[tokio::test]
async fn test_zk_stark_proof_verification() {
    let proof = ZkStarkProof::new(
        "test_circuit",
        vec!["input1".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).unwrap();

    // Verify valid proof
    assert!(proof.verify().unwrap());

    // Test with empty proof data
    let mut invalid_proof = proof.clone();
    invalid_proof.proof_data = String::new();
    assert!(invalid_proof.verify().is_err());

    // Test with invalid commitment
    let mut invalid_proof2 = proof.clone();
    invalid_proof2.commitment = "invalid_commitment".to_string();
    assert!(invalid_proof2.verify().is_err());

    // Test with future timestamp
    let mut future_proof = proof.clone();
    future_proof.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + 7200; // 2 hours in future
    assert!(future_proof.verify().is_err());

    // Test with old timestamp
    let mut old_proof = proof.clone();
    old_proof.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() - 100000; // Very old
    assert!(old_proof.verify().is_err());
}

#[tokio::test]
async fn test_zk_stark_proof_metadata() {
    let proof = ZkStarkProof::new(
        "test_circuit",
        vec!["input1".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).unwrap();

    let metadata = proof.get_metadata();

    assert_eq!(metadata.proof_id, proof.proof_id);
    assert_eq!(metadata.circuit_id, proof.circuit_id);
    assert_eq!(metadata.round, proof.round);
    assert_eq!(metadata.validator_id, proof.validator_id);
    assert_eq!(metadata.timestamp, proof.timestamp);
    assert_eq!(metadata.proof_size_bytes, proof.proof_size_bytes);
    assert_eq!(metadata.verification_time_ms, proof.verification_time_ms);
    assert_eq!(metadata.is_valid, proof.is_valid);
}

#[tokio::test]
async fn test_zk_stark_circuit_creation() {
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string(), "constraint2".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );

    assert_eq!(circuit.circuit_id, "test_circuit");
    assert_eq!(circuit.circuit_type, CircuitType::TransactionValidation);
    assert_eq!(circuit.constraints.len(), 2);
    assert_eq!(circuit.public_inputs.len(), 1);
    assert_eq!(circuit.private_inputs.len(), 1);
    assert_eq!(circuit.output_definitions.len(), 1);
    assert!(circuit.complexity_score > 0);
}

#[tokio::test]
async fn test_zk_stark_circuit_validation() {
    // Valid circuit
    let valid_circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::StateTransition,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    assert!(valid_circuit.validate().unwrap());

    // Invalid circuit - empty ID
    let mut invalid_circuit = valid_circuit.clone();
    invalid_circuit.circuit_id = String::new();
    assert!(invalid_circuit.validate().is_err());

    // Invalid circuit - no constraints
    let mut invalid_circuit2 = valid_circuit.clone();
    invalid_circuit2.constraints.clear();
    assert!(invalid_circuit2.validate().is_err());

    // Invalid circuit - no inputs
    let mut invalid_circuit3 = valid_circuit.clone();
    invalid_circuit3.public_inputs.clear();
    invalid_circuit3.private_inputs.clear();
    assert!(invalid_circuit3.validate().is_err());

    // Invalid circuit - no outputs
    let mut invalid_circuit4 = valid_circuit.clone();
    invalid_circuit4.output_definitions.clear();
    assert!(invalid_circuit4.validate().is_err());
}

#[tokio::test]
async fn test_zk_stark_circuit_type_string() {
    let transaction_circuit = ZkStarkCircuit::new(
        "transaction_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["input1".to_string()],
        vec!["private1".to_string()],
        vec!["output1".to_string()],
    );
    assert_eq!(transaction_circuit.get_circuit_type_string(), "transaction_validation");

    let state_circuit = ZkStarkCircuit::new(
        "state_circuit",
        CircuitType::StateTransition,
        vec!["constraint1".to_string()],
        vec!["input1".to_string()],
        vec!["private1".to_string()],
        vec!["output1".to_string()],
    );
    assert_eq!(state_circuit.get_circuit_type_string(), "state_transition");

    let custom_circuit = ZkStarkCircuit::new(
        "custom_circuit",
        CircuitType::Custom("my_custom_type".to_string()),
        vec!["constraint1".to_string()],
        vec!["input1".to_string()],
        vec!["private1".to_string()],
        vec!["output1".to_string()],
    );
    assert_eq!(custom_circuit.get_circuit_type_string(), "custom_my_custom_type");
}

#[tokio::test]
async fn test_zk_stark_manager_creation() {
    let manager = ZkStarkManager::new(1_000_000, 5000); // 1MB, 5s

    // Test initial state
    let stats = manager.get_zk_stark_stats().await;
    assert_eq!(stats.total_circuits, 0);
    assert_eq!(stats.total_proofs, 0);
    assert_eq!(stats.valid_proofs, 0);
    assert_eq!(stats.total_aggregations, 0);
    assert_eq!(stats.cached_verifications, 0);
    assert_eq!(stats.max_proof_size_bytes, 1_000_000);
    assert_eq!(stats.max_verification_time_ms, 5000);
}

#[tokio::test]
async fn test_zk_stark_manager_circuit_registration() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );

    // Register circuit
    assert!(manager.register_circuit(circuit).await.is_ok());

    // Try to register duplicate circuit
    let duplicate_circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::StateTransition,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    assert!(manager.register_circuit(duplicate_circuit).await.is_err());

    // Get circuit
    let retrieved_circuit = manager.get_circuit("test_circuit").await;
    assert!(retrieved_circuit.is_some());
    
    let circuit = retrieved_circuit.unwrap();
    assert_eq!(circuit.circuit_id, "test_circuit");
    assert_eq!(circuit.circuit_type, CircuitType::TransactionValidation);
}

#[tokio::test]
async fn test_zk_stark_manager_proof_generation() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit first
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate proof
    let proof = manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string(), "input2".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).await.unwrap();

    assert_eq!(proof.circuit_id, "test_circuit");
    assert_eq!(proof.round, 1);
    assert_eq!(proof.validator_id, "test_validator");
    assert_eq!(proof.public_inputs, vec!["input1", "input2"]);
    assert_eq!(proof.proof_data, "test_proof_data");
    assert!(proof.is_valid);

    // Try to generate proof for non-existent circuit
    assert!(manager.generate_proof(
        "non_existent_circuit",
        vec!["input1".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).await.is_err());
}

#[tokio::test]
async fn test_zk_stark_manager_proof_verification() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate and verify proof
    let proof = manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).await.unwrap();

    let verification = manager.verify_proof(&proof).await;
    assert!(verification.is_valid);
    assert!(verification.error_message.is_none());
    assert!(verification.verification_time_ms > 0);
    assert_eq!(verification.proof_size_bytes, proof.proof_size_bytes);

    // Test invalid proof
    let mut invalid_proof = proof.clone();
    invalid_proof.proof_data = String::new();
    
    let verification = manager.verify_proof(&invalid_proof).await;
    assert!(!verification.is_valid);
    assert!(verification.error_message.is_some());
}

#[tokio::test]
async fn test_zk_stark_manager_aggregation() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate multiple proofs
    let proof1 = manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string()],
        "proof_data_1",
        1,
        "validator1"
    ).await.unwrap();

    let proof2 = manager.generate_proof(
        "test_circuit",
        vec!["input2".to_string()],
        "proof_data_2",
        1,
        "validator2"
    ).await.unwrap();

    // Create aggregation
    let aggregation = manager.create_aggregation(
        "test_aggregation",
        1,
        vec![proof1.proof_id.clone(), proof2.proof_id.clone()]
    ).await.unwrap();

    assert_eq!(aggregation.aggregation_id, "test_aggregation");
    assert_eq!(aggregation.round, 1);
    assert_eq!(aggregation.proofs.len(), 2);
    assert!(aggregation.is_valid);
    assert!(aggregation.total_size_bytes > 0);
    assert!(aggregation.verification_time_ms > 0);

    // Verify aggregation
    assert!(aggregation.verify().unwrap());

    // Get aggregation
    let retrieved_aggregation = manager.get_aggregation("test_aggregation").await;
    assert!(retrieved_aggregation.is_some());
}

#[tokio::test]
async fn test_zk_stark_manager_proof_retrieval() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate proof
    let proof = manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).await.unwrap();

    // Get proof by ID
    let retrieved_proof = manager.get_proof(&proof.proof_id).await;
    assert!(retrieved_proof.is_some());
    
    let retrieved_proof = retrieved_proof.unwrap();
    assert_eq!(retrieved_proof.proof_id, proof.proof_id);
    assert_eq!(retrieved_proof.circuit_id, proof.circuit_id);

    // Get non-existent proof
    let non_existent_proof = manager.get_proof("non_existent_proof").await;
    assert!(non_existent_proof.is_none());
}

#[tokio::test]
async fn test_zk_stark_manager_round_proofs() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate proofs for different rounds
    let proof1 = manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string()],
        "proof_data_1",
        1,
        "validator1"
    ).await.unwrap();

    let proof2 = manager.generate_proof(
        "test_circuit",
        vec!["input2".to_string()],
        "proof_data_2",
        1,
        "validator2"
    ).await.unwrap();

    let proof3 = manager.generate_proof(
        "test_circuit",
        vec!["input3".to_string()],
        "proof_data_3",
        2,
        "validator3"
    ).await.unwrap();

    // Get proofs for round 1
    let round1_proofs = manager.get_proofs_for_round(1).await;
    assert_eq!(round1_proofs.len(), 2);

    // Get proofs for round 2
    let round2_proofs = manager.get_proofs_for_round(2).await;
    assert_eq!(round2_proofs.len(), 1);

    // Get proofs for non-existent round
    let round999_proofs = manager.get_proofs_for_round(999).await;
    assert_eq!(round999_proofs.len(), 0);
}

#[tokio::test]
async fn test_zk_stark_manager_statistics() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate proofs
    for i in 0..3 {
        manager.generate_proof(
            "test_circuit",
            vec![format!("input{}", i)],
            &format!("proof_data_{}", i),
            1,
            &format!("validator{}", i)
        ).await.unwrap();
    }

    // Get statistics
    let stats = manager.get_zk_stark_stats().await;
    assert_eq!(stats.total_circuits, 1);
    assert_eq!(stats.total_proofs, 3);
    assert_eq!(stats.valid_proofs, 3);
    assert_eq!(stats.total_aggregations, 0);
    assert!(stats.avg_proof_size_bytes > 0);
    assert!(stats.avg_verification_time_ms > 0);
}

#[tokio::test]
async fn test_zk_stark_manager_cleanup() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate proof
    manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).await.unwrap();

    // Check initial state
    let stats = manager.get_zk_stark_stats().await;
    assert_eq!(stats.total_proofs, 1);
    assert_eq!(stats.cached_verifications, 1);

    // Clean up old data
    manager.cleanup_old_data(1).await; // 1 second max age

    // Check state after cleanup (should still have recent data)
    let stats = manager.get_zk_stark_stats().await;
    assert_eq!(stats.total_proofs, 1);
    assert_eq!(stats.cached_verifications, 1);
}

#[tokio::test]
async fn test_zk_stark_proof_serialization() {
    let proof = ZkStarkProof::new(
        "test_circuit",
        vec!["input1".to_string(), "input2".to_string()],
        "test_proof_data",
        1,
        "test_validator"
    ).unwrap();

    // Test serialization
    let serialized = serde_json::to_string(&proof).unwrap();
    let deserialized: ZkStarkProof = serde_json::from_str(&serialized).unwrap();

    assert_eq!(proof.proof_id, deserialized.proof_id);
    assert_eq!(proof.circuit_id, deserialized.circuit_id);
    assert_eq!(proof.public_inputs, deserialized.public_inputs);
    assert_eq!(proof.proof_data, deserialized.proof_data);
    assert_eq!(proof.commitment, deserialized.commitment);
    assert_eq!(proof.round, deserialized.round);
    assert_eq!(proof.validator_id, deserialized.validator_id);
}

#[tokio::test]
async fn test_zk_stark_circuit_serialization() {
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );

    // Test serialization
    let serialized = serde_json::to_string(&circuit).unwrap();
    let deserialized: ZkStarkCircuit = serde_json::from_str(&serialized).unwrap();

    assert_eq!(circuit.circuit_id, deserialized.circuit_id);
    assert_eq!(circuit.circuit_type, deserialized.circuit_type);
    assert_eq!(circuit.constraints, deserialized.constraints);
    assert_eq!(circuit.public_inputs, deserialized.public_inputs);
    assert_eq!(circuit.private_inputs, deserialized.private_inputs);
    assert_eq!(circuit.output_definitions, deserialized.output_definitions);
    assert_eq!(circuit.complexity_score, deserialized.complexity_score);
}

#[tokio::test]
async fn test_zk_stark_aggregation_serialization() {
    let manager = ZkStarkManager::new(1_000_000, 5000);

    // Register circuit
    let circuit = ZkStarkCircuit::new(
        "test_circuit",
        CircuitType::TransactionValidation,
        vec!["constraint1".to_string()],
        vec!["public_input1".to_string()],
        vec!["private_input1".to_string()],
        vec!["output1".to_string()],
    );
    manager.register_circuit(circuit).await.unwrap();

    // Generate proofs
    let proof1 = manager.generate_proof(
        "test_circuit",
        vec!["input1".to_string()],
        "proof_data_1",
        1,
        "validator1"
    ).await.unwrap();

    let proof2 = manager.generate_proof(
        "test_circuit",
        vec!["input2".to_string()],
        "proof_data_2",
        1,
        "validator2"
    ).await.unwrap();

    // Create aggregation
    let aggregation = ZkStarkAggregation::new(
        "test_aggregation",
        1,
        vec![proof1, proof2]
    ).unwrap();

    // Test serialization
    let serialized = serde_json::to_string(&aggregation).unwrap();
    let deserialized: ZkStarkAggregation = serde_json::from_str(&serialized).unwrap();

    assert_eq!(aggregation.aggregation_id, deserialized.aggregation_id);
    assert_eq!(aggregation.round, deserialized.round);
    assert_eq!(aggregation.proofs.len(), deserialized.proofs.len());
    assert_eq!(aggregation.is_valid, deserialized.is_valid);
    assert_eq!(aggregation.total_size_bytes, deserialized.total_size_bytes);
    assert_eq!(aggregation.verification_time_ms, deserialized.verification_time_ms);
} 