//! DHT replication module
//! 
//! Handles replication of DHT records across multiple nodes for availability.

use crate::{dht::{DhtNode, DhtRecord}, error::IppanError, Result};
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Replication manager
pub struct ReplicationManager {
    /// Replication configuration
    config: super::DhtConfig,
    /// Pending replications
    pending_replications: HashMap<[u8; 32], PendingReplication>,
    /// Replication statistics
    stats: ReplicationStats,
}

/// Pending replication
#[derive(Debug, Clone)]
pub struct PendingReplication {
    /// Record to replicate
    pub record: DhtRecord,
    /// Target nodes
    pub target_nodes: Vec<DhtNode>,
    /// Attempts made
    pub attempts: u32,
    /// Last attempt time
    pub last_attempt: Option<Instant>,
    /// Success count
    pub success_count: u32,
    /// Required success count
    pub required_success: u32,
}

/// Replication statistics
#[derive(Debug, Clone)]
pub struct ReplicationStats {
    /// Total replications attempted
    pub total_attempts: u64,
    /// Successful replications
    pub successful: u64,
    /// Failed replications
    pub failed: u64,
    /// Pending replications
    pub pending: usize,
    /// Average replication time
    pub avg_replication_time: Duration,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new(config: super::DhtConfig) -> Self {
        Self {
            config,
            pending_replications: HashMap::new(),
            stats: ReplicationStats {
                total_attempts: 0,
                successful: 0,
                failed: 0,
                pending: 0,
                avg_replication_time: Duration::from_secs(0),
            },
        }
    }
    
    /// Start the replication manager
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting DHT replication manager");
        
        // Start replication loop
        self.run_replication_loop().await?;
        
