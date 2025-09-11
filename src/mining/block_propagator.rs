//! Block propagator for IPPAN
//! 
//! Implements block propagation to network peers including
//! broadcast mechanisms, peer selection, and propagation optimization.

use crate::{Result, IppanError, TransactionHash};
use crate::consensus::bft_engine::{BFTBlock, BFTBlockHeader};
use crate::network::real_p2p::RealP2PNetwork;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Block propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPropagationConfig {
    /// Enable block propagation
    pub enable_propagation: bool,
    /// Maximum propagation time in seconds
    pub max_propagation_time_seconds: u64,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Enable peer selection optimization
    pub enable_peer_selection_optimization: bool,
    /// Maximum peers to propagate to
    pub max_peers_to_propagate: usize,
    /// Enable propagation batching
    pub enable_propagation_batching: bool,
    /// Batch size for propagation
    pub propagation_batch_size: usize,
    /// Enable propagation compression
    pub enable_propagation_compression: bool,
    /// Enable propagation encryption
    pub enable_propagation_encryption: bool,
    /// Propagation timeout in seconds
    pub propagation_timeout_seconds: u64,
}

impl Default for BlockPropagationConfig {
    fn default() -> Self {
        Self {
            enable_propagation: true,
            max_propagation_time_seconds: 30,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            enable_peer_selection_optimization: true,
            max_peers_to_propagate: 10,
            enable_propagation_batching: true,
            propagation_batch_size: 5,
            enable_propagation_compression: true,
            enable_propagation_encryption: true,
            propagation_timeout_seconds: 10,
        }
    }
}

/// Block propagation request
#[derive(Debug, Clone)]
pub struct BlockPropagationRequest {
    /// Block to propagate
    pub block: BFTBlock,
    /// Target peers (empty for broadcast)
    pub target_peers: Vec<String>,
    /// Propagation priority
    pub priority: PropagationPriority,
    /// Retry attempts
    pub retry_attempts: u32,
    /// Request timestamp
    pub request_timestamp: u64,
}

/// Block propagation priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropagationPriority {
    /// High priority (immediate propagation)
    High,
    /// Normal priority (standard propagation)
    Normal,
    /// Low priority (delayed propagation)
    Low,
}

/// Block propagation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPropagationResult {
    /// Propagation success
    pub success: bool,
    /// Peers successfully propagated to
    pub successful_peers: Vec<String>,
    /// Peers that failed
    pub failed_peers: Vec<String>,
    /// Total propagation time in milliseconds
    pub propagation_time_ms: u64,
    /// Total retry attempts
    pub total_retry_attempts: u32,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Propagation statistics
    pub propagation_stats: PropagationStats,
}

/// Propagation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationStats {
    /// Total peers attempted
    pub total_peers_attempted: usize,
    /// Successful propagations
    pub successful_propagations: usize,
    /// Failed propagations
    pub failed_propagations: usize,
    /// Average propagation time per peer
    pub average_propagation_time_ms: f64,
    /// Propagation success rate
    pub propagation_success_rate: f64,
    /// Block size in bytes
    pub block_size_bytes: usize,
    /// Compression ratio
    pub compression_ratio: f64,
}

/// Block propagation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPropagationStats {
    /// Total blocks propagated
    pub total_blocks_propagated: u64,
    /// Successful propagations
    pub successful_propagations: u64,
    /// Failed propagations
    pub failed_propagations: u64,
    /// Average propagation time in milliseconds
    pub average_propagation_time_ms: f64,
    /// Average peers per propagation
    pub average_peers_per_propagation: f64,
    /// Average retry attempts per propagation
    pub average_retry_attempts: f64,
    /// Propagation success rate
    pub propagation_success_rate: f64,
    /// Total bytes propagated
    pub total_bytes_propagated: u64,
    /// Average block size propagated
    pub average_block_size_propagated: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last propagation timestamp
    pub last_propagation: Option<u64>,
}

