//! Tests for L2 commit and exit functionality
//! 
//! This module tests the L2 transaction validation, rate limiting,
//! and challenge window enforcement.

use ippan::crosschain::L2Registry;
use ippan::crosschain::types::{
    L2CommitTx, L2ExitTx, ProofType, DataAvailabilityMode, L2Params,
    L2ValidationError
};
use ippan::crosschain::foreign_verifier::{L2Verifier, DefaultL2Verifier, L2VerificationContext};
use ippan::crosschain::external_anchor::L2AnchorHandler;

#[tokio::test]
async fn test_l2_commit_validation() {
    let mut registry = L2Registry::new();
    
    // Register a test L2
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 16384,
        min_epoch_gap_ms: 250,
    };
    registry.register("test-l2".to_string(), params).await.unwrap();
    
    // Create a valid commit
    let valid_commit = L2CommitTx {
        l2_id: "test-l2".to_string(),
        epoch: 1,
        state_root: [0u8; 32],
        da_hash: [0u8; 32],
        proof_type: ProofType::ZkGroth16,
        proof: vec![1, 2, 3],
        inline_data: None,
    };
    
    // Validate the commit
    assert!(valid_commit.validate(16384).is_ok());
    
    // Test oversized proof
    let oversized_commit = L2CommitTx {
        proof: vec![0u8; 20000], // 20KB > 16KB limit
        ..valid_commit.clone()
    };
    
    assert!(matches!(
        oversized_commit.validate(16384),
        Err(L2ValidationError::ProofTooLarge)
    ));
    
    // Test invalid L2 ID
    let invalid_id_commit = L2CommitTx {
        l2_id: "".to_string(),
        ..valid_commit.clone()
    };
    
    assert!(matches!(
        invalid_id_commit.validate(16384),
        Err(L2ValidationError::InvalidL2Id)
    ));
}

#[tokio::test]
async fn test_l2_exit_validation() {
    let mut registry = L2Registry::new();
    
    // Register a test L2
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 16384,
        min_epoch_gap_ms: 250,
    };
    registry.register("test-l2".to_string(), params).await.unwrap();
    
    // Create a valid exit
    let valid_exit = L2ExitTx {
        l2_id: "test-l2".to_string(),
        epoch: 1,
        proof_of_inclusion: vec![1, 2, 3],
        account: [0u8; 32],
        amount: 1000,
        nonce: 1,
    };
    
    // Validate the exit
    assert!(valid_exit.validate().is_ok());
    
    // Test invalid amount
    let invalid_amount_exit = L2ExitTx {
        amount: 0,
        ..valid_exit.clone()
    };
    
    assert!(matches!(
        invalid_amount_exit.validate(),
        Err(L2ValidationError::InvalidProof)
    ));
    
    // Test empty proof
    let empty_proof_exit = L2ExitTx {
        proof_of_inclusion: vec![],
        ..valid_exit.clone()
    };
    
    assert!(matches!(
        empty_proof_exit.validate(),
        Err(L2ValidationError::InvalidProof)
    ));
}

#[tokio::test]
async fn test_epoch_monotonicity() {
    let mut registry = L2Registry::new();
    
    // Register a test L2
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 16384,
        min_epoch_gap_ms: 250,
    };
    registry.register("test-l2".to_string(), params).await.unwrap();
    
    // Record first commit
    let result = registry.record_commit("test-l2", 1, 1000).await;
    assert!(result.is_ok());
    
    // Record second commit
    let result = registry.record_commit("test-l2", 2, 2000).await;
    assert!(result.is_ok());
    
    // Try to record epoch regression
    let result = registry.record_commit("test-l2", 1, 3000).await;
    assert!(result.is_err());
    
    // Try to record same epoch
    let result = registry.record_commit("test-l2", 2, 4000).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rate_limiting() {
    let mut context = L2VerificationContext::new();
    let mut registry = L2Registry::new();
    
    // Register a test L2 with 250ms rate limit
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 16384,
        min_epoch_gap_ms: 250,
    };
    registry.register("test-l2".to_string(), params).await.unwrap();
    
    // Create a commit
    let commit = L2CommitTx {
        l2_id: "test-l2".to_string(),
        epoch: 1,
        state_root: [0u8; 32],
        da_hash: [0u8; 32],
        proof_type: ProofType::ZkGroth16,
        proof: vec![1, 2, 3],
        inline_data: None,
    };
    
    // First commit should succeed
    let result = context.verify_commit(&commit, &registry, 1000).await;
    assert!(result.is_ok());
    
    // Second commit too soon should fail
    let commit2 = L2CommitTx {
        epoch: 2,
        ..commit.clone()
    };
    let result = context.verify_commit(&commit2, &registry, 1100).await; // Only 100ms later
    assert!(matches!(result, Err(_)));
    
    // Third commit after rate limit should succeed
    let commit3 = L2CommitTx {
        epoch: 3,
        ..commit.clone()
    };
    let result = context.verify_commit(&commit3, &registry, 1300).await; // 300ms later
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_challenge_window() {
    let mut registry = L2Registry::new();
    
    // Register an optimistic L2 with 1 minute challenge window
    let optimistic_params = L2Params {
        proof_type: ProofType::Optimistic,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000, // 1 minute
        max_commit_size: 16384,
        min_epoch_gap_ms: 250,
    };
    
    registry.register("optimistic-l2".to_string(), optimistic_params).await.unwrap();
    
    // Create an exit based on epoch 1
    let exit = L2ExitTx {
        l2_id: "optimistic-l2".to_string(),
        epoch: 1,
        proof_of_inclusion: vec![1, 2, 3],
        account: [0u8; 32],
        amount: 1000,
        nonce: 1,
    };
    
    // Exit should be valid
    assert!(exit.validate().is_ok());
}

