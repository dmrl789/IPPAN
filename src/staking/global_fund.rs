use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::staking::rewards::RewardMetrics;
use crate::consensus::round::RoundId;

/// Global Fund - autonomous reward distribution system
pub struct GlobalFund {
    /// Total accumulated funds (in satoshi units)
    total_funds: u64,
    /// Weekly distribution history
    distributions: Vec<WeeklyDistribution>,
    /// Current week's metrics collection
    current_week_metrics: HashMap<String, NodeMetrics>,
    /// Week start timestamp
    week_start: u64,
}

/// Weekly distribution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyDistribution {
    /// Week number
    pub week: u64,
    /// Total funds distributed
    pub total_distributed: u64,
    /// Number of eligible nodes
    pub eligible_nodes: u32,
    /// Distribution timestamp
    pub timestamp: u64,
    /// Node rewards
    pub node_rewards: HashMap<String, u64>,
}

/// Node performance metrics for reward calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// Node ID
    pub node_id: String,
    /// Uptime percentage (0-100)
    pub uptime_percentage: f64,
    /// Number of blocks validated
    pub blocks_validated: u64,
    /// Number of blocks produced
    pub blocks_produced: u64,
    /// Storage availability score (0-100)
    pub storage_score: f64,
    /// Traffic served (bytes)
    pub traffic_served: u64,
    /// Time precision score (0-100)
    pub time_precision: f64,
    /// HashTimer accuracy score (0-100)
    pub hashtimer_accuracy: f64,
    /// Total score for this week
    pub total_score: f64,
}

impl GlobalFund {
    /// Create a new global fund
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            total_funds: 0,
            distributions: Vec::new(),
            current_week_metrics: HashMap::new(),
            week_start: now - (now % (7 * 24 * 60 * 60)), // Start of current week
        }
    }

    /// Add transaction fee to the global fund
    pub fn add_transaction_fee(&mut self, fee_amount: u64) {
        self.total_funds += fee_amount;
    }

    /// Add domain registration/renewal fee
    pub fn add_domain_fee(&mut self, fee_amount: u64) {
        self.total_funds += fee_amount;
    }

    /// Update node metrics for the current week
    pub fn update_node_metrics(&mut self, node_id: String, metrics: NodeMetrics) {
        self.current_week_metrics.insert(node_id, metrics);
    }

    /// Calculate node score based on performance metrics
    pub fn calculate_node_score(&self, metrics: &NodeMetrics) -> f64 {
        let mut score = 0.0;
        
        // Uptime weight: 25%
        score += metrics.uptime_percentage * 0.25;
        
        // Block validation weight: 20%
        let validation_score = (metrics.blocks_validated as f64).min(1000.0) / 1000.0 * 100.0;
        score += validation_score * 0.20;
        
        // Block production weight: 15%
        let production_score = (metrics.blocks_produced as f64).min(100.0) / 100.0 * 100.0;
        score += production_score * 0.15;
        
        // Storage availability weight: 20%
        score += metrics.storage_score * 0.20;
        
        // Traffic served weight: 10%
        let traffic_score = (metrics.traffic_served as f64).min(1_000_000_000.0) / 1_000_000_000.0 * 100.0;
        score += traffic_score * 0.10;
        
        // Time precision weight: 5%
        score += metrics.time_precision * 0.05;
        
        // HashTimer accuracy weight: 5%
        score += metrics.hashtimer_accuracy * 0.05;
        
        score
    }

    /// Check if it's time for weekly distribution
    pub fn should_distribute(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let week_duration = 7 * 24 * 60 * 60; // 7 days in seconds
        let current_week_start = now - (now % week_duration);
        
        current_week_start > self.week_start
    }

    /// Perform weekly distribution
    pub fn perform_weekly_distribution(&mut self) -> Result<WeeklyDistribution> {
        if self.current_week_metrics.is_empty() {
            return Err(crate::IppanError::Staking("No node metrics available for distribution".to_string()));
        }

        // Calculate scores for all nodes
        let mut node_scores: Vec<(String, f64)> = Vec::new();
        for (node_id, metrics) in &self.current_week_metrics {
            let score = self.calculate_node_score(metrics);
            node_scores.push((node_id.clone(), score));
        }

        // Filter eligible nodes (minimum score threshold)
        let eligible_nodes: Vec<(String, f64)> = node_scores
            .into_iter()
            .filter(|(_, score)| *score >= 50.0) // Minimum 50% score
            .collect();

        if eligible_nodes.is_empty() {
            return Err(crate::IppanError::Staking("No eligible nodes for distribution".to_string()));
        }

        // Calculate total score
        let total_score: f64 = eligible_nodes.iter().map(|(_, score)| score).sum();

        // Calculate distribution amounts
        let mut node_rewards: HashMap<String, u64> = HashMap::new();
        for (node_id, score) in eligible_nodes {
            let reward_ratio = score / total_score;
            let reward_amount = (self.total_funds as f64 * reward_ratio) as u64;
            node_rewards.insert(node_id, reward_amount);
        }

        // Create distribution record
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let week_number = now / (7 * 24 * 60 * 60);
        let total_distributed: u64 = node_rewards.values().sum();

        let distribution = WeeklyDistribution {
            week: week_number,
            total_distributed,
            eligible_nodes: eligible_nodes.len() as u32,
            timestamp: now,
            node_rewards,
        };

        // Update state
        self.distributions.push(distribution.clone());
        self.total_funds -= total_distributed;
        self.current_week_metrics.clear();
        self.week_start = now - (now % (7 * 24 * 60 * 60));

        Ok(distribution)
    }

    /// Get current fund balance
    pub fn get_balance(&self) -> u64 {
        self.total_funds
    }

    /// Get distribution history
    pub fn get_distributions(&self) -> &[WeeklyDistribution] {
        &self.distributions
    }

    /// Get current week metrics
    pub fn get_current_metrics(&self) -> &HashMap<String, NodeMetrics> {
        &self.current_week_metrics
    }

    /// Get node's total rewards earned
    pub fn get_node_total_rewards(&self, node_id: &str) -> u64 {
        self.distributions
            .iter()
            .filter_map(|dist| dist.node_rewards.get(node_id))
            .sum()
    }

    /// Get fund statistics
    pub fn get_statistics(&self) -> FundStatistics {
        let total_distributed: u64 = self.distributions
            .iter()
            .map(|d| d.total_distributed)
            .sum();

        let total_nodes_rewarded: u32 = self.distributions
            .iter()
            .map(|d| d.eligible_nodes)
            .sum();

        FundStatistics {
            total_funds_ever: self.total_funds + total_distributed,
            total_distributed,
            current_balance: self.total_funds,
            total_distributions: self.distributions.len() as u32,
            total_nodes_rewarded,
            average_distribution: if !self.distributions.is_empty() {
                total_distributed / self.distributions.len() as u64
            } else {
                0
            },
        }
    }
}

