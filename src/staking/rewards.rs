//! Rewards module for IPPAN
//! 
//! This module provides reward calculation and distribution functionality.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::{
    error::IppanError,
    utils::time::current_time_secs,
    NodeId,
};

use super::stake_pool::StakePool;

/// Rewards manager for IPPAN
pub struct RewardsManager {
    /// Reward calculation parameters
    params: RewardParams,
    /// Reward history
    reward_history: Vec<RewardEvent>,
    /// Performance metrics
    performance_metrics: HashMap<NodeId, PerformanceMetrics>,
}

/// Reward calculation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardParams {
    /// Base reward rate (rewards per second per stake unit)
    pub base_reward_rate: f64,
    /// Performance multiplier
    pub performance_multiplier: f64,
    /// Uptime multiplier
    pub uptime_multiplier: f64,
    /// Stake amount multiplier
    pub stake_multiplier: f64,
    /// Minimum reward threshold
    pub min_reward_threshold: u64,
    /// Maximum reward per node per period
    pub max_reward_per_node: u64,
    /// Reward distribution period in seconds
    pub distribution_period: u64,
}

/// Performance metrics for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Node ID
    pub node_id: NodeId,
    /// Performance score (0.0 to 1.0)
    pub performance_score: f64,
    /// Uptime percentage (0.0 to 100.0)
    pub uptime_percentage: f64,
    /// Storage availability score
    pub storage_availability: f64,
    /// Network contribution score
    pub network_contribution: f64,
    /// Consensus participation score
    pub consensus_participation: f64,
    /// Last update timestamp
    pub last_update: u64,
}

/// Reward event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardEvent {
    /// Event timestamp
    pub timestamp: u64,
    /// Node ID
    pub node_id: NodeId,
    /// Reward amount
    pub amount: u64,
    /// Reward type
    pub reward_type: RewardType,
    /// Performance metrics at time of reward
    pub performance_metrics: PerformanceMetrics,
    /// Calculation details
    pub calculation_details: serde_json::Value,
}

/// Reward type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardType {
    /// Base staking reward
    Staking,
    /// Performance bonus
    Performance,
    /// Storage reward
    Storage,
    /// Network contribution reward
    Network,
    /// Consensus participation reward
    Consensus,
    /// Special bonus
    Bonus,
}

/// Reward calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCalculation {
    /// Node ID
    pub node_id: NodeId,
    /// Calculated reward amount
    pub amount: u64,
    /// Performance score used
    pub performance_score: f64,
    /// Uptime percentage used
    pub uptime_percentage: f64,
    /// Stake amount used
    pub stake_amount: u64,
    /// Multipliers applied
    pub multipliers: HashMap<String, f64>,
    /// Total multiplier
    pub total_multiplier: f64,
}

impl RewardsManager {
    /// Create a new rewards manager
    pub fn new() -> Self {
        Self {
            params: RewardParams::default(),
            reward_history: Vec::new(),
            performance_metrics: HashMap::new(),
        }
    }

    /// Create a rewards manager with custom parameters
    pub fn with_params(params: RewardParams) -> Self {
        Self {
            params,
            reward_history: Vec::new(),
            performance_metrics: HashMap::new(),
        }
    }

    /// Calculate rewards for all stakes in the pool
    pub async fn calculate_rewards(&self, stake_pool: &StakePool, total_reward_amount: u64) -> Result<HashMap<NodeId, u64>, IppanError> {
        let stakes = stake_pool.get_all_stakes().await;
        let mut rewards = HashMap::new();
        let mut total_weight = 0.0;

        // Calculate weights for each stake
        let mut stake_weights = Vec::new();
        for stake in &stakes {
            let weight = self.calculate_stake_weight(stake).await?;
            stake_weights.push((stake.node_id, weight));
            total_weight += weight;
        }

        // Distribute rewards based on weights
        if total_weight > 0.0 {
            for (node_id, weight) in stake_weights {
                let reward_share = weight / total_weight;
                let reward_amount = (total_reward_amount as f64 * reward_share) as u64;
                
                if reward_amount >= self.params.min_reward_threshold {
                    rewards.insert(node_id, reward_amount);
                }
            }
        }

        Ok(rewards)
    }

    /// Calculate individual reward for a stake
    pub async fn calculate_individual_reward(&self, stake: &super::Stake) -> Result<RewardCalculation, IppanError> {
        let performance_metrics = self.get_performance_metrics(&stake.node_id).await;
        let stake_weight = self.calculate_stake_weight(stake).await?;
        
        let base_reward = self.params.base_reward_rate * stake.amount as f64;
        let total_multiplier = self.calculate_total_multiplier(stake, &performance_metrics).await;
        let final_reward = (base_reward * total_multiplier) as u64;
        
        let mut multipliers = HashMap::new();
        multipliers.insert("performance".to_string(), performance_metrics.performance_score);
        multipliers.insert("uptime".to_string(), performance_metrics.uptime_percentage / 100.0);
        multipliers.insert("stake".to_string(), self.params.stake_multiplier);

        Ok(RewardCalculation {
            node_id: stake.node_id,
            amount: final_reward.min(self.params.max_reward_per_node),
            performance_score: performance_metrics.performance_score,
            uptime_percentage: performance_metrics.uptime_percentage,
            stake_amount: stake.amount,
            multipliers,
            total_multiplier,
        })
    }

