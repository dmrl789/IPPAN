//! Tests for the consensus module

use crate::{
    consensus::{
        blockdag::{BlockDAG, Block, BlockHeader},
        hashtimer::{HashTimer, HashTimerConfig},
        ippan_time::{IppanTime, IppanTimeConfig},
        randomness::{RandomnessManager, RandomnessConfig},
        round::{RoundManager, Round, RoundStatus},
    },
    utils::{crypto::sha256_hash, time::current_time_secs},
    NodeId, BlockHash, TransactionHash,
};

use super::create_test_node_id;

/// Test BlockDAG creation and basic operations
#[tokio::test]
async fn test_blockdag_creation() {
    let blockdag = BlockDAG::new();
    assert_eq!(blockdag.get_block_count().await, 0);
    assert_eq!(blockdag.get_tip_count().await, 0);
}

/// Test block creation and addition to BlockDAG
#[tokio::test]
async fn test_block_creation_and_addition() {
    let mut blockdag = BlockDAG::new();
    let node_id = create_test_node_id();
    
    let header = BlockHeader {
        version: 1,
        timestamp: current_time_secs(),
        node_id,
        parent_hashes: vec![],
        merkle_root: [0u8; 32],
        difficulty: 1000,
        nonce: 0,
    };
    
    let block = Block {
        header,
        transactions: vec![],
        hash: [0u8; 32],
    };
    
    let result = blockdag.add_block(block).await;
    assert!(result.is_ok());
    assert_eq!(blockdag.get_block_count().await, 1);
    assert_eq!(blockdag.get_tip_count().await, 1);
}

/// Test block with parent references
#[tokio::test]
async fn test_block_with_parents() {
    let mut blockdag = BlockDAG::new();
    let node_id = create_test_node_id();
    
    // Create first block
    let header1 = BlockHeader {
        version: 1,
        timestamp: current_time_secs(),
        node_id,
        parent_hashes: vec![],
        merkle_root: [0u8; 32],
        difficulty: 1000,
        nonce: 0,
    };
    
    let block1 = Block {
        header: header1,
        transactions: vec![],
        hash: [1u8; 32],
    };
    
    blockdag.add_block(block1).await.unwrap();
    
    // Create second block with parent reference
    let header2 = BlockHeader {
        version: 1,
        timestamp: current_time_secs() + 1,
        node_id,
        parent_hashes: vec![[1u8; 32]],
        merkle_root: [0u8; 32],
        difficulty: 1000,
        nonce: 0,
    };
    
    let block2 = Block {
        header: header2,
        transactions: vec![],
        hash: [2u8; 32],
    };
    
    let result = blockdag.add_block(block2).await;
    assert!(result.is_ok());
    assert_eq!(blockdag.get_block_count().await, 2);
    assert_eq!(blockdag.get_tip_count().await, 1); // Only one tip now
}

/// Test HashTimer creation and validation
#[tokio::test]
async fn test_hashtimer_creation() {
    let config = HashTimerConfig {
        precision_nanos: 100, // 0.1 microsecond
        max_clock_skew: 1000000, // 1ms
        min_validators: 3,
    };
    
    let hashtimer = HashTimer::new(config);
    assert_eq!(hashtimer.get_config().precision_nanos, 100);
    assert_eq!(hashtimer.get_config().max_clock_skew, 1000000);
}

/// Test HashTimer time calculation
#[tokio::test]
async fn test_hashtimer_time_calculation() {
    let config = HashTimerConfig {
        precision_nanos: 100,
        max_clock_skew: 1000000,
        min_validators: 3,
    };
    
    let mut hashtimer = HashTimer::new(config);
    let node_id = create_test_node_id();
    let timestamp = current_time_secs();
    
    // Add time samples
    hashtimer.add_time_sample(node_id, timestamp * 1_000_000_000).await;
    hashtimer.add_time_sample(create_test_node_id(), (timestamp + 1) * 1_000_000_000).await;
    hashtimer.add_time_sample(create_test_node_id(), (timestamp - 1) * 1_000_000_000).await;
    
    let calculated_time = hashtimer.calculate_median_time().await;
    assert!(calculated_time.is_some());
    assert_eq!(calculated_time.unwrap(), timestamp * 1_000_000_000);
}

