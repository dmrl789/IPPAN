//! File descriptor data model for IPNDHT file tracking.

use ippan_crypto::hash_functions::{Blake3, HashFunction};
use ippan_time::{HashTimer, IppanTimeMicros};
use serde::{Deserialize, Serialize};

/// Unique identifier for a file descriptor, derived from HashTimer.
/// This provides deterministic, time-ordered IDs for file metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct FileId(pub [u8; 32]);

impl FileId {
    /// Create a new FileId from a HashTimer.
    pub fn from_hashtimer(timer: &HashTimer) -> Self {
        Self(timer.digest())
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string.
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        if hex_str.len() != 64 {
            return Err(format!(
                "FileId hex must be 64 characters, got {}",
                hex_str.len()
            ));
        }
        let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {e}"))?;
        if bytes.len() != 32 {
            return Err("FileId must be 32 bytes".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// Content hash for file data (BLAKE3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Hash the given data using BLAKE3.
    pub fn from_data(data: &[u8]) -> Self {
        let hasher = Blake3::new();
        Self(hasher.hash_fixed(data))
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string.
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        if hex_str.len() != 64 {
            return Err(format!(
                "ContentHash hex must be 64 characters, got {}",
                hex_str.len()
            ));
        }
        let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {e}"))?;
        if bytes.len() != 32 {
            return Err("ContentHash must be 32 bytes".to_string());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

/// File descriptor metadata record.
///
/// This structure contains all metadata about a file without storing the actual content.
/// IDs are deterministically generated using HashTimer + content hash for uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDescriptor {
    /// Unique identifier derived from HashTimer + content hash.
    pub id: FileId,

    /// BLAKE3 hash of file content.
    pub content_hash: ContentHash,

    /// Owner's address (32-byte).
    pub owner: [u8; 32],

    /// File size in bytes.
    pub size_bytes: u64,

    /// Creation timestamp (microseconds, HashTimer-based).
    pub created_at_us: u64,

    /// Optional MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Optional tags for categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl FileDescriptor {
    /// Create a new file descriptor with deterministic ID.
    ///
    /// The ID is computed from: HashTimer.derive(context="file", time, content_hash, owner).
    /// This ensures uniqueness while maintaining determinism and time-ordering.
    pub fn new(
        content_hash: ContentHash,
        owner: [u8; 32],
        size_bytes: u64,
        mime_type: Option<String>,
        tags: Vec<String>,
    ) -> Self {
        let time = IppanTimeMicros::now();
        Self::new_at_time(content_hash, owner, size_bytes, time, mime_type, tags)
    }

    /// Create a file descriptor with explicit timestamp (for testing/reconstruction).
    pub fn new_at_time(
        content_hash: ContentHash,
        owner: [u8; 32],
        size_bytes: u64,
        time: IppanTimeMicros,
        mime_type: Option<String>,
        tags: Vec<String>,
    ) -> Self {
        // Derive deterministic ID: context="file", domain=content_hash, payload=owner
        let timer = HashTimer::derive(
            "file",
            time,
            content_hash.as_bytes(), // domain
            &owner,                  // payload
            &[0u8; 32],              // nonce (deterministic, use zero)
            &owner,                  // node_id (use owner for consistency)
        );

        let id = FileId::from_hashtimer(&timer);

        Self {
            id,
            content_hash,
            owner,
            size_bytes,
            created_at_us: time.0,
            mime_type,
            tags,
        }
    }

    /// Validate the descriptor fields.
    pub fn validate(&self) -> Result<(), String> {
        if self.size_bytes == 0 {
            return Err("File size cannot be zero".to_string());
        }

        if let Some(mime) = &self.mime_type {
            if mime.len() > 128 {
                return Err("MIME type too long (max 128 chars)".to_string());
            }
        }

        if self.tags.len() > 32 {
            return Err("Too many tags (max 32)".to_string());
        }

        for tag in &self.tags {
            if tag.is_empty() || tag.len() > 64 {
                return Err("Tag must be 1-64 characters".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_id_deterministic() {
        let content_hash = ContentHash::from_data(b"test content");
        let owner = [1u8; 32];
        let time = IppanTimeMicros(1000000);

        let desc1 = FileDescriptor::new_at_time(content_hash, owner, 100, time, None, vec![]);
        let desc2 = FileDescriptor::new_at_time(content_hash, owner, 100, time, None, vec![]);

        assert_eq!(desc1.id, desc2.id, "IDs must be deterministic");
        assert_eq!(desc1.created_at_us, 1000000);
    }

    #[test]
    fn test_file_id_unique_for_different_content() {
        let content1 = ContentHash::from_data(b"content1");
        let content2 = ContentHash::from_data(b"content2");
        let owner = [1u8; 32];
        let time = IppanTimeMicros(1000000);

        let desc1 = FileDescriptor::new_at_time(content1, owner, 100, time, None, vec![]);
        let desc2 = FileDescriptor::new_at_time(content2, owner, 100, time, None, vec![]);

        assert_ne!(
            desc1.id, desc2.id,
            "Different content must have different IDs"
        );
    }

    #[test]
    fn test_file_id_hex_roundtrip() {
        let content_hash = ContentHash::from_data(b"test");
        let owner = [7u8; 32];
        let desc = FileDescriptor::new(content_hash, owner, 42, None, vec![]);

        let hex = desc.id.to_hex();
        assert_eq!(hex.len(), 64);

        let parsed = FileId::from_hex(&hex).unwrap();
        assert_eq!(parsed, desc.id);
    }

    #[test]
    fn test_content_hash_from_data() {
        let data = b"hello world";
        let hash1 = ContentHash::from_data(data);
        let hash2 = ContentHash::from_data(data);

        assert_eq!(hash1, hash2, "Hash must be deterministic");
        assert_ne!(hash1.0, [0u8; 32], "Hash must not be zero");
    }

    #[test]
    fn test_descriptor_validation() {
        let content_hash = ContentHash::from_data(b"test");
        let owner = [1u8; 32];

        // Valid descriptor
        let desc = FileDescriptor::new(content_hash, owner, 100, None, vec![]);
        assert!(desc.validate().is_ok());

        // Zero size - invalid
        let mut desc_zero_size = desc.clone();
        desc_zero_size.size_bytes = 0;
        assert!(desc_zero_size.validate().is_err());

        // Too many tags
        let desc_many_tags = FileDescriptor::new(
            content_hash,
            owner,
            100,
            None,
            (0..33).map(|i| format!("tag{}", i)).collect(),
        );
        assert!(desc_many_tags.validate().is_err());

        // Long MIME type
        let desc_long_mime =
            FileDescriptor::new(content_hash, owner, 100, Some("a".repeat(200)), vec![]);
        assert!(desc_long_mime.validate().is_err());
    }

    #[test]
    fn test_descriptor_with_metadata() {
        let content_hash = ContentHash::from_data(b"image data");
        let owner = [5u8; 32];
        let tags = vec!["image".to_string(), "test".to_string()];

        let desc = FileDescriptor::new(
            content_hash,
            owner,
            1024,
            Some("image/png".to_string()),
            tags.clone(),
        );

        assert_eq!(desc.size_bytes, 1024);
        assert_eq!(desc.mime_type, Some("image/png".to_string()));
        assert_eq!(desc.tags, tags);
        assert!(desc.validate().is_ok());
    }
}
