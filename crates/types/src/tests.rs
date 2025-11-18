use crate::{
    AccessKey, Amount, Block, ConfidentialEnvelope, ConfidentialProof, ConfidentialProofType,
    HandleOperation, HandleRegisterOp, HashTimer, IppanTimeMicros, RoundCertificate,
    RoundFinalizationRecord, RoundWindow, Transaction,
};
use std::collections::BTreeMap;

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
        assert_ne!(tx1.hash(), tx2.hash()); // time-based difference
    }

    #[test]
    fn test_transaction_validation() {
        use ed25519_dalek::SigningKey;
        let secret = SigningKey::from_bytes(&[7u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [2u8; 32], Amount::from_atomic(1000), 1);
        tx.sign(&secret.to_bytes()).unwrap();
        assert!(tx.is_valid());
        let invalid_tx = Transaction::new(from, [2u8; 32], Amount::zero(), 1);
        assert!(!invalid_tx.is_valid());
    }

    #[test]
    fn test_handle_transaction_zero_amount() {
        use ed25519_dalek::SigningKey;
        let secret = SigningKey::from_bytes(&[21u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [0u8; 32], Amount::zero(), 1);
        let op = HandleOperation::Register(HandleRegisterOp {
            handle: "@alice.ipn".to_string(),
            owner: from,
            metadata: BTreeMap::new(),
            expires_at: Some(1_700_000_000),
            signature: vec![0u8; 64],
        });
        tx.set_handle_operation(op);
        tx.sign(&secret.to_bytes()).unwrap();
        assert!(tx.is_valid());
    }

    #[test]
    fn test_block_creation() {
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], Amount::from_atomic(2000), 1);
        let block = Block::new(vec![[0u8; 32]], vec![tx1, tx2], 1, [5u8; 32]);
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
        let empty_block = Block::new(vec![[0u8; 32]], vec![], 1, [5u8; 32]);
        assert!(empty_block.is_valid());
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
        let mut public_inputs = std::collections::BTreeMap::new();
        public_inputs.insert("balance".to_string(), "1000".to_string());
        tx.set_confidential_proof(ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: hex::encode([1u8, 2, 3, 4]),
            public_inputs,
        });
        tx.sign(&secret.to_bytes()).unwrap();
        assert!(tx.is_valid());
    }
}

