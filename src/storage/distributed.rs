//! Distributed storage for IPPAN

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use crate::storage::encryption::{EncryptionManager, EncryptedData, EncryptionAlgorithm};

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
    /// Encrypted file data
    pub encrypted_data: Option<EncryptedData>,
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
    /// Encrypted shard data
    pub encrypted_data: Option<EncryptedData>,
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

/// Store file request
#[derive(Debug, Clone)]
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
    /// Encryption key ID (optional, will generate if not provided)
    pub encryption_key_id: Option<String>,
}

/// Retrieve file request
#[derive(Debug, Clone)]
pub struct RetrieveFileRequest {
    /// File ID
    pub file_id: String,
}

/// Storage operation
#[derive(Debug)]
pub enum StorageOperation {
    /// Store file operation
    StoreFile(StoreFileRequest),
    /// Retrieve file operation
    RetrieveFile(RetrieveFileRequest),
    /// Delete file operation
    DeleteFile(String),
    /// Replicate shard operation
    ReplicateShard(String),
    /// Health check operation
    HealthCheck,
}

/// Operation result
#[derive(Debug)]
pub enum OperationResult {
    /// Success result
    Success(String),
    /// Error result
    Error(String),
}

/// Distributed storage manager
pub struct DistributedStorage {
    /// Storage nodes
    nodes: Arc<RwLock<HashMap<String, StorageNode>>>,
    /// File metadata
    files: Arc<RwLock<HashMap<String, FileMetadata>>>,
    /// Storage shards
    shards: Arc<RwLock<HashMap<String, StorageShard>>>,
    /// Encryption manager
    encryption_manager: Arc<RwLock<EncryptionManager>>,
    /// Operation sender
    _operation_sender: mpsc::Sender<StorageOperation>,
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
        
        // Initialize encryption manager
        let encryption_manager = EncryptionManager::new(90)?; // 90-day key rotation
        let mut encryption_manager = encryption_manager;
        encryption_manager.start().await?;
        
        Ok(Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            files: Arc::new(RwLock::new(HashMap::new())),
            shards: Arc::new(RwLock::new(HashMap::new())),
            encryption_manager: Arc::new(RwLock::new(encryption_manager)),
            _operation_sender: operation_sender,
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
        let _nodes = self.nodes.clone();
        let _files = self.files.clone();
        let _shards = self.shards.clone();
        
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
        
        // Stop encryption manager
        let mut encryption_manager = self.encryption_manager.write().await;
        encryption_manager.stop().await?;
        
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

    /// Store a file with encryption
    pub async fn store_file(&self, request: StoreFileRequest) -> Result<String> {
        let file_id = request.file_id.clone();
        let file_hash = Self::calculate_file_hash(&request.data);
        
        // Get or generate encryption key
        let encryption_manager = self.encryption_manager.read().await;
        let key_id = if let Some(key_id) = request.encryption_key_id {
            key_id
        } else {
            // Generate new encryption key
            let new_key_id = format!("key_{}", uuid::Uuid::new_v4());
            encryption_manager.generate_key(&new_key_id, EncryptionAlgorithm::Aes256Gcm).await?;
            new_key_id
        };
        
        // Encrypt file data
        let encrypted_data = encryption_manager.encrypt_data(&request.data, &key_id).await?;
        
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
            encryption_key_id: Some(key_id),
            encrypted_data: Some(encrypted_data),
        };
        
        // Store metadata
        let mut files = self.files.write().await;
        files.insert(file_id.clone(), metadata);
        
        log::info!("Stored encrypted file: {} ({} bytes)", file_id, request.data.len());
        Ok(file_id)
    }

    /// Retrieve a file with decryption
    pub async fn retrieve_file(&self, request: RetrieveFileRequest) -> Result<Vec<u8>> {
        let file_id = request.file_id.clone();
        
        // Get file metadata
        let files = self.files.read().await;
        let metadata = files.get(&file_id)
            .ok_or_else(|| crate::error::IppanError::Storage(format!("File not found: {}", file_id)))?;
        
        // Check if file is encrypted
        let encrypted_data = metadata.encrypted_data.as_ref()
            .ok_or_else(|| crate::error::IppanError::Storage("File is not encrypted".to_string()))?;
        
        // Decrypt file data
        let encryption_manager = self.encryption_manager.read().await;
        let decrypted_data = encryption_manager.decrypt_data(encrypted_data).await?;
        
        log::info!("Retrieved and decrypted file: {} ({} bytes)", file_id, decrypted_data.len());
        Ok(decrypted_data)
    }

