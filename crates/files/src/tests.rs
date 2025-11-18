//! Tests for the files crate.

#[cfg(test)]
mod integration_tests {
    use crate::descriptor::{ContentHash, FileDescriptor};
    use crate::dht::{FileDhtService, StubFileDhtService};
    use crate::storage::{FileStorage, MemoryFileStorage};
    use ippan_time::IppanTimeMicros;
    use tokio::runtime::Runtime;

    #[test]
    fn test_full_workflow() {
        // Create storage and DHT
        let storage = MemoryFileStorage::new();
        let dht = StubFileDhtService::new();

        // Create a file descriptor
        let content = b"Hello, IPNDHT!";
        let content_hash = ContentHash::from_data(content);
        let owner = [7u8; 32];
        let size = content.len() as u64;

        let descriptor = FileDescriptor::new(
            content_hash,
            owner,
            size,
            Some("text/plain".to_string()),
            vec!["test".to_string(), "ipndht".to_string()],
        );

        // Validate
        assert!(descriptor.validate().is_ok());

        // Store locally
        storage.store(descriptor.clone()).unwrap();

        // Publish to DHT
        let rt = Runtime::new().expect("tokio runtime");
        let publish_result = rt.block_on(dht.publish_file(&descriptor)).unwrap();
        assert!(publish_result.success);
        assert_eq!(publish_result.file_id, descriptor.id);

        // Retrieve locally
        let retrieved = storage.get(&descriptor.id).unwrap();
        assert_eq!(retrieved, Some(descriptor.clone()));

        // Verify we can list by owner
        let owner_files = storage.list_by_owner(&owner).unwrap();
        assert_eq!(owner_files.len(), 1);
        assert_eq!(owner_files[0].id, descriptor.id);
    }

    #[test]
    fn test_multiple_files_ordering() {
        let storage = MemoryFileStorage::new();
        let owner = [1u8; 32];

        // Create files at different times
        let mut ids = Vec::new();
        for i in 0..5 {
            let content = format!("file{}", i);
            let hash = ContentHash::from_data(content.as_bytes());
            let time = IppanTimeMicros(1000000 + i * 1000);
            let desc = FileDescriptor::new_at_time(hash, owner, 100, time, None, vec![]);
            ids.push(desc.id);
            storage.store(desc).unwrap();
        }

        // List should return in reverse time order (newest first)
        let all = storage.list(0, 10).unwrap();
        assert_eq!(all.len(), 5);

        // Verify ordering
        for i in 0..4 {
            assert!(all[i].created_at_us > all[i + 1].created_at_us);
        }
    }

    #[test]
    fn test_file_id_uniqueness() {
        let owner = [1u8; 32];
        let time = IppanTimeMicros(1000000);

        // Same content, same owner, same time -> same ID
        let hash1 = ContentHash::from_data(b"content");
        let desc1 = FileDescriptor::new_at_time(hash1, owner, 100, time, None, vec![]);
        let desc2 = FileDescriptor::new_at_time(hash1, owner, 100, time, None, vec![]);
        assert_eq!(desc1.id, desc2.id);

        // Different content -> different ID
        let hash2 = ContentHash::from_data(b"different");
        let desc3 = FileDescriptor::new_at_time(hash2, owner, 100, time, None, vec![]);
        assert_ne!(desc1.id, desc3.id);

        // Different owner -> different ID
        let owner2 = [2u8; 32];
        let desc4 = FileDescriptor::new_at_time(hash1, owner2, 100, time, None, vec![]);
        assert_ne!(desc1.id, desc4.id);

        // Different time -> different ID
        let time2 = IppanTimeMicros(2000000);
        let desc5 = FileDescriptor::new_at_time(hash1, owner, 100, time2, None, vec![]);
        assert_ne!(desc1.id, desc5.id);
    }
}