/// Fund statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundStatistics {
    /// Total funds ever collected
    pub total_funds_ever: u64,
    /// Total funds distributed
    pub total_distributed: u64,
    /// Current balance
    pub current_balance: u64,
    /// Number of distributions performed
    pub total_distributions: u32,
    /// Total nodes that received rewards
    pub total_nodes_rewarded: u32,
    /// Average distribution amount
    pub average_distribution: u64,
}

impl NodeMetrics {
    /// Create new node metrics
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            uptime_percentage: 0.0,
            blocks_validated: 0,
            blocks_produced: 0,
            storage_score: 0.0,
            traffic_served: 0,
            time_precision: 0.0,
            hashtimer_accuracy: 0.0,
            total_score: 0.0,
        }
    }

    /// Update uptime percentage
    pub fn update_uptime(&mut self, uptime_percentage: f64) {
        self.uptime_percentage = uptime_percentage.min(100.0).max(0.0);
        self.recalculate_total_score();
    }

    /// Increment blocks validated
    pub fn increment_blocks_validated(&mut self) {
        self.blocks_validated += 1;
        self.recalculate_total_score();
    }

    /// Increment blocks produced
    pub fn increment_blocks_produced(&mut self) {
        self.blocks_produced += 1;
        self.recalculate_total_score();
    }

    /// Update storage score
    pub fn update_storage_score(&mut self, score: f64) {
        self.storage_score = score.min(100.0).max(0.0);
        self.recalculate_total_score();
    }

    /// Add traffic served
    pub fn add_traffic_served(&mut self, bytes: u64) {
        self.traffic_served += bytes;
        self.recalculate_total_score();
    }

    /// Update time precision score
    pub fn update_time_precision(&mut self, precision: f64) {
        self.time_precision = precision.min(100.0).max(0.0);
        self.recalculate_total_score();
    }

    /// Update HashTimer accuracy score
    pub fn update_hashtimer_accuracy(&mut self, accuracy: f64) {
        self.hashtimer_accuracy = accuracy.min(100.0).max(0.0);
        self.recalculate_total_score();
    }

    /// Recalculate total score
    fn recalculate_total_score(&mut self) {
        // This would be calculated by the GlobalFund, but we can provide a basic calculation
        self.total_score = self.uptime_percentage * 0.25
            + (self.blocks_validated as f64).min(1000.0) / 1000.0 * 100.0 * 0.20
            + (self.blocks_produced as f64).min(100.0) / 100.0 * 100.0 * 0.15
            + self.storage_score * 0.20
            + (self.traffic_served as f64).min(1_000_000_000.0) / 1_000_000_000.0 * 100.0 * 0.10
            + self.time_precision * 0.05
            + self.hashtimer_accuracy * 0.05;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_fund_creation() {
        let fund = GlobalFund::new();
        assert_eq!(fund.get_balance(), 0);
        assert!(fund.get_distributions().is_empty());
    }

    #[test]
    fn test_fee_collection() {
        let mut fund = GlobalFund::new();
        fund.add_transaction_fee(1000);
        fund.add_domain_fee(500);
        assert_eq!(fund.get_balance(), 1500);
    }

    #[test]
    fn test_node_metrics() {
        let mut metrics = NodeMetrics::new("test_node".to_string());
        metrics.update_uptime(95.5);
        metrics.increment_blocks_validated();
        metrics.increment_blocks_produced();
        metrics.update_storage_score(88.0);
        metrics.add_traffic_served(1_000_000);
        metrics.update_time_precision(99.9);
        metrics.update_hashtimer_accuracy(98.5);

        assert_eq!(metrics.uptime_percentage, 95.5);
        assert_eq!(metrics.blocks_validated, 1);
        assert_eq!(metrics.blocks_produced, 1);
        assert_eq!(metrics.storage_score, 88.0);
        assert_eq!(metrics.traffic_served, 1_000_000);
        assert_eq!(metrics.time_precision, 99.9);
        assert_eq!(metrics.hashtimer_accuracy, 98.5);
    }

    #[test]
    fn test_score_calculation() {
        let fund = GlobalFund::new();
        let mut metrics = NodeMetrics::new("test_node".to_string());
        metrics.update_uptime(100.0);
        metrics.increment_blocks_validated();
        metrics.increment_blocks_produced();
        metrics.update_storage_score(100.0);
        metrics.add_traffic_served(1_000_000_000);
        metrics.update_time_precision(100.0);
        metrics.update_hashtimer_accuracy(100.0);

        let score = fund.calculate_node_score(&metrics);
        assert!(score > 0.0);
        assert!(score <= 100.0);
    }
} 