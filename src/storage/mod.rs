//! Storage module for IPPAN
//! 
//! Handles encrypted, sharded storage with proof-of-storage via Merkle trees

pub mod encryption;
pub mod orchestrator;
pub mod proofs;
pub mod shards;
pub mod traffic;

pub use encryption::StorageEncryption;
pub use orchestrator::StorageOrchestrator;
pub use proofs::ProofOfStorage;
pub use shards::StorageShard;
pub use traffic::TrafficTracker;

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    error::{IppanError, Result},
    config::StorageConfig,
    NodeId,
};
use super::{
    encryption::StorageEncryption,
    shards::ShardManager,
    proofs::ProofManager,
    traffic::TrafficManager,
    orchestrator::StorageOrchestrator,
};

/// Storage manager (stub implementation)
pub struct StorageManager {
    config: StorageConfig,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: std::path::PathBuf,
    pub max_storage_size: u64,
    pub shard_size: usize,
    pub replication_factor: usize,
    pub enable_encryption: bool,
    pub proof_interval: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: std::path::PathBuf::from("./data"),
            max_storage_size: 100 * 1024 * 1024 * 1024, // 100 GB
            shard_size: 1024 * 1024, // 1 MB
            replication_factor: 3,
            enable_encryption: true,
            proof_interval: 3600, // 1 hour
        }
    }
}

/// File metadata stored in the DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File hash (content hash)
    pub hash: [u8; 32],
    /// File size in bytes
    pub size: u64,
    /// MIME type
    pub mime_type: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Shard information
    pub shards: Vec<ShardInfo>,
    /// Merkle root for proof-of-storage
    pub merkle_root: [u8; 32],
}

/// Information about a storage shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    /// Shard ID
    pub id: u32,
    /// Shard hash
    pub hash: [u8; 32],
    /// Nodes storing this shard
    pub nodes: Vec<[u8; 32]>,
    /// Shard size in bytes
    pub size: u64,
}

/// Storage statistics for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total storage used in bytes
    pub used_bytes: u64,
    /// Total storage capacity in bytes
    pub capacity_bytes: u64,
    /// Number of files stored
    pub file_count: u64,
    /// Number of shards stored
    pub shard_count: u64,
    /// Storage utilization percentage
    pub utilization_percent: f64,
}

impl StorageStats {
    pub fn new(used_bytes: u64, capacity_bytes: u64, file_count: u64, shard_count: u64) -> Self {
        let utilization_percent = if capacity_bytes > 0 {
            (used_bytes as f64 / capacity_bytes as f64) * 100.0
        } else {
            0.0
        };

        Self {
            used_bytes,
            capacity_bytes,
            file_count,
            shard_count,
            utilization_percent,
        }
    }
}

/// Storage operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageResult {
    /// File stored successfully
    Stored { file_hash: [u8; 32], shards: Vec<ShardInfo> },
    /// File retrieved successfully
    Retrieved { data: Vec<u8>, metadata: FileMetadata },
    /// File deleted successfully
    Deleted { file_hash: [u8; 32] },
    /// Storage proof verified
    ProofVerified { file_hash: [u8; 32], valid: bool },
}

/// Storage error types
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("File not found: {0:?}")]
    FileNotFound([u8; 32]),
    
    #[error("Insufficient storage space: needed {needed}, available {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    
    #[error("Shard error: {0}")]
    ShardError(String),
    
    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<StorageError> for crate::IppanError {
    fn from(err: StorageError) -> Self {
        crate::IppanError::Storage(err.to_string())
    }
}

impl StorageManager {
    pub async fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self { config })
    }
    
    pub async fn start(&self) -> Result<()> {
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size: u64,
    pub encrypted_files: u64,
    pub sharded_files: u64,
    pub active_proofs: u64,
}

/// Storage health information
#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub status: String,
    pub total_capacity: u64,
    pub used_capacity: u64,
    pub available_capacity: u64,
    pub error_count: u64,
}
