//! Sharding for IPPAN storage

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

/// Shard information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    /// Shard ID
    pub shard_id: String,
    /// File ID
    pub file_id: String,
    /// Shard index
    pub index: u32,
    /// Shard size (bytes)
    pub size: u64,
    /// Shard data hash
    pub data_hash: [u8; 32],
    /// Shard status
    pub status: ShardStatus,
    /// Storage nodes holding this shard
    pub storage_nodes: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
}

/// Shard status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShardStatus {
    /// Shard is healthy and available
    Healthy,
    /// Shard is being replicated
    Replicating,
    /// Shard is degraded (some replicas missing)
    Degraded,
    /// Shard is lost
    Lost,
    /// Shard is being repaired
    Repairing,
}

/// Shard placement strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementStrategy {
    /// Round-robin placement
    RoundRobin,
    /// Hash-based placement
    HashBased,
    /// Geographic placement
    Geographic,
    /// Load-balanced placement
    LoadBalanced,
}

/// Shard manager
#[derive(Debug)]
pub struct ShardManager {
    /// Shard information
    shards: Arc<RwLock<HashMap<String, ShardInfo>>>,
    /// Storage nodes
    storage_nodes: Arc<RwLock<HashMap<String, StorageNodeInfo>>>,
    /// Placement strategy
    placement_strategy: PlacementStrategy,
    /// Replication factor
    replication_factor: u32,
    /// Shard size (bytes)
    shard_size: u64,
    /// Running flag
    running: bool,
}

/// Storage node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageNodeInfo {
    /// Node ID
    pub node_id: String,
    /// Node address
    pub address: String,
    /// Available capacity (bytes)
    pub available_capacity: u64,
    /// Used capacity (bytes)
    pub used_capacity: u64,
    /// Node status
    pub status: NodeStatus,
    /// Geographic location
    pub location: Option<String>,
    /// Load score (0.0 to 1.0)
    pub load_score: f64,
    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,
}

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Node is online and available
    Online,
    /// Node is offline
    Offline,
    /// Node is maintenance mode
    Maintenance,
    /// Node is full
    Full,
}

impl ShardManager {
    /// Create a new shard manager
    pub fn new(
        placement_strategy: PlacementStrategy,
        replication_factor: u32,
        shard_size: u64,
    ) -> Self {
        Self {
            shards: Arc::new(RwLock::new(HashMap::new())),
            storage_nodes: Arc::new(RwLock::new(HashMap::new())),
            placement_strategy,
            replication_factor,
            shard_size,
            running: false,
        }
    }

    /// Start the shard manager
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting shard manager");
        self.running = true;
        