    /// Calculate stake weight for reward distribution
    async fn calculate_stake_weight(&self, stake: &super::Stake) -> Result<f64, IppanError> {
        let performance_metrics = self.get_performance_metrics(&stake.node_id).await;
        let total_multiplier = self.calculate_total_multiplier(stake, &performance_metrics).await;
        
        Ok(stake.amount as f64 * total_multiplier)
    }

    /// Calculate total multiplier for a stake
    async fn calculate_total_multiplier(&self, stake: &super::Stake, metrics: &PerformanceMetrics) -> f64 {
        let performance_mult = 1.0 + (metrics.performance_score - 0.5) * self.params.performance_multiplier;
        let uptime_mult = 1.0 + (metrics.uptime_percentage - 50.0) / 100.0 * self.params.uptime_multiplier;
        let stake_mult = 1.0 + (stake.amount as f64 / 100_000_000_000.0) * self.params.stake_multiplier; // Normalized to 100 IPN
        
        performance_mult * uptime_mult * stake_mult
    }

    /// Update performance metrics for a node
    pub async fn update_performance_metrics(&mut self, node_id: NodeId, metrics: PerformanceMetrics) {
        self.performance_metrics.insert(node_id, metrics);
    }

    /// Get performance metrics for a node
    pub async fn get_performance_metrics(&self, node_id: &NodeId) -> PerformanceMetrics {
        self.performance_metrics.get(node_id).cloned().unwrap_or_else(|| PerformanceMetrics {
            node_id: *node_id,
            performance_score: 0.5,
            uptime_percentage: 50.0,
            storage_availability: 0.5,
            network_contribution: 0.5,
            consensus_participation: 0.5,
            last_update: current_time_secs(),
        })
    }

    /// Record a reward event
    pub async fn record_reward_event(&mut self, node_id: NodeId, amount: u64, reward_type: RewardType) {
        let performance_metrics = self.get_performance_metrics(&node_id).await;
        
        let event = RewardEvent {
            timestamp: current_time_secs(),
            node_id,
            amount,
            reward_type,
            performance_metrics,
            calculation_details: serde_json::json!({
                "base_reward_rate": self.params.base_reward_rate,
                "performance_multiplier": self.params.performance_multiplier,
                "uptime_multiplier": self.params.uptime_multiplier,
                "stake_multiplier": self.params.stake_multiplier,
            }),
        };
        
        self.reward_history.push(event);
    }

    /// Get reward history for a node
    pub async fn get_reward_history(&self, node_id: &NodeId, limit: Option<usize>) -> Vec<RewardEvent> {
        let mut events: Vec<RewardEvent> = self.reward_history
            .iter()
            .filter(|event| event.node_id == *node_id)
            .cloned()
            .collect();
        
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            events.truncate(limit);
        }
        
        events
    }

    /// Get total rewards for a node
    pub async fn get_total_rewards(&self, node_id: &NodeId) -> u64 {
        self.reward_history
            .iter()
            .filter(|event| event.node_id == *node_id)
            .map(|event| event.amount)
            .sum()
    }

    /// Get reward statistics
    pub async fn get_reward_stats(&self) -> RewardStats {
        let total_rewards: u64 = self.reward_history.iter().map(|e| e.amount).sum();
        let unique_nodes = self.reward_history.iter().map(|e| e.node_id).collect::<std::collections::HashSet<_>>().len();
        
        let avg_reward = if !self.reward_history.is_empty() {
            total_rewards / self.reward_history.len() as u64
        } else {
            0
        };

        let mut reward_by_type = HashMap::new();
        for event in &self.reward_history {
            let entry = reward_by_type.entry(&event.reward_type).or_insert(0u64);
            *entry += event.amount;
        }

        RewardStats {
            total_rewards,
            total_events: self.reward_history.len(),
            unique_nodes,
            average_reward: avg_reward,
            rewards_by_type: reward_by_type,
        }
    }

    /// Calculate storage rewards based on availability
    pub async fn calculate_storage_rewards(&self, storage_metrics: &HashMap<NodeId, StorageMetrics>) -> HashMap<NodeId, u64> {
        let mut rewards = HashMap::new();
        
        for (node_id, metrics) in storage_metrics {
            let availability_score = metrics.availability_percentage / 100.0;
            let base_reward = self.params.base_reward_rate * 1000.0; // Base storage reward
            let reward = (base_reward * availability_score) as u64;
            
            if reward >= self.params.min_reward_threshold {
                rewards.insert(*node_id, reward);
            }
        }
        
        rewards
    }

    /// Calculate network contribution rewards
    pub async fn calculate_network_rewards(&self, network_metrics: &HashMap<NodeId, NetworkMetrics>) -> HashMap<NodeId, u64> {
        let mut rewards = HashMap::new();
        
        for (node_id, metrics) in network_metrics {
            let contribution_score = (metrics.peers_helped as f64 / metrics.total_peers as f64).min(1.0);
            let base_reward = self.params.base_reward_rate * 500.0; // Base network reward
            let reward = (base_reward * contribution_score) as u64;
            
            if reward >= self.params.min_reward_threshold {
                rewards.insert(*node_id, reward);
            }
        }
        
        rewards
    }

    /// Get reward parameters
    pub fn get_params(&self) -> &RewardParams {
        &self.params
    }

    /// Update reward parameters
    pub fn update_params(&mut self, params: RewardParams) {
        self.params = params;
    }
}

