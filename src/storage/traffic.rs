use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Traffic statistics for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrafficStats {
    /// File hash
    pub file_hash: [u8; 32],
    /// Total bytes served
    pub bytes_served: u64,
    /// Number of requests
    pub request_count: u64,
    /// First request timestamp
    pub first_request: u64,
    /// Last request timestamp
    pub last_request: u64,
    /// Average response time in milliseconds
    pub avg_response_time: u64,
    /// Total response time for calculating average
    pub total_response_time: u64,
}

/// Traffic statistics for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTrafficStats {
    /// Node ID
    pub node_id: [u8; 32],
    /// Total bytes served
    pub total_bytes_served: u64,
    /// Total requests handled
    pub total_requests: u64,
    /// Files served
    pub files_served: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Start timestamp
    pub start_timestamp: u64,
    /// Last activity timestamp
    pub last_activity: u64,
}

/// Traffic tracker for monitoring file serving
pub struct TrafficTracker {
    /// File traffic statistics
    file_stats: RwLock<HashMap<[u8; 32], FileTrafficStats>>,
    /// Node traffic statistics
    node_stats: RwLock<NodeTrafficStats>,
    /// Request history for recent requests
    request_history: RwLock<Vec<RequestRecord>>,
    /// Maximum history size
    max_history_size: usize,
}

/// Record of a file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    /// File hash
    pub file_hash: [u8; 32],
    /// Request timestamp
    pub timestamp: u64,
    /// Bytes served
    pub bytes_served: u64,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Client IP (anonymized)
    pub client_ip_hash: [u8; 32],
}

impl TrafficTracker {
    /// Create a new traffic tracker
    pub fn new(node_id: [u8; 32]) -> Self {
        let start_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            file_stats: RwLock::new(HashMap::new()),
            node_stats: RwLock::new(NodeTrafficStats {
                node_id,
                total_bytes_served: 0,
                total_requests: 0,
                files_served: 0,
                uptime_seconds: 0,
                start_timestamp,
                last_activity: start_timestamp,
            }),
            request_history: RwLock::new(Vec::new()),
            max_history_size: 10000,
        }
    }

    /// Record a file request
    pub async fn record_request(
        &self,
        file_hash: [u8; 32],
        bytes_served: u64,
        response_time_ms: u64,
        client_ip_hash: [u8; 32],
    ) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update file statistics
        {
            let mut file_stats = self.file_stats.write().await;
            let stats = file_stats.entry(file_hash).or_insert_with(|| FileTrafficStats {
                file_hash,
                bytes_served: 0,
                request_count: 0,
                first_request: timestamp,
                last_request: timestamp,
                avg_response_time: 0,
                total_response_time: 0,
            });

            stats.bytes_served += bytes_served;
            stats.request_count += 1;
            stats.last_request = timestamp;
            stats.total_response_time += response_time_ms;
            stats.avg_response_time = stats.total_response_time / stats.request_count;
        }

        // Update node statistics
        {
            let mut node_stats = self.node_stats.write().await;
            node_stats.total_bytes_served += bytes_served;
            node_stats.total_requests += 1;
            node_stats.last_activity = timestamp;
            node_stats.uptime_seconds = timestamp - node_stats.start_timestamp;
        }

        // Record in history
        {
            let mut history = self.request_history.write().await;
            history.push(RequestRecord {
                file_hash,
                timestamp,
                bytes_served,
                response_time_ms,
                client_ip_hash,
            });

            // Trim history if too large
            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }

        Ok(())
    }

    /// Get file traffic statistics
    pub async fn get_file_stats(&self, file_hash: &[u8; 32]) -> Option<FileTrafficStats> {
        self.file_stats.read().await.get(file_hash).cloned()
    }

    /// Get all file statistics
    pub async fn get_all_file_stats(&self) -> Vec<FileTrafficStats> {
        self.file_stats.read().await.values().cloned().collect()
    }

    /// Get node traffic statistics
    pub async fn get_node_stats(&self) -> NodeTrafficStats {
        let mut stats = self.node_stats.read().await.clone();
        
        // Update uptime
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        stats.uptime_seconds = current_time - stats.start_timestamp;
        
        stats
    }

    /// Get recent request history
    pub async fn get_recent_requests(&self, limit: usize) -> Vec<RequestRecord> {
        let history = self.request_history.read().await;
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        history[start..].to_vec()
    }

    /// Get traffic statistics for Global Fund rewards
    pub async fn get_reward_stats(&self) -> RewardStats {
        let node_stats = self.get_node_stats().await;
        let file_stats = self.get_all_file_stats().await;
        
        let total_files_served = file_stats.len() as u64;
        let total_unique_clients = self.get_unique_clients().await;
        
        RewardStats {
            node_id: node_stats.node_id,
            total_bytes_served: node_stats.total_bytes_served,
            total_requests: node_stats.total_requests,
            files_served: total_files_served,
            unique_clients: total_unique_clients,
            uptime_seconds: node_stats.uptime_seconds,
            avg_response_time: self.calculate_avg_response_time().await,
        }
    }

    /// Calculate average response time across all requests
    async fn calculate_avg_response_time(&self) -> u64 {
        let history = self.request_history.read().await;
        if history.is_empty() {
            return 0;
        }
        
        let total_time: u64 = history.iter().map(|r| r.response_time_ms).sum();
        total_time / history.len() as u64
    }

    /// Get number of unique clients
    async fn get_unique_clients(&self) -> u64 {
        let history = self.request_history.read().await;
        let mut unique_clients = std::collections::HashSet::new();
        
        for record in history.iter() {
            unique_clients.insert(record.client_ip_hash);
        }
        
        unique_clients.len() as u64
    }

    /// Get traffic statistics for a time period
    pub async fn get_traffic_for_period(&self, start_time: u64, end_time: u64) -> PeriodTrafficStats {
        let history = self.request_history.read().await;
        let mut period_stats = PeriodTrafficStats {
            start_time,
            end_time,
            total_bytes: 0,
            total_requests: 0,
            unique_files: std::collections::HashSet::new(),
            unique_clients: std::collections::HashSet::new(),
            avg_response_time: 0,
            total_response_time: 0,
        };

        for record in history.iter() {
            if record.timestamp >= start_time && record.timestamp <= end_time {
                period_stats.total_bytes += record.bytes_served;
                period_stats.total_requests += 1;
                period_stats.unique_files.insert(record.file_hash);
                period_stats.unique_clients.insert(record.client_ip_hash);
                period_stats.total_response_time += record.response_time_ms;
            }
        }

        if period_stats.total_requests > 0 {
            period_stats.avg_response_time = period_stats.total_response_time / period_stats.total_requests;
        }

        period_stats
    }

    /// Clear old history
    pub async fn clear_old_history(&self, cutoff_time: u64) -> Result<()> {
        let mut history = self.request_history.write().await;
        history.retain(|record| record.timestamp >= cutoff_time);
        Ok(())
    }

    /// Reset all statistics
    pub async fn reset_stats(&self) -> Result<()> {
        {
            let mut file_stats = self.file_stats.write().await;
            file_stats.clear();
        }
        
        {
            let mut node_stats = self.node_stats.write().await;
            let start_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            *node_stats = NodeTrafficStats {
                node_id: node_stats.node_id,
                total_bytes_served: 0,
                total_requests: 0,
                files_served: 0,
                uptime_seconds: 0,
                start_timestamp,
                last_activity: start_timestamp,
            };
        }
        
        {
            let mut history = self.request_history.write().await;
            history.clear();
        }
        
        Ok(())
    }
}

