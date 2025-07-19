//! Distributed storage for IPPAN

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

/// Storage node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageNode {
    /// Node ID
    pub node_id: String,
    /// Node address
    pub address: String,
    /// Available storage capacity (bytes)
    pub capacity: u64,
    /// Used storage (bytes)
    pub used_storage: u64,
    /// Node status
    pub status: NodeStatus,
    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,
    /// Node score
    pub score: f64,
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

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File ID
    pub file_id: String,
    /// File name
    pub name: String,
    /// File size (bytes)
    pub size: u64,
    /// File hash
    pub hash: [u8; 32],
    /// MIME type
    pub mime_type: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
    /// Replication factor
    pub replication_factor: u32,
    /// Shard count
    pub shard_count: u32,
    /// Encryption key ID
    pub encryption_key_id: Option<String>,
    /// File data (for testing)
    pub file_data: Option<Vec<u8>>,
}

/// Storage shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageShard {
    /// Shard ID
    pub shard_id: String,
    /// File ID
    pub file_id: String,
    /// Shard index
    pub index: u32,
    /// Shard data hash
    pub data_hash: [u8; 32],
    /// Shard size (bytes)
    pub size: u64,
    /// Storage nodes holding this shard
    pub storage_nodes: Vec<String>,
    /// Shard status
    pub status: ShardStatus,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
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
}

/// Storage operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageOperation {
    /// Store file
    StoreFile(StoreFileRequest),
    /// Retrieve file
    RetrieveFile(RetrieveFileRequest),
    /// Delete file
    DeleteFile(DeleteFileRequest),
    /// Replicate shard
    ReplicateShard(ReplicateShardRequest),
    /// Health check
    HealthCheck(HealthCheckRequest),
}

/// Store file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreFileRequest {
    /// File ID
    pub file_id: String,
    /// File name
    pub name: String,
    /// File data
    pub data: Vec<u8>,
    /// MIME type
    pub mime_type: String,
    /// Replication factor
    pub replication_factor: u32,
    /// Encryption key ID
    pub encryption_key_id: Option<String>,
}

/// Retrieve file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveFileRequest {
    /// File ID
    pub file_id: String,
    /// Shard index (optional, for partial retrieval)
    pub shard_index: Option<u32>,
}

/// Delete file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFileRequest {
    /// File ID
    pub file_id: String,
}

/// Replicate shard request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicateShardRequest {
    /// Shard ID
    pub shard_id: String,
    /// Target node ID
    pub target_node_id: String,
    /// Shard data
    pub shard_data: Vec<u8>,
}

/// Health check request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRequest {
    /// Node ID
    pub node_id: String,
    /// Available storage
    pub available_storage: u64,
    /// Used storage
    pub used_storage: u64,
}

/// Distributed storage manager
pub struct DistributedStorage {
    /// Storage nodes
    nodes: Arc<RwLock<HashMap<String, StorageNode>>>,
    /// File metadata
    files: Arc<RwLock<HashMap<String, FileMetadata>>>,
    /// Storage shards
    shards: Arc<RwLock<HashMap<String, StorageShard>>>,
    /// Operation sender
    operation_sender: mpsc::Sender<StorageOperation>,
    /// Replication factor
    replication_factor: u32,
    /// Shard size (bytes)
    shard_size: u64,
    /// Running flag
    running: bool,
}

