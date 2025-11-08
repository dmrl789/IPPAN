use crate::{
    AccessKey, Amount, Block, ConfidentialEnvelope, ConfidentialProof, ConfidentialProofType,
    HashTimer, IppanTimeMicros, RoundCertificate, RoundFinalizationRecord, RoundWindow,
    Transaction,
};

#[cfg(test)]
mod type_tests {
    use super::*;

    #[test]
    fn test_hashtimer_creation() {
        let nonce = [1u8; 32];
        let hashtimer = HashTimer::now_tx("test", b"payload", &nonce, b"node");
        assert_eq!(hashtimer.entropy.len(), 32);
        assert_eq!(hashtimer.to_hex().len(), 64);
        assert!(hashtimer.timestamp_us > 0);
    }

    #[test]
    fn test_hashtimer_deterministic() {
        let time = IppanTimeMicros(1000000);
        let domain = "test";
        let payload = b"payload";
        let nonce = [1u8; 32];
        let node_id = b"node";

        let h1 = HashTimer::derive(domain, time, domain.as_bytes(), payload, &nonce, node_id);
        let h2 = HashTimer::derive(domain, time, domain.as_bytes(), payload, &nonce, node_id);

        assert_eq!(h1.to_hex(), h2.to_hex());
    }

    #[test]
    fn test_hashtimer_ordering() {
        let n1 = [1u8; 32];
        let early = HashTimer::now_tx("test", b"payload1", &n1, b"node");
        std::thread::sleep(std::time::Duration::from_micros(10));
        let n2 = [2u8; 32];
        let later = HashTimer::now_tx("test", b"payload2", &n2, b"node");

        assert!(early.timestamp_us <= later.timestamp_us);
    }

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        assert_eq!(tx.from, [1u8; 32]);
        assert_eq!(tx.to, [2u8; 32]);
        assert_eq!(tx.amount, Amount::from_atomic(1000));
        assert_eq!(tx.nonce, 1);
        assert!(!tx.hash().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_transaction_hash_deterministic() {
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);

        // Hashes should be different due to different hashtimers (time-based)
        assert_ne!(tx1.hash(), tx2.hash());
    }

    #[test]
    fn test_transaction_validation() {
        use ed25519_dalek::SigningKey;
        let secret = SigningKey::from_bytes(&[7u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [2u8; 32], Amount::from_atomic(1000), 1);
        tx.sign(&secret.to_bytes()).unwrap();
        assert!(tx.is_valid());

        // Test invalid transaction (zero amount)
        let invalid_tx = Transaction::new(from, [2u8; 32], Amount::zero(), 1);
        assert!(!invalid_tx.is_valid());
    }

    #[test]
    fn test_block_creation() {
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], Amount::from_atomic(2000), 1);
        let transactions = vec![tx1, tx2];

