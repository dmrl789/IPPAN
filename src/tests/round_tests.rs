//! Tests for IPPAN Round Management system

use ippan::consensus::round::{
    Round, RoundManager, RoundState, Proposal, Vote, RoundTimeoutConfig, RoundEvent
};
use ippan::consensus::hashtimer::HashTimer;
use ippan::consensus::validator::{ValidatorManager, ValidatorSelectionParams, SelectionMethod};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_round_creation() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let round = Round::new(1, validator_set, 30000, 2);

    assert_eq!(round.round_number, 1);
    assert_eq!(round.state, RoundState::Initializing);
    assert_eq!(round.timeout_duration_ms, 30000);
    assert_eq!(round.min_votes_required, 2);
    assert_eq!(round.received_votes, 0);
    assert_eq!(round.received_proposals, 0);
    assert!(round.consensus_hash.is_none());
}

#[tokio::test]
async fn test_round_state_transitions() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let mut round = Round::new(1, validator_set, 30000, 2);

    // Valid transitions
    assert!(round.transition_state(RoundState::Collecting).is_ok());
    assert_eq!(round.state, RoundState::Collecting);

    assert!(round.transition_state(RoundState::Validating).is_ok());
    assert_eq!(round.state, RoundState::Validating);

    assert!(round.transition_state(RoundState::Finalizing).is_ok());
    assert_eq!(round.state, RoundState::Finalizing);

    assert!(round.transition_state(RoundState::Completed).is_ok());
    assert_eq!(round.state, RoundState::Completed);

    // Invalid transitions
    assert!(round.transition_state(RoundState::Collecting).is_err());
}

#[tokio::test]
async fn test_round_proposal_management() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let mut round = Round::new(1, validator_set, 30000, 2);
    round.transition_state(RoundState::Collecting).unwrap();

    let hashtimer = HashTimer::new("node1", 1, 1);
    let proposal = Proposal {
        validator_id: "node1".to_string(),
        round_number: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        data_hash: "data_hash_1".to_string(),
        signature: "signature_1".to_string(),
        hashtimer,
        priority: 1,
        is_valid: true,
    };

    // Add proposal
    assert!(round.add_proposal(proposal).is_ok());
    assert_eq!(round.received_proposals, 1);
    assert_eq!(round.proposals.len(), 1);

    // Try to add proposal in wrong state
    round.transition_state(RoundState::Validating).unwrap();
    let hashtimer2 = HashTimer::new("node2", 1, 2);
    let proposal2 = Proposal {
        validator_id: "node2".to_string(),
        round_number: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        data_hash: "data_hash_2".to_string(),
        signature: "signature_2".to_string(),
        hashtimer: hashtimer2,
        priority: 2,
        is_valid: true,
    };

    assert!(round.add_proposal(proposal2).is_err());
}

#[tokio::test]
async fn test_round_vote_management() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let mut round = Round::new(1, validator_set, 30000, 2);
    round.transition_state(RoundState::Validating).unwrap();

    let hashtimer = HashTimer::new("node1", 1, 1);
    let vote = Vote {
        validator_id: "node1".to_string(),
        round_number: 1,
        proposal_hash: "proposal_hash_1".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        signature: "signature_1".to_string(),
        hashtimer,
        is_approval: true,
        is_valid: true,
    };

    // Add vote
    assert!(round.add_vote(vote).is_ok());
    assert_eq!(round.received_votes, 1);
    assert_eq!(round.votes.len(), 1);

    // Try to add vote in wrong state
    round.transition_state(RoundState::Finalizing).unwrap();
    let hashtimer2 = HashTimer::new("node2", 1, 2);
    let vote2 = Vote {
        validator_id: "node2".to_string(),
        round_number: 1,
        proposal_hash: "proposal_hash_2".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        signature: "signature_2".to_string(),
        hashtimer: hashtimer2,
        is_approval: true,
        is_valid: true,
    };

    assert!(round.add_vote(vote2).is_err());
}

#[tokio::test]
async fn test_round_sufficient_votes() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let round = Round::new(1, validator_set, 30000, 2);

    // Should require 2 votes (min_votes_required)
    assert!(!round.has_sufficient_votes());

    let mut round_with_votes = round;
    round_with_votes.received_votes = 2;
    assert!(round_with_votes.has_sufficient_votes());
}

