//! DHT lookup module
//! 
//! Handles lookup and retrieval of records from the distributed hash table.

use crate::{dht::{DhtNode, DhtValue}, error::IppanError, Result};
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Lookup manager
pub struct LookupManager {
    /// Lookup configuration
    config: super::DhtConfig,
    /// Active lookups
    active_lookups: HashMap<[u8; 32], ActiveLookup>,
    /// Lookup statistics
    stats: LookupStats,
}

/// Active lookup
#[derive(Debug, Clone)]
pub struct ActiveLookup {
    /// Lookup key
    pub key: [u8; 32],
    /// Target nodes
    pub target_nodes: Vec<DhtNode>,
    /// Attempts made
    pub attempts: u32,
    /// Start time
    pub start_time: Instant,
    /// Last attempt time
    pub last_attempt: Option<Instant>,
    /// Found values
    pub found_values: Vec<DhtValue>,
    /// Required values count
    pub required_count: u32,
}

/// Lookup statistics
#[derive(Debug, Clone)]
pub struct LookupStats {
    /// Total lookups
    pub total_lookups: u64,
    /// Successful lookups
    pub successful: u64,
    /// Failed lookups
    pub failed: u64,
    /// Active lookups
    pub active: usize,
    /// Average lookup time
    pub avg_lookup_time: Duration,
}

impl LookupManager {
    /// Create a new lookup manager
    pub fn new(config: super::DhtConfig) -> Self {
        Self {
            config,
            active_lookups: HashMap::new(),
            stats: LookupStats {
                total_lookups: 0,
                successful: 0,
                failed: 0,
                active: 0,
                avg_lookup_time: Duration::from_secs(0),
            },
        }
    }
    
    /// Start the lookup manager
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting DHT lookup manager");
        
        // Start lookup processing loop
        self.run_lookup_loop().await?;
        