/// Test IPPAN Time creation and operations
#[tokio::test]
async fn test_ippan_time_creation() {
    let config = IppanTimeConfig {
        precision_nanos: 100,
        max_clock_skew: 1000000,
        min_validators: 3,
        update_interval: 1000,
    };
    
    let ippan_time = IppanTime::new(config);
    assert_eq!(ippan_time.get_config().precision_nanos, 100);
    assert_eq!(ippan_time.get_config().update_interval, 1000);
}

/// Test IPPAN Time synchronization
#[tokio::test]
async fn test_ippan_time_sync() {
    let config = IppanTimeConfig {
        precision_nanos: 100,
        max_clock_skew: 1000000,
        min_validators: 3,
        update_interval: 1000,
    };
    
    let mut ippan_time = IppanTime::new(config);
    let node_id = create_test_node_id();
    let timestamp = current_time_secs();
    
    // Add time samples from multiple nodes
    for i in 0..5 {
        let sample_node_id = create_test_node_id();
        let sample_time = (timestamp + i as u64) * 1_000_000_000;
        ippan_time.add_time_sample(sample_node_id, sample_time).await;
    }
    
    let synced_time = ippan_time.get_current_time().await;
    assert!(synced_time > 0);
    
    // Test time formatting
    let formatted = ippan_time.format_time(synced_time).await;
    assert!(!formatted.is_empty());
}

/// Test RandomnessManager creation and operations
#[tokio::test]
async fn test_randomness_manager_creation() {
    let config = RandomnessConfig {
        seed_entropy_bits: 256,
        update_interval: 1000,
        min_participants: 3,
        verification_threshold: 0.67,
    };
    
    let randomness_manager = RandomnessManager::new(config);
    assert_eq!(randomness_manager.get_config().seed_entropy_bits, 256);
    assert_eq!(randomness_manager.get_config().min_participants, 3);
}

/// Test randomness generation
#[tokio::test]
async fn test_randomness_generation() {
    let config = RandomnessConfig {
        seed_entropy_bits: 256,
        update_interval: 1000,
        min_participants: 3,
        verification_threshold: 0.67,
    };
    
    let mut randomness_manager = RandomnessManager::new(config);
    let node_id = create_test_node_id();
    
    // Generate random value
    let random_value = randomness_manager.generate_random_value(node_id).await;
    assert!(random_value > 0);
    
    // Test validator selection
    let validators = vec![create_test_node_id(), create_test_node_id(), create_test_node_id()];
    let selected = randomness_manager.select_validators(&validators, 2).await;
    assert_eq!(selected.len(), 2);
    assert!(selected.iter().all(|v| validators.contains(v)));
}

/// Test RoundManager creation and operations
#[tokio::test]
async fn test_round_manager_creation() {
    let round_manager = RoundManager::new();
    assert_eq!(round_manager.get_current_round().await, 0);
    assert_eq!(round_manager.get_round_count().await, 0);
}

/// Test round creation and management
#[tokio::test]
async fn test_round_creation() {
    let mut round_manager = RoundManager::new();
    let node_id = create_test_node_id();
    
    let round = Round {
        round_number: 1,
        start_time: current_time_secs(),
        end_time: current_time_secs() + 1000,
        status: RoundStatus::Active,
        validators: vec![node_id],
        blocks: vec![],
        transactions: vec![],
    };
    
    let result = round_manager.add_round(round).await;
    assert!(result.is_ok());
    assert_eq!(round_manager.get_round_count().await, 1);
    
    let current_round = round_manager.get_round(1).await;
    assert!(current_round.is_some());
    assert_eq!(current_round.unwrap().round_number, 1);
}

/// Test round status transitions
#[tokio::test]
async fn test_round_status_transitions() {
    let mut round_manager = RoundManager::new();
    let node_id = create_test_node_id();
    
    let mut round = Round {
        round_number: 1,
        start_time: current_time_secs(),
        end_time: current_time_secs() + 1000,
        status: RoundStatus::Active,
        validators: vec![node_id],
        blocks: vec![],
        transactions: vec![],
    };
    
    round_manager.add_round(round.clone()).await.unwrap();
    
    // Test status transition to completed
    let result = round_manager.complete_round(1).await;
    assert!(result.is_ok());
    
    let updated_round = round_manager.get_round(1).await.unwrap();
    assert!(matches!(updated_round.status, RoundStatus::Completed));
}