        // Start shard health monitoring
        let shards = self.shards.clone();
        let storage_nodes = self.storage_nodes.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                Self::monitor_shard_health(&shards, &storage_nodes).await;
            }
        });
        
        Ok(())
    }

    /// Stop the shard manager
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping shard manager");
        self.running = false;
        Ok(())
    }

    /// Create shards for a file
    pub async fn create_shards(
        &self,
        file_id: &str,
        file_size: u64,
        data: &[u8],
    ) -> Result<Vec<ShardInfo>> {
        let shard_count = Self::calculate_shard_count(file_size, self.shard_size);
        let mut shards = Vec::new();
        
        for i in 0..shard_count {
            let start = (i as u64 * self.shard_size) as usize;
            let end = std::cmp::min(start + self.shard_size as usize, data.len());
            let shard_data = &data[start..end];
            
            let shard_hash = Self::calculate_shard_hash(shard_data);
            let shard_id = format!("{}_{}", file_id, i);
            
            // Select storage nodes for this shard
            let storage_nodes = self.select_storage_nodes().await?;
            
            let shard = ShardInfo {
                shard_id: shard_id.clone(),
                file_id: file_id.to_string(),
                index: i,
                size: shard_data.len() as u64,
                data_hash: shard_hash,
                status: ShardStatus::Healthy,
                storage_nodes,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
            };
            
            shards.push(shard.clone());
            
            // Store shard info
            let mut shards_map = self.shards.write().await;
            shards_map.insert(shard_id, shard);
        }
        
        log::info!("Created {} shards for file: {}", shard_count, file_id);
        Ok(shards)
    }

    /// Get shard information
    pub async fn get_shard_info(&self, shard_id: &str) -> Result<ShardInfo> {
        let shards = self.shards.read().await;
        
        let shard = shards.get(shard_id).ok_or_else(|| {
            crate::error::IppanError::Storage(
                format!("Shard not found: {}", shard_id)
            )
        })?;
        
        Ok(shard.clone())
    }

    /// Update shard status
    pub async fn update_shard_status(&self, shard_id: &str, status: ShardStatus) -> Result<()> {
        let mut shards = self.shards.write().await;
        
        if let Some(shard) = shards.get_mut(shard_id) {
            let status_clone = status.clone();
            shard.status = status;
            log::info!("Updated shard status: {} -> {:?}", shard_id, status_clone);
        }
        
        Ok(())
    }

    /// Replicate shard
    pub async fn replicate_shard(&self, shard_id: &str, target_node_id: &str) -> Result<()> {
        let mut shards = self.shards.write().await;
        
        if let Some(shard) = shards.get_mut(shard_id) {
            if !shard.storage_nodes.contains(&target_node_id.to_string()) {
                shard.storage_nodes.push(target_node_id.to_string());
                shard.status = ShardStatus::Replicating;
                log::info!("Replicating shard {} to node {}", shard_id, target_node_id);
            }
        }
        
        Ok(())
    }

    /// Register storage node
    pub async fn register_storage_node(&self, node: StorageNodeInfo) -> Result<()> {
        let mut nodes = self.storage_nodes.write().await;
        let node_id = node.node_id.clone();
        nodes.insert(node_id.clone(), node);
        log::info!("Registered storage node: {}", node_id);
        Ok(())
    }

    /// Unregister storage node
    pub async fn unregister_storage_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.storage_nodes.write().await;
        if nodes.remove(node_id).is_some() {
            log::info!("Unregistered storage node: {}", node_id);
        }
        Ok(())
    }

    /// Get shard statistics
    pub async fn get_shard_stats(&self) -> ShardStats {
        let shards = self.shards.read().await;
        let nodes = self.storage_nodes.read().await;
        
        let total_shards = shards.len();
        let healthy_shards = shards.values()
            .filter(|shard| shard.status == ShardStatus::Healthy)
            .count();
        
        let degraded_shards = shards.values()
            .filter(|shard| shard.status == ShardStatus::Degraded)
            .count();
        
        let lost_shards = shards.values()
            .filter(|shard| shard.status == ShardStatus::Lost)
            .count();
        
        let total_nodes = nodes.len();
        let online_nodes = nodes.values()
            .filter(|node| node.status == NodeStatus::Online)
            .count();
        
        ShardStats {
            total_shards,
            healthy_shards,
            degraded_shards,
            lost_shards,
            total_nodes,
            online_nodes,
            replication_factor: self.replication_factor,
            shard_size: self.shard_size,
        }
    }

    /// Calculate number of shards needed
    fn calculate_shard_count(file_size: u64, shard_size: u64) -> u32 {
        ((file_size + shard_size - 1) / shard_size) as u32
    }

    /// Calculate shard hash
    fn calculate_shard_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Select storage nodes for a shard
    async fn select_storage_nodes(&self) -> Result<Vec<String>> {
        let nodes = self.storage_nodes.read().await;
        let online_nodes: Vec<_> = nodes.values()
            .filter(|node| node.status == NodeStatus::Online)
            .collect();
        
        if online_nodes.len() < self.replication_factor as usize {
            return Err(crate::error::IppanError::Storage(
                format!("Not enough online nodes: {} < {}", online_nodes.len(), self.replication_factor)
            ));
        }
        
        let selected_nodes = match self.placement_strategy {
            PlacementStrategy::RoundRobin => {
                Self::select_round_robin(&online_nodes, self.replication_factor)
            }
            PlacementStrategy::HashBased => {
                Self::select_hash_based(&online_nodes, self.replication_factor)
            }
            PlacementStrategy::Geographic => {
                Self::select_geographic(&online_nodes, self.replication_factor)
            }
            PlacementStrategy::LoadBalanced => {
                Self::select_load_balanced(&online_nodes, self.replication_factor)
            }
        };
        
        Ok(selected_nodes.into_iter().map(|node| node.node_id.clone()).collect())
    }

    /// Round-robin node selection
    fn select_round_robin<'a>(nodes: &'a [&'a StorageNodeInfo], count: u32) -> Vec<&'a StorageNodeInfo> {
        let mut selected = Vec::new();
        let mut index = 0;
        
        for _ in 0..count {
            selected.push(nodes[index % nodes.len()]);
            index += 1;
        }
        
        selected
    }

    /// Hash-based node selection
    fn select_hash_based<'a>(nodes: &'a [&'a StorageNodeInfo], count: u32) -> Vec<&'a StorageNodeInfo> {
        // Simple hash-based selection
        let mut selected = Vec::new();
        let mut hasher = Sha256::new();
        hasher.update(b"shard_placement");
        let hash = hasher.finalize();
        
        for i in 0..count {
            let index = (hash[i as usize] as usize) % nodes.len();
            selected.push(nodes[index]);
        }
        
        selected
    }

    /// Geographic node selection
    fn select_geographic<'a>(nodes: &'a [&'a StorageNodeInfo], count: u32) -> Vec<&'a StorageNodeInfo> {
        // Simple geographic selection (prefer nodes with location info)
        let mut selected = Vec::new();
        let mut nodes_with_location: Vec<_> = nodes.iter()
            .filter(|node| node.location.is_some())
            .map(|&node| node)
            .collect();
        
        if nodes_with_location.len() < count as usize {
            nodes_with_location = nodes.iter().map(|&node| node).collect();
        }
        
        for i in 0..count {
            selected.push(nodes_with_location[i as usize % nodes_with_location.len()]);
        }
        
        selected
    }

    /// Load-balanced node selection
    fn select_load_balanced<'a>(nodes: &'a [&'a StorageNodeInfo], count: u32) -> Vec<&'a StorageNodeInfo> {
        // Sort by load score (lower is better)
        let mut sorted_nodes: Vec<_> = nodes.iter().map(|&node| node).collect();
        sorted_nodes.sort_by(|a, b| a.load_score.partial_cmp(&b.load_score).unwrap());
        
        sorted_nodes.into_iter().take(count as usize).collect()
    }

    /// Monitor shard health
    async fn monitor_shard_health(
        shards: &Arc<RwLock<HashMap<String, ShardInfo>>>,
        storage_nodes: &Arc<RwLock<HashMap<String, StorageNodeInfo>>>,
    ) {
        let mut shards = shards.write().await;
        let nodes = storage_nodes.read().await;
        
        for shard in shards.values_mut() {
            let mut healthy_replicas = 0;
            
            for node_id in &shard.storage_nodes {
                if let Some(node) = nodes.get(node_id) {
                    if node.status == NodeStatus::Online {
                        healthy_replicas += 1;
                    }
                }
            }
            
            // Update shard status based on replica health
            if healthy_replicas == 0 {
                shard.status = ShardStatus::Lost;
            } else if healthy_replicas < 2 {
                shard.status = ShardStatus::Degraded;
            } else {
                shard.status = ShardStatus::Healthy;
            }
        }
    }
}

/// Shard statistics
#[derive(Debug, Serialize)]
pub struct ShardStats {
    pub total_shards: usize,
    pub healthy_shards: usize,
    pub degraded_shards: usize,
    pub lost_shards: usize,
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub replication_factor: u32,
    pub shard_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shard_manager_creation() {
        let manager = ShardManager::new(
            PlacementStrategy::RoundRobin,
            3,
            1024 * 1024,
        );
        
        assert_eq!(manager.replication_factor, 3);
        assert_eq!(manager.shard_size, 1024 * 1024);
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_shard_manager_start_stop() {
        let mut manager = ShardManager::new(
            PlacementStrategy::RoundRobin,
            3,
            1024 * 1024,
        );
        
        manager.start().await.unwrap();
        assert!(manager.running);
        
        manager.stop().await.unwrap();
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_shard_creation() {
        let manager = ShardManager::new(
            PlacementStrategy::RoundRobin,
            3,
            1024,
        );
        
        // Register some storage nodes
        let node1 = StorageNodeInfo {
            node_id: "node1".to_string(),
            address: "127.0.0.1:8080".to_string(),
            available_capacity: 1024 * 1024 * 1024,
            used_capacity: 0,
            status: NodeStatus::Online,
            location: Some("US".to_string()),
            load_score: 0.1,
            last_heartbeat: Utc::now(),
        };
        
        let node2 = StorageNodeInfo {
            node_id: "node2".to_string(),
            address: "127.0.0.1:8081".to_string(),
            available_capacity: 1024 * 1024 * 1024,
            used_capacity: 0,
            status: NodeStatus::Online,
            location: Some("EU".to_string()),
            load_score: 0.2,
            last_heartbeat: Utc::now(),
        };
        
        let node3 = StorageNodeInfo {
            node_id: "node3".to_string(),
            address: "127.0.0.1:8082".to_string(),
            available_capacity: 1024 * 1024 * 1024,
            used_capacity: 0,
            status: NodeStatus::Online,
            location: Some("ASIA".to_string()),
            load_score: 0.3,
            last_heartbeat: Utc::now(),
        };
        
        manager.register_storage_node(node1).await.unwrap();
        manager.register_storage_node(node2).await.unwrap();
        manager.register_storage_node(node3).await.unwrap();
        
        // Debug: Check how many nodes are registered
        let stats = manager.get_shard_stats().await;
        println!("Debug: Total nodes: {}, Online nodes: {}", stats.total_nodes, stats.online_nodes);
        
        // Create shards
        let data = b"Hello, World! This is a test file for sharding.";
        let shards = manager.create_shards("test_file", data.len() as u64, data).await.unwrap();
        
        assert!(!shards.is_empty());
        
        let stats = manager.get_shard_stats().await;
        assert_eq!(stats.total_shards, shards.len());
    }

    #[tokio::test]
    async fn test_shard_count_calculation() {
        let count = ShardManager::calculate_shard_count(2048, 1024);
        assert_eq!(count, 2);
        
        let count = ShardManager::calculate_shard_count(1024, 1024);
        assert_eq!(count, 1);
        
        let count = ShardManager::calculate_shard_count(512, 1024);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_shard_hash_calculation() {
        let data = b"test data";
        let hash = ShardManager::calculate_shard_hash(data);
        
        assert_eq!(hash.len(), 32);
        assert_ne!(hash, [0u8; 32]);
    }
}
