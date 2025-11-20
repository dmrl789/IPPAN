//! Storage and indexing for file descriptors.

use crate::descriptor::{FileDescriptor, FileId};
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Trait for file descriptor storage backends.
pub trait FileStorage: Send + Sync {
    /// Store a file descriptor.
    fn store(&self, descriptor: FileDescriptor) -> Result<()>;

    /// Retrieve a file descriptor by ID.
    fn get(&self, id: &FileId) -> Result<Option<FileDescriptor>>;

    /// List file descriptors by owner.
    fn list_by_owner(&self, owner: &[u8; 32]) -> Result<Vec<FileDescriptor>>;

    /// Count total descriptors.
    fn count(&self) -> Result<u64>;

    /// List all descriptors (paginated).
    fn list(&self, offset: usize, limit: usize) -> Result<Vec<FileDescriptor>>;
}

/// In-memory file descriptor storage (for testing and small deployments).
#[derive(Clone)]
pub struct MemoryFileStorage {
    inner: Arc<MemoryFileStorageInner>,
}

struct MemoryFileStorageInner {
    /// Primary index: ID -> Descriptor
    descriptors: RwLock<HashMap<FileId, FileDescriptor>>,

    /// Secondary index: Owner -> [FileId]
    by_owner: RwLock<HashMap<[u8; 32], Vec<FileId>>>,

    /// Ordered index for pagination (by creation time)
    by_time: RwLock<BTreeMap<u64, FileId>>,
}

impl MemoryFileStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MemoryFileStorageInner {
                descriptors: RwLock::new(HashMap::new()),
                by_owner: RwLock::new(HashMap::new()),
                by_time: RwLock::new(BTreeMap::new()),
            }),
        }
    }
}

impl Default for MemoryFileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStorage for MemoryFileStorage {
    fn store(&self, descriptor: FileDescriptor) -> Result<()> {
        // Validate before storing
        descriptor
            .validate()
            .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

        let id = descriptor.id;
        let owner = descriptor.owner;
        let created_at = descriptor.created_at_us;

        // Store in primary index
        {
            let mut descriptors = self.inner.descriptors.write();
            descriptors.insert(id, descriptor);
        }

        // Update owner index
        {
            let mut by_owner = self.inner.by_owner.write();
            by_owner.entry(owner).or_default().push(id);
        }

        // Update time index
        {
            let mut by_time = self.inner.by_time.write();
            by_time.insert(created_at, id);
        }

        Ok(())
    }

    fn get(&self, id: &FileId) -> Result<Option<FileDescriptor>> {
        let descriptors = self.inner.descriptors.read();
        Ok(descriptors.get(id).cloned())
    }

    fn list_by_owner(&self, owner: &[u8; 32]) -> Result<Vec<FileDescriptor>> {
        let by_owner = self.inner.by_owner.read();
        let descriptors = self.inner.descriptors.read();

        let ids = match by_owner.get(owner) {
            Some(ids) => ids.clone(),
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();
        for id in ids {
            if let Some(desc) = descriptors.get(&id) {
                results.push(desc.clone());
            }
        }

        // Sort by creation time (newest first)
        results.sort_by(|a, b| b.created_at_us.cmp(&a.created_at_us));

        Ok(results)
    }

    fn count(&self) -> Result<u64> {
        let descriptors = self.inner.descriptors.read();
        Ok(descriptors.len() as u64)
    }

    fn list(&self, offset: usize, limit: usize) -> Result<Vec<FileDescriptor>> {
        let by_time = self.inner.by_time.read();
        let descriptors = self.inner.descriptors.read();

        let ids: Vec<FileId> = by_time
            .values()
            .rev() // Newest first
            .skip(offset)
            .take(limit)
            .copied()
            .collect();

        let mut results = Vec::new();
        for id in ids {
            if let Some(desc) = descriptors.get(&id) {
                results.push(desc.clone());
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{ContentHash, FileDescriptor};
    use ippan_time::IppanTimeMicros;

    #[test]
    fn test_store_and_retrieve() {
        let storage = MemoryFileStorage::new();
        let content_hash = ContentHash::from_data(b"test content");
        let owner = [1u8; 32];

        let desc = FileDescriptor::new(content_hash, owner, 100, None, vec![]);
        let id = desc.id;

        storage.store(desc.clone()).unwrap();

        let retrieved = storage.get(&id).unwrap();
        assert_eq!(retrieved, Some(desc));
    }

    #[test]
    fn test_list_by_owner() {
        let storage = MemoryFileStorage::new();
        let owner1 = [1u8; 32];
        let owner2 = [2u8; 32];

        // Store files for owner1
        for i in 0..3 {
            let content = format!("content{}", i);
            let hash = ContentHash::from_data(content.as_bytes());
            let desc = FileDescriptor::new(hash, owner1, 100 + i as u64, None, vec![]);
            storage.store(desc).unwrap();
        }

        // Store files for owner2
        for i in 0..2 {
            let content = format!("other{}", i);
            let hash = ContentHash::from_data(content.as_bytes());
            let desc = FileDescriptor::new(hash, owner2, 200 + i as u64, None, vec![]);
            storage.store(desc).unwrap();
        }

        let owner1_files = storage.list_by_owner(&owner1).unwrap();
        assert_eq!(owner1_files.len(), 3);

        let owner2_files = storage.list_by_owner(&owner2).unwrap();
        assert_eq!(owner2_files.len(), 2);

        // All files for owner1
        for desc in &owner1_files {
            assert_eq!(desc.owner, owner1);
        }
    }

    #[test]
    fn test_count() {
        let storage = MemoryFileStorage::new();

        assert_eq!(storage.count().unwrap(), 0);

        for i in 0..5 {
            let content = format!("file{}", i);
            let hash = ContentHash::from_data(content.as_bytes());
            let desc = FileDescriptor::new(hash, [1u8; 32], 100, None, vec![]);
            storage.store(desc).unwrap();
        }

        assert_eq!(storage.count().unwrap(), 5);
    }

    #[test]
    fn test_pagination() {
        let storage = MemoryFileStorage::new();
        let owner = [1u8; 32];

        // Create 10 files at different times
        for i in 0..10 {
            let content = format!("file{}", i);
            let hash = ContentHash::from_data(content.as_bytes());
            let time = IppanTimeMicros(1000000 + i * 1000);
            let desc = FileDescriptor::new_at_time(hash, owner, 100, time, None, vec![]);
            storage.store(desc).unwrap();
        }

        // First page (5 items)
        let page1 = storage.list(0, 5).unwrap();
        assert_eq!(page1.len(), 5);

        // Second page
        let page2 = storage.list(5, 5).unwrap();
        assert_eq!(page2.len(), 5);

        // Verify order (newest first)
        assert!(page1[0].created_at_us > page1[4].created_at_us);

        // No overlap
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[test]
    fn test_validation_on_store() {
        let storage = MemoryFileStorage::new();
        let content_hash = ContentHash::from_data(b"test");
        let owner = [1u8; 32];

        // Invalid descriptor (zero size)
        let mut invalid_desc = FileDescriptor::new(content_hash, owner, 0, None, vec![]);
        invalid_desc.size_bytes = 0;

        let result = storage.store(invalid_desc);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_nonexistent() {
        let storage = MemoryFileStorage::new();
        let nonexistent_id = FileId::from_bytes([99u8; 32]);

        let result = storage.get(&nonexistent_id).unwrap();
        assert_eq!(result, None);
    }
}