impl DistributedStorage {
    /// Create a new distributed storage manager
    pub async fn new(replication_factor: u32, shard_size: u64) -> Result<Self> {
        let (operation_sender, _operation_receiver) = mpsc::channel(1000);
        
        Ok(Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            files: Arc::new(RwLock::new(HashMap::new())),
            shards: Arc::new(RwLock::new(HashMap::new())),
            operation_sender,
            replication_factor,
            shard_size,
            running: false,
        })
    }

    /// Start the distributed storage manager
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting distributed storage manager");
        self.running = true;
        
        // Start operation processing loop
        let nodes = self.nodes.clone();
        let files = self.files.clone();
        let shards = self.shards.clone();
        
        tokio::spawn(async move {
            // TODO: Implement operation processing loop
            log::info!("Distributed storage manager started");
        });
        
        Ok(())
    }

    /// Stop the distributed storage manager
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping distributed storage manager");
        self.running = false;
        Ok(())
    }

    /// Register a storage node
    pub async fn register_node(&self, node: StorageNode) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let node_id = node.node_id.clone();
        nodes.insert(node_id.clone(), node);
        log::info!("Registered storage node: {}", node_id);
        Ok(())
    }

    /// Unregister a storage node
    pub async fn unregister_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if nodes.remove(node_id).is_some() {
            log::info!("Unregistered storage node: {}", node_id);
        }
        Ok(())
    }

    /// Store a file
    pub async fn store_file(&self, request: StoreFileRequest) -> Result<String> {
        let file_id = request.file_id.clone();
        let file_hash = Self::calculate_file_hash(&request.data);
        
        // Create file metadata
        let metadata = FileMetadata {
            file_id: file_id.clone(),
            name: request.name,
            size: request.data.len() as u64,
            hash: file_hash,
            mime_type: request.mime_type,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            replication_factor: request.replication_factor,
            shard_count: Self::calculate_shard_count(request.data.len() as u64, self.shard_size),
            encryption_key_id: request.encryption_key_id,
            file_data: Some(request.data.clone()), // Store the actual data in metadata for testing
        };
        
        // Store metadata
        let mut files = self.files.write().await;
        files.insert(file_id.clone(), metadata);
        
        // Create shards
        let shards = Self::create_shards(&file_id, &request.data, self.shard_size, self.replication_factor).await?;
        
        // Store shards
        let mut shards_map = self.shards.write().await;
        for shard in &shards {
            shards_map.insert(shard.shard_id.clone(), shard.clone());
        }
        
        log::info!("Stored file: {} ({} shards)", file_id, shards.len());
        Ok(file_id)
    }

    /// Retrieve a file
    pub async fn retrieve_file(&self, request: RetrieveFileRequest) -> Result<Vec<u8>> {
        let files = self.files.read().await;
        let shards = self.shards.read().await;
        
        let metadata = files.get(&request.file_id).ok_or_else(|| {
            crate::error::IppanError::Storage(
                format!("File not found: {}", request.file_id)
            )
        })?;
        
        // For now, we'll store the actual file data in the metadata
        // In a real implementation, this would be stored in the shards
        if let Some(file_data) = &metadata.file_data {
            log::info!("Retrieved file: {} ({} bytes)", request.file_id, file_data.len());
            return Ok(file_data.clone());
        }
        
        // Fallback: collect shards (this is the current implementation that returns zeros)
        let mut file_data = Vec::new();
        let file_shards: Vec<_> = shards.values()
            .filter(|shard| shard.file_id == request.file_id)
            .collect();
        
        for shard in file_shards {
            // TODO: Actually retrieve shard data from storage nodes
            // For now, we'll simulate the data
            let shard_data = vec![0u8; shard.size as usize];
            file_data.extend_from_slice(&shard_data);
        }
        
        log::info!("Retrieved file: {} ({} bytes)", request.file_id, file_data.len());
        Ok(file_data)
    }

    /// Delete a file
    pub async fn delete_file(&self, request: DeleteFileRequest) -> Result<()> {
        let mut files = self.files.write().await;
        let mut shards = self.shards.write().await;
        
        // Remove file metadata
        if files.remove(&request.file_id).is_some() {
            // Remove associated shards
            shards.retain(|_, shard| shard.file_id != request.file_id);
            log::info!("Deleted file: {}", request.file_id);
        }
        
        Ok(())
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> StorageStats {
        let nodes = self.nodes.read().await;
        let files = self.files.read().await;
        let shards = self.shards.read().await;
        
        let total_nodes = nodes.len();
        let online_nodes = nodes.values()
            .filter(|node| node.status == NodeStatus::Online)
            .count();
        
        let total_files = files.len();
        let total_shards = shards.len();
        
        let total_capacity: u64 = nodes.values()
            .map(|node| node.capacity)
            .sum();
        
        let total_used: u64 = nodes.values()
            .map(|node| node.used_storage)
            .sum();
        
        StorageStats {
            total_nodes,
            online_nodes,
            total_files,
            total_shards,
            total_capacity,
            total_used,
            replication_factor: self.replication_factor,
        }
    }

    /// Calculate file hash
    fn calculate_file_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Calculate number of shards needed
    fn calculate_shard_count(file_size: u64, shard_size: u64) -> u32 {
        ((file_size + shard_size - 1) / shard_size) as u32
    }

    /// Create shards for a file
    async fn create_shards(
        file_id: &str,
        data: &[u8],
        shard_size: u64,
        _replication_factor: u32,
    ) -> Result<Vec<StorageShard>> {
        let mut shards = Vec::new();
        let shard_count = Self::calculate_shard_count(data.len() as u64, shard_size);
        
        for i in 0..shard_count {
            let start = (i as u64 * shard_size) as usize;
            let end = std::cmp::min(start + shard_size as usize, data.len());
            let shard_data = &data[start..end];
            
            let shard_hash = Self::calculate_file_hash(shard_data);
            let shard_id = format!("{}_{}", file_id, i);
            
            let shard = StorageShard {
                shard_id,
                file_id: file_id.to_string(),
                index: i,
                data_hash: shard_hash,
                size: shard_data.len() as u64,
                storage_nodes: Vec::new(), // TODO: Assign storage nodes
                status: ShardStatus::Healthy,
                created_at: Utc::now(),
            };
            
            shards.push(shard);
        }
        
        Ok(shards)
    }

    /// Process storage operation
    async fn process_operation(
        operation: StorageOperation,
        _nodes: &Arc<RwLock<HashMap<String, StorageNode>>>,
        _files: &Arc<RwLock<HashMap<String, FileMetadata>>>,
        _shards: &Arc<RwLock<HashMap<String, StorageShard>>>,
    ) {
        match operation {
            StorageOperation::StoreFile(request) => {
                log::info!("Processing store file request: {}", request.file_id);
                // TODO: Implement actual file storage
            }
            StorageOperation::RetrieveFile(request) => {
                log::info!("Processing retrieve file request: {}", request.file_id);
                // TODO: Implement actual file retrieval
            }
            StorageOperation::DeleteFile(request) => {
                log::info!("Processing delete file request: {}", request.file_id);
                // TODO: Implement actual file deletion
            }
            StorageOperation::ReplicateShard(request) => {
                log::info!("Processing replicate shard request: {}", request.shard_id);
                // TODO: Implement actual shard replication
            }
            StorageOperation::HealthCheck(request) => {
                log::info!("Processing health check for node: {}", request.node_id);
                // TODO: Update node health status
            }
        }
    }
}

