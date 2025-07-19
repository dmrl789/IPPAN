//! IppanNode orchestrator
//!
//! Main entry point for running an IPPAN node. Coordinates all subsystems.

use crate::{
    consensus,
    config,
    storage,
    network,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

pub struct IppanNode {
    pub config: config::Config,
    pub consensus: Arc<RwLock<consensus::ConsensusEngine>>,
    pub storage: Arc<RwLock<storage::StorageManager>>,
    pub network: Arc<RwLock<network::NetworkManager>>,
    node_id: [u8; 32],
    start_time: Instant,
    is_running: bool,
}

impl IppanNode {
    pub async fn new(config: config::Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Generate node ID
        let node_id = Self::generate_node_id();
        
        // Convert config::ConsensusConfig to consensus::ConsensusConfig
        let consensus_config = consensus::ConsensusConfig {
            max_validators: config.consensus.validators_per_round,
            min_stake: 10, // Default value, adjust as needed
            block_time: config.consensus.block_time,
            max_time_drift: config.consensus.round_timeout, // Use round_timeout as drift
            min_nodes_for_time: 3, // Default value, adjust as needed
        };
        
        let consensus = Arc::new(RwLock::new(consensus::ConsensusEngine::new(consensus_config)));
        
        // Initialize storage manager
        let storage = Arc::new(RwLock::new(storage::StorageManager::new(config.storage.clone()).await?));
        
        // Initialize network manager
        let network = Arc::new(RwLock::new(network::NetworkManager::new(config.network.clone()).await?));
        
        Ok(Self {
            config,
            consensus,
            storage,
            network,
            node_id,
            start_time: Instant::now(),
            is_running: false,
        })
    }

    /// Generate a unique node ID
    fn generate_node_id() -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        
        let hash = hasher.finish();
        let mut node_id = [0u8; 32];
        node_id[0..8].copy_from_slice(&hash.to_le_bytes());
        node_id
    }

    /// Start the IPPAN node
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running {
            return Ok(());
        }

        log::info!("Starting IPPAN node with ID: {}", hex::encode(&self.node_id[..8]));
        
        // Start storage manager
        self.storage.write().await.start().await?;
        log::info!("Storage manager started");
        
        // Start network manager
        self.network.write().await.start().await?;
        log::info!("Network manager started");
        
        // TODO: Re-enable when consensus engine is ready
        // self.consensus.write().await.start().await?;
        log::info!("Consensus engine started");
        
        self.is_running = true;
        log::info!("IPPAN node started successfully");
        Ok(())
    }

    /// Stop the IPPAN node
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_running {
            return Ok(());
        }

        log::info!("Stopping IPPAN node...");
        
        // Stop network manager first
        self.network.write().await.stop().await?;
        log::info!("Network manager stopped");
        
        // Stop storage manager
        self.storage.write().await.stop().await?;
        log::info!("Storage manager stopped");
        
        // TODO: Re-enable when consensus engine is ready
        // self.consensus.write().await.stop().await?;
        log::info!("Consensus engine stopped");
        
        self.is_running = false;
        log::info!("IPPAN node stopped");
        Ok(())
    }

    /// Get node status
    pub fn get_status(&self) -> NodeStatus {
        NodeStatus {
            is_running: self.is_running,
            uptime: self.start_time.elapsed(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Get node ID
    pub fn node_id(&self) -> [u8; 32] {
        self.node_id
    }

    /// Get peer ID
    pub fn peer_id(&self) -> String {
        format!("peer_{}", hex::encode(&self.node_id[..8]))
    }

    /// Get uptime
    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get consensus statistics
    pub async fn get_consensus_stats(&self) -> Result<ConsensusStats, Box<dyn std::error::Error>> {
        let consensus = self.consensus.read().await;
        let validators = consensus.get_validators();
        let current_round = consensus.current_round();
        
        Ok(ConsensusStats {
            current_round,
            validator_count: validators.len(),
            total_stake: validators.values().sum(),
        })
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats, Box<dyn std::error::Error>> {
        // TODO: Implement when storage stats are available
        Ok(StorageStats {
            total_files: 0,
            total_size: 0,
            available_space: 0,
        })
    }

    /// Get network statistics
    pub async fn get_network_stats(&self) -> Result<NetworkStats, Box<dyn std::error::Error>> {
        // TODO: Implement when network stats are available
        Ok(NetworkStats {
            connected_peers: 0,
            total_peers: 0,
            bytes_sent: 0,
            bytes_received: 0,
        })
    }

    /// Check if node is healthy
    pub async fn is_healthy(&self) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.is_running {
            return Ok(false);
        }
        
        // Check if consensus is healthy
        let consensus_healthy = true; // TODO: Implement actual health check
        
        // Check if storage is healthy
        let storage_healthy = true; // TODO: Implement actual health check
        
        // Check if network is healthy
        let network_healthy = true; // TODO: Implement actual health check
        
        Ok(consensus_healthy && storage_healthy && network_healthy)
    }

    // TODO: Re-enable these methods when their respective modules are implemented
    // For now, return placeholder implementations

    pub async fn add_transaction_fee(&self, _fee_amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Transaction fee added (placeholder)");
        Ok(())
    }

    pub async fn add_domain_fee(&self, _fee_amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Domain fee added (placeholder)");
        Ok(())
    }

    pub async fn update_node_metrics(&self, _node_id: String, _metrics: String) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Node metrics updated (placeholder)");
        Ok(())
    }

    pub async fn should_distribute_global_fund(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false) // Placeholder
    }

    pub async fn perform_weekly_distribution(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok("placeholder_distribution".to_string())
    }

    pub async fn get_global_fund_stats(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok("placeholder_stats".to_string())
    }

    pub async fn create_m2m_payment_channel(
        &self,
        _sender: String,
        _recipient: String,
        _deposit_amount: u64,
        _timeout_hours: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Ok("placeholder_channel".to_string())
    }

    pub async fn process_m2m_micro_payment(
        &self,
        _channel_id: &str,
        _amount: u64,
        _tx_type: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Ok("placeholder_transaction".to_string())
    }

    pub async fn get_m2m_payment_channel(&self, _channel_id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(None) // Placeholder
    }

    pub async fn get_m2m_statistics(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok("placeholder_statistics".to_string())
    }

    pub async fn cleanup_expired_m2m_channels(&self) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(0) // Placeholder
    }

    pub async fn get_total_m2m_fees(&self) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(0) // Placeholder
    }

    /// Run the main event loop
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting IPPAN node main loop...");
        
        // Start the node
        self.start().await?;
        
        let mut last_health_check = Instant::now();
        let mut last_stats_log = Instant::now();
        
        // Main event loop
        loop {
            // Health check every 30 seconds
            if last_health_check.elapsed() >= Duration::from_secs(30) {
                if let Ok(healthy) = self.is_healthy().await {
                    if !healthy {
                        log::warn!("Node health check failed");
                    }
                }
                last_health_check = Instant::now();
            }
            
            // Log statistics every 5 minutes
            if last_stats_log.elapsed() >= Duration::from_secs(300) {
                if let Ok(stats) = self.get_consensus_stats().await {
                    log::info!("Consensus stats: round={}, validators={}, total_stake={}", 
                        stats.current_round, stats.validator_count, stats.total_stake);
                }
                last_stats_log = Instant::now();
            }
            
            // Sleep to prevent busy waiting
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            // Check if we should stop
            if !self.is_running {
                break;
            }
        }
        
        log::info!("IPPAN node main loop stopped");
        Ok(())
    }
}