        Ok(())
    }
    
    /// Run replication loop
    async fn run_replication_loop(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(self.config.replication_interval);
        
        loop {
            interval.tick().await;
            
            // Process pending replications
            self.process_pending_replications().await?;
            
            // Clean up completed replications
            self.cleanup_completed_replications();
        }
    }
    
    /// Replicate a record to a target node
    pub async fn replicate_record(&mut self, target_node: &DhtNode, record: &DhtRecord) -> Result<()> {
        let start_time = Instant::now();
        
        // Check if we have enough storage
        if target_node.available_storage < record.value.data.len() as u64 {
            warn!("Target node {} has insufficient storage", target_node.peer_id);
            return Err(IppanError::StorageError("Insufficient storage".to_string()));
        }
        
        // Attempt replication
        match self.attempt_replication(target_node, record).await {
            Ok(_) => {
                let replication_time = start_time.elapsed();
                self.stats.successful += 1;
                self.stats.total_attempts += 1;
                
                // Update average replication time
                let total_time = self.stats.avg_replication_time * self.stats.successful + replication_time;
                self.stats.avg_replication_time = total_time / (self.stats.successful + 1);
                
                debug!("Successfully replicated record to {} in {:?}", 
                    target_node.peer_id, replication_time);
                Ok(())
            }
            Err(e) => {
                self.stats.failed += 1;
                self.stats.total_attempts += 1;
                
                warn!("Failed to replicate record to {}: {}", target_node.peer_id, e);
                
                // Add to pending replications for retry
                self.add_pending_replication(record, vec![target_node.clone()]);
                Err(e)
            }
        }
    }
    
    /// Attempt replication to a target node
    async fn attempt_replication(&self, target_node: &DhtNode, record: &DhtRecord) -> Result<()> {
        // In a real implementation, you'd send the record over the network
        // For now, we'll simulate the replication process
        
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate success/failure based on node reputation
        if target_node.reputation > 0.5 {
            Ok(())
        } else {
            Err(IppanError::NetworkError("Low reputation node".to_string()))
        }
    }
    
    /// Add a pending replication
    fn add_pending_replication(&mut self, record: &DhtRecord, target_nodes: Vec<DhtNode>) {
        let pending = PendingReplication {
            record: record.clone(),
            target_nodes,
            attempts: 0,
            last_attempt: None,
            success_count: 0,
            required_success: self.config.replication_factor,
        };
        
        self.pending_replications.insert(record.key, pending);
        self.stats.pending = self.pending_replications.len();
    }
    
    /// Process pending replications
    async fn process_pending_replications(&mut self) -> Result<()> {
        let max_retries = 3;
        let retry_delay = Duration::from_secs(60);
        
        for (key, pending) in &mut self.pending_replications {
            // Check if we should retry
            if pending.attempts >= max_retries {
                continue;
            }
            
            if let Some(last_attempt) = pending.last_attempt {
                if last_attempt.elapsed() < retry_delay {
                    continue;
                }
            }
            
            // Attempt replication to remaining nodes
            let mut remaining_nodes = Vec::new();
            
            for node in &pending.target_nodes {
                match self.attempt_replication(node, &pending.record).await {
                    Ok(_) => {
                        pending.success_count += 1;
                        debug!("Retry replication successful to {}", node.peer_id);
                    }
                    Err(e) => {
                        remaining_nodes.push(node.clone());
                        debug!("Retry replication failed to {}: {}", node.peer_id, e);
                    }
                }
            }
            
            pending.attempts += 1;
            pending.last_attempt = Some(Instant::now());
            pending.target_nodes = remaining_nodes;
            
            // Check if we have enough successful replications
            if pending.success_count >= pending.required_success {
                info!("Replication completed for key {:?} with {} successful copies", 
                    key, pending.success_count);
            }
        }
    }
    
    /// Clean up completed replications
    fn cleanup_completed_replications(&mut self) {
        self.pending_replications.retain(|_, pending| {
            // Keep if still attempting or not enough successful replications
            pending.attempts < 3 && pending.success_count < pending.required_success
        });
        
        self.stats.pending = self.pending_replications.len();
    }
    
    /// Get replication statistics
    pub fn get_stats(&self) -> ReplicationStats {
        self.stats.clone()
    }
    
    /// Get pending replications
    pub fn get_pending_replications(&self) -> &HashMap<[u8; 32], PendingReplication> {
        &self.pending_replications
    }
    
    /// Check if a record is sufficiently replicated
    pub fn is_sufficiently_replicated(&self, key: &[u8; 32]) -> bool {
        if let Some(pending) = self.pending_replications.get(key) {
            pending.success_count >= pending.required_success
        } else {
            true // No pending replication means it's already replicated
        }
    }
    
    /// Get replication status for a record
    pub fn get_replication_status(&self, key: &[u8; 32]) -> Option<ReplicationStatus> {
        self.pending_replications.get(key).map(|pending| ReplicationStatus {
            key: *key,
            attempts: pending.attempts,
            success_count: pending.success_count,
            required_success: pending.required_success,
            last_attempt: pending.last_attempt,
            target_nodes: pending.target_nodes.len(),
        })
    }
}

/// Replication status
#[derive(Debug, Clone)]
pub struct ReplicationStatus {
    /// Record key
    pub key: [u8; 32],
    /// Number of attempts made
    pub attempts: u32,
    /// Number of successful replications
    pub success_count: u32,
    /// Required number of successful replications
    pub required_success: u32,
    /// Last attempt time
    pub last_attempt: Option<Instant>,
    /// Number of remaining target nodes
    pub target_nodes: usize,
}

/// Replication error types
#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Node unavailable: {0}")]
    NodeUnavailable(String),
    #[error("Insufficient storage")]
    InsufficientStorage,
    #[error("Replication timeout")]
    Timeout,
}

impl From<ReplicationError> for IppanError {
    fn from(error: ReplicationError) -> Self {
        match error {
            ReplicationError::NetworkError(msg) => IppanError::NetworkError(msg),
            ReplicationError::StorageError(msg) => IppanError::StorageError(msg),
            ReplicationError::NodeUnavailable(msg) => IppanError::NetworkError(msg),
            ReplicationError::InsufficientStorage => IppanError::StorageError("Insufficient storage".to_string()),
            ReplicationError::Timeout => IppanError::NetworkError("Replication timeout".to_string()),
        }
    }
}
