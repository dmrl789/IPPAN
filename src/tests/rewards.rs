//! Tests for the rewards module

use crate::{
    staking::{
        rewards::{RewardsManager, RewardParams, PerformanceMetrics, RewardType, StorageMetrics, NetworkMetrics},
        Stake, StakeStatus,
    },
    utils::time::current_time_secs,
    NodeId,
    MIN_STAKE_AMOUNT,
};

use super::create_test_node_id;

/// Test reward calculation with default parameters
#[tokio::test]
async fn test_reward_calculation_default() {
    let rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    let stake = Stake {
        node_id,
        amount: MIN_STAKE_AMOUNT,
        start_time: current_time_secs(),
        end_time: None,
        status: StakeStatus::Active,
        last_reward_time: current_time_secs(),
        total_rewards: 0,
        performance_score: 1.0,
        uptime_percentage: 100.0,
    };

    let calculation = rewards_manager.calculate_individual_reward(&stake).await.unwrap();
    
    assert_eq!(calculation.node_id, node_id);
    assert!(calculation.amount > 0);
    assert_eq!(calculation.stake_amount, MIN_STAKE_AMOUNT);
    assert!(calculation.performance_score >= 0.0 && calculation.performance_score <= 1.0);
    assert!(calculation.uptime_percentage >= 0.0 && calculation.uptime_percentage <= 100.0);
}

/// Test reward calculation with custom parameters
#[tokio::test]
async fn test_reward_calculation_custom_params() {
    let params = RewardParams {
        base_reward_rate: 0.001,
        performance_multiplier: 2.0,
        uptime_multiplier: 1.5,
        stake_multiplier: 1.2,
        min_reward_threshold: 1000,
        max_reward_per_node: 1000000,
        distribution_period: 86400,
    };
    
    let rewards_manager = RewardsManager::with_params(params);
    let node_id = create_test_node_id();
    
    let stake = Stake {
        node_id,
        amount: MIN_STAKE_AMOUNT * 2,
        start_time: current_time_secs(),
        end_time: None,
        status: StakeStatus::Active,
        last_reward_time: current_time_secs(),
        total_rewards: 0,
        performance_score: 0.9,
        uptime_percentage: 95.0,
    };

    let calculation = rewards_manager.calculate_individual_reward(&stake).await.unwrap();
    
    assert_eq!(calculation.node_id, node_id);
    assert!(calculation.amount >= 1000); // Should meet minimum threshold
    assert_eq!(calculation.stake_amount, MIN_STAKE_AMOUNT * 2);
    assert_eq!(calculation.performance_score, 0.9);
    assert_eq!(calculation.uptime_percentage, 95.0);
}

/// Test performance metrics update
#[tokio::test]
async fn test_performance_metrics_update() {
    let mut rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    let metrics = PerformanceMetrics {
        node_id,
        performance_score: 0.85,
        uptime_percentage: 92.5,
        storage_availability: 0.95,
        network_contribution: 0.88,
        consensus_participation: 0.91,
        last_update: current_time_secs(),
    };
    
    rewards_manager.update_performance_metrics(node_id, metrics.clone()).await;
    
    let retrieved_metrics = rewards_manager.get_performance_metrics(&node_id).await;
    assert_eq!(retrieved_metrics.node_id, node_id);
    assert_eq!(retrieved_metrics.performance_score, 0.85);
    assert_eq!(retrieved_metrics.uptime_percentage, 92.5);
    assert_eq!(retrieved_metrics.storage_availability, 0.95);
    assert_eq!(retrieved_metrics.network_contribution, 0.88);
    assert_eq!(retrieved_metrics.consensus_participation, 0.91);
}

/// Test reward event recording
#[tokio::test]
async fn test_reward_event_recording() {
    let mut rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    let reward_amount = 50000;
    
    rewards_manager.record_reward_event(node_id, reward_amount, RewardType::Staking).await;
    
    let history = rewards_manager.get_reward_history(&node_id, None).await;
    assert_eq!(history.len(), 1);
    
    let event = &history[0];
    assert_eq!(event.node_id, node_id);
    assert_eq!(event.amount, reward_amount);
    assert!(matches!(event.reward_type, RewardType::Staking));
}

/// Test reward history with limit
#[tokio::test]
async fn test_reward_history_with_limit() {
    let mut rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    // Record multiple events
    for i in 0..10 {
        rewards_manager.record_reward_event(node_id, 1000 * (i + 1), RewardType::Staking).await;
    }
    
    let history = rewards_manager.get_reward_history(&node_id, Some(5)).await;
    assert_eq!(history.len(), 5);
    
    // Should get the most recent events
    assert_eq!(history[0].amount, 10000);
    assert_eq!(history[4].amount, 6000);
}

/// Test total rewards calculation
#[tokio::test]
async fn test_total_rewards_calculation() {
    let mut rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    // Record multiple reward events
    rewards_manager.record_reward_event(node_id, 1000, RewardType::Staking).await;
    rewards_manager.record_reward_event(node_id, 2000, RewardType::Performance).await;
    rewards_manager.record_reward_event(node_id, 1500, RewardType::Storage).await;
    
    let total_rewards = rewards_manager.get_total_rewards(&node_id).await;
    assert_eq!(total_rewards, 4500);
}

