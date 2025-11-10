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

    fn create_signed_tx() -> Transaction {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let from = key.verifying_key().to_bytes();
        let mut tx = Transaction::new(from, [8u8; 32], Amount::from_atomic(1000), 1);
        tx.sign(&key.to_bytes()).unwrap();
        tx
    }

    fn create_block() -> Block {
        let tx = create_signed_tx();
        Block::new(vec![[1u8; 32]], vec![tx], 5, [9u8; 32])
    }

    #[test]
    fn test_transaction_json_roundtrip() {
        let tx = create_signed_tx();
        let json = serde_json::to_string(&tx).unwrap();
        let decoded: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx.id, decoded.id);
        assert_eq!(tx.hash(), decoded.hash());
        assert!(decoded.is_valid());
    }

    #[test]
    fn test_confidential_tx_roundtrip() {
        let mut tx = create_signed_tx();
        let envelope = ConfidentialEnvelope {
            enc_algo: "AES-256".into(),
            iv: "abcd".into(),
            ciphertext: "1234".into(),
            access_keys: vec![AccessKey {
                recipient_pub: "a".into(),
                enc_key: "b".into(),
            }],
        };
        tx.set_confidential_envelope(envelope);
        let json = serde_json::to_string(&tx).unwrap();
        let decoded: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx.confidential, decoded.confidential);
    }

    #[test]
    fn test_block_json_roundtrip() {
        let block = create_block();
        let json = serde_json::to_string(&block).unwrap();
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(block.header.id, decoded.header.id);
        assert_eq!(block.hash(), decoded.hash());
        assert!(decoded.is_valid());
    }

    #[test]
    fn test_deterministic_serialization() {
        let tx = create_signed_tx();
        let s1 = serde_json::to_string(&tx).unwrap();
        let s2 = serde_json::to_string(&tx).unwrap();
        assert_eq!(s1, s2);

        let block = create_block();
        let b1 = serde_json::to_string(&block).unwrap();
        let b2 = serde_json::to_string(&block).unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn test_round_structs_roundtrip() {
        let window = RoundWindow {
            id: 1,
            start_us: IppanTimeMicros(1000),
            end_us: IppanTimeMicros(2000),
        };
        let cert = RoundCertificate {
            round: 1,
            block_ids: vec![[1u8; 32], [2u8; 32]],
            agg_sig: vec![0xAA],
        };
        let record = RoundFinalizationRecord {
            round: 1,
            window,
            ordered_tx_ids: vec![[3u8; 32]],
            fork_drops: vec![],
            state_root: [4u8; 32],
            proof: cert.clone(),
        };

        let json = serde_json::to_string(&record).unwrap();
        let decoded: RoundFinalizationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, decoded);
    }
}