/// Statistics for Global Fund rewards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardStats {
    /// Node ID
    pub node_id: [u8; 32],
    /// Total bytes served
    pub total_bytes_served: u64,
    /// Total requests handled
    pub total_requests: u64,
    /// Number of files served
    pub files_served: u64,
    /// Number of unique clients
    pub unique_clients: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Average response time in milliseconds
    pub avg_response_time: u64,
}

/// Traffic statistics for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodTrafficStats {
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: u64,
    /// Total bytes served
    pub total_bytes: u64,
    /// Total requests
    pub total_requests: u64,
    /// Unique files served
    pub unique_files: std::collections::HashSet<[u8; 32]>,
    /// Unique clients
    pub unique_clients: std::collections::HashSet<[u8; 32]>,
    /// Average response time
    pub avg_response_time: u64,
    /// Total response time for calculation
    pub total_response_time: u64,
}

impl PeriodTrafficStats {
    /// Get number of unique files
    pub fn unique_files_count(&self) -> usize {
        self.unique_files.len()
    }

    /// Get number of unique clients
    pub fn unique_clients_count(&self) -> usize {
        self.unique_clients.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_traffic_tracking() {
        let node_id = [1u8; 32];
        let tracker = TrafficTracker::new(node_id);
        
        let file_hash = [2u8; 32];
        let client_ip = [3u8; 32];
        
        // Record a request
        tracker.record_request(file_hash, 1024, 50, client_ip).await.unwrap();
        
        // Check file stats
        let file_stats = tracker.get_file_stats(&file_hash).await.unwrap();
        assert_eq!(file_stats.bytes_served, 1024);
        assert_eq!(file_stats.request_count, 1);
        
        // Check node stats
        let node_stats = tracker.get_node_stats().await;
        assert_eq!(node_stats.total_bytes_served, 1024);
        assert_eq!(node_stats.total_requests, 1);
    }

    #[tokio::test]
    async fn test_reward_stats() {
        let node_id = [1u8; 32];
        let tracker = TrafficTracker::new(node_id);
        
        let file_hash = [2u8; 32];
        let client_ip = [3u8; 32];
        
        // Record multiple requests
        for _ in 0..5 {
            tracker.record_request(file_hash, 1024, 50, client_ip).await.unwrap();
        }
        
        let reward_stats = tracker.get_reward_stats().await;
        assert_eq!(reward_stats.total_bytes_served, 5120); // 5 * 1024
        assert_eq!(reward_stats.total_requests, 5);
        assert_eq!(reward_stats.files_served, 1);
        assert_eq!(reward_stats.unique_clients, 1);
    }

    #[tokio::test]
    async fn test_period_traffic_stats() {
        let node_id = [1u8; 32];
        let tracker = TrafficTracker::new(node_id);
        
        let file_hash = [2u8; 32];
        let client_ip = [3u8; 32];
        
        // Record requests
        tracker.record_request(file_hash, 1024, 50, client_ip).await.unwrap();
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let period_stats = tracker.get_traffic_for_period(current_time - 100, current_time + 100).await;
        assert_eq!(period_stats.total_bytes, 1024);
        assert_eq!(period_stats.total_requests, 1);
        assert_eq!(period_stats.unique_files_count(), 1);
        assert_eq!(period_stats.unique_clients_count(), 1);
    }
}