/// Storage statistics
#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub total_files: usize,
    pub total_shards: usize,
    pub total_capacity: u64,
    pub total_used: u64,
    pub replication_factor: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distributed_storage_creation() {
        let storage = DistributedStorage::new(3, 1024 * 1024).await.unwrap();
        
        assert_eq!(storage.replication_factor, 3);
        assert_eq!(storage.shard_size, 1024 * 1024);
        assert!(!storage.running);
    }

    #[tokio::test]
    async fn test_distributed_storage_start_stop() {
        let mut storage = DistributedStorage::new(3, 1024 * 1024).await.unwrap();
        
        storage.start().await.unwrap();
        assert!(storage.running);
        
        storage.stop().await.unwrap();
        assert!(!storage.running);
    }

    #[tokio::test]
    async fn test_node_registration() {
        let storage = DistributedStorage::new(3, 1024 * 1024).await.unwrap();
        
        let node = StorageNode {
            node_id: "node1".to_string(),
            address: "127.0.0.1:8080".to_string(),
            capacity: 1024 * 1024 * 1024, // 1GB
            used_storage: 0,
            status: NodeStatus::Online,
            last_heartbeat: Utc::now(),
            score: 1.0,
        };
        
        storage.register_node(node).await.unwrap();
        
        let stats = storage.get_storage_stats().await;
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.online_nodes, 1);
    }

    #[tokio::test]
    async fn test_file_storage() {
        let storage = DistributedStorage::new(3, 1024 * 1024).await.unwrap();
        
        let request = StoreFileRequest {
            file_id: "test_file".to_string(),
            name: "test.txt".to_string(),
            data: b"Hello, World!".to_vec(),
            mime_type: "text/plain".to_string(),
            replication_factor: 3,
            encryption_key_id: None,
        };
        
        let file_id = storage.store_file(request).await.unwrap();
        assert_eq!(file_id, "test_file");
        
        let stats = storage.get_storage_stats().await;
        assert_eq!(stats.total_files, 1);
    }

    #[tokio::test]
    async fn test_file_hash_calculation() {
        let data = b"Hello, World!";
        let hash = DistributedStorage::calculate_file_hash(data);
        
        assert_eq!(hash.len(), 32);
        assert_ne!(hash, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_shard_count_calculation() {
        let shard_count = DistributedStorage::calculate_shard_count(2048, 1024);
        assert_eq!(shard_count, 2);
        
        let shard_count = DistributedStorage::calculate_shard_count(1024, 1024);
        assert_eq!(shard_count, 1);
        
        let shard_count = DistributedStorage::calculate_shard_count(512, 1024);
        assert_eq!(shard_count, 1);
    }
} 