        let block = Block::new(vec![[0u8; 32]], transactions.clone(), 1, [5u8; 32]);
        assert_eq!(block.header.round, 1);
        assert_eq!(block.header.creator, [5u8; 32]);
        assert_eq!(block.transactions.len(), 2);
        assert!(!block.hash().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_block_validation() {
        use ed25519_dalek::SigningKey;
        let secret = SigningKey::from_bytes(&[9u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [2u8; 32], Amount::from_atomic(1000), 1);
        tx.sign(&secret.to_bytes()).unwrap();
        let block = Block::new(vec![[0u8; 32]], vec![tx], 1, [5u8; 32]);
        assert!(block.is_valid());

        // Empty transactions are permitted; block remains valid
        let empty_block = Block::new(vec![[0u8; 32]], vec![], 1, [5u8; 32]);
        assert!(empty_block.is_valid());
    }

    #[test]
    fn test_ippan_time_monotonic() {
        let t1 = IppanTimeMicros::now();
        std::thread::sleep(std::time::Duration::from_micros(10));
        let t2 = IppanTimeMicros::now();

        assert!(t2.0 > t1.0);
    }

    #[test]
    fn test_transaction_topics() {
        let mut tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        tx.topics = vec!["transfer".to_string(), "payment".to_string()];

        assert_eq!(tx.topics.len(), 2);
        assert!(tx.topics.contains(&"transfer".to_string()));
    }

    #[test]
    fn test_confidential_transaction() {
        use ed25519_dalek::SigningKey;
        let secret = SigningKey::from_bytes(&[3u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [2u8; 32], Amount::from_atomic(1000), 1);

        let envelope = ConfidentialEnvelope {
            enc_algo: "AES-256-GCM".to_string(),
            iv: hex::encode([1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            ciphertext: hex::encode([0xdeu8, 0xad, 0xbe, 0xef]),
            access_keys: vec![AccessKey {
                recipient_pub: hex::encode([0x01u8; 32]),
                enc_key: hex::encode([0x02u8; 32]),
            }],
        };

        tx.set_confidential_envelope(envelope);
        // Confidential tx requires a proof to be considered valid
        let mut public_inputs = std::collections::BTreeMap::new();
        public_inputs.insert("balance".to_string(), "1000".to_string());
        let proof = ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: hex::encode([1u8, 2, 3, 4]),
            public_inputs,
        };
        tx.set_confidential_proof(proof);
        tx.sign(&secret.to_bytes()).unwrap();
        assert!(tx.is_valid());
    }

    #[test]
    fn test_zk_proof_transaction() {
        use ed25519_dalek::SigningKey;
        let secret = SigningKey::from_bytes(&[4u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [2u8; 32], Amount::from_atomic(1000), 1);

        let mut public_inputs = std::collections::BTreeMap::new();
        public_inputs.insert("balance".to_string(), "1000".to_string());

        let zk_proof = ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: hex::encode([0x01u8, 0x02, 0x03, 0x04]),
            public_inputs,
        };

        tx.set_confidential_proof(zk_proof);
        tx.sign(&secret.to_bytes()).unwrap();
        assert!(tx.zk_proof.is_some());
        assert!(tx.is_valid());
    }
}

// ========================================================================
// SERIALIZATION/DESERIALIZATION CONSISTENCY TESTS
// ========================================================================

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    /// Helper to create a signed transaction
    fn create_signed_transaction() -> Transaction {
        let secret = SigningKey::from_bytes(&[7u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let to = [2u8; 32];
        let amount = Amount::from_atomic(1000);
        let nonce = 1;

        let mut tx = Transaction::new(from, to, amount, nonce);
        tx.sign(&secret.to_bytes()).expect("sign transaction");
        tx
    }

    /// Helper to create a transaction with confidential data
    fn create_confidential_transaction() -> Transaction {
        let secret = SigningKey::from_bytes(&[9u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let to = [3u8; 32];
        let amount = Amount::from_atomic(2000);
        let nonce = 2;

        let mut tx = Transaction::new(from, to, amount, nonce);

        // Add confidential envelope
        let envelope = ConfidentialEnvelope {
            enc_algo: "AES-256-GCM".to_string(),
            iv: hex::encode([1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            ciphertext: hex::encode([0xdeu8, 0xad, 0xbe, 0xef]),
            access_keys: vec![AccessKey {
                recipient_pub: hex::encode([0x01u8; 32]),
                enc_key: hex::encode([0x02u8; 32]),
            }],
        };

        tx.set_confidential_envelope(envelope);

        // Add ZK proof
        let mut public_inputs = std::collections::BTreeMap::new();
        public_inputs.insert("balance".to_string(), "2000".to_string());
        let proof = ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: hex::encode([1u8, 2, 3, 4]),
            public_inputs,
        };
        tx.set_confidential_proof(proof);

        tx.sign(&secret.to_bytes()).expect("sign transaction");
        tx
    }

    /// Helper to create a block with transactions
    fn create_block_with_transactions() -> Block {
        let parent_ids = vec![[1u8; 32], [2u8; 32]];
        let transactions = vec![create_signed_transaction()];
        let round = 5;
        let creator = [10u8; 32];

        Block::new(parent_ids, transactions, round, creator)
    }

    /// Helper to create a round window
    fn create_round_window() -> RoundWindow {
        RoundWindow {
            id: 42,
            start_us: IppanTimeMicros(1_000_000),
            end_us: IppanTimeMicros(2_000_000),
        }
    }

    /// Helper to create a round certificate
    fn create_round_certificate() -> RoundCertificate {
        RoundCertificate {
            round: 42,
            block_ids: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
            agg_sig: vec![0xffu8; 64],
        }
    }

    /// Helper to create a round finalization record
    fn create_round_finalization_record() -> RoundFinalizationRecord {
        RoundFinalizationRecord {
            round: 42,
            window: create_round_window(),
            ordered_tx_ids: vec![[4u8; 32], [5u8; 32], [6u8; 32]],
            fork_drops: vec![[7u8; 32]],
            state_root: [8u8; 32],
            proof: create_round_certificate(),
        }
    }

    // ========================================================================
    // TRANSACTION SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_transaction_json_serialization_roundtrip() {
        let tx = create_signed_transaction();
        
        // Serialize to JSON
        let json = serde_json::to_string(&tx).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: Transaction = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify all fields match
        assert_eq!(tx.id, deserialized.id, "Transaction IDs do not match");
        assert_eq!(tx.from, deserialized.from, "Transaction from addresses do not match");
        assert_eq!(tx.to, deserialized.to, "Transaction to addresses do not match");
        assert_eq!(tx.amount, deserialized.amount, "Transaction amounts do not match");
        assert_eq!(tx.nonce, deserialized.nonce, "Transaction nonces do not match");
        assert_eq!(tx.visibility, deserialized.visibility, "Transaction visibility does not match");
        assert_eq!(tx.topics, deserialized.topics, "Transaction topics do not match");
        assert_eq!(tx.confidential, deserialized.confidential, "Transaction confidential data does not match");
        assert_eq!(tx.zk_proof, deserialized.zk_proof, "Transaction ZK proof does not match");
        assert_eq!(tx.signature, deserialized.signature, "Transaction signatures do not match");
        assert_eq!(tx.hashtimer, deserialized.hashtimer, "Transaction hashtimers do not match");
        assert_eq!(tx.timestamp, deserialized.timestamp, "Transaction timestamps do not match");
        
        // Verify hash is consistent
        assert_eq!(tx.hash(), deserialized.hash(), "Transaction hashes do not match");
        
        // Verify signature is still valid
        assert!(deserialized.verify(), "Deserialized transaction signature is invalid");
        assert!(deserialized.is_valid(), "Deserialized transaction is invalid");
    }

    #[test]
    fn test_transaction_json_bytes_serialization() {
        let tx = create_signed_transaction();
        
        // Serialize to JSON bytes
        let json_bytes = serde_json::to_vec(&tx).expect("serialize to JSON bytes");
        
        // Deserialize from JSON bytes
        let deserialized: Transaction = serde_json::from_slice(&json_bytes).expect("deserialize from JSON bytes");
        
        // Verify critical fields
        assert_eq!(tx.id, deserialized.id);
        assert_eq!(tx.from, deserialized.from);
        assert_eq!(tx.to, deserialized.to);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.nonce, deserialized.nonce);
        assert_eq!(tx.signature, deserialized.signature);
        
        // Verify the transaction is still valid
        assert!(deserialized.is_valid());
    }

    #[test]
    fn test_confidential_transaction_serialization_roundtrip() {
        let tx = create_confidential_transaction();
        
        // Serialize to JSON
        let json = serde_json::to_string(&tx).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: Transaction = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify all fields match
        assert_eq!(tx.id, deserialized.id);
        assert_eq!(tx.from, deserialized.from);
        assert_eq!(tx.to, deserialized.to);
        assert_eq!(tx.amount, deserialized.amount);
        assert_eq!(tx.nonce, deserialized.nonce);
        assert_eq!(tx.visibility, deserialized.visibility);
        assert_eq!(tx.signature, deserialized.signature);
        assert_eq!(tx.hashtimer, deserialized.hashtimer);
        assert_eq!(tx.timestamp, deserialized.timestamp);
        
        // Verify confidential data is preserved
        assert!(deserialized.confidential.is_some());
        assert_eq!(tx.confidential, deserialized.confidential);
        
        // Verify ZK proof is preserved
        assert!(deserialized.zk_proof.is_some());
        assert_eq!(tx.zk_proof, deserialized.zk_proof);
        
        // Verify the transaction is still valid
        assert!(deserialized.is_valid());
    }

    #[test]
    fn test_transaction_with_topics_serialization() {
        let mut tx = create_signed_transaction();
        tx.topics = vec!["transfer".to_string(), "payment".to_string(), "test".to_string()];
        
        // Re-sign after modification
        let secret = SigningKey::from_bytes(&[7u8; 32]);
        tx.sign(&secret.to_bytes()).expect("sign transaction");
        
        // Serialize and deserialize
        let json = serde_json::to_string(&tx).expect("serialize to JSON");
        let deserialized: Transaction = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify topics are preserved
        assert_eq!(tx.topics, deserialized.topics);
        assert_eq!(deserialized.topics.len(), 3);
        assert!(deserialized.topics.contains(&"transfer".to_string()));
        assert!(deserialized.topics.contains(&"payment".to_string()));
        assert!(deserialized.topics.contains(&"test".to_string()));
    }

    #[test]
    fn test_transaction_json_pretty_print() {
        let tx = create_signed_transaction();
        
        // Serialize with pretty printing
        let json_pretty = serde_json::to_string_pretty(&tx).expect("serialize to pretty JSON");
        
        // Deserialize from pretty JSON
        let deserialized: Transaction = serde_json::from_str(&json_pretty).expect("deserialize from pretty JSON");
        
        // Verify consistency
        assert_eq!(tx.id, deserialized.id);
        assert_eq!(tx.hash(), deserialized.hash());
        assert!(deserialized.is_valid());
    }

    // ========================================================================
    // BLOCK SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_block_json_serialization_roundtrip() {
        let block = create_block_with_transactions();
        
        // Serialize to JSON
        let json = serde_json::to_string(&block).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: Block = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify header fields
        assert_eq!(block.header.id, deserialized.header.id, "Block IDs do not match");
        assert_eq!(block.header.creator, deserialized.header.creator, "Block creators do not match");
        assert_eq!(block.header.round, deserialized.header.round, "Block rounds do not match");
        assert_eq!(block.header.hashtimer, deserialized.header.hashtimer, "Block hashtimers do not match");
        assert_eq!(block.header.parent_ids, deserialized.header.parent_ids, "Block parent IDs do not match");
        assert_eq!(block.header.prev_hashes, deserialized.header.prev_hashes, "Block prev hashes do not match");
        assert_eq!(block.header.payload_ids, deserialized.header.payload_ids, "Block payload IDs do not match");
        assert_eq!(block.header.merkle_payload, deserialized.header.merkle_payload, "Block merkle payloads do not match");
        assert_eq!(block.header.merkle_parents, deserialized.header.merkle_parents, "Block merkle parents do not match");
        assert_eq!(block.header.tx_root, deserialized.header.tx_root, "Block tx roots do not match");
        assert_eq!(block.header.erasure_root, deserialized.header.erasure_root, "Block erasure roots do not match");
        assert_eq!(block.header.receipt_root, deserialized.header.receipt_root, "Block receipt roots do not match");
        assert_eq!(block.header.state_root, deserialized.header.state_root, "Block state roots do not match");
        assert_eq!(block.header.validator_sigs, deserialized.header.validator_sigs, "Block validator sigs do not match");
        assert_eq!(block.header.vrf_proof, deserialized.header.vrf_proof, "Block VRF proofs do not match");
        
        // Verify block-level fields
        assert_eq!(block.signature, deserialized.signature, "Block signatures do not match");
        assert_eq!(block.prev_hashes, deserialized.prev_hashes, "Block prev_hashes do not match");
        
        // Verify transactions
        assert_eq!(block.transactions.len(), deserialized.transactions.len(), "Transaction counts do not match");
        for (i, (orig_tx, deser_tx)) in block.transactions.iter().zip(deserialized.transactions.iter()).enumerate() {
            assert_eq!(orig_tx.id, deser_tx.id, "Transaction {} IDs do not match", i);
            assert_eq!(orig_tx.hash(), deser_tx.hash(), "Transaction {} hashes do not match", i);
        }
        
        // Verify hash is consistent
        assert_eq!(block.hash(), deserialized.hash(), "Block hashes do not match");
        
        // Verify the block is still valid
        assert!(deserialized.is_valid(), "Deserialized block is invalid");
    }

    #[test]
    fn test_empty_block_serialization_roundtrip() {
        let block = Block::new(vec![], vec![], 1, [5u8; 32]);
        
        // Serialize to JSON
        let json = serde_json::to_string(&block).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: Block = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify header fields
        assert_eq!(block.header.id, deserialized.header.id);
        assert_eq!(block.header.creator, deserialized.header.creator);
        assert_eq!(block.header.round, deserialized.header.round);
        assert_eq!(block.header.hashtimer, deserialized.header.hashtimer);
        
        // Verify empty collections
        assert!(deserialized.transactions.is_empty());
        assert!(deserialized.header.parent_ids.is_empty());
        assert!(deserialized.header.payload_ids.is_empty());
        assert!(deserialized.header.prev_hashes.is_empty());
        
        // Verify the block is still valid
        assert!(deserialized.is_valid());
    }

    #[test]
    fn test_block_with_multiple_parents_serialization() {
        let parent_ids = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let transactions = vec![create_signed_transaction()];
        let block = Block::new(parent_ids, transactions, 10, [15u8; 32]);
        
        // Serialize and deserialize
        let json = serde_json::to_string(&block).expect("serialize to JSON");
        let deserialized: Block = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify parent IDs are preserved
        assert_eq!(block.header.parent_ids.len(), 4);
        assert_eq!(deserialized.header.parent_ids.len(), 4);
        assert_eq!(block.header.parent_ids, deserialized.header.parent_ids);
        
        // Verify prev_hashes match parent_ids
        assert_eq!(deserialized.header.prev_hashes.len(), 4);
        for (i, parent_id) in deserialized.header.parent_ids.iter().enumerate() {
            let expected_hex = hex::encode(parent_id);
            assert_eq!(deserialized.header.prev_hashes[i], expected_hex);
        }
        
        assert!(deserialized.is_valid());
    }

    #[test]
    fn test_block_json_bytes_serialization() {
        let block = create_block_with_transactions();
        
        // Serialize to JSON bytes
        let json_bytes = serde_json::to_vec(&block).expect("serialize to JSON bytes");
        
        // Deserialize from JSON bytes
        let deserialized: Block = serde_json::from_slice(&json_bytes).expect("deserialize from JSON bytes");
        
        // Verify critical fields
        assert_eq!(block.header.id, deserialized.header.id);
        assert_eq!(block.header.creator, deserialized.header.creator);
        assert_eq!(block.header.round, deserialized.header.round);
        assert_eq!(block.hash(), deserialized.hash());
        
        // Verify the block is still valid
        assert!(deserialized.is_valid());
    }

    #[test]
    fn test_block_with_data_availability_roots_serialization() {
        let mut block = create_block_with_transactions();
        block.set_data_availability_roots(
            Some("erasure_root_abc123".to_string()),
            Some("receipt_root_def456".to_string()),
            Some("state_root_ghi789".to_string()),
        );
        
        // Serialize and deserialize
        let json = serde_json::to_string(&block).expect("serialize to JSON");
        let deserialized: Block = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify roots are preserved
        assert_eq!(block.header.erasure_root, deserialized.header.erasure_root);
        assert_eq!(block.header.receipt_root, deserialized.header.receipt_root);
        assert_eq!(block.header.state_root, deserialized.header.state_root);
        
        assert_eq!(deserialized.header.erasure_root, Some("erasure_root_abc123".to_string()));
        assert_eq!(deserialized.header.receipt_root, Some("receipt_root_def456".to_string()));
        assert_eq!(deserialized.header.state_root, Some("state_root_ghi789".to_string()));
    }

    // ========================================================================
    // ROUND SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_round_window_json_serialization_roundtrip() {
        let window = create_round_window();
        
        // Serialize to JSON
        let json = serde_json::to_string(&window).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: RoundWindow = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify all fields match
        assert_eq!(window.id, deserialized.id, "Round IDs do not match");
        assert_eq!(window.start_us, deserialized.start_us, "Round start times do not match");
        assert_eq!(window.end_us, deserialized.end_us, "Round end times do not match");
        
        // Verify equality
        assert_eq!(window, deserialized);
    }

    #[test]
    fn test_round_certificate_json_serialization_roundtrip() {
        let cert = create_round_certificate();
        
        // Serialize to JSON
        let json = serde_json::to_string(&cert).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: RoundCertificate = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify all fields match
        assert_eq!(cert.round, deserialized.round, "Certificate rounds do not match");
        assert_eq!(cert.block_ids, deserialized.block_ids, "Certificate block IDs do not match");
        assert_eq!(cert.agg_sig, deserialized.agg_sig, "Certificate aggregate signatures do not match");
        
        // Verify equality
        assert_eq!(cert, deserialized);
    }

    #[test]
    fn test_round_finalization_record_json_serialization_roundtrip() {
        let record = create_round_finalization_record();
        
        // Serialize to JSON
        let json = serde_json::to_string(&record).expect("serialize to JSON");
        
        // Deserialize from JSON
        let deserialized: RoundFinalizationRecord = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify all fields match
        assert_eq!(record.round, deserialized.round, "Record rounds do not match");
        assert_eq!(record.window, deserialized.window, "Record windows do not match");
        assert_eq!(record.ordered_tx_ids, deserialized.ordered_tx_ids, "Record ordered tx IDs do not match");
        assert_eq!(record.fork_drops, deserialized.fork_drops, "Record fork drops do not match");
        assert_eq!(record.state_root, deserialized.state_root, "Record state roots do not match");
        assert_eq!(record.proof, deserialized.proof, "Record proofs do not match");
        
        // Verify equality
        assert_eq!(record, deserialized);
    }

    #[test]
    fn test_round_certificate_with_empty_blocks_serialization() {
        let cert = RoundCertificate {
            round: 1,
            block_ids: vec![],
            agg_sig: vec![],
        };
        
        // Serialize and deserialize
        let json = serde_json::to_string(&cert).expect("serialize to JSON");
        let deserialized: RoundCertificate = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify empty collections are preserved
        assert!(deserialized.block_ids.is_empty());
        assert!(deserialized.agg_sig.is_empty());
        assert_eq!(cert, deserialized);
    }

    #[test]
    fn test_round_finalization_record_with_multiple_forks_serialization() {
        let mut record = create_round_finalization_record();
        record.fork_drops = vec![[10u8; 32], [11u8; 32], [12u8; 32], [13u8; 32]];
        
        // Serialize and deserialize
        let json = serde_json::to_string(&record).expect("serialize to JSON");
        let deserialized: RoundFinalizationRecord = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify fork drops are preserved
        assert_eq!(record.fork_drops.len(), 4);
        assert_eq!(deserialized.fork_drops.len(), 4);
        assert_eq!(record.fork_drops, deserialized.fork_drops);
        assert_eq!(record, deserialized);
    }

    #[test]
    fn test_round_window_json_bytes_serialization() {
        let window = create_round_window();
        
        // Serialize to JSON bytes
        let json_bytes = serde_json::to_vec(&window).expect("serialize to JSON bytes");
        
        // Deserialize from JSON bytes
        let deserialized: RoundWindow = serde_json::from_slice(&json_bytes).expect("deserialize from JSON bytes");
        
        // Verify consistency
        assert_eq!(window, deserialized);
    }

    #[test]
    fn test_round_certificate_json_bytes_serialization() {
        let cert = create_round_certificate();
        
        // Serialize to JSON bytes
        let json_bytes = serde_json::to_vec(&cert).expect("serialize to JSON bytes");
        
        // Deserialize from JSON bytes
        let deserialized: RoundCertificate = serde_json::from_slice(&json_bytes).expect("deserialize from JSON bytes");
        
        // Verify consistency
        assert_eq!(cert, deserialized);
    }

    #[test]
    fn test_round_finalization_record_json_bytes_serialization() {
        let record = create_round_finalization_record();
        
        // Serialize to JSON bytes
        let json_bytes = serde_json::to_vec(&record).expect("serialize to JSON bytes");
        
        // Deserialize from JSON bytes
        let deserialized: RoundFinalizationRecord = serde_json::from_slice(&json_bytes).expect("deserialize from JSON bytes");
        
        // Verify consistency
        assert_eq!(record, deserialized);
    }

    // ========================================================================
    // CROSS-TYPE CONSISTENCY TESTS
    // ========================================================================

    #[test]
    fn test_transaction_hash_consistency_after_serialization() {
        let tx = create_signed_transaction();
        let original_hash = tx.hash();
        
        // Serialize and deserialize
        let json = serde_json::to_string(&tx).expect("serialize to JSON");
        let deserialized: Transaction = serde_json::from_str(&json).expect("deserialize from JSON");
        let deserialized_hash = deserialized.hash();
        
        // Hash must be identical
        assert_eq!(original_hash, deserialized_hash, "Transaction hash changed after serialization");
    }

    #[test]
    fn test_block_hash_consistency_after_serialization() {
        let block = create_block_with_transactions();
        let original_hash = block.hash();
        
        // Serialize and deserialize
        let json = serde_json::to_string(&block).expect("serialize to JSON");
        let deserialized: Block = serde_json::from_str(&json).expect("deserialize from JSON");
        let deserialized_hash = deserialized.hash();
        
        // Hash must be identical
        assert_eq!(original_hash, deserialized_hash, "Block hash changed after serialization");
    }

    #[test]
    fn test_block_with_transactions_hash_consistency() {
        let mut transactions = vec![];
        for i in 0..5 {
            let secret = SigningKey::from_bytes(&[i as u8 + 10; 32]);
            let from = secret.verifying_key().to_bytes();
            let to = [i as u8 + 20; 32];
            let amount = Amount::from_atomic(1000 * (i + 1) as u128);
            let nonce = i + 1;
            
            let mut tx = Transaction::new(from, to, amount, nonce);
            tx.sign(&secret.to_bytes()).expect("sign transaction");
            transactions.push(tx);
        }
        
        let block = Block::new(vec![[0u8; 32]], transactions.clone(), 1, [100u8; 32]);
        let original_hash = block.hash();
        
        // Serialize and deserialize
        let json = serde_json::to_string(&block).expect("serialize to JSON");
        let deserialized: Block = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify all transaction hashes match
        for (i, (orig_tx, deser_tx)) in block.transactions.iter().zip(deserialized.transactions.iter()).enumerate() {
            assert_eq!(orig_tx.hash(), deser_tx.hash(), "Transaction {} hash changed", i);
        }
        
        // Verify block hash matches
        assert_eq!(original_hash, deserialized.hash());
        
        // Verify payload IDs match
        assert_eq!(block.header.payload_ids, deserialized.header.payload_ids);
    }

    #[test]
    fn test_round_finalization_record_with_transactions_consistency() {
        let tx1 = create_signed_transaction();
        let tx2 = create_confidential_transaction();
        
        let ordered_tx_ids = vec![tx1.hash(), tx2.hash()];
        
        let mut record = create_round_finalization_record();
        record.ordered_tx_ids = ordered_tx_ids.clone();
        
        // Serialize and deserialize
        let json = serde_json::to_string(&record).expect("serialize to JSON");
        let deserialized: RoundFinalizationRecord = serde_json::from_str(&json).expect("deserialize from JSON");
        
        // Verify transaction IDs are preserved
        assert_eq!(record.ordered_tx_ids, deserialized.ordered_tx_ids);
        assert_eq!(deserialized.ordered_tx_ids, ordered_tx_ids);
    }

    #[test]
    fn test_multiple_serialization_deserialization_cycles() {
        let mut tx = create_signed_transaction();
        
        // Perform multiple serialization/deserialization cycles
        for i in 0..10 {
            let json = serde_json::to_string(&tx).expect(&format!("serialize cycle {}", i));
            tx = serde_json::from_str(&json).expect(&format!("deserialize cycle {}", i));
            
            // Verify transaction remains valid
            assert!(tx.is_valid(), "Transaction became invalid after cycle {}", i);
            assert!(tx.verify(), "Transaction signature became invalid after cycle {}", i);
        }
    }

    #[test]
    fn test_block_multiple_serialization_cycles() {
        let mut block = create_block_with_transactions();
        let original_hash = block.hash();
        
        // Perform multiple serialization/deserialization cycles
        for i in 0..10 {
            let json = serde_json::to_string(&block).expect(&format!("serialize cycle {}", i));
            block = serde_json::from_str(&json).expect(&format!("deserialize cycle {}", i));
            
            // Verify block remains valid and hash is consistent
            assert!(block.is_valid(), "Block became invalid after cycle {}", i);
            assert_eq!(block.hash(), original_hash, "Block hash changed after cycle {}", i);
        }
    }
}
