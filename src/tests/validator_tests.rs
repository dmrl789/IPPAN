//! Tests for IPPAN Validator system

use ippan::consensus::validator::{
    Validator, ValidatorManager, ValidatorSelectionParams, SelectionMethod,
    SlashingType, SlashingSeverity, ValidatorSet
};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_validator_creation() {
    let validator = Validator::new("test_node", 1000, "test_key", "test_address");
    
    assert_eq!(validator.node_id, "test_node");
    assert_eq!(validator.stake_amount, 1000);
    assert_eq!(validator.public_key, "test_key");
    assert_eq!(validator.address, "test_address");
    assert!(validator.is_active);
    assert_eq!(validator.performance_score, 1.0);
    assert_eq!(validator.uptime_percentage, 100.0);
    assert_eq!(validator.slashing_events, 0);
    assert_eq!(validator.commission_rate, 0.05);
}

#[tokio::test]
async fn test_validator_performance_update() {
    let mut validator = Validator::new("test_node", 1000, "test_key", "test_address");
    
    // Update performance
    validator.update_performance(10, 20);
    
    assert_eq!(validator.total_blocks_produced, 10);
    assert_eq!(validator.total_blocks_validated, 20);
    assert_eq!(validator.performance_score, 1.0); // Perfect performance initially
    
    // Update again
    validator.update_performance(5, 15);
    
    assert_eq!(validator.total_blocks_produced, 15);
    assert_eq!(validator.total_blocks_validated, 35);
}

#[tokio::test]
async fn test_validator_requirements() {
    let validator = Validator::new("test_node", 1000, "test_key", "test_address");
    
    // Should meet requirements with good values
    assert!(validator.meets_requirements(500, 0.5, 80.0));
    
    // Should fail with insufficient stake
    assert!(!validator.meets_requirements(2000, 0.5, 80.0));
    
    // Should fail with low performance
    assert!(!validator.meets_requirements(500, 0.9, 80.0));
    
    // Should fail with low uptime
    assert!(!validator.meets_requirements(500, 0.5, 95.0));
}

#[tokio::test]
async fn test_validator_selection_scores() {
    let validator = Validator::new("test_node", 1000, "test_key", "test_address");
    
    // Test different selection methods
    let stake_score = validator.get_selection_score(&SelectionMethod::StakeBased);
    assert_eq!(stake_score, 1000.0);
    
    let performance_score = validator.get_selection_score(&SelectionMethod::Performance);
    assert_eq!(performance_score, 1.0);
    
    let random_score = validator.get_selection_score(&SelectionMethod::Random);
    assert_eq!(random_score, 1.0);
    
    let hybrid_score = validator.get_selection_score(&SelectionMethod::Hybrid);
    assert!(hybrid_score > 0.0);
}

#[tokio::test]
async fn test_validator_manager_creation() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Test initial state
    let stats = manager.get_validator_stats().await;
    assert_eq!(stats.total_validators, 0);
    assert_eq!(stats.active_validators, 0);
    assert_eq!(stats.total_stake, 0);
}

#[tokio::test]
async fn test_validator_registration() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators
    assert!(manager.register_validator("node1", 1000, "key1", "addr1").await.is_ok());
    assert!(manager.register_validator("node2", 2000, "key2", "addr2").await.is_ok());
    assert!(manager.register_validator("node3", 1500, "key3", "addr3").await.is_ok());
    
    // Try to register with insufficient stake
    assert!(manager.register_validator("node4", 500, "key4", "addr4").await.is_err());
    
    // Try to register duplicate
    assert!(manager.register_validator("node1", 1000, "key1", "addr1").await.is_err());
    
    // Check stats
    let stats = manager.get_validator_stats().await;
    assert_eq!(stats.total_validators, 3);
    assert_eq!(stats.active_validators, 3);
    assert_eq!(stats.total_stake, 4500);
}

#[tokio::test]
async fn test_validator_selection() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 3,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators with different stakes
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    manager.register_validator("node4", 3000, "key4", "addr4").await.unwrap();
    
    // Select validators
    let validator_set = manager.select_validators(1).await;
    
    assert_eq!(validator_set.round, 1);
    assert_eq!(validator_set.validators.len(), 3); // Max validators
    assert!(!validator_set.primary_validator.is_empty());
    assert_eq!(validator_set.backup_validators.len(), 2);
    
    // Should select highest stake validators first
    assert!(validator_set.validators.contains(&"node4".to_string()));
    assert!(validator_set.validators.contains(&"node2".to_string()));
    assert!(validator_set.validators.contains(&"node3".to_string()));
}

#[tokio::test]
async fn test_random_validator_selection() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 3,
        selection_method: SelectionMethod::Random,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    manager.register_validator("node4", 3000, "key4", "addr4").await.unwrap();
    
    // Select validators multiple times
    let set1 = manager.select_validators(1).await;
    let set2 = manager.select_validators(2).await;
    
    // Should have same number of validators
    assert_eq!(set1.validators.len(), 3);
    assert_eq!(set2.validators.len(), 3);
    
    // Should have different seeds
    assert_ne!(set1.seed, set2.seed);
}