        Ok(())
    }
    
    /// Run lookup processing loop
    async fn run_lookup_loop(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // Process active lookups
            self.process_active_lookups().await?;
            
            // Clean up completed lookups
            self.cleanup_completed_lookups();
        }
    }
    
    /// Lookup a record from a target node
    pub async fn lookup_record(&mut self, target_node: &DhtNode, key: &[u8; 32]) -> Result<Option<DhtValue>> {
        let start_time = Instant::now();
        
        // Attempt lookup
        match self.attempt_lookup(target_node, key).await {
            Ok(value) => {
                let lookup_time = start_time.elapsed();
                self.stats.successful += 1;
                self.stats.total_lookups += 1;
                
                // Update average lookup time
                let total_time = self.stats.avg_lookup_time * self.stats.successful + lookup_time;
                self.stats.avg_lookup_time = total_time / (self.stats.successful + 1);
                
                debug!("Successfully looked up record from {} in {:?}", 
                    target_node.peer_id, lookup_time);
                Ok(value)
            }
            Err(e) => {
                self.stats.failed += 1;
                self.stats.total_lookups += 1;
                
                warn!("Failed to lookup record from {}: {}", target_node.peer_id, e);
                Err(e)
            }
        }
    }
    
    /// Attempt lookup from a target node
    async fn attempt_lookup(&self, target_node: &DhtNode, key: &[u8; 32]) -> Result<Option<DhtValue>> {
        // In a real implementation, you'd send a lookup request over the network
        // For now, we'll simulate the lookup process
        
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Simulate success/failure based on node reputation
        if target_node.reputation > 0.3 {
            // Simulate finding a value (with some probability)
            if rand::random::<f64>() > 0.7 {
                Ok(Some(DhtValue {
                    data: vec![1, 2, 3, 4, 5], // Simulated data
                    publisher: target_node.peer_id,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    expires: None,
                    signature: vec![],
                }))
            } else {
                Ok(None) // Record not found
            }
        } else {
            Err(IppanError::NetworkError("Low reputation node".to_string()))
        }
    }
    
    /// Start a parallel lookup
    pub async fn start_parallel_lookup(&mut self, key: [u8; 32], target_nodes: Vec<DhtNode>) -> Result<()> {
        let active_lookup = ActiveLookup {
            key,
            target_nodes,
            attempts: 0,
            start_time: Instant::now(),
            last_attempt: None,
            found_values: Vec::new(),
            required_count: 1, // We want at least one value
        };
        
        self.active_lookups.insert(key, active_lookup);
        self.stats.active = self.active_lookups.len();
        
        debug!("Started parallel lookup for key {:?} with {} target nodes", key, target_nodes.len());
        Ok(())
    }
    
    /// Process active lookups
    async fn process_active_lookups(&mut self) -> Result<()> {
        let max_attempts = 3;
        let retry_delay = Duration::from_secs(5);
        
        for (key, lookup) in &mut self.active_lookups {
            // Check if we should retry
            if lookup.attempts >= max_attempts {
                continue;
            }
            
            if let Some(last_attempt) = lookup.last_attempt {
                if last_attempt.elapsed() < retry_delay {
                    continue;
                }
            }
            
            // Attempt lookup from remaining nodes
            let mut remaining_nodes = Vec::new();
            
            for node in &lookup.target_nodes {
                match self.attempt_lookup(node, key).await {
                    Ok(Some(value)) => {
                        lookup.found_values.push(value);
                        debug!("Found value from {} during parallel lookup", node.peer_id);
                    }
                    Ok(None) => {
                        remaining_nodes.push(node.clone());
                        debug!("No value found from {}", node.peer_id);
                    }
                    Err(e) => {
                        remaining_nodes.push(node.clone());
                        debug!("Lookup failed from {}: {}", node.peer_id, e);
                    }
                }
            }
            
            lookup.attempts += 1;
            lookup.last_attempt = Some(Instant::now());
            lookup.target_nodes = remaining_nodes;
            
            // Check if we have enough values
            if lookup.found_values.len() >= lookup.required_count as usize {
                info!("Parallel lookup completed for key {:?} with {} values", 
                    key, lookup.found_values.len());
            }
        }
    }
    
    /// Clean up completed lookups
    fn cleanup_completed_lookups(&mut self) {
        let now = Instant::now();
        let timeout = self.config.lookup_timeout;
        
        self.active_lookups.retain(|_, lookup| {
            // Keep if still attempting or not enough values found
            (lookup.attempts < 3 && lookup.found_values.len() < lookup.required_count as usize) &&
            (now - lookup.start_time) < timeout
        });
        
        self.stats.active = self.active_lookups.len();
    }
    
    /// Get lookup result
    pub fn get_lookup_result(&self, key: &[u8; 32]) -> Option<Vec<DhtValue>> {
        self.active_lookups.get(key).map(|lookup| lookup.found_values.clone())
    }
    
    /// Get lookup statistics
    pub fn get_stats(&self) -> LookupStats {
        self.stats.clone()
    }
    
    /// Get active lookups
    pub fn get_active_lookups(&self) -> &HashMap<[u8; 32], ActiveLookup> {
        &self.active_lookups
    }
    
    /// Check if a lookup is complete
    pub fn is_lookup_complete(&self, key: &[u8; 32]) -> bool {
        if let Some(lookup) = self.active_lookups.get(key) {
            lookup.found_values.len() >= lookup.required_count as usize
        } else {
            true // No active lookup means it's complete
        }
    }
    
    /// Get lookup status
    pub fn get_lookup_status(&self, key: &[u8; 32]) -> Option<LookupStatus> {
        self.active_lookups.get(key).map(|lookup| LookupStatus {
            key: *key,
            attempts: lookup.attempts,
            found_values: lookup.found_values.len(),
            required_count: lookup.required_count,
            start_time: lookup.start_time,
            last_attempt: lookup.last_attempt,
            target_nodes: lookup.target_nodes.len(),
        })
    }
}

/// Lookup status
#[derive(Debug, Clone)]
pub struct LookupStatus {
    /// Lookup key
    pub key: [u8; 32],
    /// Number of attempts made
    pub attempts: u32,
    /// Number of values found
    pub found_values: usize,
    /// Required number of values
    pub required_count: u32,
    /// Start time
    pub start_time: Instant,
    /// Last attempt time
    pub last_attempt: Option<Instant>,
    /// Number of remaining target nodes
    pub target_nodes: usize,
}

/// Lookup error types
#[derive(Debug, thiserror::Error)]
pub enum LookupError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Record not found")]
    RecordNotFound,
    #[error("Node unavailable: {0}")]
    NodeUnavailable(String),
    #[error("Lookup timeout")]
    Timeout,
}

impl From<LookupError> for IppanError {
    fn from(error: LookupError) -> Self {
        match error {
            LookupError::NetworkError(msg) => IppanError::NetworkError(msg),
            LookupError::RecordNotFound => IppanError::StorageError("Record not found".to_string()),
            LookupError::NodeUnavailable(msg) => IppanError::NetworkError(msg),
            LookupError::Timeout => IppanError::NetworkError("Lookup timeout".to_string()),
        }
    }
}