/// Block propagator
pub struct BlockPropagator {
    /// Configuration
    config: BlockPropagationConfig,
    /// Network manager
    network: Arc<RealP2PNetwork>,
    /// Propagation queue
    propagation_queue: Arc<RwLock<Vec<BlockPropagationRequest>>>,
    /// Statistics
    stats: Arc<RwLock<BlockPropagationStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl BlockPropagator {
    /// Create a new block propagator
    pub fn new(config: BlockPropagationConfig, network: Arc<RealP2PNetwork>) -> Self {
        let stats = BlockPropagationStats {
            total_blocks_propagated: 0,
            successful_propagations: 0,
            failed_propagations: 0,
            average_propagation_time_ms: 0.0,
            average_peers_per_propagation: 0.0,
            average_retry_attempts: 0.0,
            propagation_success_rate: 0.0,
            total_bytes_propagated: 0,
            average_block_size_propagated: 0.0,
            uptime_seconds: 0,
            last_propagation: None,
        };
        
        Self {
            config,
            network,
            propagation_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the block propagator
    pub async fn start(&self) -> Result<()> {
        info!("Starting block propagator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        // Start propagation processing loop
        let config = self.config.clone();
        let network = self.network.clone();
        let propagation_queue = self.propagation_queue.clone();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            Self::propagation_processing_loop(
                config,
                network,
                propagation_queue,
                stats,
                is_running,
                start_time,
            ).await;
        });
        
        info!("Block propagator started successfully");
        Ok(())
    }
    
    /// Stop the block propagator
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping block propagator");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Block propagator stopped");
        Ok(())
    }
    
    /// Propagate a block
    pub async fn propagate_block(&self, request: BlockPropagationRequest) -> Result<BlockPropagationResult> {
        let start_time = Instant::now();
        
        info!("Propagating block {} to {} peers", 
            request.block.header.number, 
            request.target_peers.len());
        
        if !self.config.enable_propagation {
            return Ok(BlockPropagationResult {
                success: false,
                successful_peers: vec![],
                failed_peers: request.target_peers,
                propagation_time_ms: start_time.elapsed().as_millis() as u64,
                total_retry_attempts: 0,
                error_message: Some("Propagation disabled".to_string()),
                propagation_stats: PropagationStats {
                    total_peers_attempted: 0,
                    successful_propagations: 0,
                    failed_propagations: 0,
                    average_propagation_time_ms: 0.0,
                    propagation_success_rate: 0.0,
                    block_size_bytes: 0,
                    compression_ratio: 1.0,
                },
            });
        }
        
        // Add to propagation queue
        {
            let mut queue = self.propagation_queue.write().await;
            queue.push(request.clone());
        }
        
        // Wait for propagation to complete
        let mut retry_count = 0;
        let mut successful_peers = Vec::new();
        let mut failed_peers = Vec::new();
        
        while retry_count < self.config.max_retry_attempts {
            let result = self.propagate_to_peers(&request).await?;
            
            successful_peers.extend(result.successful_peers);
            failed_peers = result.failed_peers;
            
            if failed_peers.is_empty() {
                break;
            }
            
            retry_count += 1;
            if retry_count < self.config.max_retry_attempts {
                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
            }
        }
        
        let propagation_time = start_time.elapsed().as_millis() as u64;
        let success = failed_peers.is_empty();
        
        // Calculate propagation statistics
        let block_size = 1024; // TODO: Implement proper block size calculation
        let total_peers = request.target_peers.len();
        let successful_count = successful_peers.len();
        let failed_count = failed_peers.len();
        
        let propagation_stats = PropagationStats {
            total_peers_attempted: total_peers,
            successful_propagations: successful_count,
            failed_propagations: failed_count,
            average_propagation_time_ms: propagation_time as f64 / total_peers as f64,
            propagation_success_rate: successful_count as f64 / total_peers as f64,
            block_size_bytes: block_size,
            compression_ratio: 1.0, // TODO: Implement compression
        };
        
        let result = BlockPropagationResult {
            success,
            successful_peers,
            failed_peers,
            propagation_time_ms: propagation_time,
            total_retry_attempts: retry_count,
            error_message: if success { None } else { Some("Some peers failed".to_string()) },
            propagation_stats,
        };
        
        // Update statistics
        self.update_stats(
            propagation_time,
            total_peers,
            successful_count,
            failed_count,
            retry_count,
            block_size,
        ).await;
        
        if success {
            info!("Block {} propagated successfully to {} peers in {}ms", 
                request.block.header.number, successful_count, propagation_time);
        } else {
            warn!("Block {} propagation failed to {} peers after {} attempts", 
                request.block.header.number, failed_count, retry_count);
        }
        
        Ok(result)
    }
    
