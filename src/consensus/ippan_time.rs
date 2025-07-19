//! IPPAN Time management
//! 
//! Provides median time calculation across network nodes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};


/// IPPAN Time precision in microseconds (0.1 microseconds)
pub const IPPAN_TIME_PRECISION: u64 = 100; // 0.1 microseconds

/// IPPAN Time manager for calculating median time across network nodes
/// Provides 0.1 microsecond precision timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IppanTimeManager {
    /// Node time samples indexed by node ID
    node_times: HashMap<[u8; 32], u64>,
    /// Current median time in nanoseconds
    median_time_ns: u64,
    /// Minimum number of nodes required for valid median
    min_nodes: usize,
    /// Maximum allowed time drift in seconds
    max_drift_seconds: u64,
}

impl IppanTimeManager {
    /// Create a new IPPAN Time manager
    pub fn new(min_nodes: usize, max_drift_seconds: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        Self {
            node_times: HashMap::new(),
            median_time_ns: now,
            min_nodes,
            max_drift_seconds,
        }
    }

    /// Add a time sample from a node
    pub fn add_node_time(&mut self, node_id: [u8; 32], time_ns: u64) {
        self.node_times.insert(node_id, time_ns);
        self.update_median();
    }

    /// Remove a node's time sample
    pub fn remove_node_time(&mut self, node_id: &[u8; 32]) {
        self.node_times.remove(node_id);
        self.update_median();
    }

    /// Update the median time calculation
    fn update_median(&mut self) {
        if self.node_times.len() < self.min_nodes {
            // Use local time if not enough samples
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            self.median_time_ns = now;
            return;
        }

        let mut times: Vec<u64> = self.node_times.values().cloned().collect();
        times.sort();
        
        let mid = times.len() / 2;
        if times.len() % 2 == 0 {
            // Even number of samples - average the two middle values
            self.median_time_ns = (times[mid - 1] + times[mid]) / 2;
        } else {
            // Odd number of samples - use the middle value
            self.median_time_ns = times[mid];
        }
    }

    /// Get the current median time in nanoseconds
    pub fn median_time_ns(&self) -> u64 {
        self.median_time_ns
    }

    /// Get the current median time in seconds
    pub fn median_time_seconds(&self) -> u64 {
        self.median_time_ns / 1_000_000_000
    }

    /// Get the current median time in microseconds
    pub fn median_time_micros(&self) -> u64 {
        self.median_time_ns / 1_000
    }

    /// Check if we have enough node samples for valid median
    pub fn has_sufficient_samples(&self) -> bool {
        self.node_times.len() >= self.min_nodes
    }

    /// Get the number of active node samples
    pub fn sample_count(&self) -> usize {
        self.node_times.len()
    }

    /// Validate that a given time is within acceptable bounds
    pub fn is_time_valid(&self, time_ns: u64) -> bool {
        let drift_ns = if time_ns > self.median_time_ns {
            time_ns - self.median_time_ns
        } else {
            self.median_time_ns - time_ns
        };
        
        let max_drift_ns = self.max_drift_seconds * 1_000_000_000;
        drift_ns <= max_drift_ns
    }

    /// Get the time drift for a given timestamp
    pub fn get_time_drift_ns(&self, time_ns: u64) -> i64 {
        time_ns as i64 - self.median_time_ns as i64
    }

    /// Get the time drift for a given timestamp in seconds
    pub fn get_time_drift_seconds(&self, time_ns: u64) -> f64 {
        let drift_ns = self.get_time_drift_ns(time_ns);
        drift_ns as f64 / 1_000_000_000.0
    }

    /// Check if a node's time is synchronized
    pub fn is_node_synchronized(&self, node_id: &[u8; 32]) -> bool {
        if let Some(&node_time) = self.node_times.get(node_id) {
            self.is_time_valid(node_time)
        } else {
            false
        }
    }

    /// Get all node times as a vector
    pub fn get_node_times(&self) -> Vec<([u8; 32], u64)> {
        self.node_times.iter()
            .map(|(&node_id, &time)| (node_id, time))
            .collect()
    }

    /// Clear all node times (useful for testing)
    pub fn clear(&mut self) {
        self.node_times.clear();
        self.update_median();
    }

    /// Get statistics about the time samples
    pub fn get_stats(&self) -> TimeStats {
        if self.node_times.is_empty() {
            return TimeStats {
                count: 0,
                min: 0,
                max: 0,
                mean: 0.0,
                median: 0,
                std_dev: 0.0,
            };
        }

        let times: Vec<u64> = self.node_times.values().cloned().collect();
        let count = times.len();
        let min = *times.iter().min().unwrap();
        let max = *times.iter().max().unwrap();
        let sum: u64 = times.iter().sum();
        let mean = sum as f64 / count as f64;
        
        let variance = times.iter()
            .map(|&t| {
                let diff = t as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        TimeStats {
            count,
            min,
            max,
            mean,
            median: self.median_time_ns,
            std_dev,
        }
    }
}

/// Statistics about time samples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeStats {
    pub count: usize,
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub median: u64,
    pub std_dev: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_time_manager() {
        let mut manager = IppanTimeManager::new(3, 30);
        
        // Add some node times
        manager.add_node_time([1u8; 32], 1000);
        manager.add_node_time([2u8; 32], 2000);
        manager.add_node_time([3u8; 32], 3000);
        
        assert_eq!(manager.sample_count(), 3);
        assert!(manager.has_sufficient_samples());
        assert_eq!(manager.median_time_ns(), 2000);
    }

    #[tokio::test]
    async fn test_median_time_calculation() {
        let mut manager = IppanTimeManager::new(2, 30);
        
        // Add even number of samples
        manager.add_node_time([1u8; 32], 1000);
        manager.add_node_time([2u8; 32], 3000);
        
        // Median should be average of 1000 and 3000 = 2000
        assert_eq!(manager.median_time_ns(), 2000);
    }
}
