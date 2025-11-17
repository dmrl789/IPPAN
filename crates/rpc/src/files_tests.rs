//! Integration tests for file descriptor RPC endpoints.

#[cfg(test)]
mod tests {
    use ippan_files::{
        descriptor::ContentHash, dht::StubFileDhtService, FileDescriptor, FileDhtService,
        FileStorage, MemoryFileStorage,
    };
    use ippan_l1_handle_anchors::L1HandleAnchorStorage;
    use ippan_l2_handle_registry::{
        dht::HandleDhtService, dht::StubHandleDhtService, L2HandleRegistry,
    };
    use ippan_mempool::Mempool;
    use ippan_storage::MemoryStorage;
    use ippan_types::address::encode_address;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::runtime::Runtime;

    use crate::files::{FileDescriptorResponse, PublishFileRequest};
    use crate::server::{AppState, L2Config};

    fn create_test_state() -> AppState {
        let storage = Arc::new(MemoryStorage::new());
        let file_storage: Arc<dyn FileStorage> = Arc::new(MemoryFileStorage::new());
        let file_dht: Arc<dyn FileDhtService> = Arc::new(StubFileDhtService::new());
        let mempool = Arc::new(Mempool::new(1_000));
        let handle_registry = Arc::new(L2HandleRegistry::new());
        let handle_anchors = Arc::new(L1HandleAnchorStorage::new());
        let handle_dht: Arc<dyn HandleDhtService> = Arc::new(StubHandleDhtService::new());

        AppState {
            storage,
            start_time: Instant::now(),
            peer_count: Arc::new(AtomicUsize::new(0)),
            p2p_network: None,
            tx_sender: None,
            node_id: "test-node".to_string(),
            consensus_mode: "poa".into(),
            consensus: None,
            ai_status: None,
            l2_config: L2Config {
                max_commit_size: 1000,
                min_epoch_gap_ms: 1000,
                challenge_window_ms: 10000,
                da_mode: "test".to_string(),
                max_l2_count: 10,
            },
            mempool,
            unified_ui_dist: None,
            req_count: Arc::new(AtomicUsize::new(0)),
            security: None,
            metrics: None,
            file_storage: Some(file_storage),
            file_dht: Some(file_dht),
            dht_file_mode: "stub".into(),
            dev_mode: true,
            handle_registry,
            handle_anchors,
            handle_dht: Some(handle_dht),
            dht_handle_mode: "stub".into(),
        }
    }

    #[test]
    fn test_publish_request_validation() {
        let _state = create_test_state();

        // Valid request
        let valid = PublishFileRequest {
            owner: encode_address(&[1u8; 32]),
            content_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            size_bytes: 1024,
            mime_type: Some("text/plain".to_string()),
            tags: vec!["test".to_string()],
        };

        assert_eq!(valid.size_bytes, 1024);
        assert_eq!(valid.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_descriptor_to_response_conversion() {
        let content_hash = ContentHash::from_data(b"test content");
        let owner = [7u8; 32];

        let descriptor = FileDescriptor::new(
            content_hash,
            owner,
            256,
            Some("application/octet-stream".to_string()),
            vec!["binary".to_string(), "test".to_string()],
        );

        let response = FileDescriptorResponse::from(descriptor.clone());

        assert_eq!(response.id, descriptor.id.to_hex());
        assert_eq!(response.content_hash, descriptor.content_hash.to_hex());
        assert_eq!(response.owner, encode_address(&owner));
        assert_eq!(response.size_bytes, 256);
        assert_eq!(
            response.mime_type,
            Some("application/octet-stream".to_string())
        );
        assert_eq!(
            response.tags,
            vec!["binary".to_string(), "test".to_string()]
        );
    }

    #[test]
    fn test_file_storage_integration() {
        let state = create_test_state();
        let storage = state.file_storage.as_ref().unwrap();

        // Create and store a descriptor
        let content_hash = ContentHash::from_data(b"integration test");
        let owner = [42u8; 32];
        let descriptor = FileDescriptor::new(
            content_hash,
            owner,
            16,
            Some("text/plain".to_string()),
            vec![],
        );

        storage.store(descriptor.clone()).unwrap();

        // Retrieve it
        let retrieved = storage.get(&descriptor.id).unwrap();
        assert_eq!(retrieved, Some(descriptor.clone()));

        // List by owner
        let owner_files = storage.list_by_owner(&owner).unwrap();
        assert_eq!(owner_files.len(), 1);
        assert_eq!(owner_files[0].id, descriptor.id);
    }

    #[test]
    fn test_dht_stub_behavior() {
        let state = create_test_state();
        let dht = state.file_dht.as_ref().unwrap();

        let content_hash = ContentHash::from_data(b"dht test");
        let descriptor = FileDescriptor::new(content_hash, [1u8; 32], 100, None, vec![]);

        // Publish
        let rt = Runtime::new().expect("tokio runtime");
        let publish_result = rt.block_on(dht.publish_file(&descriptor)).unwrap();
        assert!(publish_result.success);
        assert_eq!(publish_result.file_id, descriptor.id);

        // Lookup (stub always returns None)
        let lookup_result = rt.block_on(dht.find_file(&descriptor.id)).unwrap();
        assert_eq!(lookup_result.descriptor, None);
    }

    #[test]
    fn test_multiple_files_workflow() {
        let state = create_test_state();
        let storage = state.file_storage.as_ref().unwrap();
        let dht = state.file_dht.as_ref().unwrap();

        let owner = [99u8; 32];

        // Publish multiple files
        let rt = Runtime::new().expect("tokio runtime");
        for i in 0..5 {
            let content = format!("file content {}", i);
            let hash = ContentHash::from_data(content.as_bytes());
            let desc = FileDescriptor::new(hash, owner, 100 + i, None, vec![]);

            storage.store(desc.clone()).unwrap();
            let publish_result = rt.block_on(dht.publish_file(&desc)).unwrap();
            assert!(publish_result.success);
        }

        // Verify count
        let owner_files = storage.list_by_owner(&owner).unwrap();
        assert_eq!(owner_files.len(), 5);

        // Verify all belong to owner
        for file in &owner_files {
            assert_eq!(file.owner, owner);
        }
    }
}
