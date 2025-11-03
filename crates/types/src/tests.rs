use crate::{
    AccessKey, Amount, Block, ConfidentialEnvelope, ConfidentialProof, ConfidentialProofType,
    HashTimer, IppanTimeMicros, Transaction,
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