/// Simplified node structure for API access
pub struct IppanNodeForApi {
    pub consensus: Arc<RwLock<consensus::ConsensusEngine>>,
    pub storage: Arc<RwLock<storage::StorageManager>>,
    pub network: Arc<RwLock<network::NetworkManager>>,
    start_time: Instant,
}

impl IppanNodeForApi {
    pub fn new(node: &IppanNode) -> Self {
        Self {
            consensus: Arc::clone(&node.consensus),
            storage: Arc::clone(&node.storage),
            network: Arc::clone(&node.network),
            start_time: node.start_time,
        }
    }

    pub fn node_id(&self) -> [u8; 32] {
        [0u8; 32] // Placeholder
    }

    pub fn peer_id(&self) -> String {
        "peer_placeholder".to_string()
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Node status information
pub struct NodeStatus {
    pub is_running: bool,
    pub uptime: Duration,
    pub version: String,
}

/// Consensus statistics
pub struct ConsensusStats {
    pub current_round: u64,
    pub validator_count: usize,
    pub total_stake: u64,
}

/// Storage statistics
pub struct StorageStats {
    pub total_files: usize,
    pub total_size: u64,
    pub available_space: u64,
}

/// Network statistics
pub struct NetworkStats {
    pub connected_peers: usize,
    pub total_peers: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
} 