#[tokio::test]
async fn test_round_sufficient_proposals() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let round = Round::new(1, validator_set, 30000, 2);

    // Should require 2 proposals (3 validators / 2 + 1)
    assert!(!round.has_sufficient_proposals());

    let mut round_with_proposals = round;
    round_with_proposals.received_proposals = 2;
    assert!(round_with_proposals.has_sufficient_proposals());
}

#[tokio::test]
async fn test_round_timeout() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let mut round = Round::new(1, validator_set, 1000, 2); // 1 second timeout

    // Should not timeout immediately
    assert!(!round.has_timed_out());

    // Wait for timeout (in real test, you'd use a shorter timeout)
    // For this test, we'll simulate by setting start_time to past
    round.start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64 - 2000; // 2 seconds ago

    assert!(round.has_timed_out());
}

#[tokio::test]
async fn test_round_consensus_hash() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let mut round = Round::new(1, validator_set, 30000, 2);

    // No proposals should return None
    assert!(round.calculate_consensus_hash().is_none());

    // Add proposals
    round.transition_state(RoundState::Collecting).unwrap();

    let hashtimer1 = HashTimer::new("node1", 1, 1);
    let proposal1 = Proposal {
        validator_id: "node1".to_string(),
        round_number: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        data_hash: "data_hash_1".to_string(),
        signature: "signature_1".to_string(),
        hashtimer: hashtimer1,
        priority: 1,
        is_valid: true,
    };

    let hashtimer2 = HashTimer::new("node2", 1, 2);
    let proposal2 = Proposal {
        validator_id: "node2".to_string(),
        round_number: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        data_hash: "data_hash_2".to_string(),
        signature: "signature_2".to_string(),
        hashtimer: hashtimer2,
        priority: 2, // Higher priority
        is_valid: true,
    };

    round.add_proposal(proposal1).unwrap();
    round.add_proposal(proposal2).unwrap();

    // Should return consensus hash for highest priority proposal
    let consensus_hash = round.calculate_consensus_hash();
    assert!(consensus_hash.is_some());
    assert!(consensus_hash.unwrap().contains("data_hash_2"));
}

#[tokio::test]
async fn test_round_completion_validation() {
    let validator_set = ippan::consensus::validator::ValidatorSet {
        round: 1,
        validators: vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        primary_validator: "node1".to_string(),
        backup_validators: vec!["node2".to_string(), "node3".to_string()],
        selection_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        seed: 12345,
    };

    let mut round = Round::new(1, validator_set, 30000, 2);

    // Should not be complete initially
    assert!(!round.validate_completion());

    // Add sufficient proposals and votes
    round.received_proposals = 2;
    round.received_votes = 2;
    round.consensus_hash = Some("consensus_hash".to_string());

    // Should be complete
    assert!(round.validate_completion());

    // Should not be complete if timed out
    round.start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64 - 40000; // Past timeout

    assert!(!round.validate_completion());
}

#[tokio::test]
async fn test_round_manager_creation() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };

    let validator_manager = Arc::new(ValidatorManager::new(params));
    let timeout_config = RoundTimeoutConfig {
        proposal_timeout_ms: 5000,
        validation_timeout_ms: 10000,
        finalization_timeout_ms: 15000,
        max_round_duration_ms: 30000,
    };

    let manager = RoundManager::new(validator_manager, timeout_config);

    // Test initial state
    let current_round = manager.get_current_round().await;
    assert!(current_round.is_none());

    let stats = manager.get_round_stats().await;
    assert_eq!(stats.total_rounds, 0);
    assert_eq!(stats.completed_rounds, 0);
}

#[tokio::test]
async fn test_round_manager_start_round() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };

    let validator_manager = Arc::new(ValidatorManager::new(params));
    let timeout_config = RoundTimeoutConfig {
        proposal_timeout_ms: 5000,
        validation_timeout_ms: 10000,
        finalization_timeout_ms: 15000,
        max_round_duration_ms: 30000,
    };

    let manager = RoundManager::new(validator_manager, timeout_config);

    // Start round
    assert!(manager.start_round(1).await.is_ok());

    // Check current round
    let current_round = manager.get_current_round().await;
    assert!(current_round.is_some());
    
    let round = current_round.unwrap();
    assert_eq!(round.round_number, 1);
    assert_eq!(round.state, RoundState::Collecting);

    // Try to start another round while one is active
    assert!(manager.start_round(2).await.is_err());
}

