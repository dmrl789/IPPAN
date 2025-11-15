use crate::{Address, HashTimer};
use blake3::Hasher;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use thiserror::Error;

const FILE_DESCRIPTOR_CONTEXT: &str = "ipn-file-descriptor";

/// 32-byte content hash identifier (BLAKE3 preferred).
pub type ContentHash = [u8; 32];

/// Errors emitted when decoding or constructing file descriptor identifiers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FileDescriptorIdError {
    #[error("file descriptor id must be 64 hex characters, got {0}")]
    InvalidLength(usize),
    #[error("file descriptor id must be valid hex: {0}")]
    InvalidHex(String),
}

/// Canonical identifier for file descriptors (HashTimer + content hash domain separation).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FileDescriptorId([u8; 32]);

impl FileDescriptorId {
    /// Derive a deterministic identifier from the HashTimer, content hash, and owner.
    pub fn derive(timer: &HashTimer, content_hash: &ContentHash, owner: &Address) -> Self {
        let digest = timer.digest();
        let mut hasher = Hasher::new();
        hasher.update(FILE_DESCRIPTOR_CONTEXT.as_bytes());
        hasher.update(&digest);
        hasher.update(content_hash);
        hasher.update(&owner.0);
        let hash = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&hash.as_bytes()[0..32]);
        FileDescriptorId(id)
    }

    /// Construct from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        FileDescriptorId(bytes)
    }

    /// Borrow the raw byte representation.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consume into owned bytes.
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Render as lowercase hexadecimal.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for FileDescriptorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileDescriptorId({})", self.to_hex())
    }
}

impl From<FileDescriptorId> for String {
    fn from(value: FileDescriptorId) -> Self {
        value.to_hex()
    }
}

impl TryFrom<String> for FileDescriptorId {
    type Error = FileDescriptorIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 64 {
            return Err(FileDescriptorIdError::InvalidLength(value.len()));
        }
        let bytes = hex::decode(&value)
            .map_err(|err| FileDescriptorIdError::InvalidHex(err.to_string()))?;
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes);
        Ok(FileDescriptorId(id))
    }
}

/// Metadata describing a published file hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDescriptor {
    /// Deterministic identifier derived from the HashTimer, content hash, and owner.
    pub id: FileDescriptorId,
    /// Content hash encoded as 32 raw bytes (serialized as hex).
    #[serde(
        serialize_with = "serialize_content_hash",
        deserialize_with = "deserialize_content_hash"
    )]
    pub content_hash: ContentHash,
    /// Publishing account owner.
    pub owner: Address,
    /// File size in bytes.
    pub size_bytes: u64,
    /// HashTimer capturing when the descriptor was created.
    pub created_at: HashTimer,
    /// Optional MIME type for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional free-form tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl FileDescriptor {
    /// Build a descriptor and derive its identifier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        created_at: HashTimer,
        owner: Address,
        content_hash: ContentHash,
        size_bytes: u64,
        mime_type: Option<String>,
        tags: Vec<String>,
    ) -> Self {
        let id = FileDescriptorId::derive(&created_at, &content_hash, &owner);
        Self {
            id,
            content_hash,
            owner,
            size_bytes,
            created_at,
            mime_type,
            tags,
        }
    }
}

fn serialize_content_hash<S>(hash: &ContentHash, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&hex::encode(hash))
}

fn deserialize_content_hash<'de, D>(deserializer: D) -> Result<ContentHash, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let bytes = hex::decode(&value)
        .map_err(|err| serde::de::Error::custom(format!("invalid hex: {err}")))?;
    if bytes.len() != 32 {
        return Err(serde::de::Error::custom(format!(
            "content hash must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HashTimer;
    use crate::IppanTimeMicros;

    fn sample_descriptor() -> FileDescriptor {
        let owner = Address([1u8; 32]);
        let time = IppanTimeMicros(42);
        let hashtimer =
            HashTimer::derive("file", time, b"files", b"payload", &[0u8; 32], &[0u8; 32]);
        FileDescriptor::new(
            hashtimer,
            owner,
            [3u8; 32],
            512,
            Some("text/plain".into()),
            vec![],
        )
    }

    #[test]
    fn derives_consistent_ids() {
        let descriptor = sample_descriptor();
        let id_again = FileDescriptorId::derive(
            &descriptor.created_at,
            &descriptor.content_hash,
            &descriptor.owner,
        );
        assert_eq!(descriptor.id, id_again);
    }

    #[test]
    fn id_hex_roundtrip() {
        let descriptor = sample_descriptor();
        let hex = descriptor.id.to_hex();
        let parsed = FileDescriptorId::try_from(hex.clone()).expect("parse id");
        assert_eq!(parsed, descriptor.id);
        assert_eq!(String::from(parsed), hex);
    }

    #[test]
    fn serialize_descriptor_to_json() {
        let descriptor = sample_descriptor();
        let json = serde_json::to_string(&descriptor).expect("serialize descriptor");
        assert!(json.contains("\"content_hash\":\"0303"));
        let restored: FileDescriptor = serde_json::from_str(&json).expect("deserialize descriptor");
        assert_eq!(restored.id, descriptor.id);
        assert_eq!(restored.content_hash, descriptor.content_hash);
    }
}
