//! DHT hooks for publishing and looking up file descriptors via IPNDHT.

use crate::descriptor::{FileDescriptor, FileId};
use anyhow::Result;
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
pub trait FileDhtService: Send + Sync {
    /// Publish a file descriptor to the DHT.
    /// 
    /// This stores a DHT record mapping file_id -> (content_hash, owner, size)
    /// so other nodes can discover the file's metadata.
    fn publish_file(&self, descriptor: &FileDescriptor) -> Result<DhtPublishResult>;
    
    /// Find a file descriptor by ID from the DHT.
    /// 
    /// This queries the DHT for the record associated with file_id.
    fn find_file(&self, id: &FileId) -> Result<DhtLookupResult>;
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

impl FileDhtService for StubFileDhtService {
    fn publish_file(&self, descriptor: &FileDescriptor) -> Result<DhtPublishResult> {
        // Stub: always succeeds but doesn't actually publish
        Ok(DhtPublishResult {
            file_id: descriptor.id,
            success: true,
            message: Some("stub: not published to DHT".to_string()),
        })
    }
    
    fn find_file(&self, id: &FileId) -> Result<DhtLookupResult> {
        // Stub: never finds anything
        Ok(DhtLookupResult {
            file_id: *id,
            descriptor: None,
            providers: vec![],
        })
    }
}

/// DHT service backed by libp2p Kademlia.
/// 
/// This integrates with the existing libp2p network stack to provide
/// actual DHT functionality for file descriptor discovery.
#[cfg(feature = "libp2p")]
pub struct Libp2pFileDhtService {
    // TODO: Add libp2p network handle once integrated
    // For now, this is a placeholder structure
    _placeholder: (),
}

#[cfg(feature = "libp2p")]
impl Libp2pFileDhtService {
    pub fn new(/* libp2p_network: Arc<Libp2pNetwork> */) -> Self {
        Self {
            _placeholder: (),
        }
    }
}

#[cfg(feature = "libp2p")]
impl FileDhtService for Libp2pFileDhtService {
    fn publish_file(&self, descriptor: &FileDescriptor) -> Result<DhtPublishResult> {
        // TODO: Implement actual DHT put using libp2p kad::Behaviour::put_record
        // Key: file_id as bytes
        // Value: serialized descriptor metadata (or minimal: content_hash + owner)
        
        Ok(DhtPublishResult {
            file_id: descriptor.id,
            success: true,
            message: Some("libp2p: published to DHT (placeholder)".to_string()),
        })
    }
    
    fn find_file(&self, id: &FileId) -> Result<DhtLookupResult> {
        // TODO: Implement actual DHT get using libp2p kad::Behaviour::get_record
        // Query for key = file_id
        // Parse returned record into FileDescriptor
        
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

    #[test]
    fn test_stub_publish() {
        let dht = StubFileDhtService::new();
        let content_hash = ContentHash::from_data(b"test");
        let desc = FileDescriptor::new(content_hash, [1u8; 32], 100, None, vec![]);
        
        let result = dht.publish_file(&desc).unwrap();
        assert_eq!(result.file_id, desc.id);
        assert!(result.success);
    }

    #[test]
    fn test_stub_find() {
        let dht = StubFileDhtService::new();
        let id = FileId::from_bytes([42u8; 32]);
        
        let result = dht.find_file(&id).unwrap();
        assert_eq!(result.file_id, id);
        assert_eq!(result.descriptor, None);
    }
}