/// Test block validation in consensus
#[tokio::test]
async fn test_block_validation() {
    let mut blockdag = BlockDAG::new();
    let node_id = create_test_node_id();
    
    let header = BlockHeader {
        version: 1,
        timestamp: current_time_secs(),
        node_id,
        parent_hashes: vec![],
        merkle_root: [0u8; 32],
        difficulty: 1000,
        nonce: 0,
    };
    
    let block = Block {
        header,
        transactions: vec![],
        hash: [0u8; 32],
    };
    
    // Test block validation
    let is_valid = blockdag.validate_block(&block).await;
    assert!(is_valid);
}

/// Test consensus finalization
#[tokio::test]
async fn test_consensus_finalization() {
    let mut blockdag = BlockDAG::new();
    let node_id = create_test_node_id();
    
    // Create multiple blocks
    for i in 0..5 {
        let header = BlockHeader {
            version: 1,
            timestamp: current_time_secs() + i,
            node_id,
            parent_hashes: if i == 0 { vec![] } else { vec![[(i-1) as u8; 32]] },
            merkle_root: [0u8; 32],
            difficulty: 1000,
            nonce: 0,
        };
        
        let block = Block {
            header,
            transactions: vec![],
            hash: [i as u8; 32],
        };
        
        blockdag.add_block(block).await.unwrap();
    }
    
    // Test finalization
    let finalized_blocks = blockdag.get_finalized_blocks().await;
    assert!(!finalized_blocks.is_empty());
}

/// Test consensus performance metrics
#[tokio::test]
async fn test_consensus_performance() {
    let blockdag = BlockDAG::new();
    let randomness_manager = RandomnessManager::new(RandomnessConfig::default());
    let round_manager = RoundManager::new();
    
    // Test performance metrics
    let block_count = blockdag.get_block_count().await;
    let round_count = round_manager.get_round_count().await;
    
    assert_eq!(block_count, 0);
    assert_eq!(round_count, 0);
}

/// Test consensus error handling
#[tokio::test]
async fn test_consensus_error_handling() {
    let mut blockdag = BlockDAG::new();
    
    // Test adding invalid block
    let invalid_block = Block {
        header: BlockHeader {
            version: 0, // Invalid version
            timestamp: 0,
            node_id: [0u8; 32],
            parent_hashes: vec![],
            merkle_root: [0u8; 32],
            difficulty: 0,
            nonce: 0,
        },
        transactions: vec![],
        hash: [0u8; 32],
    };
    
    let is_valid = blockdag.validate_block(&invalid_block).await;
    assert!(!is_valid);
}

/// Test consensus with multiple validators
#[tokio::test]
async fn test_multi_validator_consensus() {
    let mut round_manager = RoundManager::new();
    let validators = vec![
        create_test_node_id(),
        create_test_node_id(),
        create_test_node_id(),
    ];
    
    let round = Round {
        round_number: 1,
        start_time: current_time_secs(),
        end_time: current_time_secs() + 1000,
        status: RoundStatus::Active,
        validators: validators.clone(),
        blocks: vec![],
        transactions: vec![],
    };
    
    round_manager.add_round(round).await.unwrap();
    
    let retrieved_round = round_manager.get_round(1).await.unwrap();
    assert_eq!(retrieved_round.validators.len(), 3);
    assert!(validators.iter().all(|v| retrieved_round.validators.contains(v)));
}

/// Test consensus time synchronization
#[tokio::test]
async fn test_consensus_time_sync() {
    let config = IppanTimeConfig {
        precision_nanos: 100,
        max_clock_skew: 1000000,
        min_validators: 3,
        update_interval: 1000,
    };
    
    let mut ippan_time = IppanTime::new(config);
    
    // Add time samples from multiple nodes
    let base_time = current_time_secs() * 1_000_000_000;
    for i in 0..5 {
        let sample_time = base_time + (i * 1000); // Small variations
        ippan_time.add_time_sample(create_test_node_id(), sample_time).await;
    }
    
    let synced_time = ippan_time.get_current_time().await;
    assert!(synced_time >= base_time);
    assert!(synced_time <= base_time + 4000); // Should be within range
}