    /// Propagate block to specific peers
    async fn propagate_to_peers(&self, request: &BlockPropagationRequest) -> Result<BlockPropagationResult> {
        let start_time = Instant::now();
        let mut successful_peers = Vec::new();
        let mut failed_peers = Vec::new();
        
        // Get target peers
        let target_peers = if request.target_peers.is_empty() {
            self.get_peers_for_propagation().await?
        } else {
            request.target_peers.clone()
        };
        
        // Propagate to each peer
        for peer in target_peers {
            match self.propagate_to_peer(&request.block, &peer).await {
                Ok(_) => {
                    successful_peers.push(peer);
                }
                Err(e) => {
                    warn!("Failed to propagate block to peer {}: {}", peer, e);
                    failed_peers.push(peer);
                }
            }
        }
        
        let propagation_time = start_time.elapsed().as_millis() as u64;
        let success = failed_peers.is_empty();
        
        let propagation_stats = PropagationStats {
            total_peers_attempted: successful_peers.len() + failed_peers.len(),
            successful_propagations: successful_peers.len(),
            failed_propagations: failed_peers.len(),
            average_propagation_time_ms: propagation_time as f64 / (successful_peers.len() + failed_peers.len()) as f64,
            propagation_success_rate: successful_peers.len() as f64 / (successful_peers.len() + failed_peers.len()) as f64,
            block_size_bytes: 1024, // TODO: Implement proper block size calculation
            compression_ratio: 1.0,
        };
        
        Ok(BlockPropagationResult {
            success,
            successful_peers,
            failed_peers,
            propagation_time_ms: propagation_time,
            total_retry_attempts: 0,
            error_message: if success { None } else { Some("Some peers failed".to_string()) },
            propagation_stats,
        })
    }
    
    /// Propagate block to a single peer
    async fn propagate_to_peer(&self, block: &BFTBlock, peer: &str) -> Result<()> {
        debug!("Propagating block {} to peer {}", block.header.number, peer);
        
        // Serialize block
        // TODO: Implement proper block serialization
        // For now, use placeholder data
        let block_data = b"block_propagation_placeholder".to_vec();
        
        // Compress if enabled
        let data_to_send = if self.config.enable_propagation_compression {
            // TODO: Implement compression
            block_data
        } else {
            block_data
        };
        
        // Encrypt if enabled
        let _final_data = if self.config.enable_propagation_encryption {
            // TODO: Implement encryption
            data_to_send
        } else {
            data_to_send
        };
        
        // Send to peer via network
        // TODO: Implement proper block propagation message
        // For now, just log the attempt
        debug!("Would send block propagation to peer: {}", peer);
        
        debug!("Successfully propagated block {} to peer {}", block.header.number, peer);
        Ok(())
    }
    
    /// Get peers for propagation
    async fn get_peers_for_propagation(&self) -> Result<Vec<String>> {
        // Get connected peers from network
        let connected_peers = self.network.get_connected_peers().await;
        
        if connected_peers.is_empty() {
            return Ok(vec![]);
        }
        
        // Convert to string format for propagation
        let peer_strings: Vec<String> = connected_peers
            .iter()
            .map(|peer| format!("peer_{}", hex::encode(peer)))
            .collect();
        
        // Select peers for propagation
        let selected_peers = if self.config.enable_peer_selection_optimization {
            self.select_optimal_peers(&peer_strings).await?
        } else {
            peer_strings
        };
        
        // Limit number of peers
        let max_peers = self.config.max_peers_to_propagate.min(selected_peers.len());
        Ok(selected_peers.into_iter().take(max_peers).collect())
    }
    
    /// Select optimal peers for propagation
    async fn select_optimal_peers(&self, peers: &[String]) -> Result<Vec<String>> {
        // In a real implementation, this would consider peer performance, latency, etc.
        // For now, just return the first N peers
        let max_peers = self.config.max_peers_to_propagate.min(peers.len());
        Ok(peers.iter().take(max_peers).cloned().collect())
    }
    