/// Test storage rewards calculation
#[tokio::test]
async fn test_storage_rewards_calculation() {
    let rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    let storage_metrics = StorageMetrics {
        node_id,
        availability_percentage: 95.5,
        storage_used: 1024 * 1024 * 100, // 100 MB
        storage_capacity: 1024 * 1024 * 1000, // 1 GB
        files_stored: 50,
        last_update: current_time_secs(),
    };
    
    let mut metrics_map = std::collections::HashMap::new();
    metrics_map.insert(node_id, storage_metrics);
    
    let rewards = rewards_manager.calculate_storage_rewards(&metrics_map).await;
    assert!(rewards.contains_key(&node_id));
    assert!(rewards[&node_id] > 0);
}

/// Test network rewards calculation
#[tokio::test]
async fn test_network_rewards_calculation() {
    let rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    let network_metrics = NetworkMetrics {
        node_id,
        peers_helped: 25,
        total_peers: 100,
        bandwidth_contributed: 1024 * 1024 * 1024, // 1 GB
        last_update: current_time_secs(),
    };
    
    let mut metrics_map = std::collections::HashMap::new();
    metrics_map.insert(node_id, network_metrics);
    
    let rewards = rewards_manager.calculate_network_rewards(&metrics_map).await;
    assert!(rewards.contains_key(&node_id));
    assert!(rewards[&node_id] > 0);
}

/// Test reward stats
#[tokio::test]
async fn test_reward_stats() {
    let mut rewards_manager = RewardsManager::new();
    let node_id1 = create_test_node_id();
    let node_id2 = create_test_node_id();
    
    // Record events for multiple nodes
    rewards_manager.record_reward_event(node_id1, 1000, RewardType::Staking).await;
    rewards_manager.record_reward_event(node_id1, 2000, RewardType::Performance).await;
    rewards_manager.record_reward_event(node_id2, 1500, RewardType::Storage).await;
    
    let stats = rewards_manager.get_reward_stats().await;
    assert_eq!(stats.total_rewards, 4500);
    assert_eq!(stats.total_events, 3);
    assert_eq!(stats.unique_nodes, 2);
    assert_eq!(stats.average_reward, 1500);
}

/// Test reward parameters update
#[tokio::test]
async fn test_reward_params_update() {
    let mut rewards_manager = RewardsManager::new();
    let original_params = rewards_manager.get_params().clone();
    
    let new_params = RewardParams {
        base_reward_rate: 0.002,
        performance_multiplier: 3.0,
        uptime_multiplier: 2.0,
        stake_multiplier: 1.5,
        min_reward_threshold: 2000,
        max_reward_per_node: 2000000,
        distribution_period: 172800,
    };
    
    rewards_manager.update_params(new_params.clone());
    let updated_params = rewards_manager.get_params();
    
    assert_eq!(updated_params.base_reward_rate, 0.002);
    assert_eq!(updated_params.performance_multiplier, 3.0);
    assert_eq!(updated_params.uptime_multiplier, 2.0);
    assert_eq!(updated_params.stake_multiplier, 1.5);
    assert_eq!(updated_params.min_reward_threshold, 2000);
    assert_eq!(updated_params.max_reward_per_node, 2000000);
    assert_eq!(updated_params.distribution_period, 172800);
}

/// Test reward calculation with zero performance
#[tokio::test]
async fn test_reward_calculation_zero_performance() {
    let rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    let stake = Stake {
        node_id,
        amount: MIN_STAKE_AMOUNT,
        start_time: current_time_secs(),
        end_time: None,
        status: StakeStatus::Active,
        last_reward_time: current_time_secs(),
        total_rewards: 0,
        performance_score: 0.0,
        uptime_percentage: 0.0,
    };

    let calculation = rewards_manager.calculate_individual_reward(&stake).await.unwrap();
    
    // Should still get some reward due to base rate
    assert!(calculation.amount > 0);
    assert_eq!(calculation.performance_score, 0.0);
    assert_eq!(calculation.uptime_percentage, 0.0);
}

/// Test reward calculation with maximum performance
#[tokio::test]
async fn test_reward_calculation_max_performance() {
    let rewards_manager = RewardsManager::new();
    let node_id = create_test_node_id();
    
    let stake = Stake {
        node_id,
        amount: MIN_STAKE_AMOUNT,
        start_time: current_time_secs(),
        end_time: None,
        status: StakeStatus::Active,
        last_reward_time: current_time_secs(),
        total_rewards: 0,
        performance_score: 1.0,
        uptime_percentage: 100.0,
    };

    let calculation = rewards_manager.calculate_individual_reward(&stake).await.unwrap();
    
    assert!(calculation.amount > 0);
    assert_eq!(calculation.performance_score, 1.0);
    assert_eq!(calculation.uptime_percentage, 100.0);
    
    // Should have higher multiplier than zero performance
    let zero_performance_stake = Stake {
        node_id,
        amount: MIN_STAKE_AMOUNT,
        start_time: current_time_secs(),
        end_time: None,
        status: StakeStatus::Active,
        last_reward_time: current_time_secs(),
        total_rewards: 0,
        performance_score: 0.0,
        uptime_percentage: 0.0,
    };
    
    let zero_calculation = rewards_manager.calculate_individual_reward(&zero_performance_stake).await.unwrap();
    assert!(calculation.total_multiplier > zero_calculation.total_multiplier);
}