#[tokio::test]
async fn test_round_manager_proposal_submission() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };

    let validator_manager = Arc::new(ValidatorManager::new(params));
    let timeout_config = RoundTimeoutConfig {
        proposal_timeout_ms: 5000,
        validation_timeout_ms: 10000,
        finalization_timeout_ms: 15000,
        max_round_duration_ms: 30000,
    };

    let manager = RoundManager::new(validator_manager, timeout_config);

    // Start round
    manager.start_round(1).await.unwrap();

    // Create proposal
    let hashtimer = HashTimer::new("node1", 1, 1);
    let proposal = Proposal {
        validator_id: "node1".to_string(),
        round_number: 1,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        data_hash: "data_hash_1".to_string(),
        signature: "signature_1".to_string(),
        hashtimer,
        priority: 1,
        is_valid: true,
    };

    // Submit proposal
    assert!(manager.submit_proposal(proposal).await.is_ok());

    // Check round state
    let current_round = manager.get_current_round().await.unwrap();
    assert_eq!(current_round.received_proposals, 1);
}

#[tokio::test]
async fn test_round_manager_vote_submission() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };

    let validator_manager = Arc::new(ValidatorManager::new(params));
    let timeout_config = RoundTimeoutConfig {
        proposal_timeout_ms: 5000,
        validation_timeout_ms: 10000,
        finalization_timeout_ms: 15000,
        max_round_duration_ms: 30000,
    };

    let manager = RoundManager::new(validator_manager, timeout_config);

    // Start round
    manager.start_round(1).await.unwrap();

    // Submit enough proposals to move to validating state
    for i in 0..3 {
        let hashtimer = HashTimer::new(&format!("node{}", i), 1, i);
        let proposal = Proposal {
            validator_id: format!("node{}", i),
            round_number: 1,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            data_hash: format!("data_hash_{}", i),
            signature: format!("signature_{}", i),
            hashtimer,
            priority: i as u32,
            is_valid: true,
        };
        manager.submit_proposal(proposal).await.unwrap();
    }

    // Create vote
    let hashtimer = HashTimer::new("node1", 1, 1);
    let vote = Vote {
        validator_id: "node1".to_string(),
        round_number: 1,
        proposal_hash: "proposal_hash_1".to_string(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        signature: "signature_1".to_string(),
        hashtimer,
        is_approval: true,
        is_valid: true,
    };

    // Submit vote
    assert!(manager.submit_vote(vote).await.is_ok());

    // Check round state
    let current_round = manager.get_current_round().await.unwrap();
    assert_eq!(current_round.received_votes, 1);
}

#[tokio::test]
async fn test_round_manager_completion() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };

    let validator_manager = Arc::new(ValidatorManager::new(params));
    let timeout_config = RoundTimeoutConfig {
        proposal_timeout_ms: 5000,
        validation_timeout_ms: 10000,
        finalization_timeout_ms: 15000,
        max_round_duration_ms: 30000,
    };

    let manager = RoundManager::new(validator_manager, timeout_config);

    // Start round
    manager.start_round(1).await.unwrap();

    // Complete round
    assert!(manager.complete_round().await.is_ok());

    // Check that round is moved to history
    let current_round = manager.get_current_round().await;
    assert!(current_round.is_none());

    let history = manager.get_round_history().await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].round_number, 1);
}

#[tokio::test]
async fn test_round_manager_statistics() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };

    let validator_manager = Arc::new(ValidatorManager::new(params));
    let timeout_config = RoundTimeoutConfig {
        proposal_timeout_ms: 5000,
        validation_timeout_ms: 10000,
        finalization_timeout_ms: 15000,
        max_round_duration_ms: 30000,
    };

    let manager = RoundManager::new(validator_manager, timeout_config);

    // Start and complete a round
    manager.start_round(1).await.unwrap();
    manager.complete_round().await.unwrap();

    // Get statistics
    let stats = manager.get_round_stats().await;
    assert_eq!(stats.total_rounds, 1);
    assert_eq!(stats.completed_rounds, 0); // Should be 0 since round failed
    assert_eq!(stats.failed_rounds, 1);
    assert!(stats.active_round_number.is_none());
} 