#[tokio::test]
async fn test_performance_based_selection() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 3,
        selection_method: SelectionMethod::Performance,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    
    // Update performance
    manager.update_validator_performance("node1", 10, 20).await.unwrap();
    manager.update_validator_performance("node2", 5, 15).await.unwrap();
    manager.update_validator_performance("node3", 15, 25).await.unwrap();
    
    // Select validators
    let validator_set = manager.select_validators(1).await;
    
    assert_eq!(validator_set.validators.len(), 3);
    // Should prioritize performance over stake
}

#[tokio::test]
async fn test_validator_rotation() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 3,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 10, // Rotate every 10 rounds
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    
    // Select for round 1 (should create new set)
    let set1 = manager.rotate_validators(1).await;
    
    // Select for round 5 (should keep same set)
    let set5 = manager.rotate_validators(5).await;
    assert_eq!(set1.validators, set5.validators);
    
    // Select for round 10 (should rotate)
    let set10 = manager.rotate_validators(10).await;
    assert_ne!(set1.seed, set10.seed);
}

#[tokio::test]
async fn test_slashing_events() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validator
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    
    // Record minor slashing event
    assert!(manager.record_slashing_event(
        "node1",
        SlashingType::DoubleSigning,
        "evidence1",
        SlashingSeverity::Minor
    ).await.is_ok());
    
    // Check validator state
    let validator = manager.get_validator("node1").await.unwrap();
    assert_eq!(validator.slashing_events, 1);
    assert!(validator.is_active); // Should still be active
    
    // Record critical slashing event
    assert!(manager.record_slashing_event(
        "node1",
        SlashingType::NetworkAttack,
        "evidence2",
        SlashingSeverity::Critical
    ).await.is_ok());
    
    // Check validator state
    let validator = manager.get_validator("node1").await.unwrap();
    assert_eq!(validator.slashing_events, 2);
    assert!(!validator.is_active); // Should be deactivated
}

#[tokio::test]
async fn test_validator_stake_updates() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validator
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    
    // Update stake
    assert!(manager.update_stake("node1", 2000).await.is_ok());
    
    // Check updated stake
    let validator = manager.get_validator("node1").await.unwrap();
    assert_eq!(validator.stake_amount, 2000);
    
    // Try to update to insufficient stake
    assert!(manager.update_stake("node1", 500).await.is_err());
}

#[tokio::test]
async fn test_validator_unregistration() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validator
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    
    // Check initial state
    let validator = manager.get_validator("node1").await.unwrap();
    assert!(validator.is_active);
    
    // Unregister validator
    assert!(manager.unregister_validator("node1").await.is_ok());
    
    // Check state after unregistration
    let validator = manager.get_validator("node1").await.unwrap();
    assert!(!validator.is_active);
    
    // Try to unregister non-existent validator
    assert!(manager.unregister_validator("nonexistent").await.is_err());
}

#[tokio::test]
async fn test_validator_set_queries() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 3,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    
    // Select validators
    let validator_set = manager.select_validators(1).await;
    
    // Test validator queries
    assert!(manager.is_validator("node1").await);
    assert!(manager.is_validator("node2").await);
    assert!(manager.is_validator("node3").await);
    assert!(!manager.is_validator("node4").await);
    
    // Test primary validator
    assert!(manager.is_primary_validator(&validator_set.primary_validator).await);
    assert!(!manager.is_primary_validator("node2").await);
}

#[tokio::test]
async fn test_validator_statistics() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 10,
        selection_method: SelectionMethod::StakeBased,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    
    // Update performance
    manager.update_validator_performance("node1", 10, 20).await.unwrap();
    manager.update_validator_performance("node2", 15, 25).await.unwrap();
    manager.update_validator_performance("node3", 20, 30).await.unwrap();
    
    // Get statistics
    let stats = manager.get_validator_stats().await;
    
    assert_eq!(stats.total_validators, 3);
    assert_eq!(stats.active_validators, 3);
    assert_eq!(stats.total_stake, 4500);
    assert!(stats.avg_performance > 0.0);
    assert!(stats.avg_uptime > 0.0);
    assert_eq!(stats.total_slashing_events, 0);
}

#[tokio::test]
async fn test_hybrid_selection() {
    let params = ValidatorSelectionParams {
        min_stake: 1000,
        max_validators: 3,
        selection_method: SelectionMethod::Hybrid,
        rotation_interval: 100,
        performance_threshold: 0.8,
        uptime_threshold: 90.0,
    };
    
    let manager = ValidatorManager::new(params);
    
    // Register validators with different characteristics
    manager.register_validator("node1", 1000, "key1", "addr1").await.unwrap();
    manager.register_validator("node2", 2000, "key2", "addr2").await.unwrap();
    manager.register_validator("node3", 1500, "key3", "addr3").await.unwrap();
    
    // Update performance differently
    manager.update_validator_performance("node1", 20, 30).await.unwrap(); // High performance
    manager.update_validator_performance("node2", 10, 20).await.unwrap(); // Medium performance
    manager.update_validator_performance("node3", 5, 10).await.unwrap();  // Low performance
    
    // Select validators
    let validator_set = manager.select_validators(1).await;
    
    assert_eq!(validator_set.validators.len(), 3);
    // Should balance stake and performance
} 