/// Storage metrics for reward calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Node ID
    pub node_id: NodeId,
    /// Storage availability percentage
    pub availability_percentage: f64,
    /// Total storage used in bytes
    pub storage_used: u64,
    /// Total storage capacity in bytes
    pub storage_capacity: u64,
    /// Number of files stored
    pub files_stored: u32,
    /// Last update timestamp
    pub last_update: u64,
}

/// Network metrics for reward calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Node ID
    pub node_id: NodeId,
    /// Number of peers helped
    pub peers_helped: u32,
    /// Total number of peers
    pub total_peers: u32,
    /// Bandwidth contributed in bytes
    pub bandwidth_contributed: u64,
    /// Last update timestamp
    pub last_update: u64,
}

/// Reward statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardStats {
    /// Total rewards distributed
    pub total_rewards: u64,
    /// Total reward events
    pub total_events: usize,
    /// Number of unique nodes rewarded
    pub unique_nodes: usize,
    /// Average reward per event
    pub average_reward: u64,
    /// Rewards by type
    pub rewards_by_type: HashMap<&'static str, u64>,
}

impl Default for RewardParams {
    fn default() -> Self {
        Self {
            base_reward_rate: 0.000001, // 0.0001% per second per stake unit
            performance_multiplier: 2.0,
            uptime_multiplier: 1.0,
            stake_multiplier: 0.5,
            min_reward_threshold: 1000, // 0.00001 IPN
            max_reward_per_node: 1_000_000_000, // 10 IPN
            distribution_period: 604800, // 1 week
        }
    }
}

impl Default for RewardsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::crypto::random_bytes;

    fn create_test_node_id() -> NodeId {
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&random_bytes(32)[..32]);
        node_id
    }

    fn create_test_stake(node_id: NodeId) -> super::super::Stake {
        super::super::Stake {
            node_id,
            amount: 10_000_000_000, // 10 IPN
            start_time: current_time_secs(),
            end_time: None,
            status: super::super::StakeStatus::Active,
            last_reward_time: current_time_secs(),
            total_rewards: 0,
            performance_score: 0.8,
            uptime_percentage: 95.0,
        }
    }

    #[tokio::test]
    async fn test_rewards_manager_creation() {
        let manager = RewardsManager::new();
        let params = manager.get_params();
        assert_eq!(params.base_reward_rate, 0.000001);
    }

    #[tokio::test]
    async fn test_individual_reward_calculation() {
        let manager = RewardsManager::new();
        let node_id = create_test_node_id();
        let stake = create_test_stake(node_id);
        
        let calculation = manager.calculate_individual_reward(&stake).await.unwrap();
        assert!(calculation.amount > 0);
        assert_eq!(calculation.node_id, node_id);
    }

    #[tokio::test]
    async fn test_performance_metrics_update() {
        let mut manager = RewardsManager::new();
        let node_id = create_test_node_id();
        
        let metrics = PerformanceMetrics {
            node_id,
            performance_score: 0.9,
            uptime_percentage: 98.0,
            storage_availability: 0.95,
            network_contribution: 0.8,
            consensus_participation: 0.9,
            last_update: current_time_secs(),
        };
        
        manager.update_performance_metrics(node_id, metrics.clone()).await;
        let retrieved_metrics = manager.get_performance_metrics(&node_id).await;
        assert_eq!(retrieved_metrics.performance_score, metrics.performance_score);
    }

    #[tokio::test]
    async fn test_reward_event_recording() {
        let mut manager = RewardsManager::new();
        let node_id = create_test_node_id();
        
        manager.record_reward_event(node_id, 1000, RewardType::Staking).await;
        
        let history = manager.get_reward_history(&node_id, None).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].amount, 1000);
    }

    #[tokio::test]
    async fn test_reward_stats() {
        let mut manager = RewardsManager::new();
        let node_id = create_test_node_id();
        
        manager.record_reward_event(node_id, 1000, RewardType::Staking).await;
        manager.record_reward_event(node_id, 500, RewardType::Performance).await;
        
        let stats = manager.get_reward_stats().await;
        assert_eq!(stats.total_rewards, 1500);
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.unique_nodes, 1);
    }
}