// ========================================================================
// SERIALIZATION / DESERIALIZATION CONSISTENCY TESTS
// ========================================================================

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    /// Test round-trip serialization/deserialization for Transaction (JSON)
    #[test]
    fn test_transaction_json_roundtrip() {
        let secret = SigningKey::from_bytes(&[5u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [6u8; 32], Amount::from_atomic(5000), 42);
        tx.sign(&secret.to_bytes()).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&tx).expect("Failed to serialize transaction");

        // Deserialize from JSON
        let deserialized: Transaction =
            serde_json::from_str(&json).expect("Failed to deserialize transaction");

        // Verify all fields match
        assert_eq!(tx.id, deserialized.id, "Transaction ID mismatch");
        assert_eq!(tx.from, deserialized.from, "From address mismatch");
        assert_eq!(tx.to, deserialized.to, "To address mismatch");
        assert_eq!(tx.amount, deserialized.amount, "Amount mismatch");
        assert_eq!(tx.nonce, deserialized.nonce, "Nonce mismatch");
        assert_eq!(
            tx.visibility, deserialized.visibility,
            "Visibility mismatch"
        );
        assert_eq!(tx.topics, deserialized.topics, "Topics mismatch");
        assert_eq!(
            tx.confidential, deserialized.confidential,
            "Confidential envelope mismatch"
        );
        assert_eq!(tx.zk_proof, deserialized.zk_proof, "ZK proof mismatch");
        assert_eq!(tx.signature, deserialized.signature, "Signature mismatch");
        assert_eq!(tx.hashtimer, deserialized.hashtimer, "HashTimer mismatch");
        assert_eq!(tx.timestamp, deserialized.timestamp, "Timestamp mismatch");

        // Verify hash consistency
        assert_eq!(
            tx.hash(),
            deserialized.hash(),
            "Transaction hash mismatch after deserialization"
        );

        // Verify the deserialized transaction is still valid
        assert!(
            deserialized.is_valid(),
            "Deserialized transaction is invalid"
        );
    }

    /// Test round-trip serialization/deserialization for Transaction with topics (JSON)
    #[test]
    fn test_transaction_with_topics_json_roundtrip() {
        let secret = SigningKey::from_bytes(&[7u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [8u8; 32], Amount::from_atomic(7500), 99);
        tx.topics = vec![
            "transfer".to_string(),
            "payment".to_string(),
            "metadata:test".to_string(),
        ];
        tx.sign(&secret.to_bytes()).unwrap();

        let json = serde_json::to_string(&tx).expect("Failed to serialize transaction");
        let deserialized: Transaction =
            serde_json::from_str(&json).expect("Failed to deserialize transaction");

        assert_eq!(tx.topics, deserialized.topics, "Topics mismatch");
        assert_eq!(tx.topics.len(), 3);
        assert!(tx.topics.contains(&"transfer".to_string()));
        assert!(deserialized.is_valid());
    }

    /// Test round-trip serialization/deserialization for confidential Transaction (JSON)
    #[test]
    fn test_confidential_transaction_json_roundtrip() {
        let secret = SigningKey::from_bytes(&[9u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [10u8; 32], Amount::from_atomic(10000), 5);

        let envelope = ConfidentialEnvelope {
            enc_algo: "AES-256-GCM".to_string(),
            iv: hex::encode([1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            ciphertext: hex::encode([0xdeu8, 0xad, 0xbe, 0xef, 0xca, 0xfe]),
            access_keys: vec![
                AccessKey {
                    recipient_pub: hex::encode([0x01u8; 32]),
                    enc_key: hex::encode([0x02u8; 32]),
                },
                AccessKey {
                    recipient_pub: hex::encode([0x03u8; 32]),
                    enc_key: hex::encode([0x04u8; 32]),
                },
            ],
        };
        tx.set_confidential_envelope(envelope);

        let mut public_inputs = std::collections::BTreeMap::new();
        public_inputs.insert("balance".to_string(), "10000".to_string());
        public_inputs.insert("proof_version".to_string(), "1".to_string());

        let proof = ConfidentialProof {
            proof_type: ConfidentialProofType::Stark,
            proof: hex::encode([1u8, 2, 3, 4, 5, 6, 7, 8]),
            public_inputs,
        };
        tx.set_confidential_proof(proof);
        tx.sign(&secret.to_bytes()).unwrap();

        let json =
            serde_json::to_string(&tx).expect("Failed to serialize confidential transaction");
        let deserialized: Transaction =
            serde_json::from_str(&json).expect("Failed to deserialize confidential transaction");

        assert_eq!(
            tx.confidential, deserialized.confidential,
            "Confidential envelope mismatch"
        );
        assert_eq!(tx.zk_proof, deserialized.zk_proof, "ZK proof mismatch");
        assert_eq!(tx.visibility, deserialized.visibility);
        assert!(deserialized.confidential.is_some());
        assert!(deserialized.zk_proof.is_some());
        assert!(deserialized.is_valid());
    }

    /// Test round-trip serialization/deserialization for Block (JSON)
    #[test]
    fn test_block_json_roundtrip() {
        let secret1 = SigningKey::from_bytes(&[11u8; 32]);
        let secret2 = SigningKey::from_bytes(&[12u8; 32]);
        let mut tx1 = Transaction::new(
            secret1.verifying_key().to_bytes(),
            [13u8; 32],
            Amount::from_atomic(1500),
            1,
        );
        tx1.sign(&secret1.to_bytes()).unwrap();

        let mut tx2 = Transaction::new(
            secret2.verifying_key().to_bytes(),
            [14u8; 32],
            Amount::from_atomic(2500),
            2,
        );
        tx2.sign(&secret2.to_bytes()).unwrap();

        let parent_id = [15u8; 32];
        let creator = [16u8; 32];
        let round = 42;

        let block = Block::new(vec![parent_id], vec![tx1, tx2], round, creator);

        // Serialize to JSON
        let json = serde_json::to_string(&block).expect("Failed to serialize block");

        // Deserialize from JSON
        let deserialized: Block = serde_json::from_str(&json).expect("Failed to deserialize block");

        // Verify header fields match
        assert_eq!(block.header.id, deserialized.header.id, "Block ID mismatch");
        assert_eq!(
            block.header.creator, deserialized.header.creator,
            "Creator mismatch"
        );
        assert_eq!(
            block.header.round, deserialized.header.round,
            "Round mismatch"
        );
        assert_eq!(
            block.header.hashtimer, deserialized.header.hashtimer,
            "HashTimer mismatch"
        );
        assert_eq!(
            block.header.parent_ids, deserialized.header.parent_ids,
            "Parent IDs mismatch"
        );
        assert_eq!(
            block.header.payload_ids, deserialized.header.payload_ids,
            "Payload IDs mismatch"
        );
        assert_eq!(
            block.header.merkle_payload, deserialized.header.merkle_payload,
            "Merkle payload mismatch"
        );
        assert_eq!(
            block.header.merkle_parents, deserialized.header.merkle_parents,
            "Merkle parents mismatch"
        );
        assert_eq!(
            block.header.tx_root, deserialized.header.tx_root,
            "TX root mismatch"
        );
        assert_eq!(
            block.header.prev_hashes, deserialized.header.prev_hashes,
            "Prev hashes mismatch"
        );
        assert_eq!(
            block.signature, deserialized.signature,
            "Signature mismatch"
        );
        assert_eq!(
            block.transactions.len(),
            deserialized.transactions.len(),
            "Transaction count mismatch"
        );
        assert_eq!(
            block.prev_hashes, deserialized.prev_hashes,
            "Block prev_hashes mismatch"
        );

        // Verify hash consistency
        assert_eq!(
            block.hash(),
            deserialized.hash(),
            "Block hash mismatch after deserialization"
        );

        // Verify the deserialized block is still valid
        assert!(deserialized.is_valid(), "Deserialized block is invalid");
    }

    /// Test round-trip serialization/deserialization for empty Block (JSON)
    #[test]
    fn test_empty_block_json_roundtrip() {
        let creator = [20u8; 32];
        let round = 1;
        let block = Block::new(vec![], vec![], round, creator);

        let json = serde_json::to_string(&block).expect("Failed to serialize empty block");
        let deserialized: Block =
            serde_json::from_str(&json).expect("Failed to deserialize empty block");

        assert_eq!(block.header.id, deserialized.header.id);
        assert_eq!(block.header.round, deserialized.header.round);
        assert!(deserialized.transactions.is_empty());
        assert!(deserialized.header.payload_ids.is_empty());
        assert!(deserialized.is_valid());
    }

    /// Test round-trip serialization/deserialization for RoundWindow (JSON)
    #[test]
    fn test_round_window_json_roundtrip() {
        let window = RoundWindow {
            id: 100,
            start_us: IppanTimeMicros(1000000),
            end_us: IppanTimeMicros(2000000),
        };

        let json = serde_json::to_string(&window).expect("Failed to serialize RoundWindow");
        let deserialized: RoundWindow =
            serde_json::from_str(&json).expect("Failed to deserialize RoundWindow");

        assert_eq!(window.id, deserialized.id, "Round ID mismatch");
        assert_eq!(
            window.start_us, deserialized.start_us,
            "Start time mismatch"
        );
        assert_eq!(window.end_us, deserialized.end_us, "End time mismatch");
        assert_eq!(window, deserialized, "RoundWindow equality check failed");
    }

    /// Test round-trip serialization/deserialization for RoundCertificate (JSON)
    #[test]
    fn test_round_certificate_json_roundtrip() {
        let cert = RoundCertificate {
            round: 50,
            block_ids: vec![[1u8; 32], [2u8; 32], [3u8; 32]],
            agg_sig: vec![0xAAu8, 0xBB, 0xCC, 0xDD],
        };

        let json = serde_json::to_string(&cert).expect("Failed to serialize RoundCertificate");
        let deserialized: RoundCertificate =
            serde_json::from_str(&json).expect("Failed to deserialize RoundCertificate");

        assert_eq!(cert.round, deserialized.round, "Round ID mismatch");
        assert_eq!(cert.block_ids, deserialized.block_ids, "Block IDs mismatch");
        assert_eq!(
            cert.agg_sig, deserialized.agg_sig,
            "Aggregate signature mismatch"
        );
        assert_eq!(cert, deserialized, "RoundCertificate equality check failed");
    }

    /// Test round-trip serialization/deserialization for RoundFinalizationRecord (JSON)
    #[test]
    fn test_round_finalization_record_json_roundtrip() {
        let window = RoundWindow {
            id: 75,
            start_us: IppanTimeMicros(3000000),
            end_us: IppanTimeMicros(4000000),
        };

        let cert = RoundCertificate {
            round: 75,
            block_ids: vec![[10u8; 32], [11u8; 32]],
            agg_sig: vec![0x01u8, 0x02, 0x03],
        };

        let record = RoundFinalizationRecord {
            round: 75,
            window,
            ordered_tx_ids: vec![[20u8; 32], [21u8; 32], [22u8; 32]],
            fork_drops: vec![[30u8; 32]],
            state_root: [40u8; 32],
            proof: cert,
            total_fees_atomic: Some(1_234_567u128),
            treasury_fees_atomic: Some(987_654u128),
            applied_payments: Some(42),
            rejected_payments: Some(3),
        };

        let json =
            serde_json::to_string(&record).expect("Failed to serialize RoundFinalizationRecord");
        let deserialized: RoundFinalizationRecord =
            serde_json::from_str(&json).expect("Failed to deserialize RoundFinalizationRecord");

        assert_eq!(record.round, deserialized.round, "Round ID mismatch");
        assert_eq!(record.window, deserialized.window, "Window mismatch");
        assert_eq!(
            record.ordered_tx_ids, deserialized.ordered_tx_ids,
            "Ordered TX IDs mismatch"
        );
        assert_eq!(
            record.fork_drops, deserialized.fork_drops,
            "Fork drops mismatch"
        );
        assert_eq!(
            record.state_root, deserialized.state_root,
            "State root mismatch"
        );
        assert_eq!(record.proof, deserialized.proof, "Proof mismatch");
        assert_eq!(
            record, deserialized,
            "RoundFinalizationRecord equality check failed"
        );
    }

    /// Test that serialization is deterministic for the same Transaction
    #[test]
    fn test_transaction_serialization_determinism() {
        let secret = SigningKey::from_bytes(&[25u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [26u8; 32], Amount::from_atomic(8888), 123);
        tx.sign(&secret.to_bytes()).unwrap();

        // Serialize multiple times
        let json1 = serde_json::to_string(&tx).expect("Serialization 1 failed");
        let json2 = serde_json::to_string(&tx).expect("Serialization 2 failed");
        let json3 = serde_json::to_string(&tx).expect("Serialization 3 failed");

        // All should be identical
        assert_eq!(json1, json2, "Serialization is not deterministic");
        assert_eq!(json2, json3, "Serialization is not deterministic");
    }

    /// Test that serialization is deterministic for the same Block
    #[test]
    fn test_block_serialization_determinism() {
        let secret = SigningKey::from_bytes(&[27u8; 32]);
        let mut tx = Transaction::new(
            secret.verifying_key().to_bytes(),
            [28u8; 32],
            Amount::from_atomic(9999),
            456,
        );
        tx.sign(&secret.to_bytes()).unwrap();

        let block = Block::new(vec![[29u8; 32]], vec![tx], 88, [30u8; 32]);

        // Serialize multiple times
        let json1 = serde_json::to_string(&block).expect("Serialization 1 failed");
        let json2 = serde_json::to_string(&block).expect("Serialization 2 failed");
        let json3 = serde_json::to_string(&block).expect("Serialization 3 failed");

        // All should be identical
        assert_eq!(json1, json2, "Block serialization is not deterministic");
        assert_eq!(json2, json3, "Block serialization is not deterministic");
    }

    /// Test that HashTimer serialization maintains temporal consistency
    #[test]
    fn test_hashtimer_serialization_consistency() {
        // Create a transaction with a deterministic time
        let secret = SigningKey::from_bytes(&[31u8; 32]);
        let mut tx = Transaction::new(
            secret.verifying_key().to_bytes(),
            [32u8; 32],
            Amount::from_atomic(7777),
            789,
        );
        tx.sign(&secret.to_bytes()).unwrap();

        // Serialize and deserialize
        let json = serde_json::to_string(&tx).expect("Failed to serialize");
        let deserialized: Transaction = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify HashTimer consistency
        assert_eq!(
            tx.hashtimer.timestamp_us, deserialized.hashtimer.timestamp_us,
            "HashTimer timestamp mismatch"
        );
        assert_eq!(
            tx.hashtimer.entropy, deserialized.hashtimer.entropy,
            "HashTimer entropy mismatch"
        );
        assert_eq!(
            tx.hashtimer.to_hex(),
            deserialized.hashtimer.to_hex(),
            "HashTimer hex representation mismatch"
        );
    }

    /// Test serialization with all optional fields present
    #[test]
    fn test_block_with_all_optional_fields_json_roundtrip() {
        let secret = SigningKey::from_bytes(&[33u8; 32]);
        let mut tx = Transaction::new(
            secret.verifying_key().to_bytes(),
            [34u8; 32],
            Amount::from_atomic(5555),
            111,
        );
        tx.sign(&secret.to_bytes()).unwrap();

        let mut block = Block::new(vec![[35u8; 32], [36u8; 32]], vec![tx], 99, [37u8; 32]);

        // Set all optional fields
        block.set_data_availability_roots(
            Some("erasure_root_hex".to_string()),
            Some("receipt_root_hex".to_string()),
            Some("state_root_hex".to_string()),
        );
        block.push_validator_signature("sig1".to_string());
        block.push_validator_signature("sig2".to_string());
        block.header.vrf_proof = vec![0xAAu8, 0xBB, 0xCC];

        let json =
            serde_json::to_string(&block).expect("Failed to serialize block with optional fields");
        let deserialized: Block =
            serde_json::from_str(&json).expect("Failed to deserialize block with optional fields");

        assert_eq!(
            block.header.erasure_root, deserialized.header.erasure_root,
            "Erasure root mismatch"
        );
        assert_eq!(
            block.header.receipt_root, deserialized.header.receipt_root,
            "Receipt root mismatch"
        );
        assert_eq!(
            block.header.state_root, deserialized.header.state_root,
            "State root mismatch"
        );
        assert_eq!(
            block.header.validator_sigs, deserialized.header.validator_sigs,
            "Validator signatures mismatch"
        );
        assert_eq!(
            block.header.vrf_proof, deserialized.header.vrf_proof,
            "VRF proof mismatch"
        );
    }

    /// Test edge case: Transaction with maximum nonce value
    #[test]
    fn test_transaction_max_nonce_json_roundtrip() {
        let secret = SigningKey::from_bytes(&[38u8; 32]);
        let from = secret.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [39u8; 32], Amount::from_atomic(1234), u64::MAX);
        tx.sign(&secret.to_bytes()).unwrap();

        let json = serde_json::to_string(&tx).expect("Failed to serialize");
        let deserialized: Transaction = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(tx.nonce, deserialized.nonce);
        assert_eq!(tx.nonce, u64::MAX);
        assert!(deserialized.is_valid());
    }
}