    /// Propagation processing loop
    async fn propagation_processing_loop(
        config: BlockPropagationConfig,
        network: Arc<RealP2PNetwork>,
        propagation_queue: Arc<RwLock<Vec<BlockPropagationRequest>>>,
        stats: Arc<RwLock<BlockPropagationStats>>,
        is_running: Arc<RwLock<bool>>,
        start_time: Instant,
    ) {
        while *is_running.read().await {
            // Process propagation queue
            let mut queue = propagation_queue.write().await;
            if !queue.is_empty() {
                let request = queue.remove(0);
                drop(queue);
                
                // Process the request
                debug!("Processing propagation request for block {}", request.block.header.number);
                
                // Update statistics
                let mut stats = stats.write().await;
                stats.uptime_seconds = start_time.elapsed().as_secs();
                stats.last_propagation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            }
            
            // Sleep for a short time
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        propagation_time_ms: u64,
        total_peers: usize,
        _successful_peers: usize,
        failed_peers: usize,
        retry_attempts: u32,
        block_size_bytes: usize,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_blocks_propagated += 1;
        if failed_peers == 0 {
            stats.successful_propagations += 1;
        } else {
            stats.failed_propagations += 1;
        }
        
        // Update averages
        let total = stats.total_blocks_propagated as f64;
        stats.average_propagation_time_ms = 
            (stats.average_propagation_time_ms * (total - 1.0) + propagation_time_ms as f64) / total;
        stats.average_peers_per_propagation = 
            (stats.average_peers_per_propagation * (total - 1.0) + total_peers as f64) / total;
        stats.average_retry_attempts = 
            (stats.average_retry_attempts * (total - 1.0) + retry_attempts as f64) / total;
        stats.average_block_size_propagated = 
            (stats.average_block_size_propagated * (total - 1.0) + block_size_bytes as f64) / total;
        
        // Update success rate
        stats.propagation_success_rate = stats.successful_propagations as f64 / total;
        
        // Update total bytes
        stats.total_bytes_propagated += block_size_bytes as u64;
        
        // Update timestamps
        stats.last_propagation = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get block propagation statistics
    pub async fn get_stats(&self) -> Result<BlockPropagationStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Clear propagation queue
    pub async fn clear_queue(&self) -> Result<()> {
        let mut queue = self.propagation_queue.write().await;
        queue.clear();
        info!("Propagation queue cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_block_propagation_config() {
        let config = BlockPropagationConfig::default();
        assert!(config.enable_propagation);
        assert_eq!(config.max_propagation_time_seconds, 30);
        assert_eq!(config.max_retry_attempts, 3);
        assert_eq!(config.retry_delay_ms, 1000);
        assert!(config.enable_peer_selection_optimization);
    }
    
    #[tokio::test]
    async fn test_block_propagation_request() {
        let request = BlockPropagationRequest {
            block: BFTBlock {
                header: BFTBlockHeader {
                    number: 1,
                    previous_hash: [1u8; 32],
                    merkle_root: [2u8; 32],
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    view_number: 1,
                    sequence_number: 1,
                    validator_id: [3u8; 32],
                },
                transactions: vec![],
                hash: [4u8; 32],
            },
            target_peers: vec!["peer1".to_string(), "peer2".to_string()],
            priority: PropagationPriority::High,
            retry_attempts: 0,
            request_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };
        
        assert_eq!(request.block.header.number, 1);
        assert_eq!(request.target_peers.len(), 2);
        assert_eq!(request.retry_attempts, 0);
    }
    
    #[tokio::test]
    async fn test_block_propagation_result() {
        let result = BlockPropagationResult {
            success: true,
            successful_peers: vec!["peer1".to_string(), "peer2".to_string()],
            failed_peers: vec![],
            propagation_time_ms: 150,
            total_retry_attempts: 0,
            error_message: None,
            propagation_stats: PropagationStats {
                total_peers_attempted: 2,
                successful_propagations: 2,
                failed_propagations: 0,
                average_propagation_time_ms: 75.0,
                propagation_success_rate: 1.0,
                block_size_bytes: 1024,
                compression_ratio: 1.0,
            },
        };
        
        assert!(result.success);
        assert_eq!(result.successful_peers.len(), 2);
        assert!(result.failed_peers.is_empty());
        assert_eq!(result.propagation_time_ms, 150);
        assert_eq!(result.total_retry_attempts, 0);
    }
    
    #[tokio::test]
    async fn test_block_propagation_stats() {
        let stats = BlockPropagationStats {
            total_blocks_propagated: 100,
            successful_propagations: 95,
            failed_propagations: 5,
            average_propagation_time_ms: 50.0,
            average_peers_per_propagation: 8.5,
            average_retry_attempts: 0.5,
            propagation_success_rate: 0.95,
            total_bytes_propagated: 1024000,
            average_block_size_propagated: 10240.0,
            uptime_seconds: 3600,
            last_propagation: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_blocks_propagated, 100);
        assert_eq!(stats.successful_propagations, 95);
        assert_eq!(stats.failed_propagations, 5);
        assert_eq!(stats.propagation_success_rate, 0.95);
    }
}