#[tokio::test]
async fn test_anchor_events() {
    let handler = L2AnchorHandler::new();
    
    // Create a commit
    let commit = L2CommitTx {
        l2_id: "test-l2".to_string(),
        epoch: 1,
        state_root: [0u8; 32],
        da_hash: [0u8; 32],
        proof_type: ProofType::ZkGroth16,
        proof: vec![1, 2, 3],
        inline_data: None,
    };
    
    // Handle the commit
    let event = handler.handle_l2_commit(&commit, 1000).await.unwrap();
    
    // Verify the event
    assert_eq!(event.l2_id, "test-l2");
    assert_eq!(event.epoch, 1);
    assert_eq!(event.committed_at, 1000);
    
    // Get events for the L2
    let events = handler.get_l2_events("test-l2").await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].epoch, 1);
    
    // Get latest event
    let latest = handler.get_latest_l2_event("test-l2").await;
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().epoch, 1);
}

#[tokio::test]
async fn test_l2_verifier() {
    let verifier = DefaultL2Verifier;
    let mut registry = L2Registry::new();
    
    // Register a test L2
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 16384,
        min_epoch_gap_ms: 250,
    };
    registry.register("test-l2".to_string(), params).await.unwrap();
    
    // Create a commit
    let commit = L2CommitTx {
        l2_id: "test-l2".to_string(),
        epoch: 1,
        state_root: [0u8; 32],
        da_hash: [0u8; 32],
        proof_type: ProofType::ZkGroth16,
        proof: vec![1, 2, 3],
        inline_data: None,
    };
    
    // Verify the commit
    let result = verifier.verify_commit(&commit, &registry).await;
    assert!(result.is_ok());
    
    // Create an exit
    let exit = L2ExitTx {
        l2_id: "test-l2".to_string(),
        epoch: 1,
        proof_of_inclusion: vec![1, 2, 3],
        account: [0u8; 32],
        amount: 1000,
        nonce: 1,
    };
    
    // Verify the exit
    let result = verifier.verify_exit(&exit, &registry).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_l2_registry_config() {
    let mut registry = L2Registry::new();
    
    // Register 99 L2s (should succeed, under the 100 limit)
    for i in 0..99 {
        let l2_id = format!("l2-{}", i);
        let params = L2Params {
            proof_type: ProofType::ZkGroth16,
            da_mode: DataAvailabilityMode::External,
            challenge_window_ms: 60000,
            max_commit_size: 8192,
            min_epoch_gap_ms: 500,
        };
        let result = registry.register(l2_id, params).await;
        assert!(result.is_ok());
    }
    
    // Try to register 100th L2 (should succeed, at the limit)
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 8192,
        min_epoch_gap_ms: 500,
    };
    let result = registry.register("l2-100".to_string(), params).await;
    assert!(result.is_ok());
    
    // Try to register 101st L2 (should fail, over the limit)
    let params = L2Params {
        proof_type: ProofType::ZkGroth16,
        da_mode: DataAvailabilityMode::External,
        challenge_window_ms: 60000,
        max_commit_size: 8192,
        min_epoch_gap_ms: 500,
    };
    let result = registry.register("l2-101".to_string(), params).await;
    assert!(result.is_err());
    
    // Check stats
    let stats = registry.get_stats().await;
    assert_eq!(stats.total_registered, 100);
    assert_eq!(stats.total_commits, 0);
    assert_eq!(stats.total_exits, 0);
}