    /// Delete a file
    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        let mut files = self.files.write().await;
        if files.remove(file_id).is_some() {
            log::info!("Deleted file: {}", file_id);
        }
        Ok(())
    }

    /// Get file metadata
    pub async fn get_file_metadata(&self, file_id: &str) -> Result<Option<FileMetadata>> {
        let files = self.files.read().await;
        Ok(files.get(file_id).cloned())
    }

    /// List all files
    pub async fn list_files(&self) -> Result<Vec<FileMetadata>> {
        let files = self.files.read().await;
        Ok(files.values().cloned().collect())
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let nodes = self.nodes.read().await;
        let files = self.files.read().await;
        let shards = self.shards.read().await;
        
        let total_capacity: u64 = nodes.values().map(|n| n.capacity).sum();
        let used_storage: u64 = nodes.values().map(|n| n.used_storage).sum();
        let file_count = files.len();
        let shard_count = shards.len();
        
        Ok(StorageStats {
            node_count: nodes.len(),
            total_capacity,
            used_storage,
            file_count,
            shard_count,
        })
    }

    /// Calculate file hash
    fn calculate_file_hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Calculate shard count
    fn calculate_shard_count(file_size: u64, shard_size: u64) -> u32 {
        ((file_size + shard_size - 1) / shard_size) as u32
    }

    /// Create shards from file data
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
            
            let shard_id = format!("{}_{}", file_id, i);
            let data_hash = Self::calculate_file_hash(shard_data);
            
            let shard = StorageShard {
                shard_id,
                file_id: file_id.to_string(),
                index: i,
                data_hash,
                size: shard_data.len() as u64,
                storage_nodes: Vec::new(),
                status: ShardStatus::Healthy,
                created_at: Utc::now(),
                encrypted_data: None, // Shards will be encrypted separately if needed
            };
            
            shards.push(shard);
        }
        
        Ok(shards)
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Number of storage nodes
    pub node_count: usize,
    /// Total storage capacity (bytes)
    pub total_capacity: u64,
    /// Used storage (bytes)
    pub used_storage: u64,
    /// Number of files
    pub file_count: usize,
    /// Number of shards
    pub shard_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_node_registration() {
        let storage = DistributedStorage::new(3, 1024 * 1024).await.unwrap();
        
        let node = StorageNode {
            node_id: "test_node".to_string(),
            address: "127.0.0.1:8080".to_string(),
            capacity: 1024 * 1024 * 1024,
            used_storage: 0,
            status: NodeStatus::Online,
            last_heartbeat: Utc::now(),
            score: 1.0,
        };
        
        storage.register_node(node).await.unwrap();
        
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.node_count, 1);
    }

    #[tokio::test]
    async fn test_file_storage_and_retrieval() {
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
        
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.file_count, 1);
    }

    #[tokio::test]
    async fn test_encryption_integration() {
        let storage = DistributedStorage::new(3, 1024 * 1024).await.unwrap();
        
        // Test data
        let test_data = b"This is sensitive data that should be encrypted!";
        
        let request = StoreFileRequest {
            file_id: "encrypted_test_file".to_string(),
            name: "secret.txt".to_string(),
            data: test_data.to_vec(),
            mime_type: "text/plain".to_string(),
            replication_factor: 3,
            encryption_key_id: None, // Will generate a new key
        };
        
        // Store encrypted file
        let file_id = storage.store_file(request).await.unwrap();
        assert_eq!(file_id, "encrypted_test_file");
        
        // Retrieve and decrypt file
        let retrieve_request = RetrieveFileRequest {
            file_id: "encrypted_test_file".to_string(),
        };
        
        let retrieved_data = storage.retrieve_file(retrieve_request).await.unwrap();
        assert_eq!(retrieved_data, test_data);
        
        // Verify the data was actually encrypted (metadata should show encryption)
        let metadata = storage.get_file_metadata("encrypted_test_file").await.unwrap().unwrap();
        assert!(metadata.encryption_key_id.is_some());
        assert!(metadata.encrypted_data.is_some());
        
        println!("✅ Encryption integration test passed!");
        println!("   - File stored with encryption key: {}", metadata.encryption_key_id.unwrap());
        println!("   - Data successfully encrypted and decrypted");
        println!("   - Original data: {} bytes", test_data.len());
        println!("   - Retrieved data: {} bytes", retrieved_data.len());
    }
} 