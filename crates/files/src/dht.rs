//! DHT hooks for publishing and looking up file descriptors via IPNDHT.

use crate::descriptor::{FileDescriptor, FileId};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result of publishing a file descriptor to the DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtPublishResult {
    /// File ID that was published.
    pub file_id: FileId,

    /// Whether the publish operation succeeded.
    pub success: bool,

    /// Optional message (e.g., error details).
    pub message: Option<String>,
}

/// Result of looking up a file descriptor from the DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtLookupResult {
    /// File ID that was queried.
    pub file_id: FileId,

    /// Found descriptor, if any.
    pub descriptor: Option<FileDescriptor>,

    /// Optional provider peer IDs (if supported).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<String>,
}

/// Service for publishing and finding file descriptors via DHT.
///
/// This is a thin adapter layer over libp2p Kademlia DHT.
/// For this initial version, it provides basic put/get semantics.
#[async_trait]
pub trait FileDhtService: Send + Sync {
    /// Publish a file descriptor to the DHT.
    ///
    /// This stores a DHT record mapping file_id -> (content_hash, owner, size)
    /// so other nodes can discover the file's metadata.
    async fn publish_file(&self, descriptor: &FileDescriptor) -> Result<DhtPublishResult>;

    /// Find a file descriptor by ID from the DHT.
    ///
    /// This queries the DHT for the record associated with file_id.
    async fn find_file(&self, id: &FileId) -> Result<DhtLookupResult>;
}

/// Stub implementation for testing/local mode without DHT.
pub struct StubFileDhtService;

impl StubFileDhtService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubFileDhtService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileDhtService for StubFileDhtService {
    async fn publish_file(&self, descriptor: &FileDescriptor) -> Result<DhtPublishResult> {
        // Stub: always succeeds but doesn't actually publish
        Ok(DhtPublishResult {
            file_id: descriptor.id,
            success: true,
            message: Some("stub: not published to DHT".to_string()),
        })
    }

    async fn find_file(&self, id: &FileId) -> Result<DhtLookupResult> {
        // Stub: never finds anything
        Ok(DhtLookupResult {
            file_id: *id,
            descriptor: None,
            providers: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{ContentHash, FileDescriptor};

    #[tokio::test]
    async fn test_stub_publish() {
        let dht = StubFileDhtService::new();
        let content_hash = ContentHash::from_data(b"test");
        let desc = FileDescriptor::new(content_hash, [1u8; 32], 100, None, vec![]);

        let result = dht.publish_file(&desc).await.unwrap();
        assert_eq!(result.file_id, desc.id);
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_stub_find() {
        let dht = StubFileDhtService::new();
        let id = FileId::from_bytes([42u8; 32]);

        let result = dht.find_file(&id).await.unwrap();
        assert_eq!(result.file_id, id);
        assert_eq!(result.descriptor, None);
    }
}
